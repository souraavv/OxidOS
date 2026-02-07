#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

// core is effectively: the minimum implementation of the Rust language itself.
use core::panic::PanicInfo;

mod vga_buffer;

/// This function is called on panic.
#[panic_handler] 
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[unsafe(no_mangle)] // don't mangle the name of this function
// pub: means visible to the linker
// extern "C" - select the C ABI (how argument are passed, stack handling)
// Because bootloader are linker understand C calling convention
pub extern "C" fn _start() -> ! {
    // this function is the entry point, since the linker 
    // looks for a function named `_start` by default
    println!("Hello world{}", "Sourav");
    loop {}
}