use libvmm::msr::Msr;

use super::cpuid::CpuId;

pub fn frequency() -> u16 {
    static CPU_FREQUENCY: spin::Once<u16> = spin::Once::new();
    *CPU_FREQUENCY.call_once(|| {
        const DEFAULT: u16 = 4000;
        CpuId::new()
            .get_processor_frequency_info()
            .map(|info| info.processor_base_frequency())
            .unwrap_or(DEFAULT)
            .max(DEFAULT)
    })
}

pub fn current_cycle() -> u64 {
    let mut aux = 0;
    unsafe { core::arch::x86_64::__rdtscp(&mut aux) }
}

pub fn current_time_nanos() -> u64 {
    current_cycle() * 1000 / frequency() as u64
}

pub fn thread_pointer() -> usize {
    let ret;
    unsafe { core::arch::asm!("mov {0}, gs:0", out(reg) ret, options(nostack)) }; // PerCpu::self_vaddr
    ret
}

pub fn set_thread_pointer(tp: usize) {
    unsafe { Msr::IA32_GS_BASE.write(tp as u64) };
}
