
// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use lazy_static::lazy_static;
use x86_64::structures::idt::{
    InterruptDescriptorTable, 
    InterruptStackFrame, 
    PageFaultErrorCode
};
use x86_64::instructions::port::Port;
use x86_64::registers::control::Cr2;
use pic8259_simple::ChainedPics;
use spin::Mutex;
use crate::{println, gdt};

#[cfg(test)]
use crate::{serial_print, serial_println};

// ---------------------------------------------------------------------------
// STATIC INITIALISATIONS
// ---------------------------------------------------------------------------

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

lazy_static! {
    /// The interrupt descriptor table.
    /// 
    /// Initialisation is done using lazy_static to allow a statically 
    /// allocated mutable reference.
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // ---- CPU EXCEPTIONS ----
        idt.breakpoint.set_handler_fn(breakpoint_hander);
        idt.page_fault.set_handler_fn(page_fault_handler);

        // NOTE: USE OF UNSAFE
        //  This code is unsafe since the argument to `set_stack_index` must
        //  be a valid stack index not used by any other interrupts. This is
        //  enforced by using the global constant index from the GDT module.
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        // ---- HARDWARE INTERRUPTS ----
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

/// Chained PIC static for dealing with hardware interrupts.
/// 
/// NOTE: USE OF UNSAFE
///     The use of unsafe here required since invalid offsets can cause 
///     undefined behaviour. Safety is enforced through the use of constants.
pub static PICS: Mutex<ChainedPics> = Mutex::new(
    unsafe{ ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

// ---------------------------------------------------------------------------
// DATA STRUCTURE DEFINITIONS
// ---------------------------------------------------------------------------

/// Interrupt index enum.
/// 
/// Contains the indexes of all hardware interrupts in the PICs.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// ---------------------------------------------------------------------------
// PUBLIC FUNCTIONS
// ---------------------------------------------------------------------------

/// Initialise the interrupt descriptor table.
pub fn init_idt() {
    IDT.load();
}

// ---------------------------------------------------------------------------
// CPU EXCEPTION HANDLER FUNCTIONS
// ---------------------------------------------------------------------------

/// Handle the breakpoint exception.
extern "x86-interrupt" fn breakpoint_hander(
    stack_frame: &mut InterruptStackFrame
) {
    println!("[CPU-EXCEPTION] BREAKPOINT\n{:#?}", stack_frame);
}

/// Handle double fault exception.
/// 
/// Note that unlike most handlers this one is diverging.
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame, 
    _error_code: u64
) -> ! {
    panic!("[CPU-EXCEPTION] DOUBLE FAULT\n{:#?}", stack_frame);
}

/// Handle page faults.
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode
) {
    println!("[CPU-EXCEPTION] PAGE FAULT");
    println!("Address accessed: {:?}", Cr2::read());
    println!("Error code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    crate::halt_loop();
}

// ---------------------------------------------------------------------------
// HARDWARE INTERRUPT HANDLER FUNCTIONS
// ---------------------------------------------------------------------------

/// Handle the hardware timer interrupt.
extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: &mut InterruptStackFrame
) {
    // TODO Perform timer syncing?

    // NOTE: USE OF UNSAFE
    //  Notify end of interrupt can be unsafe if the index is not valid. Safety
    //  is enforced by use of the `InterruptIndex` enum.
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

/// Handle keyboard interrupts by adding the scancode into the keyboard task 
/// queue.
extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: &mut InterruptStackFrame
) {

    // Get the keyboard port
    let mut port = Port::new(0x60);

    // Read the scancode and add it to the keyboard proc queue.
    //
    // NOTE: USE OF UNSAFE
    //  Reading from a port can be memory safety sideaffects. 
    //  FIXME: Safety mitigation
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::push_scancode(scancode);

    // NOTE: USE OF UNSAFE
    //  Notify end of interrupt can be unsafe if the index is not valid. Safety
    //  is enforced by use of the `InterruptIndex` enum.
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }

}

// ---------------------------------------------------------------------------
// TEST CASES
// ---------------------------------------------------------------------------

#[test_case]
fn test_breakpoint() {
    serial_print!("interrupts::breakpoint ");

    // Invoke the breakpoint exception
    x86_64::instructions::interrupts::int3();

    serial_println!("[ok]");
}