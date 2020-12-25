use libvmm::msr::Msr;
use libvmm::vmx::{
    self,
    flags::{FeatureControl, FeatureControlFlags, VmxBasic},
    vmcs::{VmcsField16Guest, VmcsField32Guest, VmcsField64Guest},
    vmcs::{VmcsField16Host, VmcsField32Host, VmcsField64Host},
    vmcs::{VmcsField32Control, VmcsField64Control},
    Vmcs, VmxExitReason,
};
use x86::segmentation::SegmentSelector;
use x86_64::registers::control::{Cr0, Cr0Flags, Cr3, Cr4, Cr4Flags};

use super::super::cpuid::CpuFeatures;
use super::super::segmentation::{Segment, SegmentAccessRights};
use super::super::tables::{GDTStruct, GDT, IDT};
use super::super::{GuestPageTable, GuestRegisters, LinuxContext};
use super::structs::{MsrBitmap, VmxRegion};
use crate::cell::Cell;
use crate::error::HvResult;
use crate::memory::addr::align_down;

pub struct Vcpu {
    /// VMXON region, required by VMX
    _vmxon_region: VmxRegion,
    /// VMCS of this CPU, required by VMX
    vmcs_region: VmxRegion,
}

lazy_static! {
    static ref MSR_BITMAP: MsrBitmap = MsrBitmap::default();
}

macro_rules! set_guest_segment {
    ($seg: expr, $reg: ident) => {{
        use VmcsField16Guest::*;
        use VmcsField32Guest::*;
        use VmcsField64Guest::*;
        concat_idents!($reg, _SELECTOR).write($seg.selector.bits())?;
        concat_idents!($reg, _BASE).write($seg.base)?;
        concat_idents!($reg, _LIMIT).write($seg.limit)?;
        concat_idents!($reg, _AR_BYTES).write($seg.access_rights.bits())?;
    }};
}

impl Vcpu {
    pub fn new(linux: &LinuxContext, cell: &Cell) -> HvResult<Self> {
        super::check_hypervisor_feature()?;

        // make sure all perf counters are off
        if CpuFeatures::new().perf_monitor_version_id() > 0 {
            unsafe { Msr::IA32_PERF_GLOBAL_CTRL.write(0) };
        }

        // Check control registers.
        let _cr0 = linux.cr0;
        let cr4 = linux.cr4;
        // TODO: check reserved bits
        if cr4.contains(Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS) {
            return hv_result_err!(EIO, "VMX is already turned on!");
        }

        // Enable VMXON, if required.
        let ctrl = FeatureControl::read();
        let locked = ctrl.contains(FeatureControlFlags::LOCKED);
        let vmxon_outside = ctrl.contains(FeatureControlFlags::VMXON_ENABLED_OUTSIDE_SMX);
        if !locked {
            FeatureControl::write(
                ctrl | FeatureControlFlags::LOCKED | FeatureControlFlags::VMXON_ENABLED_OUTSIDE_SMX,
            );
        } else if !vmxon_outside {
            return hv_result_err!(ENODEV, "VMX disabled by BIOS!");
        }

        // Init VMX regions.
        let vmx_basic = VmxBasic::read();
        let vmxon_region = VmxRegion::new(vmx_basic.revision_id, false)?;
        let vmcs_region = VmxRegion::new(vmx_basic.revision_id, false)?;

        let cr0 = Cr0Flags::PAGING
            | Cr0Flags::WRITE_PROTECT
            | Cr0Flags::NUMERIC_ERROR
            | Cr0Flags::TASK_SWITCHED
            | Cr0Flags::MONITOR_COPROCESSOR
            | Cr0Flags::PROTECTED_MODE_ENABLE;
        let mut cr4 = Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS | Cr4Flags::PHYSICAL_ADDRESS_EXTENSION;
        if CpuFeatures::new().has_xsave() {
            cr4 |= Cr4Flags::OSXSAVE;
        }

        unsafe {
            // Update control registers.
            Cr0::write(cr0);
            Cr4::write(cr4);

            // Execute VMXON.
            vmx::vmxon(vmxon_region.paddr() as _)?;
        }
        info!("successed to turn on VMX.");

        // Setup VMCS.
        let mut ret = Self {
            _vmxon_region: vmxon_region,
            vmcs_region,
        };
        ret.vmcs_setup(linux, cell)?;

        Ok(ret)
    }

    pub fn exit(&self, linux: &mut LinuxContext) -> HvResult {
        self.load_vmcs_guest(linux)?;
        Vmcs::clear(self.vmcs_region.paddr())?;
        unsafe { vmx::vmxoff()? };
        info!("successed to turn off VMX.");
        Ok(())
    }

    pub fn activate_vmm(&self, linux: &LinuxContext) -> HvResult {
        unsafe { vmx_entry(linux) };
        error!(
            "Activate hypervisor failed: {:?}",
            Vmcs::instruction_error()
        );
        hv_result_err!(EIO)
    }

    pub fn deactivate_vmm(&self, linux: &LinuxContext, guest_regs: &GuestRegisters) -> HvResult {
        unsafe { return_to_linux(linux, guest_regs) };
    }

    pub fn inject_fault(&self) -> HvResult {
        Vmcs::inject_interrupt(
            super::super::exception::ExceptionType::GeneralProtectionFault,
            Some(0),
        )?;
        Ok(())
    }

    pub fn guest_is_privileged(&self) -> HvResult<bool> {
        let cs_atrr =
            SegmentAccessRights::from_bits_truncate(VmcsField32Guest::CS_AR_BYTES.read()?);
        Ok(cs_atrr.dpl() == 0)
    }

    pub fn in_hypercall(&self) -> bool {
        if let Ok(info) = Vmcs::exit_info() {
            info.exit_reason == VmxExitReason::VMCALL
        } else {
            false
        }
    }

    pub fn guest_page_table(&self) -> GuestPageTable {
        use crate::memory::GenericPageTable;
        let cr3 = self.get_guest_cr(3).expect("Failed to read guest CR3") as usize;
        unsafe { GuestPageTable::from_root(align_down(cr3)) }
    }

    fn vmcs_setup(&mut self, linux: &LinuxContext, cell: &Cell) -> HvResult {
        let paddr = self.vmcs_region.paddr();
        Vmcs::clear(paddr)?;
        Vmcs::load(paddr)?;
        self.setup_vmcs_host()?;
        self.setup_vmcs_guest(linux)?;
        self.setup_vmcs_control(cell)?;
        Ok(())
    }

    fn setup_vmcs_host(&mut self) -> HvResult {
        VmcsField64Host::IA32_PAT.write(Msr::IA32_PAT.read())?;
        VmcsField64Host::IA32_EFER.write(Msr::IA32_EFER.read())?;

        VmcsField64Host::CR0.write(Cr0::read_raw())?;
        VmcsField64Host::CR3.write(Cr3::read().0.start_address().as_u64())?;
        VmcsField64Host::CR4.write(Cr4::read_raw())?;

        VmcsField16Host::CS_SELECTOR.write(GDTStruct::KCODE_SELECTOR.bits())?;
        VmcsField16Host::DS_SELECTOR.write(0)?;
        VmcsField16Host::ES_SELECTOR.write(0)?;
        VmcsField16Host::SS_SELECTOR.write(0)?;
        VmcsField16Host::FS_SELECTOR.write(0)?;
        VmcsField16Host::GS_SELECTOR.write(0)?;
        VmcsField16Host::TR_SELECTOR.write(GDTStruct::TSS_SELECTOR.bits())?;
        VmcsField64Host::FS_BASE.write(0)?;
        VmcsField64Host::GS_BASE.write(Msr::IA32_GS_BASE.read())?;
        VmcsField64Host::TR_BASE.write(0)?;

        VmcsField64Host::GDTR_BASE.write(GDT.lock().pointer().base as _)?;
        VmcsField64Host::IDTR_BASE.write(IDT.lock().pointer().base as _)?;

        VmcsField64Host::IA32_SYSENTER_ESP.write(0)?;
        VmcsField64Host::IA32_SYSENTER_EIP.write(0)?;
        VmcsField32Host::IA32_SYSENTER_CS.write(0)?;

        VmcsField64Host::RSP.write(crate::PerCpu::from_local_base().stack_top() as _)?;
        VmcsField64Host::RIP.write(vmx_exit as usize as _)?;
        Ok(())
    }

    pub(super) fn get_guest_cr(&self, cr_idx: usize) -> HvResult<u64> {
        Ok(match cr_idx {
            0 => VmcsField64Guest::CR0.read()?,
            3 => VmcsField64Guest::CR3.read()?,
            4 => {
                let host_mask = VmcsField64Control::CR4_GUEST_HOST_MASK.read()?;
                (VmcsField64Control::CR4_READ_SHADOW.read()? & host_mask)
                    | (VmcsField64Guest::CR4.read()? & !host_mask)
            }
            _ => unreachable!(),
        })
    }

    pub(super) fn set_guest_cr(&mut self, cr_idx: usize, val: u64) -> HvResult {
        match cr_idx {
            0 => {
                // Retrieve/validate restrictions on CR0
                //
                // In addition to what the VMX MSRs tell us, make sure that
                // - NW and CD are kept off as they are not updated on VM exit and we
                //   don't want them enabled for performance reasons while in root mode
                // - PE and PG can be freely chosen (by the guest) because we demand
                //   unrestricted guest mode support anyway
                // - ET is ignored
                let must0 = Msr::IA32_VMX_CR0_FIXED1.read()
                    & !(Cr0Flags::NOT_WRITE_THROUGH | Cr0Flags::CACHE_DISABLE).bits();
                let must1 = Msr::IA32_VMX_CR0_FIXED0.read()
                    & !(Cr0Flags::PAGING | Cr0Flags::PROTECTED_MODE_ENABLE).bits();
                VmcsField64Guest::CR0.write((val & must0) | must1)?;
                VmcsField64Control::CR0_READ_SHADOW.write(val)?;
                VmcsField64Control::CR0_GUEST_HOST_MASK.write(must1 | !must0)?;
            }
            3 => VmcsField64Guest::CR3.write(val)?,
            4 => {
                // Retrieve/validate restrictions on CR4
                let must0 = Msr::IA32_VMX_CR4_FIXED1.read();
                let must1 = Msr::IA32_VMX_CR4_FIXED0.read();
                let val = val | Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS.bits();
                VmcsField64Guest::CR4.write((val & must0) | must1)?;
                VmcsField64Control::CR4_READ_SHADOW.write(val)?;
                VmcsField64Control::CR4_GUEST_HOST_MASK.write(must1 | !must0)?;
            }
            _ => unreachable!(),
        };
        Ok(())
    }

    fn setup_vmcs_guest(&mut self, linux: &LinuxContext) -> HvResult {
        VmcsField64Guest::IA32_PAT.write(linux.pat)?;
        VmcsField64Guest::IA32_EFER.write(linux.efer)?;

        self.set_guest_cr(0, linux.cr0.bits())?;
        self.set_guest_cr(4, linux.cr4.bits())?;
        VmcsField64Guest::CR3.write(linux.cr3 as _)?;

        set_guest_segment!(linux.cs, CS);
        set_guest_segment!(linux.ds, DS);
        set_guest_segment!(linux.es, ES);
        set_guest_segment!(linux.fs, FS);
        set_guest_segment!(linux.gs, GS);
        set_guest_segment!(linux.tss, TR);
        let invalid_seg = Segment::invalid();
        set_guest_segment!(invalid_seg, SS);
        set_guest_segment!(invalid_seg, LDTR);

        VmcsField64Guest::GDTR_BASE.write(linux.gdt.base as _)?;
        VmcsField32Guest::GDTR_LIMIT.write(linux.gdt.limit as _)?;
        VmcsField64Guest::IDTR_BASE.write(linux.idt.base as _)?;
        VmcsField32Guest::IDTR_LIMIT.write(linux.idt.limit as _)?;

        VmcsField64Guest::RSP.write(linux.rsp as _)?;
        VmcsField64Guest::RIP.write(linux.rip as _)?;
        VmcsField64Guest::RFLAGS.write(0x2)?;

        VmcsField32Guest::SYSENTER_CS.write(Msr::IA32_SYSENTER_CS.read() as _)?;
        VmcsField64Guest::SYSENTER_ESP.write(Msr::IA32_SYSENTER_ESP.read())?;
        VmcsField64Guest::SYSENTER_EIP.write(Msr::IA32_SYSENTER_EIP.read())?;

        VmcsField64Guest::DR7.write(0x400)?;
        VmcsField64Guest::IA32_DEBUGCTL.write(0)?;

        VmcsField32Guest::ACTIVITY_STATE.write(0)?;
        VmcsField32Guest::INTERRUPTIBILITY_INFO.write(0)?;
        VmcsField64Guest::PENDING_DBG_EXCEPTIONS.write(0)?;

        VmcsField64Guest::VMCS_LINK_POINTER.write(core::u64::MAX)?;
        VmcsField32Guest::VMX_PREEMPTION_TIMER_VALUE.write(0)?;
        Ok(())
    }

    fn load_vmcs_guest(&self, linux: &mut LinuxContext) -> HvResult {
        linux.rip = VmcsField64Guest::RIP.read()? as _;
        linux.rsp = VmcsField64Guest::RSP.read()? as _;
        linux.cr0 = Cr0Flags::from_bits_truncate(VmcsField64Guest::CR0.read()?);
        linux.cr3 = VmcsField64Guest::CR3.read()? as _;
        linux.cr4 = Cr4Flags::from_bits_truncate(VmcsField64Guest::CR4.read()?)
            - Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS;

        linux.cs.selector = SegmentSelector::from_raw(VmcsField16Guest::CS_SELECTOR.read()?);
        linux.ds.selector = SegmentSelector::from_raw(VmcsField16Guest::DS_SELECTOR.read()?);
        linux.es.selector = SegmentSelector::from_raw(VmcsField16Guest::ES_SELECTOR.read()?);
        linux.fs.selector = SegmentSelector::from_raw(VmcsField16Guest::FS_SELECTOR.read()?);
        linux.fs.base = VmcsField64Guest::FS_BASE.read()?;
        linux.gs.selector = SegmentSelector::from_raw(VmcsField16Guest::GS_SELECTOR.read()?);
        linux.gs.base = VmcsField64Guest::GS_BASE.read()?;
        linux.tss.selector = SegmentSelector::from_raw(VmcsField16Guest::TR_SELECTOR.read()?);

        linux.gdt.base = VmcsField64Guest::GDTR_BASE.read()? as _;
        linux.gdt.limit = VmcsField32Guest::GDTR_LIMIT.read()? as _;
        linux.idt.base = VmcsField64Guest::IDTR_BASE.read()? as _;
        linux.idt.limit = VmcsField32Guest::IDTR_LIMIT.read()? as _;

        unsafe {
            Msr::IA32_SYSENTER_CS.write(VmcsField32Guest::SYSENTER_CS.read()? as _);
            Msr::IA32_SYSENTER_ESP.write(VmcsField64Guest::SYSENTER_ESP.read()?);
            Msr::IA32_SYSENTER_EIP.write(VmcsField64Guest::SYSENTER_EIP.read()?);
        }

        Ok(())
    }

    fn setup_vmcs_control(&mut self, cell: &Cell) -> HvResult {
        use vmx::flags::PinVmExecControls as PinCtrl;
        Vmcs::set_control(
            VmcsField32Control::PIN_BASED_VM_EXEC_CONTROL,
            Msr::IA32_VMX_PINBASED_CTLS.read(),
            // NO INTR_EXITING to pass-through interrupts
            PinCtrl::NMI_EXITING.bits(),
            0,
        )?;

        use vmx::flags::PrimaryVmExecControls as CpuCtrl;
        Vmcs::set_control(
            VmcsField32Control::PROC_BASED_VM_EXEC_CONTROL,
            Msr::IA32_VMX_PROCBASED_CTLS.read(),
            // NO UNCOND_IO_EXITING to pass-through PIO
            (CpuCtrl::USE_MSR_BITMAPS | CpuCtrl::SEC_CONTROLS).bits(),
            (CpuCtrl::CR3_LOAD_EXITING | CpuCtrl::CR3_STORE_EXITING).bits(),
        )?;

        use vmx::flags::SecondaryVmExecControls as CpuCtrl2;
        let mut val = CpuCtrl2::EPT | CpuCtrl2::UNRESTRICTED_GUEST;
        let features = CpuFeatures::new();
        if features.has_rdtscp() {
            val |= CpuCtrl2::RDTSCP;
        }
        if features.has_invpcid() {
            val |= CpuCtrl2::INVPCID;
        }
        if features.has_xsaves_xrstors() {
            val |= CpuCtrl2::XSAVES;
        }
        Vmcs::set_control(
            VmcsField32Control::SECONDARY_VM_EXEC_CONTROL,
            Msr::IA32_VMX_PROCBASED_CTLS2.read(),
            val.bits(),
            0,
        )?;

        use vmx::flags::VmExitControls as ExitCtrl;
        Vmcs::set_control(
            VmcsField32Control::VM_EXIT_CONTROLS,
            Msr::IA32_VMX_EXIT_CTLS.read(),
            (ExitCtrl::HOST_ADDR_SPACE_SIZE
                | ExitCtrl::SAVE_IA32_PAT
                | ExitCtrl::LOAD_IA32_PAT
                | ExitCtrl::SAVE_IA32_EFER
                | ExitCtrl::LOAD_IA32_EFER)
                .bits(),
            0,
        )?;

        use vmx::flags::VmEntryControls as EntryCtrl;
        Vmcs::set_control(
            VmcsField32Control::VM_ENTRY_CONTROLS,
            Msr::IA32_VMX_ENTRY_CTLS.read(),
            (EntryCtrl::IA32E_MODE | EntryCtrl::LOAD_IA32_PAT | EntryCtrl::LOAD_IA32_EFER).bits(),
            0,
        )?;

        VmcsField32Control::VM_EXIT_MSR_STORE_COUNT.write(0)?;
        VmcsField32Control::VM_EXIT_MSR_LOAD_COUNT.write(0)?;
        VmcsField32Control::VM_ENTRY_MSR_LOAD_COUNT.write(0)?;

        VmcsField64Control::CR4_GUEST_HOST_MASK.write(0)?;
        VmcsField32Control::CR3_TARGET_COUNT.write(0)?;

        unsafe { cell.gpm.read().activate() }; // Set EPT_POINTER

        VmcsField64Control::MSR_BITMAP.write(MSR_BITMAP.paddr() as _)?;
        VmcsField32Control::EXCEPTION_BITMAP.write(0)?;

        Ok(())
    }
}

unsafe fn vmx_entry(linux: &LinuxContext) {
    asm!("
        mov rbp, {0}
        vmlaunch",
        in(reg) linux.rbp,
        in("r15") linux.r15,
        in("r14") linux.r14,
        in("r13") linux.r13,
        in("r12") linux.r12,
        in("rbx") linux.rbx,
        in("rax") 0,
    );
    // Never return if successful
}

#[naked]
#[inline(never)]
unsafe extern "sysv64" fn vmx_exit() -> ! {
    // See crate::arch::context::GuestRegisters
    asm!("
        push rax
        push rcx
        push rdx
        push rbx
        sub rsp, 8
        push rbp
        push rsi
        push rdi
        push r8
        push r9
        push r10
        push r11
        push r12
        push r13
        push r14
        push r15

        call {0}

        pop r15
        pop r14
        pop r13
        pop r12
        pop r11
        pop r10
        pop r9
        pop r8
        pop rdi
        pop rsi
        pop rbp
        add rsp, 8
        pop rbx
        pop rdx
        pop rcx
        pop rax

        vmresume",
        sym super::super::vmexit::vmexit_handler,
    );
    panic!("VM resume failed: {:?}", Vmcs::instruction_error());
}

unsafe fn return_to_linux(linux: &LinuxContext, guest_regs: &GuestRegisters) -> ! {
    asm!("
        mov rsp, rax
        push {linux_rip}
        push {guest_rax}
        mov rax, rsp
        mov rsp, {guest_regs}

        pop r15
        pop r14
        pop r13
        pop r12
        pop r11
        pop r10
        pop r9
        pop r8
        pop rdi
        pop rsi
        pop rbp
        add rsp, 8
        pop rbx
        pop rdx
        pop rcx

        mov rsp, rax
        pop rax
        ret",
        linux_rip = in(reg) linux.rip,
        guest_rax = in(reg) guest_regs.rax,
        guest_regs = in(reg) guest_regs,
        in("rax") linux.rsp,
    );
    unreachable!()
}
