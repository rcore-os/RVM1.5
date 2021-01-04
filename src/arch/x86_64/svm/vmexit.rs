use libvmm::svm::{SvmExitCode, VmExitInfo};

use crate::arch::vmm::{VcpuAccessGuestState, VmExit};
use crate::error::HvResult;

impl VmExit<'_> {
    pub fn handle_nested_page_fault(&mut self, exit_info: &VmExitInfo) -> HvResult {
        let guest_paddr = exit_info.exit_info_2;
        warn!(
            "#VMEXIT(NPF) @ {:#x} RIP({:#x}, {})",
            guest_paddr,
            exit_info.guest_rip,
            exit_info.guest_rip - exit_info.guest_rip,
        );
        hv_result_err!(ENOSYS)
    }

    pub fn handle_exit(&mut self) -> HvResult {
        let vcpu = &mut self.cpu_data.vcpu;
        vcpu.regs_mut().rax = vcpu.vmcb.save.rax;

        // All guest state is marked unmodified; individual handlers must clear
        // the bits as needed.
        vcpu.vmcb.control.clean_bits = 0xffff_ffff;

        let exit_info = VmExitInfo::new(&vcpu.vmcb);
        let exit_code = match exit_info.exit_code {
            Ok(code) => code,
            Err(code) => {
                error!("Unknown #VMEXIT exit code: {:#x}", code);
                return hv_result_err!(EIO);
            }
        };

        let res = match exit_code {
            SvmExitCode::INVALID => panic!("VM entry failed: {:#x?}", exit_info),
            SvmExitCode::CPUID => self.handle_cpuid(),
            SvmExitCode::VMMCALL => self.handle_hypercall(),
            SvmExitCode::NPF => self.handle_nested_page_fault(&exit_info),
            SvmExitCode::MSR => match exit_info.exit_info_1 {
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
                {:#x?}\n\n\
                Guest State Dump:\n\
                {:#x?}",
                res, exit_info, vcpu,
            );
        }
        vcpu.vmcb.save.rax = vcpu.regs().rax;
        res
    }
}
