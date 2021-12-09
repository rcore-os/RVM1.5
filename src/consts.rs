pub use crate::memory::PAGE_SIZE;
pub use crate::percpu::PER_CPU_SIZE;

pub const HV_BASE: usize = 0xffff_ff00_0000_0000;

pub const HV_STACK_SIZE: usize = 512 * 1024; // 512 KB
pub const HV_HEAP_SIZE: usize = 32 * 1024 * 1024; // 32 MB
