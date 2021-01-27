pub fn id() -> usize {
    super::cpuid::CpuId::new()
        .get_feature_info()
        .unwrap()
        .initial_local_apic_id() as usize
}

pub fn time_now() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}
