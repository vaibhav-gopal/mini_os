// for the theory see: https://os.phil-opp.com/cpu-exceptions/#overview
// there is a lot of "magic" that goes behind the scenes (setting up the stack, pointers, registers etc...)

// the x86 crate provides us with idt structs and enums to make setup easier
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::{println, print};
use lazy_static::lazy_static;
use spin::Mutex;
use pic8259::ChainedPics;
use crate::hlt_loop;

// the difference between hardware interrupts and cpu exceptions is that the former is asynchronous, but both are still interrupts by nature
// therefore both have entries in the IDT (interrupt descriptor table; in protected mode) and/or IVT (interrupt vector table ; in real mode)
// the default configuration for PIC uses interrupt index/vector numbers that overlap CPU exception interrupt index/vector numbers
// therfore we offset it --> 32-47 is typically chosen
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// we want safe mutable access to this struct so wrap it in a spinlock mutex
// we then create a ChainedPics (represents the chained primary/secondary PIC of the intel8259) instance with our offsets
pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

// TESTS ===================================

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}

// END TESTS ===============================

// Store different types of hardware interrupts for the intel 8259 as an enum
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET, // The timer is the first interrupt for the intel 8259 PIC
    Keyboard, // keyboard is the second interrupt --> no need for setting a value b/c it is assumed to be: prev + 1
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

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
        // InterruptDescriptorTable implements IndexMut which allows array indexing syntax -> set the timer interrupt handler func
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler); 
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler); // set keyboard interrupt handler func
        idt.page_fault.set_handler_fn(page_fault_handler); // set page fault handler
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

// the hardware timer interrupt handler --> notice the CPU reacts identically to CPU exceptions and external interrupts (proof: "x86-interrupt" ABI)
// only difference is that some exceptions push an error code
// the hardwire timer in this system is called the PIT chip
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");
    unsafe {
        // the intel 8259 PIC expects an EOI (end of interrupt signal) to continue processing interrupts
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

// Note: we can only handle PS/2 keyboards here, not USB keyboards. However, the mainboard/QEMU emulates USB keyboards as PS/2 devices
// so we can safely ignore USB keyboards until we have USB support in our kernel!
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(ScancodeSet1::new(), layouts::Us104Key,
                HandleControl::Ignore)
            );
    }

    let mut keyboard = KEYBOARD.lock(); // lock the mutex on each interrupt
    let mut port = Port::new(0x60); // set up the 0x60 port (data port for the PS/2 keyboard)

    let scancode: u8 = unsafe { port.read() }; // read the scancode from the keyboard
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) { // process/decode the scancode --> bind to key_event
        if let Some(key) = keyboard.process_keyevent(key_event) { // get only the key (not release or pressed info) --> bind to key
            match key {
                DecodedKey::Unicode(character) => print!("{}", character), // decoded key is either unicode or raw --> print it
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

// page fault occurs when accessing unmapped or out of bounds memory + others (different from segmentation fault)
// NOTE: guard pages (stack overflow protection) cause page faults to catch stack overflows, however when a stack overflow occurs
// two page faults will be called in succession because pushing the interrupt stack frame is also invalid, 
// which means even though we set up a page fault handler on stack overflow the double fault exception will be the one called
extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2; // cr2 register contains the virtual addr that caused the page fault

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}