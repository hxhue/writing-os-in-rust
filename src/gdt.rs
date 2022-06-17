use lazy_static::lazy_static;
use x86_64::registers::segmentation::SegmentSelector;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    // In x64, TSS's functions change. It now only have 2 tables' information,
    // along with the I/O map base address.
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(unsafe {&STACK});
            let stack_end = stack_start + STACK_SIZE;
            // End of the chunk. Because stack grows from high address to low 
            // address.
            stack_end
        };
        tss
    };

    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        // add_entry() must be called in order.
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors {code_selector, tss_selector})
    };
}

pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};

    // Load our new GDT. Old temporary GDT set by "bootimage" will not used
    // anymore.
    GDT.0.load();
    // Since code and tss selector are indexes of GDT enties, they should be 
    // loaded after GDT.
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

