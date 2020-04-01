#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(scos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use core::panic::PanicInfo;
use scos::{println, serial_print, serial_println};

// ---------------------------------------------------------------------------
// CORE FUNCTIONS
// ---------------------------------------------------------------------------

/// Main entry point for this test
#[no_mangle] 
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

/// Panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    scos::test_panic_handler(info)
}

// ---------------------------------------------------------------------------
// TESTS
// ---------------------------------------------------------------------------

#[test_case]
fn test_println() {
    serial_print!("basic_boot::println ");
    println!("Test println please ignore");
    serial_println!("[ok]");
}