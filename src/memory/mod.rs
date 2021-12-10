//! Hypervisor Memory Layout
//!
//!     +--------------------------------------+ - lower address (HV_BASE: 0xffff_ff00_0000_0000)
//!     | HvHeader                             |
//!     +--------------------------------------+
//!     | Text Segment                         |
//!     |                                      |
//!     +--------------------------------------+
//!     | Read-only Data Segment               |
//!     |                                      |
//!     +--------------------------------------+
//!     | Data Segment                         |
//!     |                                      |
//!     +--------------------------------------+
//!     | BSS Segment                          |
//!     | (includes hypervisor heap)           |
//!     |                                      |
//!     +--------------------------------------+ - core_end (HV_BASE + core_size)
//!     |  +--------------------------------+  |
//!     |  | Per-CPU Data 0                 |  |
//!     |  |                                |  |
//!     |  +--------------------------------+  |
//!     |  | Per-CPU Data 1                 |  |
//!     |  |                                |  |
//!     |  +--------------------------------+  |
//!     :  :                                :  :
//!     :  :                                :  :
//!     |  +--------------------------------+  |
//!     |  | Per-CPU Data n-1               |  |
//!     |  |                                |  |
//!     |  +--------------------------------+  |
//!     |  | HvSystemConfig                 |  |
//!     |  | +----------------------------+ |  |
//!     |  | | CellConfigLayout           | |  |
//!     |  | |                            | |  |
//!     |  | +----------------------------+ |  |
//!     |  +--------------------------------+  |
//!     +--------------------------------------|
//!     |  Dynamic Page Pool                   |
//!     :                                      :
//!     :                                      :
//!     |                                      |
//!     +--------------------------------------+ - higher address (HV_BASE + sys_config.hypervisor_memory.size)
//!

mod frame;
mod heap;
mod mapper;
mod mm;
mod paging;

pub mod addr;
pub mod gaccess;

use core::ops::{Deref, DerefMut};

use bitflags::bitflags;

use crate::arch::HostPageTable;
use crate::config::HvSystemConfig;
use crate::consts::HV_BASE;
use crate::error::HvResult;
use crate::header::HvHeader;

pub use addr::{GuestPhysAddr, GuestVirtAddr, HostPhysAddr, HostVirtAddr, PhysAddr, VirtAddr};
pub use frame::Frame;
pub use mm::{MemoryRegion, MemorySet};
pub use paging::{GenericPTE, PagingInstr};
pub use paging::{GenericPageTable, GenericPageTableImmut, Level4PageTable, Level4PageTableImmut};

pub const PAGE_SIZE: usize = paging::PageSize::Size4K as usize;

bitflags! {
    pub struct MemFlags: u64 {
        const READ          = 1 << 0;
        const WRITE         = 1 << 1;
        const EXECUTE       = 1 << 2;
        const DMA           = 1 << 3;
        const IO            = 1 << 4;
        const NO_HUGEPAGES  = 1 << 8;
        const USER          = 1 << 9;
    }
}

/// Page table used for hypervisor.
static HV_PT: spin::Once<MemorySet<HostPageTable>> = spin::Once::new();

pub fn hv_page_table<'a>() -> &'a MemorySet<HostPageTable> {
    HV_PT.get().expect("Uninitialized hypervisor page table!")
}

pub fn init_heap() {
    // Set PHYS_VIRT_OFFSET early.
    unsafe {
        addr::PHYS_VIRT_OFFSET =
            HV_BASE - HvSystemConfig::get().hypervisor_memory.phys_start as usize
    };
    heap::init();
}

pub fn init_frame_allocator() {
    frame::init();
}

pub fn init_hv_page_table() -> HvResult {
    let header = HvHeader::get();
    let sys_config = HvSystemConfig::get();
    let cell_config = sys_config.root_cell.config();
    let hv_phys_start = sys_config.hypervisor_memory.phys_start as usize;
    let hv_phys_size = sys_config.hypervisor_memory.size as usize;

    let mut hv_pt = MemorySet::new();

    // Map hypervisor memory.
    // TODO: Fine-grained permissions setting
    hv_pt.insert(MemoryRegion::new_with_offset_mapper(
        HV_BASE,
        hv_phys_start,
        header.core_size,
        MemFlags::READ | MemFlags::WRITE | MemFlags::EXECUTE,
    ))?;
    // Map per-CPU data, configurations & page pool.
    hv_pt.insert(MemoryRegion::new_with_offset_mapper(
        HV_BASE + header.core_size,
        hv_phys_start + header.core_size,
        hv_phys_size - header.core_size,
        MemFlags::READ | MemFlags::WRITE,
    ))?;

    // Map all guest RAM to directly access in hypervisor.
    for region in cell_config.mem_regions() {
        if region.flags.contains(MemFlags::DMA) {
            let hv_virt_start = addr::phys_to_virt(region.virt_start as GuestPhysAddr);
            if hv_virt_start < region.virt_start as GuestPhysAddr {
                return hv_result_err!(
                    EINVAL,
                    format!(
                        "Guest physical address {:#x} is too large",
                        region.virt_start
                    )
                );
            }
            hv_pt.insert(MemoryRegion::new_with_offset_mapper(
                hv_virt_start,
                region.phys_start as HostPhysAddr,
                region.size as usize,
                MemFlags::READ | MemFlags::WRITE,
            ))?;
        }
    }
    info!("Hypervisor page table init end.");
    debug!("Hypervisor virtual memory set: {:#x?}", hv_pt);

    HV_PT.call_once(|| hv_pt);
    Ok(())
}

#[repr(align(4096))]
pub struct AlignedPage([u8; PAGE_SIZE]);

impl AlignedPage {
    pub const fn new() -> Self {
        Self([0; PAGE_SIZE])
    }
}

impl Deref for AlignedPage {
    type Target = [u8; PAGE_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AlignedPage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
