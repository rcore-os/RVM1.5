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

impl GenericPTE for PTE {
    fn addr(&self) -> PhysAddr {
        PTE::addr(self).as_u64() as _
    }
    fn flags(&self) -> MemFlags {
        PTE::flags(self).into()
    }
    fn is_unused(&self) -> bool {
        PTE::is_unused(self)
    }
    fn is_present(&self) -> bool {
        self.flags().contains(PTF::PRESENT)
    }
    fn is_huge(&self) -> bool {
        self.flags().contains(PTF::HUGE_PAGE)
    }

    fn set_addr(&mut self, paddr: PhysAddr) {
        PTE::set_addr(self, X86PhysAddr::new(paddr as _), self.flags())
    }
    fn set_flags(&mut self, flags: MemFlags, is_huge: bool) {
        let mut flags: PTF = flags.into();
        if is_huge {
            flags |= PTF::HUGE_PAGE;
        }
        PTE::set_flags(self, flags | PTF::USER_ACCESSIBLE) // FIXME: hack for SVM NPT
    }
    fn set_table(&mut self, paddr: PhysAddr) {
        PTE::set_addr(
            self,
            X86PhysAddr::new(paddr as _),
            PTF::PRESENT | PTF::WRITABLE | PTF::USER_ACCESSIBLE,
        )
    }
    fn clear(&mut self) {
        self.set_unused()
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

pub type PageTable = Level4PageTable<VirtAddr, PTE, X86PagingInstr>;
