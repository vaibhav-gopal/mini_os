#![no_std]
#![cfg_attr(test, no_main)] //conditionally add the no_main attribute when lib.rs is tested
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)] // see interrupts.rs

pub mod serial;
pub mod vga_buffer;
pub mod interrupts;
pub mod gdt;
pub mod memory;
pub mod allocator;

// use built-in alloc crate --> subset of the standard library --> building for custom target (have to recompile --> see .cargo/config.toml)
extern crate alloc;

use core::panic::PanicInfo;

// use the `hlt` instruction to create an energy-efficient endless loop rather than burning CPU resources
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

// lib.rs TESTS ================================================

#[test_case]
fn trivial_lib_assertion() {
    assert_eq!(1, 1);
}

// CONFIG TEST FUNCS (for main.rs, lib.rs and all integration tests)===============================

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

// Custom test runner function --> automatically runned by test_main() and inputs all test cases
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    // run all tests
    for test in tests {
        test.run();
    }
    // exit qemu --> cargo test considers all exit codes other than 0 to be failures, but we literally can't exit with code 0 as discussed above
    // b/c of qemu restrictions on isa-debug-exit --> workaround bootimage crate lets us remap exit codes, see Cargo.toml
    exit_qemu(QemuExitCode::Success);
}

// what to do when the test fails
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

// EXIT QEMU FUNCS ======================================

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

// INIT FUNCTIONS ====================================================

pub fn init() {
    gdt::init(); // initialize the Global Descriptor Table (GDT) and Task State Segment (TSS) needed by the IDT
    interrupts::init_idt(); // Set up the interrupt table (IDT: Interrupt Descriptor Table) to handle interrupts and handler functions
    unsafe { interrupts::PICS.lock().initialize() }; // Initialize both PIC's (primary and secondary) with our offsets
    x86_64::instructions::interrupts::enable(); // enable interrupts on our CPU
}

// ENTRY FUNCTIONS (for `cargo test` in lib.rs) =======================

#[cfg(test)]
use bootloader::{entry_point, BootInfo};

#[cfg(test)]
entry_point!(test_kernel_main); // no longer need to explicitly delcare _start entry point --> see main.rs

/// Entry point for `cargo test`
/// lib.rs is tested independently of main.rs so we need a entry point AND panic handler here too (only in test mode)
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    // like before
    init();
    test_main(); //test harness entry func --> see crate/lib attributes (top of file) and test runner
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}