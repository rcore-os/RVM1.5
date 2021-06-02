#![allow(dead_code)]

use core::fmt::{Debug, Formatter, Result};
use core::marker::PhantomData;
use core::mem::size_of;

use super::addr::{page_offset, phys_to_virt, GuestPhysAddr, GuestVirtAddr};
use super::GenericPageTableImmut;
use crate::arch::GuestPageTableImmut;
use crate::error::HvResult;

pub struct GuestPtr<'a, T> {
    gvaddr: GuestVirtAddr,
    guest_pt: &'a GuestPageTableImmut,
    mark: PhantomData<T>,
}

pub trait AsGuestPtr: Copy {
    fn as_guest_ptr<T>(self, guest_pt: &GuestPageTableImmut) -> GuestPtr<'_, T>;
}

impl AsGuestPtr for GuestVirtAddr {
    fn as_guest_ptr<T>(self, guest_pt: &GuestPageTableImmut) -> GuestPtr<'_, T> {
        GuestPtr {
            gvaddr: self,
            guest_pt,
            mark: PhantomData,
        }
    }
}

impl AsGuestPtr for u64 {
    fn as_guest_ptr<T>(self, guest_pt: &GuestPageTableImmut) -> GuestPtr<'_, T> {
        (self as GuestVirtAddr).as_guest_ptr(guest_pt)
    }
}

impl<T> Debug for GuestPtr<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:#x?}", self.gvaddr)
    }
}

impl<T> GuestPtr<'_, T> {
    pub fn guest_vaddr(&self) -> GuestVirtAddr {
        self.gvaddr
    }

    pub fn as_guest_paddr(&self) -> HvResult<GuestPhysAddr> {
        let gpaddr = self.guest_pt.query(self.gvaddr)?.0;
        Self::check_gpaddr(gpaddr)?;
        Ok(gpaddr)
    }

    fn check_gpaddr(_gpaddr: GuestPhysAddr) -> HvResult {
        // TODO
        Ok(())
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

    fn check_ptr(&self) -> HvResult {
        Self::check_raw(self.gvaddr)
    }

    pub fn read(&self) -> HvResult<T> {
        self.check_ptr()?;
        let mut ret = core::mem::MaybeUninit::uninit();
        let mut dst = ret.as_mut_ptr() as *mut u8;

        let mut gvaddr = self.gvaddr;
        let mut size = size_of::<T>();
        while size > 0 {
            let (gpaddr, _, pg_size) = self.guest_pt.query(gvaddr)?;
            Self::check_gpaddr(gpaddr)?;
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

    pub fn _write(&mut self, data: T) -> HvResult {
        self.check_ptr()?;
        let mut src = &data as *const _ as *const u8;

        let mut gvaddr = self.gvaddr;
        let mut size = size_of::<T>();
        while size > 0 {
            let (gpaddr, _, pg_size) = self.guest_pt.query(gvaddr)?;
            Self::check_gpaddr(gpaddr)?;
            let pgoff = pg_size.page_offset(gvaddr);
            let write_size = (pg_size as usize - pgoff).min(size);
            gvaddr += write_size;
            size -= write_size;
            let dst = phys_to_virt(gpaddr) as *mut u8;
            unsafe {
                dst.copy_from_nonoverlapping(src, write_size);
                src = src.add(write_size);
            }
        }
        Ok(())
    }

    pub fn as_ref(&self) -> HvResult<&T> {
        self.check_ptr()?;
        let size = size_of::<T>();
        let (gpaddr, _, pg_size) = self.guest_pt.query(self.gvaddr)?;
        Self::check_gpaddr(gpaddr)?;
        if page_offset(gpaddr) + size > pg_size as usize {
            return hv_result_err!(
                EINVAL,
                "GuestPtr::as_ref() requires data layout not to cross pages"
            );
        }
        let ptr = phys_to_virt(gpaddr) as *const _;
        unsafe { Ok(&*ptr) }
    }

    pub fn as_mut(&mut self) -> HvResult<&mut T> {
        self.check_ptr()?;
        let size = size_of::<T>();
        let (gpaddr, _, pg_size) = self.guest_pt.query(self.gvaddr)?;
        Self::check_gpaddr(gpaddr)?;
        if page_offset(gpaddr) + size > pg_size as usize {
            return hv_result_err!(
                EINVAL,
                "GuestPtr::as_mut() requires data layout not to cross pages"
            );
        }
        let ptr = phys_to_virt(gpaddr) as *mut _;
        unsafe { Ok(&mut *ptr) }
    }

    pub fn gpaddr_to_ref_mut(gpaddr: GuestPhysAddr) -> HvResult<&'static mut T> {
        Self::check_raw(gpaddr)?;
        Self::check_gpaddr(gpaddr)?;
        let ptr = unsafe { &mut *(phys_to_virt(gpaddr) as *mut T) };
        Ok(ptr)
    }
}
