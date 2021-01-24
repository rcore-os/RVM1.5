use crate::arch::page_table::PTEntry;
use crate::memory::addr::{GuestPhysAddr, HostPhysAddr};
use crate::memory::{GenericPTE, Level4PageTable, MemFlags, PagingInstr};

#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct NPTEntry(PTEntry);

impl GenericPTE for NPTEntry {
    fn addr(&self) -> HostPhysAddr {
        self.0.addr()
    }
    fn flags(&self) -> MemFlags {
        self.0.flags()
    }
    fn is_unused(&self) -> bool {
        self.0.is_unused()
    }
    fn is_present(&self) -> bool {
        self.0.is_present()
    }
    fn is_huge(&self) -> bool {
        self.0.is_huge()
    }
    fn set_addr(&mut self, paddr: HostPhysAddr) {
        self.0.set_addr(paddr);
    }
    fn set_flags(&mut self, flags: MemFlags, is_huge: bool) {
        // See APMv2, Section 15.25.5:
        // A table walk for the guest page itself is always treated as a user
        // access at the nested page table level.
        self.0.set_flags(flags | MemFlags::USER, is_huge)
    }
    fn set_table(&mut self, paddr: HostPhysAddr) {
        self.0.set_table(paddr)
    }
    fn clear(&mut self) {
        self.0.clear()
    }
}

pub struct NPTInstr;

impl PagingInstr for NPTInstr {
    unsafe fn activate(_root_paddr: HostPhysAddr) {}
    fn flush(_vaddr: Option<usize>) {}
}

pub type NestedPageTable = Level4PageTable<GuestPhysAddr, NPTEntry, NPTInstr>;
