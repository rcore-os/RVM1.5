#![allow(dead_code)]

use core::fmt::{Debug, Formatter, Result};
use core::marker::PhantomData;
use core::mem::size_of;

use super::addr::{page_offset, phys_to_virt, GuestPhysAddr, GuestVirtAddr};
use super::GenericPageTable;
use crate::arch::GuestPageTable;
use crate::error::HvResult;

pub struct GuestPtr<'a, T, P: Policy> {
    gvaddr: GuestVirtAddr,
    guest_pt: &'a GuestPageTable,
    mark: PhantomData<(T, P)>,
}

pub trait Policy {}
pub trait Read: Policy {}
pub trait Write: Policy {}
pub enum In {}
pub enum Out {}
pub enum InOut {}

impl Policy for In {}
impl Policy for Out {}
impl Policy for InOut {}
impl Read for In {}
impl Write for Out {}
impl Read for InOut {}
impl Write for InOut {}

pub type GuestInPtr<'a, T> = GuestPtr<'a, T, In>;
pub type GuestOutPtr<'a, T> = GuestPtr<'a, T, Out>;

pub trait AsGuestPtr: Copy {
    fn as_guest_ptr<T, P: Policy>(self, guest_pt: &GuestPageTable) -> GuestPtr<'_, T, P>;
}

impl AsGuestPtr for GuestVirtAddr {
    fn as_guest_ptr<T, P: Policy>(self, guest_pt: &GuestPageTable) -> GuestPtr<'_, T, P> {
        GuestPtr {
            gvaddr: self,
            guest_pt,
            mark: PhantomData,
        }
    }
}

impl AsGuestPtr for u64 {
    fn as_guest_ptr<T, P: Policy>(self, guest_pt: &GuestPageTable) -> GuestPtr<'_, T, P> {
        GuestPtr {
            gvaddr: self as _,
            guest_pt,
            mark: PhantomData,
        }
    }
}

impl<T, P: Policy> Debug for GuestPtr<'_, T, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:#x?}", self.gvaddr)
    }
}

impl<T, P: Policy> GuestPtr<'_, T, P> {
    pub fn _guest_vaddr(&self) -> GuestVirtAddr {
        self.gvaddr
    }

    pub fn as_guest_paddr(&self) -> HvResult<GuestPhysAddr> {
        Ok(self.guest_pt.query(self.gvaddr)?.0)
    }

    fn check_raw(addr: usize) -> HvResult {
        if addr == 0 {
            return hv_result_err!(EFAULT, "GuestPtr is null");
        }
        if addr % core::mem::align_of::<T>() != 0 {
            return hv_result_err!(EINVAL, format!("GuestPtr {:#x?} is not aligned", addr));
        }
        Ok(())
    }

    fn check(&self) -> HvResult {
        Self::check_raw(self.gvaddr)
    }
}

impl<T, P: Read> GuestPtr<'_, T, P> {
    pub fn read(&self) -> HvResult<T> {
        self.check()?;
        let mut ret = core::mem::MaybeUninit::uninit();
        let mut dst = ret.as_mut_ptr() as *mut u8;

        let mut gvaddr = self.gvaddr;
        let mut size = size_of::<T>();
        while size > 0 {
            let (gpaddr, _, pg_size) = self.guest_pt.query(gvaddr)?;
            let pgoff = pg_size.page_offset(gvaddr);
            let read_size = (pg_size as usize - pgoff).min(size);
            gvaddr += read_size;
            size -= read_size;
            unsafe {
                dst.copy_from_nonoverlapping(phys_to_virt(gpaddr) as *const _, read_size);
                dst = dst.add(read_size);
            }
        }
        unsafe { Ok(ret.assume_init()) }
    }

    pub fn read_from_gpaddr(gpaddr: GuestPhysAddr) -> HvResult<T> {
        Self::check_raw(gpaddr)?;
        let mut ret = core::mem::MaybeUninit::uninit();
        unsafe {
            (ret.as_mut_ptr() as *mut T)
                .copy_from_nonoverlapping(phys_to_virt(gpaddr) as *const T, 1);
            Ok(ret.assume_init())
        }
    }

    pub fn as_ref(&self) -> HvResult<&T> {
        self.check()?;
        let size = size_of::<T>();
        let (gpaddr, _, pg_size) = self.guest_pt.query(self.gvaddr)?;
        if page_offset(gpaddr) + size > pg_size as usize {
            return hv_result_err!(
                EINVAL,
                "GuestPtr::as_ref() requires data layout not to cross pages"
            );
        }
        let ptr = phys_to_virt(gpaddr) as *const _;
        unsafe { Ok(&*ptr) }
    }
}

impl<T, P: Write> GuestPtr<'_, T, P> {
    pub fn as_mut(&mut self) -> HvResult<&mut T> {
        self.check()?;
        let size = size_of::<T>();
        let (gpaddr, _, pg_size) = self.guest_pt.query(self.gvaddr)?;
        if page_offset(gpaddr) + size > pg_size as usize {
            return hv_result_err!(
                EINVAL,
                "GuestPtr::as_mut() requires data layout not to cross pages"
            );
        }
        let ptr = phys_to_virt(gpaddr) as *mut _;
        unsafe { Ok(&mut *ptr) }
    }
}
