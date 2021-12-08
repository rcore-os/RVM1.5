use x86::bits64::rflags::{self, RFlags};
use x86::vmx::{Result, VmFail};

use super::flags::{InvEptDescriptor, InvEptType};

pub use x86::bits64::vmx::{vmxoff, vmxon};

/// Helper used to extract VMX-specific Result in accordance with
/// conventions described in Intel SDM, Volume 3C, Section 30.2.
// We inline this to provide an obstruction-free path from this function's
// call site to the moment where `rflags::read()` reads RFLAGS. Otherwise it's
// possible for RFLAGS register to be clobbered by a function prologue,
// see https://github.com/gz/rust-x86/pull/50.
#[inline(always)]
fn vmx_capture_status() -> Result<()> {
    let flags = rflags::read();

    if flags.contains(RFlags::FLAGS_ZF) {
        Err(VmFail::VmFailValid)
    } else if flags.contains(RFlags::FLAGS_CF) {
        Err(VmFail::VmFailInvalid)
    } else {
        Ok(())
    }
}

/// Invalidate Translations Derived from EPT.
///
/// # Safety
///
/// This function is unsafe because the caller must ensure that the given
/// EPT pointer `eptp` is valid, and it's possible to violate memory safety
/// through execution.
pub unsafe fn invept(invalidation: InvEptType, eptp: u64) -> Result<()> {
    let descriptor = InvEptDescriptor::new(eptp);
    asm!("invept {}, [{}]", in(reg) invalidation as u64, in(reg) &descriptor);
    vmx_capture_status()
}
