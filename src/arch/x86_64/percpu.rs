use libvmm::msr::Msr;
use x86::{segmentation, segmentation::SegmentSelector};

use super::tables::{GdtStruct, TssStruct, IDT};

pub struct ArchPerCpu {
    tss: TssStruct,
    gdt: GdtStruct,
}

impl ArchPerCpu {
    pub fn init(&mut self) {
        self.tss = TssStruct::alloc();

        self.gdt = GdtStruct::alloc();
        self.gdt.init(&self.tss);

        // Setup new GDT, IDT, CS, TSS
        self.gdt.load();
        unsafe {
            segmentation::load_es(SegmentSelector::from_raw(0));
            segmentation::load_cs(GdtStruct::KCODE_SELECTOR);
            segmentation::load_ss(SegmentSelector::from_raw(0));
            segmentation::load_ds(SegmentSelector::from_raw(0));
        }
        IDT.lock().load();
        self.gdt.load_tss(GdtStruct::TSS_SELECTOR);

        // PAT0: WB, PAT1: WC, PAT2: UC
        unsafe { Msr::IA32_PAT.write(0x070106) };
    }
}
