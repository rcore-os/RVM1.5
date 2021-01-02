#[cfg(feature = "vmx")]
#[path = "vmx/mod.rs"]
mod vendor;

#[cfg(feature = "svm")]
#[path = "svm/mod.rs"]
mod vendor;

use x86_64::registers::control::Cr4Flags;

use super::GuestRegisters;
use crate::error::HvResult;
use crate::percpu::PerCpu;

pub use vendor::{check_hypervisor_feature, HvPageTable, Vcpu};

pub trait VcpuAccessGuestState {
    // Architecture independent methods:

    fn regs(&self) -> &GuestRegisters;
    fn regs_mut(&mut self) -> &mut GuestRegisters;
    fn instr_pointer(&self) -> u64;
    fn stack_pointer(&self) -> u64;
    fn frame_pointer(&self) -> u64 {
        self.regs().rbp
    }
    fn set_stack_pointer(&mut self, sp: u64);
    fn set_return_val(&mut self, ret_val: usize) {
        self.regs_mut().rax = ret_val as _
    }

    // Methods only available for x86 cpus:

    fn rflags(&self) -> u64;
    fn cr(&self, cr_idx: usize) -> u64;
    fn set_cr(&mut self, cr_idx: usize, val: u64);
}

pub const VM_EXIT_LEN_CPUID: u8 = 2;
pub const VM_EXIT_LEN_RDMSR: u8 = 2;
pub const VM_EXIT_LEN_WRMSR: u8 = 2;
pub const VM_EXIT_LEN_HYPERCALL: u8 = 3;

pub(super) struct VmExit<'a> {
    pub cpu_data: &'a mut PerCpu,
}

impl VmExit<'_> {
    pub fn new() -> Self {
        Self {
            cpu_data: PerCpu::from_local_base_mut(),
        }
    }

    pub fn handle_cpuid(&mut self) -> HvResult {
        use super::cpuid::{cpuid, CpuIdEax, FeatureInfoFlags};
        let signature = unsafe { &*("RVMRVMRVMRVM".as_ptr() as *const [u32; 3]) };
        let cr4_flags = Cr4Flags::from_bits_truncate(self.cpu_data.vcpu.cr(4));
        let guest_regs = self.cpu_data.vcpu.regs_mut();
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
            guest_regs.rcx = flags.bits();
        }
        self.cpu_data.vcpu.advance_rip(VM_EXIT_LEN_CPUID)?;
        Ok(())
    }

    pub fn handle_hypercall(&mut self) -> HvResult {
        use crate::hypercall::HyperCall;
        self.cpu_data.vcpu.advance_rip(VM_EXIT_LEN_HYPERCALL)?;
        let guest_regs = self.cpu_data.vcpu.regs();
        let (code, arg0, arg1) = (guest_regs.rax, guest_regs.rdi, guest_regs.rsi);
        HyperCall::new(&mut self.cpu_data).hypercall(code as _, arg0, arg1)?;
        Ok(())
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

pub(super) fn vmexit_handler() {
    let mut vmexit = VmExit::new();
    let res = vmexit.handle_exit();
    if let Err(err) = res {
        error!(
            "Failed to handle VM exit, inject fault to guest...\n{:?}",
            err
        );
        vmexit.cpu_data.fault().unwrap();
    }
}
