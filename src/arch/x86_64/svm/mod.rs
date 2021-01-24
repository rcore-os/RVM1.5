mod npt;
mod vcpu;
mod vmexit;

use libvmm::svm::flags::{VmCr, VmCrFlags};

use crate::error::HvResult;

pub use npt::NestedPageTable as HvPageTable;
pub use vcpu::Vcpu;

pub fn check_hypervisor_feature() -> HvResult {
    if VmCr::read().contains(VmCrFlags::SVMDIS) {
        return hv_result_err!(ENODEV, "SVM disabled by BIOS!");
    }
    // TODO: check cpuid
    Ok(())
}
