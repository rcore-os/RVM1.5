use core::fmt::{Debug, Formatter, Result};

use libvmm::{msr::Msr, svm::Vmcb};
use x86_64::registers::control::{Cr0, Cr4};
use x86_64::registers::model_specific::{Efer, EferFlags};

use crate::arch::vmm::VcpuAccessGuestState;
use crate::arch::{GuestPageTable, GuestRegisters, LinuxContext};
use crate::cell::Cell;
use crate::error::HvResult;
use crate::memory::Frame;

pub struct Vcpu {
    /// Save guest general registers when VM exits.
    guest_regs: GuestRegisters,
    /// host state-save area.
    host_save_area: Frame,
    /// Virtual machine control block.
    vmcb: Vmcb,
}

impl Vcpu {
    pub fn new(linux: &LinuxContext, cell: &Cell) -> HvResult<Self> {
        super::check_hypervisor_feature()?;

        // TODO: check linux CR0, CR4

        let efer = Efer::read();
        if efer.contains(EferFlags::SECURE_VIRTUAL_MACHINE_ENABLE) {
            return hv_result_err!(EBUSY, "SVM is already turned on!");
        }
        let host_save_area = Frame::new()?;
        unsafe { Efer::write(efer | EferFlags::SECURE_VIRTUAL_MACHINE_ENABLE) };
        unsafe { Msr::VM_HSAVE_PA.write(host_save_area.start_paddr() as _) };
        info!("successed to turn on SVM.");

        // bring CR0 and CR4 into well-defined states.
        unsafe {
            Cr0::write(super::super::HOST_CR0);
            Cr4::write(super::super::HOST_CR4);
        }

        let mut ret = Self {
            guest_regs: Default::default(),
            host_save_area,
            vmcb: Default::default(),
        };
        ret.vmcb_setup(linux, cell)?;

        Ok(ret)
    }

    pub fn exit(&self, linux: &mut LinuxContext) -> HvResult {
        self.load_vmcb_guest(linux)?;
        unsafe { Efer::write(Efer::read() - EferFlags::SECURE_VIRTUAL_MACHINE_ENABLE) };
        unsafe { Msr::VM_HSAVE_PA.write(0) };
        info!("successed to turn off SVM.");
        Ok(())
    }

    pub fn activate_vmm(&self, linux: &LinuxContext) -> HvResult {
        todo!()
    }

    pub fn deactivate_vmm(&self, linux: &LinuxContext) -> HvResult {
        self.guest_regs.return_to_linux(linux)
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

impl Vcpu {
    fn vmcb_setup(&mut self, linux: &LinuxContext, cell: &Cell) -> HvResult {
        Ok(())
    }

    fn load_vmcb_guest(&self, linux: &mut LinuxContext) -> HvResult {
        Ok(())
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
