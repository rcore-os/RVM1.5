use crate::arch::NestedPageTable;
use crate::config::{CellConfig, HvSystemConfig};
use crate::error::HvResult;
use crate::memory::addr::{GuestPhysAddr, HostPhysAddr};
use crate::memory::{MemFlags, MemoryRegion, MemorySet};

#[derive(Debug)]
pub struct Cell<'a> {
    /// Cell configuration.
    pub config: CellConfig<'a>,
    /// Guest physical memory set.
    pub gpm: MemorySet<NestedPageTable>,
}

impl Cell<'_> {
    fn new_root() -> HvResult<Self> {
        let sys_config = HvSystemConfig::get();
        let cell_config = sys_config.root_cell.config();
        let hv_phys_start = sys_config.hypervisor_memory.phys_start as usize;
        let hv_phys_size = sys_config.hypervisor_memory.size as usize;

        let mut gpm = MemorySet::new();

        // Map hypervisor memory to the empty page.
        gpm.insert(MemoryRegion::new_with_empty_mapper(
            hv_phys_start,
            hv_phys_size,
            MemFlags::READ | MemFlags::NO_HUGEPAGES,
        ))?;
        // Map all physical memory regions.
        for region in cell_config.mem_regions() {
            gpm.insert(MemoryRegion::new_with_offset_mapper(
                region.virt_start as GuestPhysAddr,
                region.phys_start as HostPhysAddr,
                region.size as usize,
                region.flags,
            ))?;
        }
        trace!("Guest phyiscal memory set: {:#x?}", gpm);

        Ok(Self {
            config: cell_config,
            gpm,
        })
    }
}

static ROOT_CELL: spin::Once<Cell> = spin::Once::new();

pub fn root_cell<'a>() -> &'a Cell<'a> {
    ROOT_CELL.get().expect("Uninitialized root cell!")
}

pub fn init() -> HvResult {
    crate::arch::vmm::check_hypervisor_feature()?;

    let root_cell = Cell::new_root()?;
    info!("Root cell init end.");
    debug!("{:#x?}", root_cell);

    ROOT_CELL.call_once(|| root_cell);
    Ok(())
}
