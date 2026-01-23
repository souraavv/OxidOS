#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

// core is effectively: the minimum implementation of the Rust language itself.
use core::panic::PanicInfo;

/// This function is called on panic.
#[panic_handler] 
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static HELLO: &[u8] = b"Hello World!";

#[unsafe(no_mangle)] // don't mangle the name of this function
// pub: means visible to the linker
// extern "C" - select the C ABI (how argument are passed, stack handling)
// Because bootloader are linker understand C calling convention
pub extern "C" fn _start() -> ! {
    // this function is the entry point, since the linker 
    // looks for a function named `_start` by default

    // this is a mutable raw pointer to bytes
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        // rust cannot prove that the raw poiner we created are valid
        // Using unsafe is telling compiler that we are absolutely
        // sure that the operation are valid
        unsafe {
            // raw pointer dereference - that's why unsafe
            *vga_buffer.offset(i as isize * 2) = byte; 
            // Writing into memory mapped I/O (no sys call, no driver)
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {}
}