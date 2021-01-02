mod npt;
mod vcpu;
mod vmexit;

use crate::error::HvResult;

pub use npt::NestedPageTable as HvPageTable;
pub use vcpu::Vcpu;

pub fn check_hypervisor_feature() -> HvResult {
    todo!()
}
