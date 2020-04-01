#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use core::panic::PanicInfo;
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use scos::{serial_print, serial_println, QemuExitCode, exit_qemu};

// ---------------------------------------------------------------------------
// FUNCTIONS
// ---------------------------------------------------------------------------

/// Main entry point for the test
#[no_mangle]
pub extern "C" fn _start() {
    serial_print!("stack_overflow ");

    // Initiailise necessary items
    scos::gdt::init();
    init_test_idt();

    // Trigger stack overflow
    stack_overflow();

    // Panic if we continue
    panic!("Execution continued after stack overflow");
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow()
}

/// Panic handler
#[panic_handler] 
fn panic(info: &PanicInfo) -> ! {
    scos::test_panic_handler(info)
}

// ---------------------------------------------------------------------------
// IDT RELATED ITEMS
// ---------------------------------------------------------------------------

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // NOTE: USE OF UNSAFE
        //  This code is unsafe since the argument to `set_stack_index` must
        //  be a valid stack index not used by any other interrupts. This is
        //  enforced by using the global constant index from the GDT module.
        unsafe {
            idt.double_fault.set_handler_fn(test_double_fault_handler)
                .set_stack_index(scos::gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

/// Double fault handler for use during this test.
/// 
/// Unlike the standard double fault handler this implementation prints an
/// "[ok]" message and exits from the simulation environment (QEMU).
extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}