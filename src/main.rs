// MiniOS -> Custom OS by Vaibhav Gopalakrishnan
//
// An OS Kernal in a bare-metal executable
//
// This entire project is based off the "Blog OS" which details the instructions to build your own OS in rust
// https://os.phil-opp.com
//
// This project code cannot use ANY OS level abstractions (and by extension the Rust Standard Library and C Standard Library):
// - Threads, Files, Heap Memory, Networks, Random Numbers, Standard Output, + more
//
// We can however use:
// - Iterators, Closures, Pattern Matching, Option/Result, String formatting and the Ownership/borrowing system

// Disable the Standard Library
#![no_std]

// Don't use rust standard entry point chain, which uses an underlying C runtime library called crt0 and invokes a smaller rust runtime
// These runtime libraries set up the stack, registers, stack overflow guards, backtraces and finally call the main function
#![no_main]

// No std library --> Implement panic handling ourselves
use core::panic::PanicInfo;

// Called on panic --> loop infinitely for now --> diverging function returns "never" type
// FIXME: The duplicate lang item `panic_impl` error is cased by rust_analyzer in vscode --> FIXED, see .vscode/settings.json
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// We use no_mangle to tell rust not to generate some cryptic symbol for the _start function instead
// Use extern "C" to use the C calling convention instead of the unspecified rust calling convention which uses system defaults
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // this function is the entry point, since the linker looks for a function
    // named `_start` by default, (LLVM and LLD/Rust-LLD standards)
    loop {}
}
