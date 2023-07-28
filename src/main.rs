//! MiniOS -> Custom OS by Vaibhav Gopalakrishnan
//!
//! An OS Kernal in a bare-metal executable
//!
//! This entire project is based off the "Blog OS" which details the instructions to build your own OS in rust
//! https://os.phil-opp.com
//!
//! This project code cannot use ANY OS level abstractions (and by extension the Rust Standard Library and C Standard Library):
//! - Threads, Files, Heap Memory, Networks, Random Numbers, Standard Output, + more
//!
//! We can however use:
//! - Iterators, Closures, Pattern Matching, Option/Result, String formatting and the Ownership/borrowing system
//!
//! To run: `cargo run`
//! Uses QEMU to run (emulate) the OS

// Disable the Standard Library
#![no_std]

// Don't use rust standard entry point chain, which uses an underlying C runtime library called crt0 and invokes a smaller rust runtime
// These runtime libraries set up the stack, registers, stack overflow guards, backtraces and finally call the main function
#![no_main]

// Since we cannot use the test functionality provided by rust via std library, we implement our own via the custom test frameworks feature
// All tests will be passed to the test_runner function which we set up
// Also the entry point for when `cargo test` is run should change from the main() function (which we disabled) to test_main() instead
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Use our vga text buffer implementation
mod vga_buffer;

// use serial module to communicate with host system
mod serial;

// EXIT QEMU ======================================

// define an enum to represent our possible exit status', see exit_qemu() for more info
// we also represent the enum variants as u32 because we defined the "port size" as 4 bytes so u32 would equal the max value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    // enable use of special port I/O cpu instructions via rust abstractions
    use x86_64::instructions::port::Port;

    // passing a value into the isa-debug-exit QEMU port exits with an exit status of: "(value << 1) | 1"
    // the success and failed exit status codes don't matter as long we don't interefere with QEMU's default exit codes which mean special things
    // ex. we can't choose success to exit with 0 because that would mean "(0 << 1) | 1 = 1", and exit status 1 means there was an error in running QEMU
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

// PANIC HANDLING =================================

// No std library --> Implement panic handling ourselves
use core::panic::PanicInfo;

// Called on panic (not in test mode) --> loop infinitely for now --> diverging function returns "never" type
// FIXME: The duplicate lang item `panic_impl` error is cased by rust_analyzer in vscode --> FIXED, see .vscode/settings.json
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

// the panic handler when run `cargo test` --> print via serial to host system and exit qemu
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[Failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {} // still need this loop here because the compiler does not know that isa-debug-exit device and exit_qemu() func causes the program to exit
}

// CONFIG TEST FUNCS ===============================

// Custom test runner function --> automatically runned by test_main() and inputs all test cases
#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    // run all tests
    for test in tests {
        test.run();
    }
    // exit qemu --> cargo test considers all exit codes other than 0 to be failures, but we literally can't exit with code 0 as discussed above
    // b/c of qemu restrictions on isa-debug-exit --> workaround bootimage crate lets us remap exit codes, see Cargo.toml
    exit_qemu(QemuExitCode::Success);
}

pub trait Testable {
    fn run(&self) -> ();
}

// implement testable trait for all functions which implement Fn() which prints test messages to the host system via serial ports
// therefore no need to manually print serial messages in each test case code
impl<T> Testable for T
where 
    T: Fn(),
{
    fn run(&self) -> () {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

// TESTS ================================================

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

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

// TESTS END =============================================

// We use no_mangle to tell rust not to generate some cryptic symbol for the _start function instead
// Use extern "C" to use the C calling convention instead of the unspecified rust calling convention which uses system defaults
// this function is the entry point, since the linker looks for a function
// named `_start` by default, (LLVM and LLD/Rust-LLD standards)
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World!!!!");
    print!("Hello yet again :(");
    println!(" --> Some numbers: {} {}", 42, 1.337);

    #[cfg(test)]
    test_main();

    loop {}
}
