use core::alloc::Layout;
use core::panic::PanicInfo;

use crate::error::HvResult;
use crate::percpu::{CpuState, PerCpu};

fn try_handle_panic(cpu_data: &mut PerCpu) -> HvResult {
    let ret_code = if cpu_data.state != CpuState::HvDisabled && cpu_data.vcpu.in_hypercall() {
        hv_err!(EIO).code() as usize
    } else {
        0
    };
    match cpu_data.state {
        CpuState::HvEnabled => cpu_data.deactivate_vmm(ret_code)?,
        _ => return hv_result_err!(EIO, "Hypervisor is not enabled!"),
    }
    Ok(())
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let cpu_data = PerCpu::current_mut();
    error!("\n{}\nCurrent Cpu: {:#x?}", info, cpu_data);
    let err = try_handle_panic(cpu_data);
    error!("Try handle panic failed: {:?}", err);
    loop {}
}

#[lang = "oom"]
fn oom(_: Layout) -> ! {
    panic!("out of memory");
}
