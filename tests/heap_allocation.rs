#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(scos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use scos::{serial_print, serial_println};
use alloc::{boxed::Box, vec::Vec};

// ---------------------------------------------------------------------------
// CORE FUNCTIONS
// ---------------------------------------------------------------------------

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    scos::init(boot_info);

    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    scos::test_panic_handler(info)
}

#[test_case]
fn simple_allocation() {
    serial_print!("heap_allocation::simple_allocation ");
    let heap_val = Box::new(41);
    assert_eq!(*heap_val, 41);
    serial_println!("[ok]");
}

#[test_case]
fn large_vec() {
    serial_print!("heap_allocation::large_vec ");
    let n = 100;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
    serial_println!("[ok]");
}

#[test_case]
fn many_boxes() {
    serial_print!("heap_allocation::many_boxes ");
    for i in 0..scos::allocator::HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    serial_println!("[ok]");
}