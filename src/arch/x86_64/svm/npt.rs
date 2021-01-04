use x86_64::structures::paging::page_table::PageTableEntry as PTE;

use crate::memory::{GuestPhysAddr, HostPhysAddr, Level4PageTable, PagingInstr};

pub struct NPTInstr;

impl PagingInstr for NPTInstr {
    unsafe fn activate(_root_paddr: HostPhysAddr) {}
    fn flush(_vaddr: Option<usize>) {}
}

pub type NestedPageTable = Level4PageTable<GuestPhysAddr, PTE, NPTInstr>;
