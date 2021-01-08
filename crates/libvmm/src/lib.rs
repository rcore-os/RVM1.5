#![cfg_attr(not(test), no_std)]
#![feature(asm)]

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;
