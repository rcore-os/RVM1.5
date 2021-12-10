#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, allow(dead_code))]
#![feature(asm)]
#![feature(lang_items)]
#![feature(global_asm)]
#![feature(concat_idents)]
#![feature(naked_functions)]
#![allow(unaligned_references)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

#[macro_use]
mod logging;
#[macro_use]
mod error;

mod cell;
mod config;
mod consts;
mod ffi;
mod header;
mod hypercall;
mod memory;
mod percpu;
mod stats;

#[cfg(not(test))]
mod lang;

#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
mod arch;

use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use config::HvSystemConfig;
use error::HvResult;
use header::HvHeader;
use percpu::PerCpu;

static ENTERED_CPUS: AtomicUsize = AtomicUsize::new(0);
static INITED_CPUS: AtomicUsize = AtomicUsize::new(0);
static INIT_EARLY_OK: AtomicUsize = AtomicUsize::new(0);
static INIT_LATE_OK: AtomicUsize = AtomicUsize::new(0);
static ERROR_NUM: AtomicI32 = AtomicI32::new(0);

fn has_err() -> bool {
    ERROR_NUM.load(Ordering::Acquire) != 0
}

fn wait_for_other_completed(counter: &AtomicUsize, max_value: usize) -> HvResult {
    while !has_err() && counter.load(Ordering::Acquire) < max_value {
        core::hint::spin_loop();
    }
    if has_err() {
        hv_result_err!(EBUSY, "Other cpu init failed!")
    } else {
        Ok(())
    }
}

fn primary_init_early() -> HvResult {
    logging::init();
    info!("Primary CPU init early...");

    let system_config = HvSystemConfig::get();
    println!(
        "\n\
        Initializing hypervisor...\n\
        config_signature = {:?}\n\
        config_revision = {}\n\
        build_mode = {}\n\
        log_level = {}\n\
        arch = {}\n\
        vendor = {}\n\
        stats = {}\n\
        ",
        core::str::from_utf8(&system_config.signature),
        system_config.revision,
        option_env!("MODE").unwrap_or(""),
        option_env!("LOG").unwrap_or(""),
        option_env!("ARCH").unwrap_or(""),
        option_env!("VENDOR").unwrap_or(""),
        option_env!("STATS").unwrap_or("off"),
    );

    memory::init_heap();
    system_config.check()?;
    info!("Hypervisor header: {:#x?}", HvHeader::get());
    debug!("System config: {:#x?}", system_config);

    memory::init_frame_allocator();
    memory::init_hv_page_table()?;
    cell::init()?;

    INIT_EARLY_OK.store(1, Ordering::Release);
    Ok(())
}

fn primary_init_late() {
    info!("Primary CPU init late...");
    // Do nothing...
    INIT_LATE_OK.store(1, Ordering::Release);
}

fn main(cpu_data: &mut PerCpu, linux_sp: usize) -> HvResult {
    let online_cpus = HvHeader::get().online_cpus as usize;
    let is_primary = ENTERED_CPUS.fetch_add(1, Ordering::SeqCst) == 0;
    wait_for_other_completed(&ENTERED_CPUS, online_cpus)?;
    println!(
        "{} CPU {} entered.",
        if is_primary { "Primary" } else { "Secondary" },
        cpu_data.id
    );

    if is_primary {
        primary_init_early()?;
    } else {
        wait_for_other_completed(&INIT_EARLY_OK, 1)?;
    }

    cpu_data.init(linux_sp, cell::root_cell())?;
    println!("CPU {} init OK.", cpu_data.id);
    INITED_CPUS.fetch_add(1, Ordering::SeqCst);
    wait_for_other_completed(&INITED_CPUS, online_cpus)?;

    if is_primary {
        primary_init_late();
    } else {
        wait_for_other_completed(&INIT_LATE_OK, 1)?;
    }

    cpu_data.activate_vmm()
}

extern "sysv64" fn entry(cpu_data: &mut PerCpu, linux_sp: usize) -> i32 {
    if let Err(e) = main(cpu_data, linux_sp) {
        error!("{:?}", e);
        ERROR_NUM.store(e.code(), Ordering::Release);
    }
    let code = ERROR_NUM.load(Ordering::Acquire);
    println!(
        "CPU {} return back to driver with code {}.",
        cpu_data.id, code
    );
    code
}
