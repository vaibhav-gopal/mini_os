use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;

// similar to the VGA buffer we create a static global serial "writer"
// We use lazy static because we have to dereference a raw pointer (port address via SerialPort::new()) at runtime b/c we can't at compile time
// We use mutex because we want to avoid data races when the writer is accessed from multiple processes and we still need interior mutability
// We spinlocks/spin mutexes rather than regular ones because we don't have the concept of threads and blocking (and other OS abstractions)
// We’re passing the port address 0x3F8, which is the standard port number for the first serial interface.
lazy_static!{
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

// IMPLEMENTING MACROS --> very similar to VGA buffer except SerialPort already implements Write trait which we don't need to do here
// the write trait implementation uses the SerialPort::send() function internally to send bytes through the port which we initialize on first use via lazy static

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}