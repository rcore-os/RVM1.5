#![allow(dead_code)]

use core::fmt::{Debug, Formatter, Result};
use core::marker::PhantomData;

use libvmm::vmx::vmcs::{VmcsField16Guest, VmcsField64Guest};
use x86::segmentation::SegmentSelector;
use x86_64::registers::control::{Cr0Flags, Cr4Flags};
use x86_64::registers::rflags::RFlags;

use super::super::GuestRegisters;
use super::Vcpu;
use crate::error::HvResult;
use crate::percpu::PerCpu;

pub trait Policy {}
pub trait Read: Policy {}
pub trait Write: Policy {}
pub enum In {}
pub enum Out {}

impl Policy for In {}
impl Policy for Out {}
impl Read for In {}
impl Write for Out {}

pub type VcpuGuestStateMut<'a> = VcpuGuestState<'a, Out>;

pub struct VcpuGuestState<'a, P: Policy = In> {
    vcpu: &'a mut Vcpu,
    mark: PhantomData<P>,
}

impl<'a, P: Policy> VcpuGuestState<'a, P> {
    #[allow(clippy::cast_ref_to_mut)]
    pub fn from(_cpu_data: &'a PerCpu) -> Self {
        Self {
            vcpu: &mut PerCpu::from_local_base_mut().vcpu,
            mark: PhantomData,
        }
    }

    pub fn regs(&self) -> &GuestRegisters {
        &self.vcpu.guest_regs
    }

    pub fn rip(&self) -> u64 {
        VmcsField64Guest::RIP.read().unwrap()
    }

    pub fn rsp(&self) -> u64 {
        VmcsField64Guest::RSP.read().unwrap()
    }

    pub fn rbp(&self) -> u64 {
        self.regs().rbp
    }

    pub fn rflags(&self) -> u64 {
        VmcsField64Guest::RFLAGS.read().unwrap()
    }

    pub fn cr(&self, cr_idx: usize) -> u64 {
        self.vcpu.get_guest_cr(cr_idx).unwrap()
    }
}

impl<'a> VcpuGuestStateMut<'a> {
    pub fn regs_mut(&mut self) -> &mut GuestRegisters {
        &mut self.vcpu.guest_regs
    }

    pub fn set_rsp(&mut self, rsp: u64) {
        VmcsField64Guest::RSP.write(rsp).unwrap()
    }
}

impl<'a, P: Policy> Debug for VcpuGuestState<'a, P> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        (|| -> HvResult<Result> {
            Ok(f.debug_struct("VcpuGuestState")
                .field("regs", &self.regs())
                .field("rip", &self.rip())
                .field("rsp", &self.rsp())
                .field("rflags", unsafe {
                    &RFlags::from_bits_unchecked(self.rflags())
                })
                .field("cr0", unsafe { &Cr0Flags::from_bits_unchecked(self.cr(0)) })
                .field("cr3", &self.cr(3))
                .field("cr4", unsafe { &Cr4Flags::from_bits_unchecked(self.cr(4)) })
                .field(
                    "cs",
                    &SegmentSelector::from_raw(VmcsField16Guest::CS_SELECTOR.read()?),
                )
                .field(
                    "ds",
                    &SegmentSelector::from_raw(VmcsField16Guest::DS_SELECTOR.read()?),
                )
                .field(
                    "es",
                    &SegmentSelector::from_raw(VmcsField16Guest::ES_SELECTOR.read()?),
                )
                .field(
                    "fs",
                    &SegmentSelector::from_raw(VmcsField16Guest::FS_SELECTOR.read()?),
                )
                .field("fs_base", &VmcsField64Guest::FS_BASE.read()?)
                .field(
                    "gs",
                    &SegmentSelector::from_raw(VmcsField16Guest::GS_SELECTOR.read()?),
                )
                .field("gs_base", &VmcsField64Guest::GS_BASE.read()?)
                .field(
                    "tss",
                    &SegmentSelector::from_raw(VmcsField16Guest::TR_SELECTOR.read()?),
                )
                .finish())
        })()
        .unwrap()
    }
}
