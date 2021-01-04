use core::convert::TryFrom;

use libvmm::svm::SvmExitCode;

use crate::arch::vmm::{VcpuAccessGuestState, VmExit};
use crate::error::HvResult;

impl VmExit<'_> {
    pub fn handle_exit(&mut self) -> HvResult {
        let vcpu = &mut self.cpu_data.vcpu;
        vcpu.regs_mut().rax = vcpu.vmcb.save.rax;

        let exit_code = match SvmExitCode::try_from(vcpu.vmcb.control.exit_code) {
            Ok(code) => code,
            Err(code) => {
                error!("Unknown #VMEXIT exit code: {:#x}", code);
                return hv_result_err!(EIO);
            }
        };

        let res = match exit_code {
            SvmExitCode::CPUID => self.handle_cpuid(),
            SvmExitCode::VMMCALL => self.handle_hypercall(),
            SvmExitCode::MSR => match vcpu.vmcb.control.exit_info_1 {
                0 => self.handle_msr_read(),
                1 => self.handle_msr_write(),
                _ => hv_result_err!(EIO),
            },
            _ => hv_result_err!(ENOSYS),
        };

        let vcpu = &mut self.cpu_data.vcpu;
        if res.is_err() {
            warn!(
                "#VMEXIT handler returned {:?}:\n\
                EXITCODE: {:?}\n\
                EXITINFO1: {:#x}\n\
                EXITINFO2: {:#x}\n\n\
                Guest State Dump:\n\
                {:#x?}",
                res, exit_code, vcpu.vmcb.control.exit_info_1, vcpu.vmcb.control.exit_info_2, vcpu,
            );
        }
        vcpu.vmcb.save.rax = vcpu.regs().rax;
        res
    }
}
