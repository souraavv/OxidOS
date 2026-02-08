#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"] //sets the name of the entry point

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

    #[cfg(test)]
    test_main();
    exit_qemu(QemuExitCode::Success);

    loop {}
}

// Exiting Qemu for test case runs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}


#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}

#[test_case]
fn trivial_assertion() {
    println!("trivial assertion");
    assert_eq!(1, 1);
    println!("[ok]")
}

