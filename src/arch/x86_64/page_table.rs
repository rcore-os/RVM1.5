use x86_64::{
    addr::{PhysAddr as X86PhysAddr, VirtAddr as X86VirtAddr},
    instructions::tlb,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::page_table::{PageTableEntry as PTE, PageTableFlags as PTF},
    structures::paging::PhysFrame,
};

use crate::memory::{GenericPTE, Level4PageTable, MemFlags, PagingInstr, PhysAddr, VirtAddr};

impl From<MemFlags> for PTF {
    fn from(f: MemFlags) -> Self {
        if f.is_empty() {
            return Self::empty();
        }
        let mut ret = Self::PRESENT;
        if f.contains(MemFlags::WRITE) {
            ret |= Self::WRITABLE;
        }
        if !f.contains(MemFlags::EXECUTE) {
            ret |= Self::NO_EXECUTE;
        }
        if f.contains(MemFlags::USER) {
            ret |= Self::USER_ACCESSIBLE;
        }
        ret
    }
}

impl From<PTF> for MemFlags {
    fn from(f: PTF) -> Self {
        if f.is_empty() {
            return Self::empty();
        }
        let mut ret = Self::READ;
        if f.contains(PTF::WRITABLE) {
            ret |= Self::WRITE;
        }
        if !f.contains(PTF::NO_EXECUTE) {
            ret |= Self::EXECUTE;
        }
        if f.contains(PTF::USER_ACCESSIBLE) {
            ret |= Self::USER;
        }
        ret
    }
}

#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct PTEntry(PTE);

impl GenericPTE for PTEntry {
    fn addr(&self) -> PhysAddr {
        self.0.addr().as_u64() as _
    }
    fn flags(&self) -> MemFlags {
        self.0.flags().into()
    }
    fn is_unused(&self) -> bool {
        self.0.is_unused()
    }
    fn is_present(&self) -> bool {
        self.0.flags().contains(PTF::PRESENT)
    }
    fn is_huge(&self) -> bool {
        self.0.flags().contains(PTF::HUGE_PAGE)
    }

    fn set_addr(&mut self, paddr: PhysAddr) {
        self.0
            .set_addr(X86PhysAddr::new(paddr as _), self.0.flags())
    }
    fn set_flags(&mut self, flags: MemFlags, is_huge: bool) {
        let mut flags = flags.into();
        if is_huge {
            flags |= PTF::HUGE_PAGE;
        }
        self.0.set_flags(flags)
    }
    fn set_table(&mut self, paddr: PhysAddr) {
        self.0.set_addr(
            X86PhysAddr::new(paddr as _),
            PTF::PRESENT | PTF::WRITABLE | PTF::USER_ACCESSIBLE,
        )
    }
    fn clear(&mut self) {
        self.0.set_unused()
    }
}

pub struct X86PagingInstr;

impl PagingInstr for X86PagingInstr {
    unsafe fn activate(root_paddr: PhysAddr) {
        Cr3::write(
            PhysFrame::containing_address(X86PhysAddr::new(root_paddr as u64)),
            Cr3Flags::empty(),
        );
    }

    fn flush(vaddr: Option<usize>) {
        if let Some(vaddr) = vaddr {
            tlb::flush(X86VirtAddr::new(vaddr as u64))
        } else {
            tlb::flush_all()
        }
    }
}

pub type PageTable = Level4PageTable<VirtAddr, PTEntry, X86PagingInstr>;
