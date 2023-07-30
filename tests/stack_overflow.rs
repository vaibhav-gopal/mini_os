#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)] // see below

use core::panic::PanicInfo;
use mini_os::{ serial_print, exit_qemu, QemuExitCode, serial_println };
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

// NOTE: this test does not have any test harness and test runner func --> see should_panic.rs for more info
// (this is why we must serial print the test name and other stuff)

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow...\t");

    // don't use the mini_os::init() func because we want to custom init_idt() func apart from the one in mini_os::interrupts::init_idt()
    mini_os::gdt::init();
    init_test_idt();

    // trigger a stack overflow
    stack_overflow();

    panic!("Execution continued after stack overflow");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    mini_os::test_panic_handler(info) // fail if execution panics rather than passing to handler function (see below)
}

#[allow(unconditional_recursion)] // silence compiler warning about endless recursion
fn stack_overflow() {
    stack_overflow(); // for each recursion, the return address is pushed
    volatile::Volatile::new(0).read(); // prevent tail recursion optimizations
}

// Custom IDT initialization ==============================

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(mini_os::gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}