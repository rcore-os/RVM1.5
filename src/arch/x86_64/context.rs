use libvmm::msr::Msr;
use x86::{segmentation, task};
use x86_64::registers::control::{Cr0, Cr0Flags, Cr3, Cr3Flags, Cr4, Cr4Flags};
use x86_64::{addr::PhysAddr, structures::paging::PhysFrame, structures::DescriptorTablePointer};

use super::segmentation::Segment;
use super::tables::{GdtStruct, IdtStruct};

const SAVED_LINUX_REGS: usize = 8;

#[derive(Debug)]
pub struct LinuxContext {
    pub rsp: u64,
    pub rip: u64,

    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,

    pub es: Segment,
    pub cs: Segment,
    pub ss: Segment,
    pub ds: Segment,
    pub fs: Segment,
    pub gs: Segment,
    pub tss: Segment,
    pub gdt: DescriptorTablePointer,
    pub idt: DescriptorTablePointer,

    pub cr0: Cr0Flags,
    pub cr3: u64,
    pub cr4: Cr4Flags,

    pub efer: u64,
    pub star: u64,
    pub lstar: u64,
    pub cstar: u64,
    pub fmask: u64,
    pub kernel_gsbase: u64,
    pub pat: u64,
    pub mtrr_def_type: u64,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct GeneralRegisters {
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rbx: u64,
    _unused_rsp: u64,
    pub rbp: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

macro_rules! save_regs_to_stack {
    () => {
        "
        push r15
        push r14
        push r13
        push r12
        push r11
        push r10
        push r9
        push r8
        push rdi
        push rsi
        push rbp
        sub rsp, 8
        push rbx
        push rdx
        push rcx
        push rax"
    };
}

macro_rules! restore_regs_from_stack {
    () => {
        "
        pop rax
        pop rcx
        pop rdx
        pop rbx
        add rsp, 8
        pop rbp
        pop rsi
        pop rdi
        pop r8
        pop r9
        pop r10
        pop r11
        pop r12
        pop r13
        pop r14
        pop r15"
    };
}

impl LinuxContext {
    /// Load linux callee-saved registers from the stack, and other system registers.
    pub fn load_from(linux_sp: usize) -> Self {
        let regs = unsafe { core::slice::from_raw_parts(linux_sp as *const u64, SAVED_LINUX_REGS) };
        let gdt = GdtStruct::sgdt();
        let mut fs = Segment::from_selector(segmentation::fs(), &gdt);
        let mut gs = Segment::from_selector(segmentation::gs(), &gdt);
        fs.base = Msr::IA32_FS_BASE.read();
        gs.base = regs[0];

        Self {
            rsp: regs.as_ptr_range().end as _,
            r15: regs[1],
            r14: regs[2],
            r13: regs[3],
            r12: regs[4],
            rbx: regs[5],
            rbp: regs[6],
            rip: regs[7],
            es: Segment::from_selector(segmentation::es(), &gdt),
            cs: Segment::from_selector(segmentation::cs(), &gdt),
            ss: Segment::from_selector(segmentation::ss(), &gdt),
            ds: Segment::from_selector(segmentation::ds(), &gdt),
            fs,
            gs,
            tss: Segment::from_selector(unsafe { task::tr() }, &gdt),
            gdt,
            idt: IdtStruct::sidt(),
            cr0: Cr0::read(),
            cr3: Cr3::read().0.start_address().as_u64(),
            cr4: Cr4::read(),
            efer: Msr::IA32_EFER.read(),
            star: Msr::IA32_STAR.read(),
            lstar: Msr::IA32_LSTAR.read(),
            cstar: Msr::IA32_CSTAR.read(),
            fmask: Msr::IA32_FMASK.read(),
            kernel_gsbase: Msr::IA32_KERNEL_GSBASE.read(),
            pat: Msr::IA32_PAT.read(),
            mtrr_def_type: Msr::IA32_MTRR_DEF_TYPE.read(),
        }
    }

    /// Restore system registers.
    pub fn restore(&self) {
        unsafe {
            Msr::IA32_EFER.write(self.efer);
            Msr::IA32_STAR.write(self.star);
            Msr::IA32_LSTAR.write(self.lstar);
            Msr::IA32_CSTAR.write(self.cstar);
            Msr::IA32_FMASK.write(self.fmask);
            Msr::IA32_KERNEL_GSBASE.write(self.kernel_gsbase);
            Msr::IA32_PAT.write(self.pat);

            Cr0::write(self.cr0);
            Cr4::write(self.cr4);
            // cr3 must be last in case cr4 enables PCID
            Cr3::write(
                PhysFrame::containing_address(PhysAddr::new(self.cr3)),
                Cr3Flags::empty(), // clear PCID
            );

            // Copy Linux TSS descriptor into our GDT, clearing the busy flag,
            // then reload TR from it. We can't use Linux' GDT as it is r/o.
            let mut hv_gdt = GdtStruct::from_pointer(&GdtStruct::sgdt());
            let liunx_gdt = GdtStruct::from_pointer(&self.gdt);
            let tss_idx = self.tss.selector.index() as usize;
            hv_gdt[tss_idx] = liunx_gdt[tss_idx];
            hv_gdt[tss_idx + 1] = liunx_gdt[tss_idx + 1];
            hv_gdt.load_tss(self.tss.selector);

            GdtStruct::lgdt(&self.gdt);
            IdtStruct::lidt(&self.idt);

            segmentation::load_es(self.es.selector);
            segmentation::load_cs(self.cs.selector);
            segmentation::load_ss(self.ss.selector);
            segmentation::load_ds(self.ds.selector);
            segmentation::load_fs(self.fs.selector);
            segmentation::load_gs(self.gs.selector);

            Msr::IA32_FS_BASE.write(self.fs.base);
        }
    }

    /// Restore linux general-purpose registers and stack, then return back to linux.
    pub fn return_to_linux(&self, guest_regs: &GeneralRegisters) -> ! {
        unsafe {
            Msr::IA32_GS_BASE.write(self.gs.base);
            core::arch::asm!(
                "mov rsp, {linux_rsp}",
                "push {linux_rip}",
                "mov rcx, rsp",
                "mov rsp, {guest_regs}",
                "mov [rsp + {guest_regs_size}], rcx",
                restore_regs_from_stack!(),
                "pop rsp",
                "ret",
                linux_rsp = in(reg) self.rsp,
                linux_rip = in(reg) self.rip,
                guest_regs = in(reg) guest_regs,
                guest_regs_size = const core::mem::size_of::<GeneralRegisters>(),
                options(noreturn),
            );
        }
    }
}
