// more info on the GDT (global descriptor table) and TSS:
// https://web.archive.org/web/20190217233448/https://www.flingos.co.uk/docs/reference/Global-Descriptor-Table/ 
// https://en.wikipedia.org/wiki/X86_memory_segmentation
// https://pages.cs.wisc.edu/~remzi/OSTEP/
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use x86_64::VirtAddr;
use lazy_static::lazy_static;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static!{
    static ref TSS: TaskStateSegment = {
        // we create a new TSS instance --> create a new stack for all double fault exceptions for the CPU to switch to
        // this is for cases like stack overflow where new exceptions cause new faults --> prevent TRIPLE FAULTS
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            // manually create a stack via `static mut`, static b/c it is a stack and mut because we need to be able to change it
            // this is a very archaic stack definition --> there are no guard pages to protect against stack overflow corruption
            // also b/c we are using static muts and unsafe blocks directly
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            // in x86 stacks fill from high address to low address (top to bottom)
            // therefore we pass the stacks end pointer as the stack pointer
            let stack_start = VirtAddr::from_ptr(unsafe {&STACK});
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector
}

// x86 processers still use some basic form of the segmentation system (as opposed to memory paging, which is newer + better)
// in order for the CPU to use the TSS we need to set a segment descriptor pointing to the TSS segment
lazy_static!{
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors {code_selector, tss_selector})
    };
}

pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};

    GDT.0.load();
    unsafe {
        // since we created a new GDT (from the one the bootloader loads in) we have to reload the CS (code segment) register since the old one could point to something else
        // we also need to tell the cpu to use the TSS instance via the `ltr` x86 instruction
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}