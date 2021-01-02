use core::fmt::{Debug, Formatter, Result};

use libvmm::svm::Vmcb;

use crate::arch::vmm::VcpuAccessGuestState;
use crate::arch::{GuestPageTable, GuestRegisters, LinuxContext};
use crate::cell::Cell;
use crate::error::HvResult;
use crate::memory::Frame;

pub struct Vcpu {
    /// Save guest general registers when VM exits.
    guest_regs: GuestRegisters,
    /// host state-save area.
    save_area: Frame,
    /// Virtual machine control block.
    vmvb: Vmcb,
}

impl Vcpu {
    pub fn new(linux: &LinuxContext, cell: &Cell) -> HvResult<Self> {
        todo!()
    }

    pub fn exit(&self, linux: &mut LinuxContext) -> HvResult {
        todo!()
    }

    pub fn activate_vmm(&self, linux: &LinuxContext) -> HvResult {
        todo!()
    }

    pub fn deactivate_vmm(&self, linux: &LinuxContext) -> HvResult {
        todo!()
    }

    pub fn inject_fault(&mut self) -> HvResult {
        todo!()
    }

    pub fn advance_rip(&mut self, instr_len: u8) -> HvResult {
        todo!()
    }

    pub fn guest_is_privileged(&self) -> HvResult<bool> {
        todo!()
    }

    pub fn in_hypercall(&self) -> bool {
        todo!()
    }

    pub fn guest_page_table(&self) -> GuestPageTable {
        todo!()
    }
}

impl VcpuAccessGuestState for Vcpu {
    fn regs(&self) -> &GuestRegisters {
        todo!()
    }

    fn regs_mut(&mut self) -> &mut GuestRegisters {
        todo!()
    }

    fn instr_pointer(&self) -> u64 {
        todo!()
    }

    fn stack_pointer(&self) -> u64 {
        todo!()
    }

    fn set_stack_pointer(&mut self, sp: u64) {
        todo!()
    }

    fn rflags(&self) -> u64 {
        todo!()
    }

    fn cr(&self, cr_idx: usize) -> u64 {
        todo!()
    }

    fn set_cr(&mut self, cr_idx: usize, val: u64) {
        todo!()
    }
}

impl Debug for Vcpu {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("Vcpu")
            .field("guest_regs", &self.guest_regs)
            .finish()
    }
}
