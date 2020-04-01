#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use core::panic::PanicInfo;
use scos::{QemuExitCode, exit_qemu, serial, serial_print, serial_println};

// ---------------------------------------------------------------------------
// CORE FUNCTIONS
// ---------------------------------------------------------------------------

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    serial::divider(b'-');
    serial_println!("\nSCOS TESTS\n");
    serial_println!("Running {} tests", tests.len());
    serial::divider(b'-');
    serial_println!();

    for test in tests {
        test();
        serial_println!("[test did not panic]");
        exit_qemu(QemuExitCode::Failed);
    }

    serial::divider(b'-');
    serial_println!("\nTests complete\n");

    exit_qemu(QemuExitCode::Success);
}

// ---------------------------------------------------------------------------
// TESTS
// ---------------------------------------------------------------------------

#[test_case]
fn basic_assert() {
    serial_print!("should_panic::basic_assert ");
    assert_eq!(0, 1);
}