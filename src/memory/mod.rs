//! Hypervisor Memory Layout
//!
//!     +--------------------------------------+ - lower address (HV_BASE: 0xffff_ff00_0000_0000)
//!     | HvHeader                             |
//!     +--------------------------------------+
//!     | BSS Segment                          |
//!     | (includes hypervisor heap)           |
//!     |                                      |
//!     +--------------------------------------+
//!     | Text Segment                         |
//!     |                                      |
//!     +--------------------------------------+
//!     | Read-only Data Segment               |
//!     |                                      |
//!     +--------------------------------------+
//!     | Data Segment                         |
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

pub mod addr;
mod frame;
pub mod gaccess;
mod heap;
mod mapper;
mod mm;
mod paging;

use core::ops::{Deref, DerefMut};

use bitflags::bitflags;

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

pub fn init() {
    heap::init();
    frame::init();
}
