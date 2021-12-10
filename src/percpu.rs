use core::fmt::{Debug, Formatter, Result};
use core::mem::size_of;
use core::sync::atomic::{AtomicIsize, Ordering};

use crate::arch::vmm::{Vcpu, VcpuAccessGuestState};
use crate::arch::{cpu, LinuxContext};
use crate::cell::Cell;
use crate::consts::HV_STACK_SIZE;
use crate::error::HvResult;
use crate::ffi::PER_CPU_ARRAY_PTR;
use crate::header::HvHeader;

pub const PER_CPU_SIZE: usize = size_of::<PerCpu>();

static ACTIVATED_CPUS: AtomicIsize = AtomicIsize::new(0);

#[derive(Debug, Eq, PartialEq)]
pub enum CpuState {
    HvDisabled,
    HvEnabled,
}

#[repr(C, align(4096))]
pub struct PerCpu {
    /// Referenced by arch::cpu::thread_pointer() for x86_64.
    self_vaddr: usize,

    pub id: usize,
    pub phys_id: usize,
    pub state: CpuState,
    pub vcpu: Vcpu,
    linux: LinuxContext,

    stack: [usize; HV_STACK_SIZE / size_of::<usize>()],
}

impl PerCpu {
    pub fn init_early<'a>(cpu_id: usize) -> &'a mut Self {
        let ret = unsafe {
            &mut core::slice::from_raw_parts_mut(
                PER_CPU_ARRAY_PTR,
                HvHeader::get().max_cpus as usize,
            )[cpu_id]
        };
        ret.id = cpu_id;
        ret.self_vaddr = ret as *const _ as usize;
        cpu::set_thread_pointer(ret.self_vaddr);
        ret
    }

    pub fn current<'a>() -> &'a Self {
        Self::current_mut()
    }

    pub fn current_mut<'a>() -> &'a mut Self {
        unsafe { &mut *(cpu::thread_pointer() as *mut Self) }
    }

    pub fn stack_top(&self) -> usize {
        self.stack.as_ptr_range().end as _
    }

    pub fn activated_cpus() -> usize {
        ACTIVATED_CPUS.load(Ordering::Acquire) as _
    }

    pub fn init(&mut self, linux_sp: usize, cell: &Cell) -> HvResult {
        info!("CPU {} init...", self.id);

        self.phys_id = cpu::phys_id();
        self.state = CpuState::HvDisabled;
        self.linux = LinuxContext::load_from(linux_sp);
        cpu::init();

        unsafe {
            // Activate hypervisor page table on each cpu.
            crate::memory::hv_page_table().activate();
            // avoid dropping, same below
            core::ptr::write(&mut self.vcpu, Vcpu::new(&self.linux, cell)?);
        }

        self.state = CpuState::HvEnabled;
        Ok(())
    }

    pub fn activate_vmm(&mut self) -> HvResult {
        println!("Activating hypervisor on CPU {}...", self.id);
        ACTIVATED_CPUS.fetch_add(1, Ordering::SeqCst);

        self.vcpu.enter(&self.linux)?;
        unreachable!()
    }

    pub fn deactivate_vmm(&mut self, ret_code: usize) -> HvResult {
        println!("Deactivating hypervisor on CPU {}...", self.id);
        ACTIVATED_CPUS.fetch_add(-1, Ordering::SeqCst);

        self.vcpu.set_return_val(ret_code);
        self.vcpu.exit(&mut self.linux)?;
        self.linux.restore();
        self.state = CpuState::HvDisabled;
        self.linux.return_to_linux(self.vcpu.regs());
    }

    pub fn fault(&mut self) -> HvResult {
        warn!("VCPU fault: {:#x?}", self);
        self.vcpu.inject_fault()?;
        Ok(())
    }
}

impl Debug for PerCpu {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut res = f.debug_struct("PerCpu");
        res.field("id", &self.id)
            .field("phys_id", &self.phys_id)
            .field("self_vaddr", &self.self_vaddr)
            .field("state", &self.state);
        if self.state != CpuState::HvDisabled {
            res.field("vcpu", &self.vcpu);
        } else {
            res.field("linux", &self.linux);
        }
        res.finish()
    }
}
