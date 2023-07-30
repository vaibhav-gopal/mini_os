// for the theory see: https://os.phil-opp.com/cpu-exceptions/#overview
// there is a lot of "magic" that goes behind the scenes (setting up the stack, pointers, registers etc...)

// the x86 crate provides us with idt structs and enums to make setup easier
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::println;
use lazy_static::lazy_static;

// TESTS ===================================

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}

// END TESTS ===============================

// the idt struct has to have a static lifetime (i.e. live for the whole lifetime of the os) 
// b/c handling exceptions/interrupts are required till the very end of the program
// however we must also mutate the struct --> mutating statics (with no protection) is unsafe --> use lazy statics instead (still uses unsafe code bts)
lazy_static! {
    // create a new idt instance which contains fields for every type of interrupt
    // the breakpoint exception occurs when `int3` instruction is executed, temporarily pausing the program
    // we then set the handler function to handle that cpu exception
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            // switch to different stack before invoking handler function --> recover from stack overflow
            // and also prevent triple faults
            idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

// the "x86-interrupt" calling convention makes sure that all registers before the exception are preserved (typically by backing up to the stack)
// many required steps are also executed: (using the `iretq` instruction to return from the handler func, aligning the stack, etc...)
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

// the double fault handler must be a diverging function b/c x86 arch does not allow returning from a double fault exception
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}