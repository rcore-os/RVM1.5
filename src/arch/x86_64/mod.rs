pub mod consts;
#[macro_use]
mod context;
pub mod cpu;
mod cpuid;
mod exception;
pub mod io;
mod page_table;
mod segmentation;
mod tables;
pub mod vmm;

pub use context::{GuestRegisters, LinuxContext};
pub use exception::ExceptionType;
pub use page_table::PageTable as HostPageTable;
pub use page_table::PageTable as GuestPageTable;
pub use vmm::HvPageTable;

use crate::percpu::PerCpu;

unsafe extern "sysv64" fn switch_stack(cpu_id: usize, linux_sp: usize) -> i32 {
    let cpu_data = PerCpu::from_id(cpu_id);
    let hv_sp = cpu_data.stack_top();
    let mut ret;
    asm!("
        mov rcx, rsp
        mov rsp, {0}
        push rcx
        call {1}
        pop rsp",
        in(reg) hv_sp,
        sym crate::entry,
        in("rdi") cpu_id,
        in("rsi") linux_sp,
        lateout("rax") ret,
    );
    ret
}

#[naked]
#[no_mangle]
pub unsafe extern "C" fn arch_entry(_cpu_id: usize) -> i32 {
    asm!("
        // rip is pushed
        cli
        push rbp
        push rbx
        push r12
        push r13
        push r14
        push r15

        mov rsi, rsp
        call {0}

        pop r15
        pop r14
        pop r13
        pop r12
        pop rbx
        pop rbp
        ret
        // rip will pop when return",
        sym switch_stack,
    );
    unreachable!()
}
