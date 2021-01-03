use bitflags::bitflags;

use crate::x86_64::msr::{Msr, MsrReadWrite};

bitflags! {
    /// VM_CR MSR flags.
   pub struct VmCrFlags: u64 {
       /// If set, disables HDT and certain internal debug features.
       const DPD        = 1 << 0;
       /// If set, non-intercepted INIT signals are converted into an #SX
       /// exception.
       const R_INIT     = 1 << 1;
       /// If set, disables A20 masking.
       const DIS_A20M   = 1 << 2;
       /// When this bit is set, writes to LOCK and SVMDIS are silently ignored.
       /// When this bit is clear, VM_CR bits 3 and 4 can be written. Once set,
       /// LOCK can only be cleared using the SVM_KEY MSR (See Section 15.31.)
       /// This bit is not affected by INIT or SKINIT.
       const LOCK       = 1 << 3;
       /// When this bit is set, writes to EFER treat the SVME bit as MBZ. When
       /// this bit is clear, EFER.SVME can be written normally. This bit does
       ///  not prevent CPUID from reporting that SVM is available. Setting
       /// SVMDIS while EFER.SVME is 1 generates a #GP fault, regardless of the
       /// current state of VM_CR.LOCK. This bit is not affected by SKINIT. It
       /// is cleared by INIT when LOCK is cleared to 0; otherwise, it is not
       /// affected.
       const SVMDIS     = 1 << 4;
   }
}

/// The VM_CR MSR controls certain global aspects of SVM.
pub struct VmCr;

impl MsrReadWrite for VmCr {
    const MSR: Msr = Msr::VM_CR;
}

impl VmCr {
    pub fn read() -> VmCrFlags {
        VmCrFlags::from_bits_truncate(Self::read_raw())
    }
}
