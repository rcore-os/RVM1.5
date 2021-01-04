use core::fmt::{Debug, Formatter, Result};

use libvmm::{msr::Msr, svm::SvmExitCode, svm::Vmcb};
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
use x86_64::registers::model_specific::{Efer, EferFlags};
use x86_64::registers::rflags::RFlags;

use crate::arch::vmm::VcpuAccessGuestState;
use crate::arch::{GuestPageTable, GuestRegisters, LinuxContext};
use crate::cell::Cell;
use crate::error::HvResult;
use crate::memory::{addr::virt_to_phys, Frame};
use crate::percpu::PerCpu;

#[repr(C)]
pub struct Vcpu {
    /// Save guest general registers when handle VM exits.
    guest_regs: GuestRegisters,
    /// RSP will be loaded from here when handle VM exits.
    host_stack_top: u64,
    /// host state-save area.
    host_save_area: Frame,
    /// Virtual machine control block.
    pub(super) vmcb: Vmcb,
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
            host_stack_top: PerCpu::from_local_base().stack_top() as _,
            vmcb: Default::default(),
        };
        assert_eq!(
            unsafe { (&ret.guest_regs as *const GuestRegisters).add(1) as u64 },
            &ret.host_stack_top as *const _ as u64
        );
        ret.vmcb_setup(linux, cell);

        Ok(ret)
    }

    pub fn exit(&self, linux: &mut LinuxContext) -> HvResult {
        self.load_vmcb_guest(linux);
        unsafe {
            asm!("stgi");
            Efer::write(Efer::read() - EferFlags::SECURE_VIRTUAL_MACHINE_ENABLE);
            Msr::VM_HSAVE_PA.write(0);
        }
        info!("successed to turn off SVM.");
        Ok(())
    }

    pub fn activate_vmm(&mut self, linux: &LinuxContext) -> HvResult {
        let regs = self.regs_mut();
        // regx.rax = VMCB paddr
        regs.rbx = linux.rbx;
        regs.rbp = linux.rbp;
        regs.r12 = linux.r12;
        regs.r13 = linux.r13;
        regs.r14 = linux.r14;
        regs.r15 = linux.r15;
        unsafe {
            asm!(
                "clgi",
                "mov rsp, {0}",
                restore_regs_from_stack!(),
                "vmload rax",
                "jmp {1}",
                in(reg) &self.guest_regs as * const _ as usize,
                sym svm_run,
                options(noreturn),
            );
        }
    }

    pub fn deactivate_vmm(&self, linux: &LinuxContext) -> HvResult {
        self.guest_regs.return_to_linux(linux)
    }

    pub fn inject_fault(&mut self) -> HvResult {
        todo!()
    }

    pub fn advance_rip(&mut self, instr_len: u8) -> HvResult {
        self.vmcb.save.rip += instr_len as u64;
        Ok(())
    }

    pub fn guest_is_privileged(&self) -> bool {
        self.vmcb.save.cpl == 0
    }

    pub fn in_hypercall(&self) -> bool {
        use core::convert::TryInto;
        matches!(
            self.vmcb.control.exit_code.try_into(),
            Ok(SvmExitCode::VMMCALL)
        )
    }

    pub fn guest_page_table(&self) -> GuestPageTable {
        use crate::memory::{addr::align_down, GenericPageTable};
        unsafe { GuestPageTable::from_root(align_down(self.vmcb.save.cr3 as _)) }
    }
}

impl Vcpu {
    fn vmcb_setup(&mut self, linux: &LinuxContext, cell: &Cell) {
        let vmcb_paddr = virt_to_phys(&self.vmcb as *const _ as usize);
        self.guest_regs.rax = vmcb_paddr as _;
    }

    fn load_vmcb_guest(&self, linux: &mut LinuxContext) {}
}

impl VcpuAccessGuestState for Vcpu {
    fn regs(&self) -> &GuestRegisters {
        &self.guest_regs
    }

    fn regs_mut(&mut self) -> &mut GuestRegisters {
        &mut self.guest_regs
    }

    fn instr_pointer(&self) -> u64 {
        self.vmcb.save.rip
    }

    fn stack_pointer(&self) -> u64 {
        self.vmcb.save.rsp
    }

    fn set_stack_pointer(&mut self, sp: u64) {
        self.vmcb.save.rsp = sp
    }

    fn rflags(&self) -> u64 {
        self.vmcb.save.rflags
    }

    fn cr(&self, cr_idx: usize) -> u64 {
        match cr_idx {
            0 => self.vmcb.save.cr0,
            3 => self.vmcb.save.cr3,
            4 => self.vmcb.save.cr4,
            _ => unreachable!(),
        }
    }

    fn set_cr(&mut self, cr_idx: usize, val: u64) {
        match cr_idx {
            0 => self.vmcb.save.cr0 = val,
            3 => self.vmcb.save.cr3 = val,
            4 => self.vmcb.save.cr4 = val,
            _ => unreachable!(),
        }
    }
}

impl Debug for Vcpu {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("Vcpu")
            .field("guest_regs", &self.guest_regs)
            .field("rip", &self.instr_pointer())
            .field("rsp", &self.stack_pointer())
            .field("rflags", unsafe {
                &RFlags::from_bits_unchecked(self.rflags())
            })
            .field("cr0", unsafe { &Cr0Flags::from_bits_unchecked(self.cr(0)) })
            .field("cr3", &self.cr(3))
            .field("cr4", unsafe { &Cr4Flags::from_bits_unchecked(self.cr(4)) })
            .field("cs", &self.vmcb.save.cs)
            .finish()
    }
}

#[naked]
unsafe extern "sysv64" fn svm_run() -> ! {
    asm!(
        "vmrun rax",
        save_regs_to_stack!(),
        "mov r14, rax",         // save host RAX to r14 for VMRUN
        "mov r15, rsp",         // save temporary RSP to r15
        "mov rsp, [rsp + {0}]", // set RSP to Vcpu::host_stack_top
        "call {1}",
        "mov rsp, r15",         // load temporary RSP from r15
        "push r14",             // push saved RAX to restore RAX later
        restore_regs_from_stack!(),
        "jmp {2}",
        const core::mem::size_of::<GuestRegisters>(),
        sym crate::arch::vmm::vmexit_handler,
        sym svm_run,
        options(noreturn),
    );
}
