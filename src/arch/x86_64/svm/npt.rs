use x86_64::structures::paging::page_table::PageTableEntry as PTE;

use crate::memory::{Level4PageTable, PagingInstr, PhysAddr, VirtAddr};

pub struct NPTInstr;

impl PagingInstr for NPTInstr {
    unsafe fn activate(root_paddr: PhysAddr) {
        todo!()
    }

    fn flush(vaddr: Option<usize>) {
        todo!()
    }
}

pub type NestedPageTable = Level4PageTable<VirtAddr, PTE, NPTInstr>;
