pub use crate::memory::PAGE_SIZE;
pub use crate::percpu::PER_CPU_SIZE;

pub const HV_BASE: usize = 0xffff_ff00_0000_0000;

pub const TEMP_MAPPING_BASE: usize = 0x0000_0080_0000_0000;
pub const NUM_TEMP_PAGES: usize = 16;
pub const LOCAL_PER_CPU_BASE: usize = TEMP_MAPPING_BASE + NUM_TEMP_PAGES * PAGE_SIZE;

pub const HV_STACK_SIZE: usize = 512 * 1024; // 512 KB
pub const HV_HEAP_SIZE: usize = 32 * 1024 * 1024; // 32 MB
