// in tests/basic_boot.rs
// This is an integration test.

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use x86_64;

#[no_mangle] 
pub extern "C" fn _start() -> ! {
    test_main();
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

#[test_case]
fn some_test() {
    assert_eq!(1 + 1, 2)
}
