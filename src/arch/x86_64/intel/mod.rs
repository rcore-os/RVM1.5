mod ept;
mod structs;
mod vcpu;
mod vmexit;

use libvmm::vmx::Vmcs;
use x86::vmx::VmFail;

use crate::arch::cpuid::CpuFeatures;
use crate::error::{HvError, HvResult};

pub use ept::ExtendedPageTable as NestedPageTable;
pub use vcpu::Vcpu;

impl From<VmFail> for HvError {
    fn from(err: VmFail) -> Self {
        match err {
            VmFail::VmFailValid => hv_err!(
                EIO,
                format!("{:?}: {:x?}", err, Vmcs::instruction_error().unwrap())
            ),
            _ => hv_err!(EIO, format!("{:?}", err)),
        }
    }
}

pub fn check_hypervisor_feature() -> HvResult {
    if CpuFeatures::new().has_vmx() {
        Ok(())
    } else {
        warn!("Feature VMX not supported!");
        hv_result_err!(ENODEV, "VMX feature checks failed!")
    }
}
