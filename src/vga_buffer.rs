// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use volatile::Volatile;
use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use core::fmt::Write;

// Serial print imports for testing purposes
#[cfg(test)]
use crate::{serial_print, serial_println};

// ---------------------------------------------------------------------------
// VGA CHARACTER DISPLAY INFORMATION
// ---------------------------------------------------------------------------

/// Valid VGA colours.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Colour {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15
}

/// VGA DisplayCode including the background and foreground colors and whether 
/// or not to blink the character.
/// 
/// `repr(transparent)` ensures that a ColorCode struct has the same layout as
/// a `u8` byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct DisplayCode(u8);

impl DisplayCode {

    /// Create a new non-blinking `DisplayCode`.
    fn new(foreground: Colour, background: Colour) -> DisplayCode {
        DisplayCode((background as u8) << 4 | (foreground as u8))
    }

    /// Create a new blinking `DisplayCode`.
    #[allow(dead_code)]
    fn new_blink(foreground: Colour, background: Colour) -> DisplayCode {
        DisplayCode(
            (1u8 << 7) | (background as u8) << 4 | (foreground as u8))
    }
}

/// A single character to be displayed, including both the character and its
/// colors/blink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct DisplayChar {
    ascii_char: u8,
    display_code: DisplayCode
}

// ---------------------------------------------------------------------------
// VGA TEXT BUFFER
// ---------------------------------------------------------------------------

/// The height of the VGA buffer.
pub const BUFFER_HEIGHT: usize = 25;

/// The width of the VGA buffer.
pub const BUFFER_WIDTH: usize = 80;

/// Buffer object which encapsulates the VGA in-memory buffer.
/// 
/// `repr(transparent)` is used to ensure the buffer has the same size as its
/// `chars` array member.
#[repr(transparent)]
struct VgaBuffer {
    chars: [[Volatile<DisplayChar>; BUFFER_WIDTH]; BUFFER_HEIGHT]
}

/// Writer object which is used to write characters to the VGA buffer.
pub struct Writer {
    col_pos: usize,
    display_code: DisplayCode,
    buffer: &'static mut VgaBuffer
}

impl Writer {

    /// Write a single byte into the buffer on the bottom row of the buffer.
    pub fn write_byte(&mut self, byte: u8) {

        // If the byte to write is a new line we must handle that as a newline
        // print, otherwise write the byte.
        match byte {
            b'\n' => self.new_line(),
            byte => {
                // If at the right-hand edge of the screen add a new line 
                // before writing.
                if self.col_pos >= BUFFER_WIDTH {
                    self.new_line()
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.col_pos;

                // Put the byte in place with the current color code
                self.buffer.chars[row][col].write(DisplayChar {
                    ascii_char: byte,
                    display_code: self.display_code
                });

                // Increment the column position
                self.col_pos += 1;
            }
        }
    }

    /// Write a string on the bottom line of the terminal.
    pub fn write_string(&mut self, string: &str) {
        for byte in string.bytes() {
            // Since rust strings are UTF-8 we need to select only the 
            // printable VGA characters. Any other character gets a placeholder.
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe)
            }
        }
    }

    /// Handle a newline by moving the buffer upwards 1 row
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                // Get the character at this position
                let chr = self.buffer.chars[row][col].read();

                // Put the charcter into the row above
                self.buffer.chars[row - 1][col].write(chr);
            }
        }

        // Clear the final row and reset the column position
        self.clear_row(BUFFER_HEIGHT - 1);
        self.col_pos = 0;
    }

    /// Empty the indexed row of characters
    fn clear_row(&mut self, row: usize) {
        // Get the emtpy code
        let blank = DisplayChar {
            ascii_char: b' ',
            display_code: self.display_code
        };

        // Write the blank cols
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

// Format implementation so we can use format!.
impl fmt::Write for Writer {
    fn write_str(&mut self, string: &str) -> fmt::Result {
        self.write_string(string);
        Ok(())
    }
}

// Use lazy_static here to get around const limitations.
lazy_static! {

    /// Global spinlocked writer to provide access to the VGA buffer.
    /// 
    /// NOTE: USE OF UNSAFE
    ///     Static references are inerently unsafe, however this is linked 
    ///     directly to the VGA memory-mapped buffer, so it's OK.
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        col_pos: 0,
        display_code: DisplayCode::new(Colour::White, Colour::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut VgaBuffer) }
    });
}

// ---------------------------------------------------------------------------
// MACRO DEFINITIONS
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {

    // Print without interrupts enabled to avoid a deadlock when printing from 
    // an interrupt.
    x86_64::instructions::interrupts::without_interrupts(|| 
        WRITER.lock().write_fmt(args).unwrap()
    );
}

// ---------------------------------------------------------------------------
// PUBLIC FUNCTION DEFINITIONS
// ---------------------------------------------------------------------------

/// Divider function which prints a divider of the given character to the 
/// screen, filling the current row.
pub fn divider(chr: u8) {
    println!("\n{}", core::str::from_utf8(&[chr; BUFFER_WIDTH]).unwrap());
}

/// Set the colours of the VGA buffer.
/// 
/// Use `vga_buffer::reset_colour()` to return to the original colors.
pub fn set_colour(foreground: Colour, background: Colour) {
    WRITER.lock().display_code = DisplayCode::new(foreground, background);
}

/// Reset the VGA buffer colours to white on black.
pub fn reset_colour() {
    WRITER.lock().display_code = DisplayCode::new(Colour::White, Colour::Black);
}

// ---------------------------------------------------------------------------
// TEST FUNCTIONS
// ---------------------------------------------------------------------------

/// Test a simple `println!` macro to ensure panics don't occur.
#[test_case]
pub fn test_println_simple() {
    serial_print!("vga_buffer::println::simple ");
    println!("Hello world!");
    serial_println!("[ok]");
}

/// Test printing 10 times the height number of lines.
#[test_case]
pub fn test_println_many() {
    serial_print!("vga_buffer::println::many ");
    for _ in 0..(10 * BUFFER_HEIGHT) {
        println!("VGA_BUFFER::PRINTLN::MANY");
    }
    serial_println!("[ok]");
}

/// Test to see that the writer places the correct bytes in the VGA buffer 
/// memory.
#[test_case]
pub fn test_println_output() {
    serial_print!("vga_buffer::println::output ");
    
    let s = "A single string which fits in one line (<80 chars)";

    // To avoid a race condition where something may print to the screen as 
    // we're reading from the raw character list we must disable interrupts, 
    // print the string, lock the writer for the duration of the loop and then
    // read in the loop.

    x86_64::instructions::interrupts::without_interrupts(|| {
        // Get the writer and print to the screen, with a new line to guarentee
        // that the final line of the screen is going to be our printed string.
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("Writeln failed!");
        
        // Loop over the characters in the bottom line and check that they 
        // match those in the string.
        for (i, c) in s.chars().enumerate() {
            let vga_chr = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();
            assert_eq!(char::from(vga_chr.ascii_char), c);
        }
    });

    serial_println!("[ok]");
}