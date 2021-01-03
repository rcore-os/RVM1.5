use spin::Mutex;
use x86::{segmentation::SegmentSelector, task, Ring};
use x86_64::instructions::tables::{lgdt, lidt};
use x86_64::structures::gdt::{Descriptor, DescriptorFlags};
use x86_64::structures::idt::{Entry, HandlerFunc, InterruptDescriptorTable};
use x86_64::structures::{tss::TaskStateSegment, DescriptorTablePointer};

use super::segmentation::SegmentAccessRights;

const TSS: TaskStateSegment = TaskStateSegment::new();

lazy_static! {
    pub(super) static ref GDT: Mutex<GDTStruct> = Mutex::new(GDTStruct::new());
    pub(super) static ref IDT: Mutex<IDTStruct> = Mutex::new(IDTStruct::new());
}

#[derive(Debug)]
pub(super) struct GDTStruct {
    table: [u64; 16],
    pointer: DescriptorTablePointer,
}

impl GDTStruct {
    pub const KCODE_SELECTOR: SegmentSelector = SegmentSelector::new(1, Ring::Ring0);
    pub const TSS_SELECTOR: SegmentSelector = SegmentSelector::new(2, Ring::Ring0);

    pub fn new() -> Self {
        let mut table = [0; 16];
        table[1] = DescriptorFlags::KERNEL_CODE64.bits();
        let tss_desc = Descriptor::tss_segment(&TSS);
        match tss_desc {
            Descriptor::SystemSegment(low, high) => {
                table[2] = low;
                table[3] = high;
            }
            _ => unreachable!(),
        }
        Self {
            table,
            pointer: DescriptorTablePointer { limit: 0, base: 0 },
        }
    }

    pub fn sgdt() -> DescriptorTablePointer {
        let mut gdt_ptr = DescriptorTablePointer { limit: 0, base: 0 };
        unsafe { asm!("sgdt [{0}]", in(reg) &mut gdt_ptr) };
        gdt_ptr
    }

    pub fn lgdt(pointer: &DescriptorTablePointer) {
        unsafe { lgdt(pointer) };
    }

    pub fn table_of(pointer: &DescriptorTablePointer) -> &[u64] {
        let entry_count = (pointer.limit as usize + 1) / core::mem::size_of::<u64>();
        unsafe { core::slice::from_raw_parts(pointer.base as *const u64, entry_count) }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn table_of_mut(pointer: &DescriptorTablePointer) -> &mut [u64] {
        let entry_count = (pointer.limit as usize + 1) / core::mem::size_of::<u64>();
        unsafe { core::slice::from_raw_parts_mut(pointer.base as *mut u64, entry_count) }
    }

    pub fn pointer(&self) -> &DescriptorTablePointer {
        &self.pointer
    }

    pub fn load(&mut self) {
        self.pointer = DescriptorTablePointer {
            base: self.table.as_ptr() as _,
            limit: core::mem::size_of_val(&self.table) as u16 - 1,
        };
        Self::lgdt(&self.pointer());
    }

    pub fn load_tss(&mut self, selector: SegmentSelector) {
        assert_ne!(self.pointer.base, 0);
        SegmentAccessRights::set_descriptor_type(
            &mut Self::table_of_mut(&self.pointer)[selector.index() as usize],
            SegmentAccessRights::TSS_AVAIL,
        );
        unsafe { task::load_tr(selector) };
    }
}

pub(super) struct IDTStruct {
    table: InterruptDescriptorTable,
    pointer: DescriptorTablePointer,
}

impl IDTStruct {
    pub fn new() -> Self {
        extern "C" {
            #[link_name = "exception_entries"]
            static ENTRIES: [extern "C" fn(); 256];
        }

        let mut ret = Self {
            table: InterruptDescriptorTable::new(),
            pointer: DescriptorTablePointer { limit: 0, base: 0 },
        };
        let entries = unsafe {
            core::slice::from_raw_parts_mut(
                &mut ret.table as *mut _ as *mut Entry<HandlerFunc>,
                256,
            )
        };
        for i in 0..256 {
            entries[i].set_handler_fn(unsafe { core::mem::transmute(ENTRIES[i]) });
        }
        ret
    }

    pub fn sidt() -> DescriptorTablePointer {
        let mut idt_ptr = DescriptorTablePointer { limit: 0, base: 0 };
        unsafe { asm!("sidt [{0}]", in(reg) &mut idt_ptr) };
        idt_ptr
    }

    pub fn lidt(pointer: &DescriptorTablePointer) {
        unsafe { lidt(pointer) };
    }

    pub fn pointer(&self) -> &DescriptorTablePointer {
        &self.pointer
    }

    pub fn load(&mut self) {
        unsafe { self.table.load_unsafe() };
        self.pointer = Self::sidt();
    }
}
