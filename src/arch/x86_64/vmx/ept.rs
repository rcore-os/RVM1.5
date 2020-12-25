use core::{convert::TryFrom, fmt};

use bit_field::BitField;
use bitflags::bitflags;
use numeric_enum_macro::numeric_enum;

use crate::memory::addr::{GuestPhysAddr, HostPhysAddr};
use crate::memory::{GenericPTE, Level4PageTable, MemFlags, PagingInstr};

bitflags! {
    struct EPTFlags: u64 {
        /// Read access.
        const READ =                1 << 0;
        /// Write access.
        const WRITE =               1 << 1;
        /// execute access.
        const EXECUTE =             1 << 2;
        /// Ignore PAT memory type
        const IGNORE_PAT =          1 << 6;
        /// Specifies that the entry maps a huge frame instead of a page table. Only allowed in
        /// P2 or P3 tables.
        const HUGE_PAGE =           1 << 7;
        /// If bit 6 of EPTP is 1, accessed flag for EPT.
        const ACCESSED =            1 << 8;
        /// If bit 6 of EPTP is 1, dirty flag for EPT;
        const DIRTY =               1 << 9;
        /// Execute access for user-mode linear addresses.
        const EXECUTE_FOR_USER =    1 << 10;
    }
}

numeric_enum! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    enum EPTMemType {
        Uncached = 0,
        WriteCombining = 1,
        WriteThrough = 4,
        WriteProtected = 5,
        WriteBack = 6,
    }
}

#[derive(Clone)]
pub struct EPTEntry(u64);

impl From<MemFlags> for EPTFlags {
    fn from(f: MemFlags) -> Self {
        if f.is_empty() {
            return Self::empty();
        }
        let mut ret = Self::empty();
        if f.contains(MemFlags::READ) {
            ret |= Self::READ;
        }
        if f.contains(MemFlags::WRITE) {
            ret |= Self::WRITE;
        }
        if f.contains(MemFlags::EXECUTE) {
            ret |= Self::EXECUTE;
        }
        ret
    }
}

impl From<EPTFlags> for MemFlags {
    fn from(f: EPTFlags) -> Self {
        let mut ret = MemFlags::empty();
        if f.contains(EPTFlags::READ) {
            ret |= Self::READ;
        }
        if f.contains(EPTFlags::WRITE) {
            ret |= Self::WRITE;
        }
        if f.contains(EPTFlags::EXECUTE) {
            ret |= Self::EXECUTE;
        }
        ret
    }
}

impl EPTMemType {
    fn empty() -> Self {
        Self::try_from(0).unwrap()
    }
}

impl GenericPTE for EPTEntry {
    fn addr(&self) -> HostPhysAddr {
        (self.0.get_bits(12..52) << 12) as usize
    }
    fn flags(&self) -> MemFlags {
        self.ept_flags().into()
    }
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    fn is_present(&self) -> bool {
        self.0.get_bits(0..3) != 0
    }
    fn is_huge(&self) -> bool {
        self.ept_flags().contains(EPTFlags::HUGE_PAGE)
    }

    fn set_addr(&mut self, paddr: HostPhysAddr) {
        self.0.set_bits(12..52, paddr as u64 >> 12);
    }
    fn set_flags(&mut self, flags: MemFlags, is_huge: bool) {
        let mut flags = flags.into();
        if is_huge {
            flags |= EPTFlags::HUGE_PAGE;
        }
        self.set_flags_and_mem_type(flags, EPTMemType::WriteBack);
    }
    fn set_table(&mut self, paddr: HostPhysAddr) {
        self.set_addr(paddr);
        self.set_flags_and_mem_type(
            EPTFlags::READ | EPTFlags::WRITE | EPTFlags::EXECUTE,
            EPTMemType::empty(),
        );
    }
    fn clear(&mut self) {
        self.0 = 0
    }
}

impl EPTEntry {
    fn ept_flags(&self) -> EPTFlags {
        EPTFlags::from_bits_truncate(self.0)
    }
    fn memory_type(&self) -> Result<EPTMemType, u8> {
        EPTMemType::try_from(self.0.get_bits(3..6) as u8)
    }
    fn set_flags_and_mem_type(&mut self, flags: EPTFlags, mem_type: EPTMemType) {
        self.0.set_bits(0..12, flags.bits());
        self.0.set_bits(3..6, mem_type as u64);
    }
}

impl fmt::Debug for EPTEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EPTEntry")
            .field("hpaddr", &self.addr())
            .field("flags", &self.ept_flags())
            .field("memory_type", &self.memory_type())
            .finish()
    }
}

pub struct EPTInstr;

impl PagingInstr for EPTInstr {
    unsafe fn activate(root_paddr: HostPhysAddr) {
        libvmm::vmx::Vmcs::set_ept_pointer(root_paddr).expect("Failed to set EPT_POINTER");
    }

    fn flush(_vaddr: Option<usize>) {
        // do nothing
    }
}

pub type ExtendedPageTable = Level4PageTable<GuestPhysAddr, EPTEntry, EPTInstr>;
