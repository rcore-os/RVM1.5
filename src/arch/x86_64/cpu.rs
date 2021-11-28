pub fn id() -> usize {
    super::cpuid::CpuId::new()
        .get_feature_info()
        .unwrap()
        .initial_local_apic_id() as usize
}

pub fn frequency() -> u16 {
    static CPU_FREQUENCY: spin::Once<u16> = spin::Once::new();
    *CPU_FREQUENCY.call_once(|| {
        const DEFAULT: u16 = 4000;
        super::cpuid::CpuId::new()
            .get_processor_frequency_info()
            .map(|info| info.processor_base_frequency())
            .unwrap_or(DEFAULT)
            .max(DEFAULT)
    })
}

pub fn current_cycle() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

pub fn current_time_nanos() -> u64 {
    current_cycle() * 1000 / frequency() as u64
}
