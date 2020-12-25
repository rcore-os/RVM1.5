use crate::ffi::HEADER_PTR;
use crate::percpu::PER_CPU_SIZE;

#[repr(C)]
#[derive(Debug)]
pub struct HvHeader {
    pub signature: [u8; 8],
    pub core_size: usize,
    pub percpu_size: usize,
    pub entry: usize,
    pub console_page: usize,
    pub gcov_info_head: usize,
    pub max_cpus: u32,
    pub online_cpus: u32,
    pub debug_console_base: usize,
    pub arm_linux_hyp_vectors: u64,
    pub arm_linux_hyp_abi: u32,
}

impl HvHeader {
    pub fn get<'a>() -> &'a Self {
        unsafe { &*HEADER_PTR }
    }
}

#[repr(C)]
struct HvHeaderStuff {
    signature: [u8; 8],
    core_size: unsafe extern "C" fn(),
    percpu_size: usize,
    entry: unsafe extern "C" fn(),
    console_page: usize,
    gcov_info_head: usize,
    max_cpus: u32,
    online_cpus: u32,
    debug_console_base: usize,
    arm_linux_hyp_vectors: u64,
    arm_linux_hyp_abi: u32,
}

extern "C" {
    fn __entry_offset();
    fn __core_size();
}

#[used]
#[link_section = ".header"]
static HEADER_STUFF: HvHeaderStuff = HvHeaderStuff {
    signature: *b"JAILHOUS",
    core_size: __core_size,
    percpu_size: PER_CPU_SIZE,
    entry: __entry_offset,
    console_page: 0,
    gcov_info_head: 0,
    max_cpus: 0,
    online_cpus: 0,
    debug_console_base: 0,
    arm_linux_hyp_vectors: 0,
    arm_linux_hyp_abi: 0,
};
