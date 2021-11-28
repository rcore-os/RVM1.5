use core::fmt::{Debug, Formatter, Result};
use core::{mem::size_of, slice};

use crate::consts::HV_BASE;
use crate::header::HvHeader;
use crate::memory::MemFlags;
use crate::percpu::PER_CPU_SIZE;

const HV_CELL_NAME_MAXLEN: usize = 31;

/// The jailhouse cell configuration.
///
/// @note Keep Config._HEADER_FORMAT in jailhouse-cell-linux in sync with this
/// structure.
#[derive(Debug)]
#[repr(C, packed)]
pub struct HvCellDesc {
    signature: [u8; 6],
    revision: u16,
    name: [u8; HV_CELL_NAME_MAXLEN + 1],
    id: u32, // set by the driver
    pub num_memory_regions: u32,
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct HvMemoryRegion {
    pub phys_start: u64,
    pub virt_start: u64,
    pub size: u64,
    pub flags: MemFlags,
}

/// General descriptor of the system.
#[derive(Debug)]
#[repr(C, packed)]
pub struct HvSystemConfig {
    pub signature: [u8; 6],
    pub revision: u16,
    /// Jailhouse's location in memory
    pub hypervisor_memory: HvMemoryRegion,
    pub root_cell: HvCellDesc,
    // CellConfigLayout placed here.
}

/// A dummy layout with all variant-size fields empty.
#[derive(Debug)]
#[repr(C, packed)]
struct CellConfigLayout {
    mem_regions: [HvMemoryRegion; 0],
}

pub struct CellConfig<'a> {
    desc: &'a HvCellDesc,
}

impl HvCellDesc {
    pub const fn config(&self) -> CellConfig {
        CellConfig::from(self)
    }

    pub const fn config_size(&self) -> usize {
        self.num_memory_regions as usize * size_of::<HvMemoryRegion>()
    }
}

impl HvSystemConfig {
    pub fn get<'a>() -> &'a Self {
        let header = HvHeader::get();
        let core_and_percpu_size =
            header.core_size as usize + header.max_cpus as usize * PER_CPU_SIZE;
        unsafe { &*((HV_BASE + core_and_percpu_size) as *const Self) }
    }

    pub const fn size(&self) -> usize {
        size_of::<Self>() + self.root_cell.config_size()
    }
}

impl<'a> CellConfig<'a> {
    const fn from(desc: &'a HvCellDesc) -> Self {
        Self { desc }
    }

    fn config_ptr<T>(&self) -> *const T {
        unsafe { (self.desc as *const HvCellDesc).add(1) as _ }
    }

    pub const fn size(&self) -> usize {
        self.desc.config_size()
    }

    pub fn mem_regions(&self) -> &[HvMemoryRegion] {
        unsafe {
            let ptr = self.config_ptr() as _;
            slice::from_raw_parts(ptr, self.desc.num_memory_regions as usize)
        }
    }
}

impl Debug for CellConfig<'_> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("CellConfig")
            .field("size", &self.size())
            .field("mem_regions", &self.mem_regions())
            .finish()
    }
}
