use crate::ffi::HEADER_PTR;
use crate::percpu::PER_CPU_SIZE;

#[repr(C)]
#[derive(Debug)]
pub struct HvHeader {
    pub signature: [u8; 8],
    pub core_size: usize,
    pub percpu_size: usize,
    pub entry: usize,
    pub max_cpus: u32,
    pub online_cpus: u32,
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
    max_cpus: u32,
    online_cpus: u32,
}

extern "C" {
    fn __entry_offset();
    fn __core_size();
}

#[used]
#[link_section = ".header"]
static HEADER_STUFF: HvHeaderStuff = HvHeaderStuff {
    signature: *b"RVMIMAGE",
    core_size: __core_size,
    percpu_size: PER_CPU_SIZE,
    entry: __entry_offset,
    max_cpus: 0,
    online_cpus: 0,
};
