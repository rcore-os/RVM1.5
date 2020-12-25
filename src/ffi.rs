use crate::header::HvHeader;
use crate::percpu::PerCpu;

extern "C" {
    fn __header_start();
    fn __core_end();
}

pub const PER_CPU_ARRAY_PTR: *mut PerCpu = __core_end as _;
pub const HEADER_PTR: *const HvHeader = __header_start as _;
