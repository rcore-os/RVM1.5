use crate::arch::vmm::VmExit;
use crate::error::HvResult;

impl VmExit<'_> {
    pub fn handle_exit(&mut self) -> HvResult {
        todo!()
    }
}
