use super::context::GuestRegisters;

global_asm!(include_str!(concat!(env!("OUT_DIR"), "/exception.S")));

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod ExceptionType {
    pub const DivideError: u8 = 0;
    pub const Debug: u8 = 1;
    pub const NonMaskableInterrupt: u8 = 2;
    pub const Breakpoint: u8 = 3;
    pub const Overflow: u8 = 4;
    pub const BoundRangeExceeded: u8 = 5;
    pub const InvalidOpcode: u8 = 6;
    pub const DeviceNotAvailable: u8 = 7;
    pub const DoubleFault: u8 = 8;
    pub const CoprocessorSegmentOverrun: u8 = 9;
    pub const InvalidTSS: u8 = 10;
    pub const SegmentNotPresent: u8 = 11;
    pub const StackSegmentFault: u8 = 12;
    pub const GeneralProtectionFault: u8 = 13;
    pub const PageFault: u8 = 14;
    pub const FloatingPointException: u8 = 16;
    pub const AlignmentCheck: u8 = 17;
    pub const MachineCheck: u8 = 18;
    pub const SIMDFloatingPointException: u8 = 19;
    pub const VirtualizationException: u8 = 20;
    pub const SecurityException: u8 = 30;

    pub const IrqStart: u8 = 32;
    pub const IrqEnd: u8 = 255;
}

#[repr(C)]
#[derive(Debug)]
pub struct ExceptionFrame {
    // Pushed by `common_exception_entry`
    regs: GuestRegisters,

    // Pushed by 'exception.S'
    num: usize,
    error_code: usize,

    // Pushed by CPU
    rip: usize,
    cs: usize,
    rflags: usize,

    rsp: usize,
    ss: usize,
}

fn exception_handler(frame: &ExceptionFrame) {
    trace!("Exception or interrupt #{:#x}", frame.num);
    match frame.num as u8 {
        ExceptionType::NonMaskableInterrupt => handle_nmi(),
        ExceptionType::IrqStart..=ExceptionType::IrqEnd => {
            error!("{:#x?}", frame);
            panic!("Unhandled interrupt #{:#x}", frame.num);
        }
        _ => {
            error!("{:#x?}", frame);
            panic!("Unhandled exception #{:#x}", frame.num);
        }
    }
}

fn handle_nmi() {
    warn!("Unhandled exception: NMI");
}

#[naked]
#[no_mangle]
#[inline(never)]
unsafe extern "sysv64" fn common_exception_entry() {
    asm!(
        save_regs_to_stack!(),
        "mov rdi, rsp",
        "call {0}",
        restore_regs_from_stack!(),
        "add rsp, 16",  // skip num, error_code
        "iretq",
        sym exception_handler,
    );
    unreachable!();
}
