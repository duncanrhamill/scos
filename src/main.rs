#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(scos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use core::panic::PanicInfo;
use scos::println;
use scos::task::{executor::Executor, Task, keyboard};
use bootloader::{BootInfo, entry_point};

#[cfg(not(test))]
use scos::vga_buffer;

// ---------------------------------------------------------------------------
// CORE FUNCTIONS
// ---------------------------------------------------------------------------

entry_point!(kernel_main);

/// Main entry point
/// 
/// This function provides the main entry point for the kernel. It is defined
/// as the entry point by the `entry_point` macro of the bootloader.
/// 
/// This is a diverging function as it cannot return anything.
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    
    println!("scos V0.1.0");

    // Perform main initialisation
    scos::init(boot_info);

    // Create and run task executor
    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

/// Panic handler for non-test builds.
/// 
/// On a panic this function will be called, it prints the panic info to the 
/// VGA buffer and then loops for ever.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Print a divider to clearly separate this from anything else
    vga_buffer::divider(b'-');
    println!("PANIC!\n");
    println!("{}", info);

    scos::halt_loop()
}

/// Panic handler for test builds.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    scos::test_panic_handler(info)
}