use libvmm::vmx::VmExitInfo;
use x86_64::registers::control::Cr4Flags;

use crate::error::HvResult;
use crate::percpu::PerCpu;

pub(super) struct VmExit<'a> {
    pub cpu_data: &'a mut PerCpu,
}

impl VmExit<'_> {
    pub fn new() -> Self {
        Self {
            cpu_data: PerCpu::from_local_base_mut(),
        }
    }

    pub fn dump_guest_state(&self) -> HvResult<alloc::string::String> {
        Ok(format!(
            "Guest State Dump:\n\
            {:#x?}",
            self.cpu_data.guest_all_state()
        ))
    }

    pub fn handle_cpuid(&mut self, exit_info: &VmExitInfo) -> HvResult {
        use super::cpuid::{cpuid, CpuIdEax, FeatureInfoFlags};
        let signature = unsafe { &*("RVMRVMRVMRVM".as_ptr() as *const [u32; 3]) };
        let cr4_flags = Cr4Flags::from_bits_truncate(self.cpu_data.guest_all_state().cr(4));
        let guest_regs = self.cpu_data.guest_regs_mut();
        let function = guest_regs.rax as u32;
        if function == CpuIdEax::HypervisorInfo as _ {
            guest_regs.rax = CpuIdEax::HypervisorFeatures as u32 as _;
            guest_regs.rbx = signature[0] as _;
            guest_regs.rcx = signature[1] as _;
            guest_regs.rdx = signature[2] as _;
        } else if function == CpuIdEax::HypervisorFeatures as _ {
            guest_regs.rax = 0;
            guest_regs.rbx = 0;
            guest_regs.rcx = 0;
            guest_regs.rdx = 0;
        } else {
            let res = cpuid!(guest_regs.rax, guest_regs.rcx);
            guest_regs.rax = res.eax as _;
            guest_regs.rbx = res.ebx as _;
            guest_regs.rcx = res.ecx as _;
            guest_regs.rdx = res.edx as _;
            let mut flags = FeatureInfoFlags::from_bits_truncate(guest_regs.rcx as _);
            if function == CpuIdEax::FeatureInfo as _ {
                if cr4_flags.contains(Cr4Flags::OSXSAVE) {
                    flags.insert(FeatureInfoFlags::OSXSAVE);
                }
                flags.remove(FeatureInfoFlags::VMX);
                flags.insert(FeatureInfoFlags::HYPERVISOR);
            } else if function == CpuIdEax::AmdFeatureInfo as _ {
                flags.remove(FeatureInfoFlags::SVM);
            }
            guest_regs.rcx = flags.bits() as _;
        }
        exit_info.advance_rip()?;
        Ok(())
    }

    pub fn handle_hypercall(&mut self, exit_info: &VmExitInfo) -> HvResult {
        use crate::hypercall::HyperCall;
        exit_info.advance_rip()?;
        let guest_regs = self.cpu_data.guest_regs();
        let (code, arg0, arg1) = (guest_regs.rax, guest_regs.rdi, guest_regs.rsi);
        HyperCall::new(&mut self.cpu_data).hypercall(code, arg0, arg1)?;
        Ok(())
    }
}

pub(super) fn vmexit_handler() {
    VmExit::new().handle_exit().unwrap();
}
