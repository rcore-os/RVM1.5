use spin::RwLock;

use crate::arch::{HostPageTable, HvPageTable};
use crate::config::{CellConfig, HvSystemConfig};
use crate::consts::{HV_BASE, PER_CPU_SIZE};
use crate::error::HvResult;
use crate::header::HvHeader;
use crate::memory::{
    addr::phys_to_virt, GuestPhysAddr, HostPhysAddr, MemFlags, MemoryRegion, MemorySet,
};

#[derive(Debug)]
pub struct Cell<'a> {
    /// Cell configuration.
    pub config: CellConfig<'a>,
    /// Guest physical memory set.
    pub gpm: RwLock<MemorySet<HvPageTable>>,
    /// Host virtual memory set.
    pub hvm: RwLock<MemorySet<HostPageTable>>,
}

impl Cell<'_> {
    pub fn new_root() -> HvResult<Self> {
        let header = HvHeader::get();
        let sys_config = HvSystemConfig::get();
        let cell_config = sys_config.root_cell.config();

        let hv_phys_start = sys_config.hypervisor_memory.phys_start as usize;
        let hv_phys_size = sys_config.hypervisor_memory.size as usize;
        let mut gpm = MemorySet::new();
        let mut hvm = MemorySet::new();

        // Init guest physical memory set, create hypervisor page table.
        //
        // hypervisor
        gpm.insert(MemoryRegion::new_with_empty_mapper(
            hv_phys_start,
            hv_phys_size,
            MemFlags::READ | MemFlags::NO_HUGEPAGES,
        ))?;
        // all physical memory regions
        for region in cell_config.mem_regions() {
            gpm.insert(MemoryRegion::new_with_offset_mapper(
                region.virt_start as GuestPhysAddr,
                region.phys_start as HostPhysAddr,
                region.size as usize,
                region.flags,
            ))?;
        }

        // Init host virtual memory set, create host page table.
        let core_and_percpu_size =
            header.core_size as usize + header.max_cpus as usize * PER_CPU_SIZE;
        // hypervisor core
        hvm.insert(MemoryRegion::new_with_offset_mapper(
            HV_BASE,
            hv_phys_start,
            header.core_size,
            MemFlags::READ | MemFlags::WRITE | MemFlags::EXECUTE,
        ))?;
        // configurations & hypervisor free memory
        hvm.insert(MemoryRegion::new_with_offset_mapper(
            HV_BASE + core_and_percpu_size,
            hv_phys_start + core_and_percpu_size,
            hv_phys_size - core_and_percpu_size,
            MemFlags::READ | MemFlags::WRITE,
        ))?;
        // guest RAM
        for region in cell_config.mem_regions() {
            if region.flags.contains(MemFlags::EXECUTE) {
                hvm.insert(MemoryRegion::new_with_offset_mapper(
                    phys_to_virt(region.virt_start as GuestPhysAddr),
                    region.phys_start as HostPhysAddr,
                    region.size as usize,
                    region.flags,
                ))?;
            }
        }

        Ok(Self {
            config: cell_config,
            gpm: RwLock::new(gpm),
            hvm: RwLock::new(hvm),
        })
    }
}

lazy_static! {
    pub static ref ROOT_CELL: Cell<'static> = Cell::new_root().unwrap();
}

pub fn init() -> HvResult {
    crate::arch::check_hypervisor_feature()?;

    lazy_static::initialize(&ROOT_CELL);

    info!("Root cell init end.");
    debug!("{:#x?}", &*ROOT_CELL);
    Ok(())
}
