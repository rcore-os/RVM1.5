use libvmm::vmx::vmcs::{EptViolationInfo, ExitInterruptionInfo};
use libvmm::vmx::{VmExitInfo, Vmcs, VmxExitReason};

use super::super::exception::ExceptionType;
use super::super::vmexit::VmExit;
use crate::error::HvResult;

impl VmExit<'_> {
    fn handle_exception_nmi(&mut self, exit_info: &VmExitInfo) -> HvResult {
        let intr_info = ExitInterruptionInfo::new()?;
        info!(
            "VM exit: Exception or NMI @ RIP({:#x}, {}): {:#x?}",
            exit_info.guest_rip, exit_info.exit_instruction_length, intr_info
        );
        match intr_info.vector {
            ExceptionType::NonMaskableInterrupt => unsafe {
                asm!("int {}", const ExceptionType::NonMaskableInterrupt)
            },
            v => warn!("Unhandled Guest Exception: #{:#x}", v),
        }
        Ok(())
    }

    fn handle_msr_read(&mut self, exit_info: &VmExitInfo) -> HvResult {
        let guest_regs = &mut self.cpu_data.vcpu.guest_regs;
        let id = guest_regs.rcx;
        warn!("VM exit: RDMSR({:#x})", id);
        // TODO
        guest_regs.rax = 0;
        guest_regs.rdx = 0;
        exit_info.advance_rip()?;
        Ok(())
    }

    fn handle_msr_write(&mut self, exit_info: &VmExitInfo) -> HvResult {
        let guest_regs = &self.cpu_data.vcpu.guest_regs;
        let id = guest_regs.rcx;
        let value = guest_regs.rax | (guest_regs.rdx << 32);
        warn!("VM exit: WRMSR({:#x}) <- {:#x}", id, value);
        // TODO
        exit_info.advance_rip()?;
        Ok(())
    }

    fn handle_ept_violation(&mut self, exit_info: &VmExitInfo) -> HvResult {
        let ept_vio_info = EptViolationInfo::new()?;
        warn!(
            "VM exit: EPT violation @ {:#x} RIP({:#x}, {}): {:#x?}",
            ept_vio_info.guest_paddr,
            exit_info.guest_rip,
            exit_info.exit_instruction_length,
            ept_vio_info
        );
        hv_result_err!(ENOSYS)
    }

    pub fn handle_exit(&mut self) -> HvResult {
        let exit_info = Vmcs::exit_info()?;
        trace!("VM exit: {:#x?}", exit_info);

        if exit_info.entry_failure {
            error!("VM entry failed: {:#x?}", exit_info);
            return hv_result_err!(EIO);
        }
        // self.test_read_guest_memory(
        //     exit_info.guest_rip as _,
        //     exit_info.exit_instruction_length as _,
        // )?;

        let res = match exit_info.exit_reason {
            VmxExitReason::EXCEPTION_NMI => self.handle_exception_nmi(&exit_info),
            VmxExitReason::CPUID => self.handle_cpuid(&exit_info),
            VmxExitReason::VMCALL => self.handle_hypercall(&exit_info),
            VmxExitReason::MSR_READ => self.handle_msr_read(&exit_info),
            VmxExitReason::MSR_WRITE => self.handle_msr_write(&exit_info),
            VmxExitReason::EPT_VIOLATION => self.handle_ept_violation(&exit_info),
            _ => hv_result_err!(ENOSYS),
        };

        if res.is_err() {
            warn!(
                "VM exit handler for reason {:?} returned {:?}:\n\
                {:#x?}\n\n\
                {}",
                exit_info.exit_reason,
                res,
                exit_info,
                self.dump_guest_state()
                    .expect("Failed to dump guest state!")
            );
        }
        res
    }

    #[allow(dead_code)]
    fn test_read_guest_memory(&self, gvaddr: usize, size: usize) -> HvResult {
        use crate::cell;
        use crate::memory::addr::phys_to_virt;
        use crate::memory::GenericPageTable;

        let pt = self.cpu_data.vcpu.guest_page_table();
        let (gpaddr, _, _) = pt.query(gvaddr)?;
        let (hpaddr, _, _) = cell::ROOT_CELL.gpm.read().page_table().query(gpaddr)?;
        let buf = unsafe { core::slice::from_raw_parts(phys_to_virt(hpaddr) as *const u8, size) };
        println!(
            "GVA({:#x?}) -> GPA({:#x?}) -> HPA({:#x?}): {:02X?}",
            gvaddr, gpaddr, hpaddr, buf
        );
        Ok(())
    }
}
