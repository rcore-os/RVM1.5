use crate::percpu::PerCpu;

use libvmm::msr::Msr;

unsafe extern "sysv64" fn switch_stack(linux_sp: usize) -> i32 {
    let linux_tp = Msr::IA32_GS_BASE.read();
    let cpu_data = match PerCpu::new() {
        Ok(c) => c,
        Err(e) => return e.code(),
    };
    let hv_sp = cpu_data.stack_top();
    let ret;
    asm!("
        mov [rsi], {linux_tp}   // save gs_base to stack
        mov rcx, rsp
        mov rsp, {hv_sp}
        push rcx
        call {entry}
        pop rsp",
        entry = sym crate::entry,
        linux_tp = in(reg) linux_tp,
        hv_sp = in(reg) hv_sp,
        in("rdi") cpu_data,
        in("rsi") linux_sp,
        lateout("rax") ret,
        out("rcx") _,
    );
    Msr::IA32_GS_BASE.write(linux_tp);
    ret
}

#[naked]
#[no_mangle]
pub unsafe extern "C" fn arch_entry() -> i32 {
    asm!("
        // rip is pushed
        cli
        push rbp
        push rbx
        push r12
        push r13
        push r14
        push r15
        push 0  // skip gs_base

        mov rdi, rsp
        call {0}

        pop r15 // skip gs_base
        pop r15
        pop r14
        pop r13
        pop r12
        pop rbx
        pop rbp
        ret
        // rip will pop when return",
        sym switch_stack,
        options(noreturn),
    );
}
