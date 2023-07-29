#![no_std]
#![no_main]

use core::panic::PanicInfo;
use mini_os::{exit_qemu, serial_print, serial_println, QemuExitCode};

// NOTE: this test does not have any test / test-runner functionality b/c there is only 1 test and that test needs to fail
// --> therefore you must set the "harness" for this test to equal "false" in Cargo.toml and all similar tests that need to fail
// Instead we just directly call the testing function in the entry _start() function

// MAIN TEST ================================================

fn should_fail() {
    serial_print!("should_panic::should_fail...\t");
    assert_eq!(0, 1);
}

// END ========================================================

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    loop{}
}
