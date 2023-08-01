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
#![test_runner(mini_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use mini_os::{print, println};

// Heap allocation --> see allocator.rs
extern crate alloc;
use alloc::{
    boxed::Box,
    vec,
    vec::Vec,
    rc::Rc,
};

use bootloader::{BootInfo, entry_point};

// here we chain the _start func to a regular rust function (i.e. _start() is still explicitly called under the hood with no mangle, extern "C", etc...)
// this is to apply signature/type checking
entry_point!(kernel_main); 

// No std library --> Implement panic handling ourselves
use core::panic::PanicInfo;

// main.rs TESTS ================================================

#[test_case]
fn trivial_main_assertion() {
    assert_eq!(1, 1);
}

// ENTRY/PANIC FUNCTIONS ======================================

// We use no_mangle to tell rust not to generate some cryptic symbol for the _start function instead
// Use extern "C" to use the C calling convention instead of the unspecified rust calling convention which uses system defaults
// this function is the entry point, since the linker looks for a function
// named `_start` by default, (LLVM and LLD/Rust-LLD standards)
// THIS EXPLICIT ENTRY POINT IS NO LONGER REQUIRED --> See kernel_main() and entry_point! macro
// #[no_mangle]
// pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
//     println!("Hello World!!!");
//     mini_os::init();
//     #[cfg(test)]
//     test_main();
//     print!("Hello yet again :( --> ");
//     println!("It did not crash!");
//     println!("Some numbers: {} {}", 42, 1.337);
//     mini_os::hlt_loop();
// }

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World!!!!");
    mini_os::init();
    #[cfg(test)]
    test_main();

    {
        // ALLOCATOR/PAGING SETUP ==========================
        use mini_os::memory::{ BootInfoFrameAllocator, self };
        use mini_os::allocator;
        use x86_64::{structures::paging::Page, VirtAddr};

        let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
        let mut mapper = unsafe { memory::init(phys_mem_offset) };
        let mut frame_allocator = unsafe {
            BootInfoFrameAllocator::init(&boot_info.memory_map)
        };
    
        // TEST PAGING ALLOCATION AND WRITE CODE ======================
        // map an unused page
        let page = Page::containing_address(VirtAddr::new(0));
        memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);
    
        // write the string `New!` to the screen through the new mapping
        let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
        unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};

        // HEAP ALLOCATION =======================================
        // initialize the heap
        allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

        // test --> allocate a number on the heap 
        let heap_value = Box::new(41);
        println!("heap_value at {:p}", heap_value);

        // test --> create a dynamically sized vector
        let mut vec = Vec::new();
        for i in 0..500 {
            vec.push(i);
        }
        println!("vec at {:p}", vec.as_slice());

        // test --> create a reference counted vector -> will be freed when count reaches 0
        let reference_counted = Rc::new(vec![1, 2, 3]);
        let cloned_reference = reference_counted.clone();
        println!("current reference count is {}", Rc::strong_count(&cloned_reference));
        core::mem::drop(reference_counted);
        println!("reference count is {} now", Rc::strong_count(&cloned_reference));
    }
    

    print!("Heelo yet again :< --> ")    ;
    println!("It did not crash!");
    println!("Some numbers: {} {}", 42, 1.337);
    mini_os::hlt_loop();
}

// Called on panic (not in test mode) --> loop infinitely for now --> diverging function returns "never" type
// FIXME: The duplicate lang item `panic_impl` error is cased by rust_analyzer in vscode --> FIXED, see .vscode/settings.json
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    mini_os::hlt_loop();
}
// the panic handler when run `cargo test` --> print via serial to host system and exit qemu
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    mini_os::test_panic_handler(info);
}