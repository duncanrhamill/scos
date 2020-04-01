// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use x86_64::{VirtAddr, structures::tss::TaskStateSegment};
use x86_64::structures::gdt::{
    GlobalDescriptorTable, Descriptor, SegmentSelector
};
use x86_64::instructions::{segmentation::set_cs, tables::load_tss};
use lazy_static::lazy_static;

// ---------------------------------------------------------------------------
// STATIC INITIALISATIONS
// ---------------------------------------------------------------------------

/// The index of the double fault CPU exception in the Interrupt Stack Table.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    /// Task State Segment static reference.
    /// 
    /// This is initialised using `lazy_static` so that we get advanced static
    /// init capabilities.
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        // TODO: Use proper stack initialisation once memory management is 
        // added.
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            // NOTE: USE OF UNSAFE
            //  The reference to a mutable static here is unsafe because the
            //  compiler can't guarentee race condition safety with mutable
            //  statics. This will be removed when the above TODO is solved.
            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };

        tss
    };
}

lazy_static! {
    /// Global descriptor table.
    /// 
    /// This is initialised using `lazy_static` so that we get advanced static
    /// init capabilities.
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();

        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));

        (gdt, Selectors { code_selector, tss_selector })
    };
}

// ---------------------------------------------------------------------------
// STRUCTURE DEFINITIONS
// ---------------------------------------------------------------------------

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector
}

// ---------------------------------------------------------------------------
// PUBLIC FUNCTIONS
// ---------------------------------------------------------------------------

/// Initialise the Global Descriptor Table.
pub fn init() {

    // Load the GDT
    GDT.0.load();
    
    // Register the selectors
    //
    // NOTE: USE OF UNSAFE
    //  The `set_cs` and `load_tss` functions are marked as unsafe so the 
    //  unsafe block is required here. This is because loading tables and
    //  setting selectors could cause some trouble if they are not valid. This
    //  usage here is OK since it's within an init function.
    unsafe {
        set_cs(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}