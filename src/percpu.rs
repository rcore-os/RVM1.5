use core::fmt::{Debug, Formatter, Result};
use core::mem::size_of;
use core::sync::atomic::{AtomicIsize, Ordering};

use crate::arch::vmm::{Vcpu, VcpuAccessGuestState};
use crate::arch::{HostPageTable, LinuxContext};
use crate::cell::Cell;
use crate::consts::{HV_STACK_SIZE, LOCAL_PER_CPU_BASE};
use crate::error::HvResult;
use crate::ffi::PER_CPU_ARRAY_PTR;
use crate::header::HvHeader;
use crate::memory::{addr::virt_to_phys, GenericPageTable, MemFlags, MemoryRegion, MemorySet};

pub const PER_CPU_SIZE: usize = size_of::<PerCpu>();

static ACTIVATED_CPUS: AtomicIsize = AtomicIsize::new(0);

#[derive(Debug, Eq, PartialEq)]
pub enum CpuState {
    HvDisabled,
    HvEnabled,
}

#[repr(align(4096))]
pub struct PerCpu {
    pub cpu_id: usize,
    pub state: CpuState,
    pub vcpu: Vcpu,
    stack: [usize; HV_STACK_SIZE / size_of::<usize>()],
    linux: LinuxContext,
    hvm: MemorySet<HostPageTable>,
}

impl PerCpu {
    pub fn from_id<'a>(cpu_id: usize) -> &'a Self {
        unsafe {
            &core::slice::from_raw_parts(PER_CPU_ARRAY_PTR, HvHeader::get().max_cpus as usize)
                [cpu_id]
        }
    }

    pub fn from_id_mut<'a>(cpu_id: usize) -> &'a mut Self {
        unsafe {
            &mut core::slice::from_raw_parts_mut(
                PER_CPU_ARRAY_PTR,
                HvHeader::get().max_cpus as usize,
            )[cpu_id]
        }
    }

    pub fn from_local_base<'a>() -> &'a Self {
        unsafe { &*(LOCAL_PER_CPU_BASE as *const Self) }
    }

    pub fn from_local_base_mut<'a>() -> &'a mut Self {
        unsafe { &mut *(LOCAL_PER_CPU_BASE as *mut Self) }
    }

    pub fn stack_top(&self) -> usize {
        self.stack.as_ptr_range().end as _
    }

    pub fn activated_cpus() -> usize {
        ACTIVATED_CPUS.load(Ordering::Acquire) as _
    }

    pub fn init(&mut self, cpu_id: usize, linux_sp: usize, cell: &Cell) -> HvResult {
        info!("CPU {} init...", cpu_id);

        self.cpu_id = cpu_id;
        self.state = CpuState::HvDisabled;
        self.linux = LinuxContext::load_from(linux_sp);

        let mut hvm = cell.hvm.read().clone();
        let vaddr = self as *const _ as usize;
        let paddr = virt_to_phys(vaddr);
        // Temporary mapping, will remove in Self::activate_vmm()
        hvm.insert(MemoryRegion::new_with_offset_mapper(
            vaddr,
            paddr,
            PER_CPU_SIZE,
            MemFlags::READ | MemFlags::WRITE,
        ))?;
        hvm.insert(MemoryRegion::new_with_offset_mapper(
            LOCAL_PER_CPU_BASE,
            paddr,
            PER_CPU_SIZE,
            MemFlags::READ | MemFlags::WRITE,
        ))?;
        trace!("PerCpu host virtual memory set: {:#x?}", hvm);
        unsafe {
            // avoid dropping, same below
            core::ptr::write(&mut self.hvm, hvm);
            self.hvm.activate();
            core::ptr::write(&mut self.vcpu, Vcpu::new(&self.linux, cell)?);
        }

        self.state = CpuState::HvEnabled;
        Ok(())
    }

    #[inline(never)]
    fn activate_vmm_local(&mut self) -> HvResult {
        self.vcpu.activate_vmm(&self.linux)?;
        unreachable!()
    }

    #[inline(never)]
    fn deactivate_vmm_common(&mut self) -> HvResult {
        self.vcpu.exit(&mut self.linux)?;
        self.linux.restore();
        self.state = CpuState::HvDisabled;
        self.vcpu.deactivate_vmm(&self.linux)?;
        unreachable!()
    }

    pub fn activate_vmm(&mut self) -> HvResult {
        println!("Activating hypervisor on CPU {}...", self.cpu_id);
        ACTIVATED_CPUS.fetch_add(1, Ordering::SeqCst);

        let local_cpu_data = Self::from_local_base_mut();
        let old_percpu_vaddr = self as *const _ as usize;
        // Switch stack to the private mapping.
        unsafe { asm!("add rsp, {}", in(reg) LOCAL_PER_CPU_BASE - old_percpu_vaddr) };
        local_cpu_data.hvm.delete(old_percpu_vaddr)?;
        local_cpu_data.hvm.page_table().flush(None);
        local_cpu_data.activate_vmm_local()
    }

    pub fn deactivate_vmm(&mut self, ret_code: usize) -> HvResult {
        println!("Deactivating hypervisor on CPU {}...", self.cpu_id);
        ACTIVATED_CPUS.fetch_add(-1, Ordering::SeqCst);

        self.vcpu.set_return_val(ret_code);

        // Restore full per_cpu region access so that we can switch
        // back to the common stack mapping and to Linux page tables.
        let common_cpu_data = Self::from_id_mut(self.cpu_id);
        let common_percpu_vaddr = common_cpu_data as *const _ as usize;

        let paddr = virt_to_phys(common_percpu_vaddr);
        self.hvm.insert(MemoryRegion::new_with_offset_mapper(
            common_percpu_vaddr,
            paddr,
            PER_CPU_SIZE,
            MemFlags::READ | MemFlags::WRITE,
        ))?;
        self.hvm.page_table().flush(None);
        unsafe { asm!("add rsp, {}", in(reg) common_percpu_vaddr - LOCAL_PER_CPU_BASE) };
        common_cpu_data.deactivate_vmm_common()
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
        res.field("cpu_id", &self.cpu_id)
            .field("state", &self.state);
        if self.state != CpuState::HvDisabled {
            res.field("vcpu", &self.vcpu);
        } else {
            res.field("linux", &self.linux);
        }
        res.finish()
    }
}
