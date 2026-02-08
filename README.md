
- [Rust Setup](#rust-setup)
- [Chapter 1. A Freestanding Rust Binary](#chapter-1-a-freestanding-rust-binary)
  - [The no\_std Attribute](#the-no_std-attribute)
  - [Panic Implementation](#panic-implementation)
  - [Concrete things compiler MUST decide](#concrete-things-compiler-must-decide)
  - [What happens when you run a program ?](#what-happens-when-you-run-a-program-)
  - [Name mangling](#name-mangling)
  - [C ABI (Application Binary Interface)](#c-abi-application-binary-interface)
  - [Linker Errors](#linker-errors)
    - [Building for a Bare Metal Target](#building-for-a-bare-metal-target)
  - [Making rust-analyzer happy](#making-rust-analyzer-happy)
- [Chapter 2. A Minimal Rust Kernel](#chapter-2-a-minimal-rust-kernel)
  - [Boot Process](#boot-process)
  - [Multiboot standard](#multiboot-standard)
  - [UEFI](#uefi)
  - [Minimal Kernel](#minimal-kernel)
  - [Target Specification](#target-specification)
  - [Memory-Related Intrinsics](#memory-related-intrinsics)
  - [Set a default target](#set-a-default-target)
  - [Printing to Screen](#printing-to-screen)
  - [Unsafe Rust](#unsafe-rust)
  - [Running our Kernel](#running-our-kernel)
    - [Creating a Bootimage](#creating-a-bootimage)
- [Chapter 3. VGA Text Mode](#chapter-3-vga-text-mode)
  - [A Rust Module](#a-rust-module)
    - [Colors](#colors)
    - [Text Buffer](#text-buffer)
    - [Printing](#printing)
  - [Formating Macros](#formating-macros)
  - [NewLine](#newline)
  - [A Global Interface](#a-global-interface)
    - [Lazy Statics](#lazy-statics)
    - [SpinLocks](#spinlocks)
    - [Safety](#safety)
    - [A println Macro](#a-println-macro)


## Rust Setup 

- On Mac

    ```bash
    brew install rustup-init
    rustup-init -y
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
    source ~/.zshrc

    # verify following shows some version 
    cargo --version
    rustc --version

    ```

## Chapter 1. A Freestanding Rust Binary

[Philipp Oppermann's blog](https://os.phil-opp.com/freestanding-rust-binary/)

### The no_std Attribute
- Disabling the Standard Library: Rust crate link the standary library, which depends on the operating system for feature such as threads, files or networking. It also depends on C standard library `libc` which closely interacts with the OS services. 
- We are developing our own OS, so we would like to not do that
- Right now our crate implicitly links the standard library.
- To disable it we will use `no_std` 

    ```rust
    #![no_std]
    ```

### Panic Implementation
- The standard library provides its own panic handler function, but in a no_std environment we need to define it ourselves:

    ```rust
    // core is effectively: the minimum implementation of the Rust language itself.
    use core::panic::PanicInfo;

    /// This function is called on panic.
    #[panic_handler] 
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
    ```

### Concrete things compiler MUST decide

When compiling this line - 
```rust
let b = a;
```

the compiler MUST choose between two very different machine behaviors:

- Option 1 - raw copy (fast, simple)
    - Copy bits from memory of `a` to memory of `b`
    - `a` is still usable
- Option 2 - move (ownership)
    - Do not copy bits
    - Make `a` invaidate
    - Later only `b` will be destroyed.
- The above changes the way CPU instruction are generated
- So the compiler has branches in its code:
    - Let say if this type is Copyable, then generate `memcpy`
    - Else move 
- but From where does a compiler get this "yes/no" from ?
    - We must define rules which compiler can read from a `trait` (interface) to figure out the let say the answer to `Copyable` question. It cannot guess. That pointers is `#[lang = "copy"]` in Rust

    ```rust
    #[lang = "copy"]
    trait Copy {}
    ```

- When a type implements a `trait`, it agrees to a compile-time contract - meaning it provides a concrete implementation for that the trait's required methods
- The Rust compiler uses this information during compilation to perform monomorphization(for generics) or to setup dynamic dispatch
- The default implementation of many high level feature comes from Rust standard library (`std`) 
  - The `std` library is build on top of `core` and `alloc`, and provides OS integration such as I/O, networking, threading and memory allocation
- On a Linux system `std` typically links agains the system C library (commonly glibc), which is usually under `/usr/lib`. However Rust does not inherently depends on libc - `core` and `alloc` can function without it
- The dependency appears when using `std`, which requires OS services
- Memory allocation in a typical Rust program using `std` flows through the global allocator
  - By default, on linux this allocator delegates to the system allocator, which oftens calls libc's `malloc` 
    - Internally libc may use system calls like `mmap`
- System calls are not normal function call and cannot be resolved by linker like regular symbols. 
  - A symbol is simply a named entity in the compiled program
    - The linker roles is:
      - Collect object files (`.o`)
      - Looks at the undefined symbols
      - Matches them with the definitions from other object file or libraries
    <details>
    <summary> More details </summary>

    ```c
    // ok.cpp
    #include<stdio.h>

    using namespace std;

    int main() {
        printf("hi");
        return 0;
    }
    ```

    ```bash 
    clang -c ok.cpp 
    ```

    ```bash
    nm ok.o
    0000000000000000 T _main
                    U _printf
    0000000000000034 s l_.str
    0000000000000000 t ltmp0
    0000000000000034 s ltmp1
    0000000000000038 s ltmp2
    ```

    - If you see you find the `U _printf` which means U is Undefined symbol
    - This object file references the printf but doesn't not defines it

    - ON mac if you try to statically link - because MAC doesn't support static linking. Apples' runtime model heavily rely on dynamic linking. But this will work in linux (Just extra info because I'm using mac)
    ```bash
    clang++ ok.cpp -static
    ok.cpp:3:17: warning: using directive refers to implicitly-defined namespace 'std'
        3 | using namespace std;
        |                 ^
    1 warning generated.
    ld: library 'crt0.o' not found
    clang++: error: linker command failed with exit code 1 (use -v to see invocation)
    ```

    - But if you try on linux this will work
      - The linker scans for ok.o, sees undefined symbol `printf` 
      - Then it searches for `libc.a`
      - Finds the object file that defines `printf` 
      - Extract it
      - Places it inside your executable
      - Resolve relocation entry and put the address
      - So `call printf` will get replace to `call 0x401230` (let say)
        - This becomes printf virtual address
    - How linker decide the address ?
      - The linker construct a virtual memory layout for the future executable
      - The linker says:
        - `.text` section will start at virtual address `0x400000`
        - `.data` follows
        - `.bss`
        - Function will live inside `.text`
      - In a statically linked ELF executable, there is something called as base virtual address
      - Historically on linux this is `0x400000` (default base)
      - Note this is not a physical RAM address. It is VA
    - When you load the program, what happens?
      - The OS loader:
        - read ELF headers
        - Sees the program expects `.text` at `0x400000`
        - Maps memory at the virtual address 
        - Note more than one processes can have same VA. It's MMU work to provide isolation and maps those to different physical addreses. VA are scope to a single process.
      - There is an other format - PIE (Position Independent Executable)
        - So instead of `call 0x4021230`, we will have `call offset_from_current_instruction`
        - This means: wherever the binary is loaded, the relative offset still works
        - So now loader can choose any start address and the program still runs correctly
    </details>

  - The kernel lives in separate privileged address space, so user program must use special CPU instruction (such as `syscall` on x86_64) to transition from user mode to kernel mode
- To perform this transition, small pieces of architecture-specific assembly codes are used
  - These are called as **thunk**
  - A thunk is tiny adaptor function that:
    - Places arguments into the correct CPU registers
    - Set the system call number
    - Execute the instruction for `syscall`
    - Return control back to the user space
  - These thunks acts as glue between high-level code and low level CPU mechanism required to enter the kernel

- Any type implementing the `trait` means it follows the contract, and thus compiler can decide what instruction to inject. Thus this becomes a **mandatory** thing for the rust compiler. The default comes from the Rust standard library (`std`), which internally depends on `libc` present in the `/usr/lib`. 
- This is the core library which implements the memory allocation functions, and a lot more. This also includes utilities which setups the syscalls (kernel functions are not normal function calls, so they can't be resolve by *linker*, instead architecture-specific language thunks are used to call into a kernel) 
- Any good language rather than making these things hardcoded like for example `Copy`, it allows flexibility. the compiler search for all `#[lang = "copy"]` to make that decision. This allows flexibility
- Ideally we should not provide custom implementation for language items (like `Copy`), it should only be done as last resort  (unless you are building a runtime / OS / core library.)
    - The reason: They are not stable and internal to RUST. They can change when compiler change
- To put it simple language items are semantic hooks. They are required by compiler to even finish the compilation 
- The compiler itself needs:
    - a trait that defines copyability
    - a trait that defines destructors
    - a trait that defines “has known size”
    - a function that defines how panic unwinds
- But the compiler does not implement these things itself. It delegates them to the library.
- This is also important from the perspective of supporting new platforms
    - Different platforms have different:
        - unwinding mechanisms
        - calling conventions
        - memory models
    - The compiler does not want to hardcode:
        - Linux’s unwinder
        - Windows’ unwinder
    - So instead, it says: "Target provide these hooks"
- When you bring Rust to a new platform, you must supply:
    - panic runtime
    - personality function
    - allocation behavior
    - atomic primitives
    - entry points
- The `eh_personality` language item marks a function that is used for implementing **stack unwinding** 
    - By default, rust use the unwinding to run the destructor of all live stack variable in case of panic
    - This ensure all memory is freed and allow parent thread to catch the panic and continue execution
    - However unwinding a complex process. So, we don't want to use it for our OxidOS
    - We will disable unwinding
        - The way we do this using `Cargo.toml`

        ```bash
        # the profile used for `cargo build`
        [profile.dev]
        panic = "abort" # disable stack unwinding on panic

        # the profile used for `cargo build --release`
        [profile.release]
        panic = "abort" # disable stack unwinding on panic

        ```

### What happens when you run a program ?
- Runtime system: create stack, start a GC, zero global memory, setup TLS
- In C, OS doesn't calls `main` it calls `_start`. 
- `_start` lives in a tiny runtime called `crt0` 
- `crt0` does things like:
    - setup stack
    - copy env variables
    - init global variables
    - then call `main(argc, argv)`
- The compiler linked it for us.
- Eg. in Java, JVM is this runtime, which setup heap, threads, JIT, GC, loads your bytecode, and finally calls `main` 
- While building our own OS, we don't have this runtime. We can not say link to C runtime or Rust runtime. There are no stacks, no global, no safety, we are writing the  runtime. 
- Rust runtime has a function like `#[lang = "start"]`. This is the function which `crt0` calls. It is the entry to the Rust runtime. It then calls `main`. This is implemented in `std`
- In a typical Rust binary that links the standard library, execution starts in a C runtime library called crt0 (“C runtime zero”)
- This includes creating a stack and placing the arguments in the right registers. The C runtime then invokes the entry point of the Rust runtime, which is marked by the start language item
- The runtime then finally calls the `main` function. Rust has very minimal runtime

### Name mangling
- Normally, the compiler renames functions internally. It encodes module, types, generics, etc.
- But for `_start` we don't want to do that. OS is dumb at this point and it need to figure out where this `_start` is
- We want the linker the name of the entry, thus we are using `#[unsafe(no_mangle)]`
- You can name it anything, but this is the convention

    ```rust
    #[unsafe(no_mangle)] // don't mangle the name of this function
    pub extern "C" fn _start() -> ! {
        // this function is the entry point, since the linker 
        // looks for a function named `_start` by default
        loop {}
    }
    ```

### C ABI (Application Binary Interface)
- In a normal Rust program `main()` is called by Rust's runtime. But in OS kernel, there is no runtime. 
- The bootloader loads your kernel into the memory and then jump to a specified symbol in the binary symbol table
  - By convention that symbol is `_start` (a well known symbol)
- Why using `extern "C"`?
  - This is about calling convention ABI 
  - A machine level rule for how functions are called
  - This includes
    - Which register hold arguments
    - Who cleans the stack
    - How return values are passed
  - The bootloader expects C-style calling convention
  - Using `extern "C"` acts as a contract
    - Now bootloader and your function agrees on 
      - Stack Layout
      - Register usage
      - Calling convention
- Note this method never return, because if it do then everything gone, so we write kernel never exists `loop {}`
- Bootloaders like GRUB already follow this 
    - So order is : Hardware -> Bootloader (often written for C ABI) -> your `_start` -> your kernel code 
- ABI is machine-level agreement about how code  humps into one other 
- Marking `extern "C"` to tell the compiler that **it should use the C calling convention for this function** (instead of **unspecified** Rust calling convention which booloader doens't understands) 
    - As i explained earlier, this is required because the entry point is not called by any function, but invoked directly by the bootloader (or any other OS)

### Linker Errors
- The linker is a program that combines the generated code into an executable
- Since the executable format different b/w linux, Window and MacOS, each system has it own linker [see this](https://github.com/souraavv/whitepapers-and-books/discussions/9#discussioncomment-15247589)
- The linker assumes our program depends on **C runtime**, which it doesn't
- To solve the issue we need to tell the linker that it should not include the C runtime. We can do this mutiple ways

#### Building for a Bare Metal Target
- By default Rust tries to build an executable that is able to run in your current system environment
- Rust uses a string called target triple shown in the output of `rustc --version --verbose` e.g., `host: x86_64-unknown-linux-gnu`
  - CPU architecture (x86_64), the vendor (unknown), os (linux), ABI (gnu)
- We will use `rustup target add thumbv7em-none-eabihf` 


- We are building a platform, not a program. We need `nightly`. It contians feature which stable Rust doesn't ship. 
- like unstable language features, unstable compiler flags, building the standard library yourself
- Without this we can not build `core`, can not control runtime, 

    ```bash
    rustup toolchain install nightly
    # set nightly as default
    rustup default nightly
    # rust-src, gives you source code for core, alloc, compile_builtins
    rustup component add rust-src llvm-tools-preview

    # thumbv7em-none-eabihf: says 'none' for the OS
    # Think none = bare metal; there is no kernel, no libc, no crt0. 
    # this is why linker stop pulling C runtime and why _start becomes 
    # our responsibility. So we are telling emit machine code for this CPU
    # do no assume any OS exists. I'm building the lowest layer.
    # e'abi'hf is the (ABI) . It defines how arguments are passed, stack 
    # layout, rules
    rustup target add thumbv7em-none-eabihf

    # -Z means: Use unstable compiler features.
    cargo build -Z build-std=core --target thumbv7em-none-eabihf
    ```

### Making rust-analyzer happy

- Test dependends on std. 
- The test harness defines its own panic implementation. 
- When you run cargo test, cargo doesn't run your program, instead it build a separate binary 

    ```rust
    fn main() {
    // setup runtime
    // setup panic handling
    // discover all #[test] functions
    // run them
    // print results to stdout
    }
    ```

- This auto-generated binary is called the test harness. 
- Our main is not usednot the panic handler and entry point (start)

    ```bash 
    [[bin]]
    name = "oxid-os"
    test = false
    bench = false
    ```

## Chapter 2. A Minimal Rust Kernel

### Boot Process
- Two firmware: BIOS and UEFI
  - UEFI is modern and has more feature than BIOS
- BIOS Boot
  - BIOS is loaded from specific flash memory located on the motherboard
  - BIOS runs self-test and init the hardware (CPU, RAM)
  - Then it looks for bootable disk
  - If it finds, then it pass control to the bootloader
  - Most bootloader are larger than 512 bytes, so bootloader are split into small stage, which fits into 512 bytes, and a second stage which loaded by the first stage
  - The bootloader has to determine the location of the kernel image on the disk and load it into the memory
  - It also needs to switch from 16-bit real mode first to 32 bit protected mode, and then to 64-bit long mode, where 64 bit register and main memory is accessible
  - Its third job job is to query certain info such as memory map from the BIOS and pass it to the OS Kernel
- Writing bootloader requires assembly language knowledge and therefore that is not covered in this

### Multiboot standard
- GNU GRUB is most popular bootloader on linux systems
- To avoid every OS create a bootloader which is only compatible with single OS. FSS created an open boatloader standard called Mutliboot.
- This standard defines an interface b/w the bootloader and os
  - The reference implementation is GNU GRUB
- To make a kernel Multiboot complaint, one just need to insert so-called Multiboot-header at the beginning of the kernel file 
- This make it very easy to boot an OS from the GRUB
- However GRUB and the multiboot standard have problems too 
  - They support only 32-bit protected mode. This means that you still have to do CPU config to switch to the 64-bit long mode
  - GRUB needs to be installed on the host system to create a bootable disk image from the kernel file. This makes development on Windows or Mac more difficult
- We will not use GRUB or the multiboot standard for this project

### UEFI
- No support

### Minimal Kernel
- We built the freestanding binary through cargo, but depending on the operating system, we needed different entry point names and compile flags. 
- That’s because cargo builds for the host system by default, i.e., the system you are running on
- Instead, we want to compile for a clearly defined target system.
- We need to setup Rust Nighly. This compiler allow us to opt-in various features. For example `asm!` macro for inline assembly by adding `#![feature(asm)]` to top of our `main.rs`


### Target Specification
- Cargo support different target system through `--target` parameter. The target described by so called target triple, which describe the CPU architecture, the vendor and OS
  - e.g., `x86_64-unknown-linux-gnu`
- For our target system, however, we require some special configuration parameters (e.g. no underlying OS)
- Fortunately, Rust allows us to define our own target through a JSON file.
- For example, a JSON file that describes the x86_64-unknown-linux-gnu target looks like this:
    ```json
    {
        "llvm-target": "x86_64-unknown-linux-gnu",
        "data-layout": "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
        "arch": "x86_64",
        "target-endian": "little",
        "target-pointer-width": 64,
        "target-c-int-width": 32,
        "os": "linux",
        "executables": true,
        "linker-flavor": "gcc",
        "pre-link-args": ["-m64"],
        "morestack": false
    }
    ```
- `data-layout` defines the size of various integer, floating point, and pointer type
- Our target spec
    ```json
    {
        "llvm-target": "x86_64-unknown-none",
        "data-layout": "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
        "arch": "x86_64",
        "target-endian": "little",
        "target-pointer-width": 64,
        "target-c-int-width": 32,
        "os": "none",
        "executables": true,
        "linker-flavor": "ld.lld",
        "linker": "rust-lld",
        "panic-strategy": "abort",
        "disable-redzone": true,
        "features": "-mmx,-sse,+soft-float",
        "rustc-abi": "x86-softfloat"
    }
    ```

- Instead of using the platform’s default linker, we use the cross-platform LLD linker that is shipped with Rust for linking our kernel (`"linker-flavor": "ld.lld"`)
- `"panic-strategy": "abort"`: target doesn't support stack-unwinding on panic
- We’re writing a kernel, so we’ll need to handle interrupts at some point. To do that safely, we have to disable a certain stack pointer optimization called the “red zone”, because it would cause stack corruption otherwise.
- The `features` field enables/disables target features. We disable the `mmx` and `sse` features by prefixing them with a minus and enable the `soft-float` feature by prefixing it with a plus.

### Memory-Related Intrinsics
- Rust compiler assumes that a certain set of built-in functions are available for all system
- Most of the functions are provided by `compiler_builtins` crate that we just recompiled
- However, there are some memory related function in the crate that are not enabled by default because they are normally provided by the C library on the system
- These functions included `memset`, `memcpy`, `memcmp`. We might not need this function now at this point of project, but we can't also link to the C library of the OS, we need an alternative way provide these function to our compiler
- It is dangerous to implement these on own, because even slighest mistake in the implementation can cause undefined behavior
  - E.g., if you attempt to use `for` loop for `memcpy`, you might end up in infinite recursion, because `for` internally use `IntoIterator::into_iter`, which may call `memcpy`
- Fortunaltely, the `compiler_builtins` crate already contains implementation for all the needed function, they are disabled by default to not collide with C library. We can enable them by setting cargo's `build-std-feature` flag
- We will use `[unstable]` section for that or `-Z` option to the compiler
    ```bash
    [unstable]
    build-std-feature = ["compiler-builtins-mem"]
    build-std = ["core", "compiler_builtins"]
    ```
- Internally the effect is that the `#[unsafe(no_mangle)]` attribute is applied to the `memcpy`, **which makes them available for the linker**
- So now we can write more complex code

### Set a default target

- To avoid passing `--target` param each time to the compiler let set that in .toml file
    ```bash
    [build]
    target = "x86_64-blog_os.json"
    ```
- Now on we can use `cargo build` and that will use target defined in cargo.toml file as default 

### Printing to Screen
- Bootloader will call to our `_start` method (remember linker looks for this method by default as a convention), 
- Let's add something to the screen output via our `_start` method
- Easiest way to do this is using VGA text buffer
- Its a special memory area mapped to the VGA hardware that contains the contents display on screen
- It normally consist of 25 lines and 80 char each
- Each character is display as ASCII
- Let's print Hello world for now
- The buffer is located at address `0xb8000` and that each character cell contains of an ASCII byte and a color byte 
- We have to write to a raw pointer, thus we will use `unsafe` code block in Rust
- We could create a VGA buffer type that encapsulates all unsafety and ensures that it is impossible to do anything wrong from the outside
- We will create such a safe VGA buffer abstraction in the next chapter.

### Unsafe Rust
- You can take five actions in `unsafe` Rust that you can’t in safe Rust, which we call `unsafe` superpowers
  - Dereference a raw pointer.
  - Call an `unsafe` function or method.
  - Access or modify a mutable static variable.
  - Implement an `unsafe` trait.
  - Access fields of unions.
- It’s important to understand that `unsafe` doesn’t turn off the borrow checker or disable any of Rust’s other safety checks
- The `unsafe` keyword only gives you access to these five features that are then not checked by the compiler for memory safety
- Keep unsafe blocks small; you’ll be thankful later when you investigate memory bugs.
  - You’ll know that any errors related to memory safety must be within an unsafe block
- We always want to minimize the use of unsafe as much as possible

### Running our Kernel
- First we need to turn our compiled kernel into a bootable disk image by linking it **with** a bootloader
- Then we can run the disk image in the QEMU virtual machine or boot it on real hardware using a USB stick

#### Creating a Bootimage
- As we have read earlier - Bootloader is reponsible for init the CPU and loading the kernel
- Instead of writing our own bootloader, which is a project on its own, we use the bootloader crate
- This crate implements a basic BIOS bootloader without any C dependencies, just Rust and inline assembly
    ```bash
    [dependencies]
    bootloader = "0.9"
    ```
- Adding the bootloader as a dependency is not enough to actually create a bootable disk image. The problem is that we need to link our kernel with the bootloader after compilation, but cargo has no support for post-build scripts.
- To solve this problem the author of original blog created a tool named `bootimage` - First compile the kernel and bootloader, and then links them together to create a bootable disk image
- The bootimage tool performs the following steps behind the scenes:
  - It compiles our kernel to an ELF file.
  - It compiles the bootloader dependency as a standalone executable
  - It links the bytes of the kernel ELF (Executable and Linkable format) file to the bootloader.
    - Layout: ELF header, Program header table, .text, .rodata, .data, section header table
- When booted, the bootloader reads and parses the appended ELF file
- It then maps the program segments to virtual addresses in the page tables, zeroes the .bss section, and sets up a stack.
- Finally, it reads the entry point address (our `_start` function) and jumps to it.

- Install `qemu`
  - `brew install qemu` on mac
  - Now boot image using: `qemu-system-x86_64 -drive format=raw,file=target/x86_64-oxid_os/debug/bootimage-oxid-os.bin`

- This is awesome - seeing your image getting boot up
    ![alt text](./images/first-boot.png)

- To avoid every time submit `qemu-system-x86_64` command. We can use target in config.toml file. 
- The following Applies to all targets whose target configuration file’s "os" field is set to "none"
    ```bash
    [target.'cfg(target_os = "none")']
    runner = "bootimage runner"
    ```
- This includes our x86_64-oxid_os.json target
- The `runner` key specifies the command that should be invoked for `cargo run`
- The command is run after a successful build with the executable path passed as the first argument


## Chapter 3. VGA Text Mode 

- VGA text mode is simple way to print text to the screen 
- To print character to the screen in VGA text mode, one has to write it to the text buffer of VGA hardware
- VGA text buffer is an array of size 25 rows and 80 columns
- Each entry in this array represents a single character
  - The first byte in represent the character that should be printed in the ASCII encoding
  - The second byte defines how the character is displayed
    - The first four bit defines the foreground
      - Bit 4 is the bright bit
    - The next three bits the background color
    - Last - whether character should blink or not 
- The VGA text buffer is accessible via the memory-mapped I/O to the address `0xb8000`
- This means the read and write to that address don't access the RAM but directly access the text buffer on the VGA hardware. 
  - This means we can read and write through normal memory operation to that address
  - Note that memory-mapped hardware might not support all normal RAM operations
- In this chapter we will encapsulate all the unsafety in a special module

### A Rust Module
- We can now create a Rust module to handle printing:
    ```rust
    // in src/main.rs
    mod vga_buffer;
    ```
- For the content of this module we will create a new `src/vga_buffer.rs` file.

#### Colors

```rust
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}
```

- `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` is like asking the compiler to automatically implements traits for your Type
  - `Debug` is used : `println!("{:?}", color)`
  - `Clone`: `let b = a.clone()`
  - `Copy`: Stronger that copy `let b = a`; no ownership transfer, no destructor, no heap
    - If a type is `Copy`, Rust also required it to be `Clone`
  - `PartialEq` Allows `a == b` or `a != b`
  - `Eq` (total and reflexive)

#### Text Buffer

- ScreenChar and Buffer
    ```rust

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(C)]
    struct ScreenChar {
        ascii_character: u8,
        color_code: Colorcode,
    }

    const BUFFER_HEIGHT = 25;
    const BUFFER_WIDTH = 80;

    #[repr(transparent)]
    struct Buffer {
        char: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
    }

    ```

- Writer
    ```rust
    pub struct Writer {
        column_position: usize,
        color_code: ColorCode,
        buffer: &'static mut Buffer,
    }
    ```

- The writer will always write to the last line and shift line up when filled
- The `'static` lifetime specifies that the reference is valid for the whole program run time

#### Printing
- Now we can use the `Writer` to modify the buffer characters. First we create a method to writea single ASCII byte
    ```rust
    impl Writer {
        pub fn write_byte(&mut self, byte: u8) {
            match byte {
                b'\n' => self.new_line(),
                byte => {
                    if self.column_position >= BUFFER_WIDTH {
                        self.new_line();
                    }

                    let row = BUFFER_HEIGHT - 1;
                    let col = self.column_position;

                    let color_code = self.color_code;
                    self.buffer.chars[row][col] = ScreenChar {
                        ascii_character: byte,
                        color_code,
                    };

                    self.column_position += 1;
                }
            }
        }

        fn new_line(&mut self) {
            /* TODO */
        }
    }
    ```

- To print whole string, we can convert them to bytes and print them one-by-one
    ```rust
    impl Writer {
        pub fn write_string(&mut self, s: &str) {
            for byte in s.bytes() {
                match byte {
                    0x20..=0x7e | b'\n' => self.write_byte(byte),
                    _ => self.write_byte(0xfe),
                }
            }
        }
    }
    ```
- The VGA text buffer only supports ASCII 
- For unprintable we are using `0xfe` 
    ```rust
    pub fn print_something() {
        let mut writer = Writer {
            column_position: 0, 
            color_code: ColorCode::new(Color:Yellow, Color::Black),
            buffer: unsafe {&mut *(0xb8000 as *mut Buffer)},
        }

        // The b' prefix creates a byte literal
        // which represents an ASCII character
        writer.write_byte(b'H');
        writer.write_string("ello ");
    }
    ```

- Brush up (can skip if already familiar)
  - Note `0xb8000` by its own is just a number (hex literal)
  - `0xb8000 as *mut Buffer`: cast number to a Raw Pointer to the `Buffer`
    - in terms of CPP this is same as `Buffer* ptr = (Buffer*) 0xb8000`
  - The leading `*` in `*(0xb8000 as *mut Buffer)` is dereference
    - Go to the memory located at that address, and treat it as Buffer value
    - This is first dangerous operation - as we are asking CPU to load memory from address `0xb8000`
    - The reason we added `unsafe` to by pass compiler checks. Rust cannot verify
      - Does memory exists?
      - Is it actually a `Buffer`?
      - Is there another mutable reference already ? (race conditions)
      - Is the memory init ?
    - At this point the Type is `Buffer`
  - The final `&mut *(...)`  
    - `&mut X` : create an exclusive, non-null, aligned, borrow-checked reference to X
- `*` goes from pointer to memory
- `&mut` goes from memory to reference. Rust need to know if it is mutable reference of defautl (immutable)
- `&mut (0xb8000 as *mut Buffer)`: This expression without `*` will give you a pointer to a pointer
  - `0xb8000 as *mut Buffer` is a pointer value
  - `&mut` would give you `&mut *mut Buffer` (a pointer to a pointer)
  - not a `&mut Buffer` 
- CPP equivalent
    ```cpp
    Buffer* ptr = reinterpret_cast<Buffer*>(0xb8000);
    Buffer& ref = *ptr;
    ```

![Again](/images/colors.png)


### Formating Macros 

- We will start using Rust Formatting macros, so that we can easily print different types, like integer or floats
- To support them we need to implement `core::fmt::Write` trait
- The only requirement of this trait is `write_str` and return type is `fmt::Result`
    ```rust
    use core::fmt;

    impl fmt::Write for Writer {
        fn write_str(&mut self, s: &str) -> {
            self.write_string(s);
            Ok(())
        }
    }

    ```
- The `Ok(())` is just `Ok` `Result` containing `()` type
  - In Rust `()` is equivalent to `void` in CPP. This is called unit type. This means there is a value here, but it carries no information 
- Now we can use Rust's built-in `write!` or `writeln!` formatting macros:

```rust
pub fn print_something() {
    use core::fmt::Write;

    let mut writer = Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    }

    writer.write_byte(b'H');
    writer.write_string("ello! ");
    write!(writer, "The number are {} and {}", 42, 1.0/3.0).unwrap();
}
```

- `unwrap()` means if it `Ok()`, give me a value. If it's `Err`, then panic
  - This isn’t a problem in our case, since writes to the VGA buffer never fail.

### NewLine

```rust
impl Writer {
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                // Move character one line up i.e., row - 1
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    // This method clears a row by overwriting all of its characters 
    // with a space character.
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };

        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

```


### A Global Interface
- To provide a global writer that can be used as an interface from other module without carrying a `Writer` instance around, we try to create a `static WRITER`

    ```rust
    pub static WRITER: Writer = Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer)},
    };
    ```

- The above will fail - because statics are init at compile time
- Rust compiler evaluates such initialization expression is called "const evaluator"
- Problem here is that Rust’s const evaluator is not able to convert raw pointers to references at compile time. 

#### Lazy Statics
- The one-time initialization of statics with non-const functions is a common problem in Rust.
- This crate provides a lazy_static! macro that defines a lazily initialized static
- Instead of computing its value at compile time, the static lazily initializes itself when accessed for the first time. 

    ```toml
    [dependencies.lazy_statics]
    version = "1.0"
    features = ["spin_no_std"]
    ```
- We need the `spin_no_std` feature, since we don’t link the standard library.
- With lazy_static, we can define our static WRITER without problems:

```rust
use lazy_static::lazy_static;

lazy_static! {
    pub static ref WRITER: Writer = Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer)},
    }
}
```
- However above `WRITER` is purely useless, becuase this is immutable
- This means we can not write anything. Since all write methods take `&mut self`
- One solution would be to use `mutable static` 
- But then every write to this would be unsafe since 
  - Using `static mut` is highly discouraged
- What we can do ?
  - Can we use an immutable static with a cell type like `RefCell` or even `UnsafeCell` that provides interior mutability
  - But problem is these types are not `Sync`, so we can't use them in static

#### SpinLocks
- To get synchronized interior mutability, users of a standard library can use `Mutex`. 
- It provides mutual exclusion, but our kernel doesn't have that
- However there is a really basic type of mutex in computer science that requires no operating system features: the spinlock.
- Instead of blocking the thread simply try to lock it again and again in tight loop, thus burning CPU time until the mutex is free again
- To use spinlock mutex, we can add the spin crate as a dependency

    ```toml
    [dependencies]
    spin = "0.5.2"
    ```

- Using spin mutex

    ```rust
    // in src/vga_buffer.rs
    using spin::Mutex;

    lazy_static {
        pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
            column_position: 0,
            color_code:ColorCode::new(Color::Yellow, Color::Black),
            writer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        });
    }
    ```
- Now we can delete `print_something` function and use directly

    ```rust
    // in src/main.rs

    #[unsafe(no_mangle)]
    pub extern "C" fn _start() -> ! {
        use core::fmt::Writer;
        vga_buffer::WRITER.lock().write_str("hello again").unwrap();
        write!(vga_buffer::WRITER.lock(), ", some number: {} {}", 42, 1.37)
                .unwrap();
        loop {}
    }
    ```

#### Safety 

#### A println Macro
- Now that we have global writer, we can add a `println` macro that can be used from anywhere 
- Rust macro syntax is bit strange (we will copy for now directly from source code of Rust)

    ```rust
    #[macro_export]
    macro_rules! print {
        ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
    }

    #[macro_export]
    macro_rules! println {
        () => ($crate::print!("\n"));
        ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
    }


    // Locks our static WRITR and calls write_fmt method on it
    // This method is from Writer trait, which we need to import 
    // function is public - macros need to be called outside the module
    //private doc - internal detail
    #[doc(hidden)]
    pub fn _print(args: fmt::Argument) {
        // Implementing a trait does NOT automatically bring its methods 
        // into scope.
        use core::fmt::Write;
        WRITER.lock().write_fmt(args).unwrap();
    }
    ```

- Like in the standard library, we add the `#[macro_export]` attribute to both macros to make them available everywhere in our crate.
- When you use  `#[macro_export]` Rust Moves the macro to the crate root. Not inside the module namespace
  - It get place in the crate root namespace `crate::print!`
  - Makes them available every where in our crate
  - so even when this is defined in `src/vga_buffer.rs` 
  - It become available as `crate::println!` and not `crate::vga_buffer::println!`
- Why did we use `$crate:print!` instead of `print!`
  - `$crate` is a special macro variable. It expands to: current crate root path
  - This is like preventing the name resolution issue, if other crate too have `print!` defined

- With all this in place, now we can write

    ```rust
    #[unsafe(no_mangle)]
    pub extern "C" fn _start() -> ! {
        println!("Hello World{}", "!")

        loop {}
    }
    ```