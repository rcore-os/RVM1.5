use core::fmt::{Debug, Formatter, Result};

use super::addr::{align_down, virt_to_phys};
use super::{AlignedPage, MemFlags, MemoryRegion, PhysAddr};

static EMPTY_PAGE: AlignedPage = AlignedPage::new();

#[derive(Clone)]
pub(super) struct Mapper {
    phys_virt_offset: Option<usize>,
}

impl Mapper {
    pub fn map_fn<VA: Into<usize>>(&self, vaddr: VA) -> PhysAddr {
        if let Some(offset) = self.phys_virt_offset {
            vaddr.into() - offset
        } else {
            virt_to_phys(EMPTY_PAGE.as_ptr() as usize)
        }
    }
}

impl Debug for Mapper {
    fn fmt(&self, f: &mut Formatter) -> Result {
        if let Some(offset) = self.phys_virt_offset {
            f.debug_struct("OffsetMapper")
                .field("phys_virt_offset", &offset)
                .finish()
        } else {
            f.debug_struct("EmptyMapper").finish()
        }
    }
}

impl<VA: From<usize> + Into<usize> + Copy> MemoryRegion<VA> {
    pub fn new_with_empty_mapper(start: VA, size: usize, flags: MemFlags) -> Self {
        Self::new(
            start,
            size,
            flags,
            Mapper {
                phys_virt_offset: None,
            },
        )
    }

    pub fn new_with_offset_mapper(
        start_vaddr: VA,
        start_paddr: PhysAddr,
        size: usize,
        flags: MemFlags,
    ) -> Self {
        let start_vaddr = align_down(start_vaddr.into());
        let start_paddr = align_down(start_paddr);
        let phys_virt_offset = Some(start_vaddr - start_paddr);
        Self::new(start_vaddr.into(), size, flags, Mapper { phys_virt_offset })
    }
}
