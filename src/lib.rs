#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(const_fn)]
#![feature(alloc_error_handler)]
#![feature(const_in_array_repeat_expressions)]
#![feature(wake_trait)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

/// SCOS main library, containing infrastructure such as test runners and panic
/// handlers for integration with cargo xtest, initialisation functions.

// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use core::panic::PanicInfo;
use x86_64::VirtAddr;
use bootloader::BootInfo;

#[cfg(test)]
use bootloader::entry_point;

extern crate alloc;

// ---------------------------------------------------------------------------
// MODULE DECLARATIONS
// ---------------------------------------------------------------------------

pub mod vga_buffer;
pub mod serial;
pub mod interrupts;
pub mod gdt;
pub mod memory;
pub mod allocator;
pub mod task;

// ---------------------------------------------------------------------------
// MODULE USE STATEMENTS
// ---------------------------------------------------------------------------

use memory::BootInfoFrameAllocator;

// ---------------------------------------------------------------------------
// PUBLIC FUNCTION DEFINITIONS
// ---------------------------------------------------------------------------

#[cfg(test)]
entry_point!(test_kernel_main);

/// Main entry point for cargo xtest.
#[cfg(test)]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {

    init(boot_info);

    test_main();

    halt_loop();
}

/// Main initialisation function
pub fn init(boot_info: &'static BootInfo) {

    vga_buffer::divider(b'-');
    println!("Initialising kernel:\n");

    // Initialise GDT and IDT
    print!("GDT... ");
    gdt::init();
    println!("complete");

    print!("IDT... ");
    interrupts::init_idt();
    println!("complete");

    // Initialise the PICs and enable interrupts
    //
    // NOTE: USE OF UNSAFE
    //  The initialisation of a misconfigured ChainedPic object can cause 
    //  undefined behaviour. Safety is enforced through use only in the init 
    //  function.
    print!("PICs... ");
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    println!("complete, interrupts enabled");

    // ---- HEAP INITIALISATION ----

    // Initialise the memory mapper
    print!("Memory mapper... ");
    let phys_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_offset) };
    println!("complete");

    // Initialise the frame allocator
    print!("Frame allocator... ");
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    println!("complete");

    print!("Kernel heap... ");
    let heap_info = allocator::init_heap(
        &mut mapper, &mut frame_allocator).expect("failed");
    println!("complete");
    println!("Kernel heap information: \n{:#?}", heap_info);

    // End of initialisations
    println!("\nInitialisation complete");
    vga_buffer::divider(b'-');
}

/// Enter a low power looping halt mode.
/// 
/// Interrupts will be handled but the CPU won't run at high speed, reducing 
/// power usage.
pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Main test runner
pub fn test_runner(tests: &[&dyn Fn()]) {
    serial::divider(b'-');
    serial_println!("\nSCOS TESTS\n");
    serial_println!("Running {} tests", tests.len());
    serial::divider(b'-');
    serial_println!();

    for test in tests {
        test();
    }

    serial::divider(b'-');
    serial_println!("\nTests complete\n");

    // Exit from the tests (assuming QEMU)
    exit_qemu(QemuExitCode::Success);
}

/// Panic handler for test builds.
/// 
/// On a panic this function will be called, it prints the panic info to the 
/// SERIAL1 serial port, exits qemu, and loops forever.
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    // Print a divider to clearly separate this from anything else
    serial::divider(b'-');
    serial_println!("PANIC DURING TEST!\n");
    serial_println!("{}", info);
    exit_qemu(QemuExitCode::Failed);
    
    halt_loop()
}

/// Exit from a QEMU session by writing to the exit port.
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;
    
    // NOTE: USE OF UNSAFE
    //  The write function is declared as unsafe because I/O port writing is
    //  not guarenteed to avoid memory kerfuffles.
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

/// Panic handler override
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

// ---------------------------------------------------------------------------
// DATA STRUCTURES
// ---------------------------------------------------------------------------

/// Exit codes for use in QEMU execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}
