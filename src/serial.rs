// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;
use core::fmt::Write;

// ---------------------------------------------------------------------------
// SERIAL PORT OBJECTS AND CONSTANTS
// ---------------------------------------------------------------------------

lazy_static! {
    /// Serial port 1, using port `0x3F8`.
    pub static ref SERIAL1: Mutex<SerialPort> = {

        // NOTE: USE OF UNSAFE
        //  Unsafe usage here is because the argument to `SerialPort::new()` 
        //  must point to a valid serial port device.
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

pub const SERIAL_WIDTH: usize = 80;

// ---------------------------------------------------------------------------
// MACRO DEFINITIONS
// ---------------------------------------------------------------------------

/// Serial equivalent of print!.
/// 
/// This macro will print it's argument to serial port SERIAL1.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Serial equivalent of println!.
/// 
/// This macro will print it's argument to serial port SERIAL1 followed by a 
/// newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

// ---------------------------------------------------------------------------
// FUNCTION DEFINITIONS
// ---------------------------------------------------------------------------

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    // Disable interrupts for this print to ensure we don't get a deadlock
    // while printing to the serial port.
    x86_64::instructions::interrupts::without_interrupts(||
        SERIAL1.lock().write_fmt(args)
            .expect("Unable to print to serial port 1")
    );
}

pub fn divider(chr: u8) {
    serial_println!("\n{}", core::str::from_utf8(&[chr; SERIAL_WIDTH]).unwrap());
}
