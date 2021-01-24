use libvmm::svm::flags::VmcbCleanBits;
use libvmm::svm::{SvmExitCode, VmExitInfo};

use crate::arch::vmm::{VcpuAccessGuestState, VmExit};
use crate::error::HvResult;

impl VmExit<'_> {
    fn handle_nmi(&mut self) -> HvResult {
        unsafe { asm!("stgi; clgi") };
        Ok(())
    }

    fn handle_exception(&mut self, vec: u8, exit_info: &VmExitInfo) -> HvResult {
        info!(
            "#VMEXIT(EXCP {}) @ RIP({:#x}): {:#x?}",
            vec, exit_info.guest_rip, exit_info
        );
        warn!("Unhandled Guest Exception: #{:#x}", vec);
        Ok(())
    }

    fn handle_nested_page_fault(&mut self, exit_info: &VmExitInfo) -> HvResult {
        let guest_paddr = exit_info.exit_info_2;
        warn!(
            "#VMEXIT(NPF) @ {:#x} RIP({:#x}, {:#x})",
            guest_paddr, exit_info.guest_rip, exit_info.guest_next_rip,
        );
        hv_result_err!(ENOSYS)
    }

    pub fn handle_exit(&mut self) -> HvResult {
        let vcpu = &mut self.cpu_data.vcpu;
        vcpu.regs_mut().rax = vcpu.vmcb.save.rax;

        // All guest state is marked unmodified; individual handlers must clear
        // the bits as needed.
        vcpu.vmcb.control.clean_bits = VmcbCleanBits::UNMODIFIED;

        let exit_info = VmExitInfo::new(&vcpu.vmcb);
        let exit_code = match exit_info.exit_code {
            Ok(code) => code,
            Err(code) => {
                error!("Unknown #VMEXIT exit code: {:#x}", code);
                return hv_result_err!(EIO);
            }
        };

        let res = match exit_code {
            SvmExitCode::INVALID => panic!("VM entry failed: {:#x?}\n{:#x?}", exit_info, vcpu.vmcb),
            SvmExitCode::EXCP(vec) => self.handle_exception(vec, &exit_info),
            SvmExitCode::NMI => self.handle_nmi(),
            SvmExitCode::CPUID => self.handle_cpuid(),
            SvmExitCode::VMMCALL => self.handle_hypercall(),
            SvmExitCode::NPF => self.handle_nested_page_fault(&exit_info),
            SvmExitCode::MSR => match exit_info.exit_info_1 {
                0 => self.handle_msr_read(),
                1 => self.handle_msr_write(),
                _ => hv_result_err!(EIO),
            },
            SvmExitCode::SHUTDOWN => {
                error!("#VMEXIT(SHUTDOWN): {:#x?}", exit_info);
                self.cpu_data.vcpu.inject_fault()?;
                Ok(())
            }
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
