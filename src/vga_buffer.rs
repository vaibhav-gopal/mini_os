// A vga text buffer is a special area in memory (0xb8000) that displays its contents on the screen
// 25 lines with 80 character cells each --> each character cell has an ASCII character + foreground and background colors + blink
// Each character cell in the vga buffer is represented by 2 bytes:
// Bits 0-7 = ASCII character
// Bits 8-11 = Foreground Color w/ Bit 4 = bright modifier
// Bits 12-15 = Background Color w/ Bit 5 = blink modifier

// As to how we are able to access I/O only by accessing memory is b/c of "memory mapped I/O"

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

// Use volatile library to wrap certain types so they don't get optimized by the compiler
use volatile::Volatile;

// Implement rusts formatting macros to use write! macro for our vga buffer
use core::fmt;

// Use lazy statics to dereference raw pointers in static variables
use lazy_static::lazy_static;

// Use spinning mutexes (spinlocks) rather than regular mutexes which require blocking support and threads (which we don't have)
// now why do we need "safe interior mutability" if our kernal won't even have the concept of threads in the first place!!!???
// Answer --> From the perspective of the compiler an interrupt handler is a thread. The interrupt handler could run in the middle of a write operation,
//   thus seeing a partial write, a write that never actually happened in the program you wrote (for example because the compiler stored a temporary value there
//   to save stack space and would write the correct value at a later time) or any other bad thing. Because interrupt handlers are implemented in this kernel,
//   we have data races. Also data races are not the only issue. It is for example not acceptable to have two mutable references to the same static mut, just like any other data.
//   static mut however makes it quite easy to cause two mutable references to it at the same time.
use spin::Mutex;

// Represent different colors as an enum --> each enum variant stored as a u8
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
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
    White = 15,
}

// A struct to represent the full color byte (the second byte in each character cell)
// Use repr(transparent) b/c "we have to use the exact same data layout as u8/Color"
// I'm guessing this is similar to just doing `type ColorCode = u8;`... research more...
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    // the x << n bitwise operator shifts all bits of x to the left by n
    // the x | n bitwise operator compares bit-to-bit of x and n and 
    //  returns a value where the bit is changed to 1 if either the bit from x or n was 1
    // i.e. 0101 | 0010 --> 0111
    // we use it in this case to ammend the foreground as the first 4 bits of the second byte and the background as the last 4 bits
    fn new(foreground: Color, background: Color) -> Self {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

// A struct that represents the full 2 bytes of data for each character cell
// We use repr(C) b/c field ordering is undefined in Rust by default --> whereas C structs are ordered as coded (we are not actually using C code here, just their implementations)
// We need this b/c the ASCII character code must come before the color code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

// a struct that represents the entire vga buffer using a 2D array of ScreenChar elements of size BUFFER_WIDTH and BUFFER_HEIGHT (80 by 25)
// again we use repr(transparent) to "ensure it has the same memory layout as its single field" --> no extra struct wrappers? (research more)
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// a writer struct that keeps track of the current position, color codes and a mutable reference to the vga buffer to write to it
// we need an explicit 'static lifetime here --> so we tell the compiler that this reference should be valid for the whole program, even if writer gets deallocated (i.e. the buffer MUST be initialized at the global scope)
//  Remember that lifetime specifiers don't actually do anything (exception of 'static in certain situations), they just help the compiler detect issues
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// To write to the buffer we will always be on the last row and add characters until the row is full or we encounter a newline character
// then we create a newline and continue the process
impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;
                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    // write the byte in the string if within printable ASCII characters range or if it is a newline character
    // otherwise we print a miscilanious spacer character 0xfe --> '■'
    // Use the write_str() method instead of this
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe)
            }
        }
    }

    fn new_line(&mut self) {
        // shift every character in a line to the line above (the top-most line gets deleted instead)
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    // clears the row by writing a blank character to every cell in the row
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

// -> Implement a global static writer, so other modules don't have to carry a spare writer instance
// problems occur --> we cannot dereference raw pointers in static variables as they are initialized at compile time
// -> Use the lazy_static crate which gives lazily evaluated static variables which are evaluated at runtime instead
// -> The buffer code may seem unusual but it is simple,
// First we set a mutable raw pointer to a Buffer type to the address 0xb8000 (which is where the VGA buffer lives)
// Then we dereference it --> giving us a Buffer type in memory and get a mutable reference to it instead
// This ensures that we use rust references rather than manipulating raw pointers which would result in unsafe blocks being in the writer implementation instead
// Rather we use a one-time unsafe block to access a specific location in memory
lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer)}
    });
}

// Redefine the println!() and print!() macro to our implementation (spinning mutex, and write into the vga buffer)

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
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

// TESTS =====================================

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

//test to verify that the characters printed to the VGA buffer are actually there
#[test_case]
fn test_println_output() {
    let s = "Some test string that fits on a single line";
    println!("{}", s);
    for (i, c) in s.chars().enumerate() {
        let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT - 2][i].read();
        assert_eq!(char::from(screen_char.ascii_character), c);
    }
}

// TESTS END ===================================