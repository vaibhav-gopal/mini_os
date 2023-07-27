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

// No std library --> Implement panic handling ourselves
use core::panic::PanicInfo;

// Called on panic --> loop infinitely for now --> diverging function returns "never" type
// FIXME: The duplicate lang item `panic_impl` error is cased by rust_analyzer in vscode --> FIXED, see .vscode/settings.json
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

//
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

// Custom test runner function --> automatically runned by test_main() and inputs all test cases
#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}

#[test_case]
fn trivial_assertion() {
    print!("trivial assertion... ");
    assert_eq!(1, 1);
    println!("[ok]");
}

// Use our vga text buffer implementation
mod vga_buffer;

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
