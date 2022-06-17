// main.rs
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use blog_os::{print, println, vga};
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    blog_os::init();

    println!("Hello World{}", "!");

    // trigger a page fault
    #[allow(unconditional_recursion)]
    fn overflow() {
        overflow();
    }
    overflow();

    #[cfg(test)]
    test_main();

    loop {
        x86_64::instructions::hlt();
    }
}
