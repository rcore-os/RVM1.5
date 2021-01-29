use core::fmt::{Debug, Formatter, Result};

use x86_64::{
    addr::{PhysAddr as X86PhysAddr, VirtAddr as X86VirtAddr},
    instructions::tlb,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::page_table::PageTableFlags as PTF,
    structures::paging::PhysFrame,
};

use crate::memory::{GenericPTE, MemFlags, PagingInstr, PhysAddr, VirtAddr};
use crate::memory::{Level4PageTable, Level4PageTableImmut};

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

const PHYS_ADDR_MASK: u64 = 0x000f_ffff_ffff_f000; // 12..52

#[derive(Clone)]
pub struct PTEntry(u64);

impl GenericPTE for PTEntry {
    fn addr(&self) -> PhysAddr {
        (self.0 & PHYS_ADDR_MASK) as _
    }
    fn flags(&self) -> MemFlags {
        PTF::from_bits_truncate(self.0).into()
    }
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    fn is_present(&self) -> bool {
        PTF::from_bits_truncate(self.0).contains(PTF::PRESENT)
    }
    fn is_huge(&self) -> bool {
        PTF::from_bits_truncate(self.0).contains(PTF::HUGE_PAGE)
    }

    fn set_addr(&mut self, paddr: PhysAddr) {
        self.0 = (self.0 & !PHYS_ADDR_MASK) | (paddr as u64 & PHYS_ADDR_MASK);
    }
    fn set_flags(&mut self, flags: MemFlags, is_huge: bool) {
        let mut flags: PTF = flags.into();
        if is_huge {
            flags |= PTF::HUGE_PAGE;
        }
        self.0 = self.addr() as u64 | flags.bits();
    }
    fn set_table(&mut self, paddr: PhysAddr) {
        self.0 = (paddr as u64 & PHYS_ADDR_MASK)
            | (PTF::PRESENT | PTF::WRITABLE | PTF::USER_ACCESSIBLE).bits();
    }
    fn clear(&mut self) {
        self.0 = 0
    }
}

impl Debug for PTEntry {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut f = f.debug_struct("PTEntry");
        f.field("raw", &self.0);
        f.field("addr", &self.addr());
        f.field("flags", &self.flags());
        f.finish()
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
pub type PageTableImmut = Level4PageTableImmut<VirtAddr, PTEntry>;
