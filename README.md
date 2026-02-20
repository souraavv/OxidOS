
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
- [Chapter 4. Testing](#chapter-4-testing)
  - [Testing in Rust](#testing-in-rust)
    - [Custom Test Framework](#custom-test-framework)
  - [Existing QEMU](#existing-qemu)
    - [I/O Ports](#io-ports)
    - [Using Exit Device](#using-exit-device)
  - [Printing to the Console](#printing-to-the-console)
    - [Serial Port](#serial-port)
    - [Print an Error Message on Panic](#print-an-error-message-on-panic)
    - [Hiding QEMU](#hiding-qemu)
    - [Timeouts](#timeouts)
    - [Insert Print Automatically](#insert-print-automatically)
  - [Testing the VGA Buffer](#testing-the-vga-buffer)
    - [Integration Tests](#integration-tests)
    - [Create a Library](#create-a-library)
- [Chapter 5. CPU Exceptions](#chapter-5-cpu-exceptions)
  - [The Interrupt Descriptor Table](#the-interrupt-descriptor-table)
  - [An IDT Type](#an-idt-type)
  - [The Interrupt Calling Convention](#the-interrupt-calling-convention)
    - [Preserved and Scratch Register](#preserved-and-scratch-register)
    - [Preserving all Registers](#preserving-all-registers)
    - [The Interrupt Stack Frame](#the-interrupt-stack-frame)
  - [Implementation](#implementation)
    - [Lazy Static to Rescue](#lazy-static-to-rescue)
    - [Test](#test)
- [Chapter 6. Double Faults](#chapter-6-double-faults)
  - [What is Double Fault ?](#what-is-double-fault-)
    - [Triggering a Double Fault](#triggering-a-double-fault)
  - [A Double Fault Handler](#a-double-fault-handler)
  - [Causes of Double Faults](#causes-of-double-faults)
    - [Kernel Stack Overflow](#kernel-stack-overflow)
  - [Switching Stacks](#switching-stacks)
    - [The IST and TSS](#the-ist-and-tss)
    - [Creating a TSS](#creating-a-tss)
    - [Loading the TSS](#loading-the-tss)
    - [The Global Descriptor Table](#the-global-descriptor-table)
      - [Creating a GDT](#creating-a-gdt)
    - [The final Steps](#the-final-steps)


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
    name = "oxid_os"
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
    target = "x86_64-oxid_os.json"
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

#### NewLine

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

## Chapter 4. Testing
- Unit and integration test in `no_std` executables
- We will use Rust's support for custom test framework to execute test functions inside our kernel. 
- To report the result out of QEMU, we will use different feature of QEMU and `bootimage` tool

### Testing in Rust
- Rust has built-in test framework - so there is no need to setup anything
  - Using `#[test]`. The `cargo test` will automatically find and execute all the test of your crate
    ```toml
    [[bin]]
    test = true
    ```
- The `[[bin]]` section defines how `cargo` should compiler our `oxid_os` executables
  - We initially set `test = false` to make `rust-analyzer` happy, but now we want to enable testing
- Unfortunately testing is complex for `no_std` application 
  - The problem is Rust's test framework uses built-in `test` library which relies on `std` library

#### Custom Test Framework
- Fortunately, rust supports replacing the default test framework through the unstable `custom_test_framework` feature
- This feature require no external libraries and thus also work in `#[no_std]` environment
  - It works by calling user specified runner function annotated with `[test_case]`
- But it lacks lot of features like `should_panic` testcase
- To implement a custom test framework for our kernel, we add the following to `main.rs`

    ```rust
    #![feature(custom_test_framework)]
    #![test_runner(crate::test_runner)]

    #[cfg(test)]
    pub fn test_runner(tests: &[&dyn Fn()]) {
        println!("Running {} test", tests.len());
        for test in tests {
            test();
        }
    }
    ```
- More details
  - `Fn()` is a trait, not a type
    ```rust
    pub trait Fn<Args>: FnMut<Args> where Args: Tuple, {
        extern "rust-call" fn call(&self, args: Args) -> Self::Output;
    }

    // Examples
    // Calling a closure
    let square = |x| x * x;
    assert_eq!(square(5), 25)

    // Using a Fn parameter

    fn call_with_one<F>(func: F) -> usize where F: Fn(usize) -> usize {
        func(1)
    }

    let double = |x| x * 2;
    assert_eq!(call_with_one(double), 2);

    ```
  - Something which can be call like function but with no args and no return value
  - We used `dyn Fn()` means a trait object implementing `Fn()`, chosen at runtime
    - Rust requires you to be explicit when you want runtime polymorphism 
  - Trait objects are unsized (they are not concrete type, they are just behavior contracts), so you must put them behind `&` a pointer
    - `&dyn Fn()`, `Box<dyn Fn()>`, `Arc<dyn Fn()>`
    - Ex.
    ```rust
    trait Speak {
        fn speak(&self);
    }

    struct Dog;
    struct Cat { age: u32 }
    struct Bird { a: u64, b: u64}

    // All of these can implement Speak

    ```
    - So what does `dyn Speak` - some unknown concrete type implementing `Speak`
      - So unknown type == unknown size
  - The outer `&[]` is a borrowed slice 
    - Inside each element is a reference to a dynamically dispatched callable 

- There is some known bug in cargo that leads to `duplicate lang item in crate core:` when we try to write unit test case when we have disabled `std` crate
    ```toml
    panic-abort-tests = true
    ```

    ```rust
    //sets the name of the entry point
    #![reexport_test_harness_main = "test_main"] 

    pub extern "C" fn _start() -> ! {
        // ...
        #[cfg(test)]
        test_main();
        // ...
        loop {}
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
    ```

    ![First test case](./images/first-test-case.png)

### Existing QEMU
- Right now we have endless loop `loop {}`
- The clean sol is to implement a proper way to shutdown our OS
  - Unfortunately this is relatively complex because it requires implementing support for either APM or ACPI power management standards
- Luckily there is a escape hatch: QEMU support special `isa-debug-exit` device, which provides an easy way to exit QEMU from the guest systems
- To enable it we need to pass a device argument to the QEMU
- We can do so by adding a `package.metadata.bootimage.test-args` in our Cargo.toml
    ```toml
    # in Cargo.toml

    [package.metadata.bootimage]
    test-args = ["-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"]
    ```
- The `bootimage` runner will append the `test-args` to the default QEMU command for all test executables
  - For a normal `cargo run` this is ingored
- Together with the device name `isa-debug-exit` we also pass parameters
  - `iobase`
  - `iosize` 
  - These specify the IO port through which the device is reachable from our kernel

#### I/O Ports
- There are different way of communicating b/w CPU and peripheral hardware on `x86_64`
  - Memory Mapped IO
  - Port Mapped IO
- We already using Memory mapped IO for accessing VGA text buffer through memory address `0xb8000`
- This address is not mapped to RAM but to some memory on VGA device
- In contract, port Mapped IP uses a separate IO bus for communication
  - Each connected peripheral has one or more port numbers
  - To communicate with such an I/O port, there are special CPU instruction called `in` and `out`, which takes out port number and data bytes
- The `isa-exit-debug` device uses port-mapped I/O
- The `iobase` parameter specifies on which port address the device should live (`0xf4` is generally unused port on x86 I/O bus)
  - and `iosize` specifies the port size (`0x04` means 4 bytes)

#### Using Exit Device
- The functionaliy of `isa-debug-exit` is very simple
- when a value is written to I/O port specified by `iobase`, it causes QEMU to exit with exit status `(value << 1) | 1`
- So when we write `0` to the port, QEMU will exit with exit status `1`and when we write `1` then `3`
- Instead manually invoking `in` and `out` assembly instructions we use abstraction provided by `x86_64` crate
- Now we can use `Port` type provided by the crate to create `exit_qemu` function
    ```rust
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u32)] // 4 bytes
    pub enum QemuExitCode {
        // To specify the exit status
        Success = 0x10, // we use a number which doesn't clash with QEMU's exit code like 0, or 1
        Failed = 0x11,
    }

    pub fn exit_qemu(exit_code: QemuExitCode) {
        use x86_64::instructions::port::Port;

        // unsafe because writing to a port can have unspecified behavior
        unsafe {
            let mut port = Port::new(0xf4);
            port.write(exit_code as u32);
        }
    }
    ```

    ```rust
    test_main();
    exit_qemu(QemuExitCode::Success);
    ```

- Cargo considers any other exit code than `0` as failure so we have to re-map our new `0`
    ```toml
    # in Cargo.toml

    [package.metadata.bootimage]
    test-args = ["-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"]
    // bootimage maps our success exit code to exit code 0, 
    // so that cargo test correctly recognizes the success case and does not 
    // count the test as failed.
    test-success-exit-code = 33 # (0x10 << 1) | 1
    ```

### Printing to the Console
- To see the test output on the console, we need to send the data from our kernel to the host system somehow
- There are various ways to achieve this, for example, by sending the data over a TCP network interface.
- However, setting up a networking stack is quite a complex task, so we will choose a simpler solution instead.

#### Serial Port
- A simple way to send the data is to use the serial port
- An old interface standard which is no longer found in modern computers.
- It is easy to program and QEMU can redirect the bytes sent over serial to the host’s standard output or a file.
- The chips implementing a serial interface are called UARTs
- There are lots of UART models on x86
- The common UARTs today are all compatible with the 16550 UART, so we will use that model for our testing framework.
- We will use the uart_16550 crate to initialize the UART and send data over the serial port.
    ```toml
    # in Cargo.toml

    [dependencies]
    uart_16550 = "0.2.0"
    ```
- The `uart_16550` crate contains a `SerialPort` struct that represents the UART registers but we still need to construct an instance of it ourselves. For that, we create a new serial module with the following content:
    ```rust
    // in src/main.rs
    mod serial;


    // in src/serial.rs

    use uart_16500::SerialPort;
    use Spin::Mutex;
    use lazy_static::lazy_static;

    lazy_static! {
        pub static ref SERIAL1: Mutex<SerialPort> = {
            let mut serial_port = unsafe { SerialPort::new(0x3F8) };
            serial_port.init();
            Mutex::new(serial_port);
        }
    }
    ```
- Like with the VGA text buffer, we use `lazy_static` and a spinlock to create a static writer instance. 
- By using `lazy_static` we can ensure that the `init` method is called exactly once on its first use.
- Like the `isa-debug-exit` device, the UART is programmed using port I/O
- Since the UART is more complex, it uses multiple I/O ports for programming different device registers.
- The `unsafe` `SerialPort::new` function expects the address of the first I/O port of the UART as an argument, from which it can calculate the addresses of all needed ports. 
- We’re passing the port address `0x3F8`, which is the standard port number for the first serial interface.
- To make the serial port easily usable, we add `serial_print!` and `serial_println!` macros:
    ```rust
    // in src/serial.rs

    #[doc(hidden)]
    pub fn _print(args: ::core::fmt::Arguments) {
        use core::fmt::Write;
        SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
    }

    /// Print to the host through the serial interface
    #[macro_export]
    macro_rules! serial_print {
        ($($arg:tt)*) => {
            $create::serial::_print(format_args!($($arg)*));
        }
    }

    #[macro_export]
    macro_rules! serial_println {
        () => ($crate::serial_print!("\n")),
        ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
        ($fmt:expr, $($arg:tt)*) => ($create::serial_print!(
            concat!($fmt, "\n"), $($arg)*));
    }

    ```
- Since the `SerialPort` type already implements the `fmt::Write` trait, we don’t need to provide our own implementation.
- Now we can print to the serial interface instead of the VGA 

    ```rust
    // in src/main.rs

    #[cfg(test)]
    fn test_runner(tests: &[&dyn Fn()]) {
        serial_println!("Running {} tests", tests.len());
        for test in tests {
            test();
        }
        
    }

    #[test_case]
    fn trivial_test_case() {
        serial_print!("trivial assertion... ");
        assert_eq!(1, 1);
        serial_println!("[ok]");
    }
    ```

#### Print an Error Message on Panic
- To exit QEMU with an error message on a panic, we can use conditional compilation to use a different panic handler in testing mode:
    ```rust
    // our existing panic handler
    #[cfg(not(test))] // new attribute
    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        println!("{}", info);
        loop {}
    }
    ```
    ```rust
    // our panic handler in test mode
    #[cfg(test)]
    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        serial_println!("[failed]\n");
        serial_println!("Error: {}\n", info);
        exit_qemu(QemuExitCode::Failed);
        loop {}
    }
    ```
- Note that we still need an endless loop after the `exit_qemu` call because the compiler does not know that the `isa-debug-exit` device causes a program exit.


#### Hiding QEMU
- Since we report out the complete test results using the `isa-debug-exit` device and the serial port, we don’t need the QEMU window anymore. We can hide it by passing the `-display none` argument to QEMU:

    ```toml
    # in Cargo.toml
    [package.metadata.bootimage]
    test-args = [
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-serial", "stdio",
        "-display", "none"
    ]
    ```
- Useful during running CI or SSH connections

#### Timeouts
- `cargo test` waits until the test runner exits, a test that never returns can block the test runner forever. 
- In our case endless loop can occur in various situations:
  - The booloader fails to load our kernel, which causes system to reboot endlessly
  - The BIOS/UEFI firmware fails to load the bootloader, which causes the same endless rebooting
  - The CPU enter a `loop {}` statement for some function because QEMu exit device doesn't not work properly
  - the hardware causes a system reset, for ex. CPU exception is not caught
- So we will use timeout
- The feature is supported by `bootimage` tool
    ```toml
    [package.metadata.bootimage]
    test-timeout = 300 
    ```

#### Insert Print Automatically
- Currently we are writing `serial_print!` each time, but we can avoid this. And this is somethign we need in each test case by default out-of-the-box
    ```rust
    #[test_case]
    fn trivial_assertion() {
        serial_print!("trivial assertion... ");
        assert_eq!(1, 1);
        serial_println!("[ok]");
    }
    ```
- Improvements
    ```rust
    pub trait Testable {
        fn run(&self) -> ();
    }

    impl<T> Testable for T where T: Fn() {
        fn run(&self) {
            serial_print!("{}...\t", core::any::type_name::<T>());
            self();
            serial_println!("[ok]");
        }
    }
    ```

    ```rust
    #[cfg(test)]
    pub fn test_runner(tests: &[&dyn Testable]) {
        serial_println!("Running {} test case", tests.len());
        for test in tests {
            test.run();
        }
        exit_qemu(QemuExitCode::Success);
    }
    ```

    ```rust
    #[test_case]
    fn trivial_case() {
        assert_eq!(1, 1);
    }
    ```

### Testing the VGA Buffer

```rust
#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}
```

```rust
#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}
```

```rust
#[test_case]
fn test_println_output() {
    let s = "Some test string that fits on a single line";
    println!("{}", s);
    for (i, c) in s.chars().enumerate() {
        let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT - 2][i].read();
        assert_eq!(char::from(screen_char.ascii_character), c);
    }
}
```

#### Integration Tests
- The convention for integration tests in Rust is to put them into a tests directory in the project root
- Both the default test framework and custom test frameworks will automatically pick up and execute all tests in that directory.
- All integration tests are their own executables and completely separate from our main.rs
- This means that each test needs to define its own entry point function.
    ```rust
    #![no_std]
    #![no_main]
    #![feature(custom_test_frameworks)]
    #![test_runner(crate::test_runner)]
    // Generate the test harness main function and export it under the name test_main.
    #![reexport_test_harness_main = "test_main"]

    use core::panic::PanicInfo;

    #[unsafe(no_mangle)] 
    pub extern "C" fn _start() -> ! {
        test_main();

        loop{}
    }

    fn test_runner(tests: &[&dyn Fn()]) {
        unimplemented();
    }

    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        loop {}
    }
    ```
- Since integration tests are separate executables, we need to provide all the crate attributes (no_std, no_main, test_runner, etc.) again
  - As well no access to the method in the main.rs, since test are built completely separately from our `main.rs` executable
  - We use the `unimplemented` macro that always panics as a placeholder for the test_runner function and just loop in the panic handler for now.
- If you run `cargo test` at this stage, you will get endless loop because the panic handler loop endlessly

#### Create a Library
- To make the required functions available to our integration test, we need to split off a library from our main.rs
  - which can be included by other crates and integration test executables

    ```rust
    // src/lib.rs

    #![no_std]
    #![cfg_attr(test, no_main)]
    #![feature(custom_test_frameworks)]
    #![test_runner(crate::test_runner)]
    #![reexport_test_harness_main = "test_main"]

    use core::panic::PanicInfo;

    pub trait Testable {
        fn run(&self) -> ();
    }

    impl<T> Testable for T
    where
        T: Fn(),
    {
        fn run(&self) {
            serial_print!("{}...\t", core::any::type_name::<T>());
            self();
            serial_println!("[ok]");
        }
    }

    pub fn test_runner(tests: &[&dyn Testable]) {
        serial_println!("Running {} tests", tests.len());
        for test in tests {
            test.run();
        }
        exit_qemu(QemuExitCode::Success);
    }

    pub fn test_panic_handler(info: &PanicInfo) -> ! {
        serial_println!("[failed]\n");
        serial_println!("Error: {}\n", info);
        exit_qemu(QemuExitCode::Failed);
        loop {}
    }

    /// Entry point for `cargo test`
    #[cfg(test)]
    #[unsafe(no_mangle)]
    pub extern "C" fn _start() -> ! {
        test_main();
        loop {}
    }

    #[cfg(test)]
    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        test_panic_handler(info)
    }

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

    ```
- To make our `test_runner` available to executables and integration tests, we make it public and don’t apply the `cfg(test)` attribute to it. 
- `feature(custom_test_frameworks)`
  - This enables nightly support for replacing Rust’s default test harness.
  - Without this:
    - Rust generates its own runner which depends on `std`
- `test_runner(crate::test_runner)`
  - Collect all `#[test_case]` functions and pass them to this function.
    ```rust
    fn test_main() {
        test_runner(&[&test1, &test2, ...]);
    }
    ```
- `cfg_attr(test, no_main)`
  - When building normally : keep normal behavior
  - When running cargo test then apply `#![no_main]`
  - Because when testing the library in kernel mode
    - There is no OS, no C runtime
    - So we must define our `_start` 
- `reexport_test_harness_main = "test_main"`
  - This exposes the generated test harness entry function under the name `test_main`.
- The library is usable like a normal external crate
  
## Chapter 5. CPU Exceptions
- CPU exceptions occur in various erroneous situations, for example, when accessing an invalid memory address or when dividing by zero
- To react to them we have setup IDT (Interrupt Descriptor Table)
- This table provides the handler functions
- An exception signals that something is wrong with the current intruction 
- On x86 there are 20 different CPU exception type
  - Page Fault: Illegal memory access (attempt to write to unmapped or read-only page)
  - Invalid OpCode
  - General Protection: such as trying to execute a privileged instruction in user-level code or writing reserved fields in configuration registers
  - Double Fault:  When an exception occurs, the CPU tries to call the corresponding handler function. If another exception occurs while calling the exception handler, the CPU raises a double fault exception
  - Triple Fault: If an exception occurs while the CPU tries to call the double fault handler function

### The Interrupt Descriptor Table
- Handle function for each CPU exception
- IDT is the table that hardware uses directly, so we need a predefined format
- Each entry must have following 16-byte structure
    | Type | Name                        | Description                                                     |
    |------|-----------------------------|-----------------------------------------------------------------|
    | u16  | Function Pointer [0:15]     | The lower bits of the pointer to the handler function.         |
    | u16  | GDT selector                | Selector of a code segment in the global descriptor table.     |
    | u16  | Options                     | (see below)                                                     |
    | u16  | Function Pointer [16:31]    | The middle bits of the pointer to the handler function.        |
    | u32  | Function Pointer [32:63]    | The remaining bits of the pointer to the handler function.     |
    | u32  | Reserved                    |

- The options field has the following format:
    | Bits  | Name                             | Description                                                                 |
    |-------|----------------------------------|-----------------------------------------------------------------------------|
    | 0-2   | Interrupt Stack Table Index      | 0: Don’t switch stacks, 1-7: Switch to the n-th stack in the Interrupt Stack Table when this handler is called. |
    | 3-7   | Reserved                         |                                                                             |
    | 8     | 0: Interrupt Gate, 1: Trap Gate  | If this bit is 0, interrupts are disabled when this handler is called.     |
    | 9-11  | must be one                      |                                                                             |
    | 12    | must be zero                     |                                                                             |
    | 13-14 | Descriptor Privilege Level (DPL) | The minimal privilege level required for calling this handler.             |
    | 15    | Present                          |                                                                             |

- Each exception has predefined index in the IDT
  - Eg. page fault has index 14
  - Thus hardware can automatically load corresponding IDT entry for each exception
- When an exception occurs, the CPU roughly does the following:
  - Push some register on the stack, including the instruction pointer and the RFLAGS register
  - Read the corresponding entry from the IDT. For example, the CPU reads the 14th entry when a page fault occurs
  - Check if the entry is present and, if not, raise a double fault
  - Disable hardware interrupts if the entry is an interrupt gate (bit 40 not set)
  - Load the specified GDT selector into the CS (code segment)
  - Jump to the specified handler function

### An IDT Type
- Instead of creating our own IDT type, we will use the `InterruptDescriptorTable` struct of the `x86_64` crate, which looks like this:
    ```rust
    #[repr(C)]
    pub struct InterruptDescriptorTable {
        pub divide_by_zero: Entry<HandlerFunc>,
        pub debug: Entry<HandlerFunc>,
        pub non_maskable_interrupt: Entry<HandlerFunc>,
        pub breakpoint: Entry<HandlerFunc>,
        pub overflow: Entry<HandlerFunc>,
        pub bound_range_exceeded: Entry<HandlerFunc>,
        pub invalid_opcode: Entry<HandlerFunc>,
        pub device_not_available: Entry<HandlerFunc>,
        pub double_fault: Entry<HandlerFuncWithErrCode>,
        pub invalid_tss: Entry<HandlerFuncWithErrCode>,
        pub segment_not_present: Entry<HandlerFuncWithErrCode>,
        pub stack_segment_fault: Entry<HandlerFuncWithErrCode>,
        pub general_protection_fault: Entry<HandlerFuncWithErrCode>,
        pub page_fault: Entry<PageFaultHandlerFunc>,
        pub x87_floating_point: Entry<HandlerFunc>,
        pub alignment_check: Entry<HandlerFuncWithErrCode>,
        pub machine_check: Entry<HandlerFunc>,
        pub simd_floating_point: Entry<HandlerFunc>,
        pub virtualization: Entry<HandlerFunc>,
        pub security_exception: Entry<HandlerFuncWithErrCode>,
        // some fields omitted
    }
    ```
- The fields have the type `idt::Entry<F>`, which is a struct that represents the fields of an IDT entry
- The type parameter `F` defines the expected handler function type
- We see some entries requires a `HanlderFunc` and some entries requires `HandlerFuncWithErrCode`
  - The page fault even has its own special type: `PageFaultHandlerFunc`
- Let's look at the `HandlerFunc` type first:
    ```rust
    type HandlerFunc = extern "x86-interrupt" fn (_: InterruptStackFrame);
    ```
- It's a type alias for an extern x86-interrupt `fn` type
- The `extern` keyword defines a function with a foreign calling convention and is often used to communicate with C code
  - This is an agreeement to the contract which both sign so that they can understand each other
- But what is `x86-interrupt` calling convention?

### The Interrupt Calling Convention
- Exceptions are quite simlar to function calls
- The CPU jumps to the first instruction of the called function and execute it.
- Afterwards, CPU jumps to the return address and continue the execution of parent function
- However, there is a major difference between exception and function calls:
  - A function call is invoked voluntarily by a compiler inserted `call` instruction
  - While an exception might occur at any instruction
- In order to understand the consequences of the differences, we need to examine function calls in more details
- Calling Convention specify the details of function call
  - For example, they specify where function parameter are placed (e.g., in register or on the stack) and how results are returned
  - On `x86_64` linux, the following rules applies to a C function (Specified by system V ABI)
    - The first six interger arguments are passed in register
      - `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9` 
    - Additional arguments are passed over the stack
    - Return results are returned in `rax` and `rdx`
- Note that Rust doesn't follow C ABI (In fact, there isn't Rust ABI yet), so the rules apply only to the functions declared with `extern "C" fn`

#### Preserved and Scratch Register
- The calling convention divides the registers into two parts:
  - preserved and scratch registers
- The values of preserved registers must remain unchanged across function calls.
- So a called function (the "callee") is only allowed to overwrite these registers if it restores their original value before returning. 
  - Therefore, these registers are called "callee-saved"
  - A common pattern is to save these registers to the stack at the function's beginning and restore them just before returning
- In contrast, a called function is allowed to overwrite scratch register without restrictions
  - If the caller wants to preserve the value of the scratch register across a function call, it needs to backup and restore it before function call (e.g., by pushing it to the stack).
  - So scratch registers are caller-saved
- On `x86_64`, the C calling convention specifies the following preserved and scratch register
    | Category            | Registers                                      | Convention     |
    |---------------------|-----------------------------------------------|---------------|
    | Preserved Registers | rbp, rbx, rsp, r12, r13, r14, r15             | Callee-saved  |
    | Scratch Registers   | rax, rcx, rdx, rsi, rdi, r8, r9, r10, r11     | Caller-saved  |
- The compiler know these rules, so it generate the code accordingly 
- For example most function begin with `push rbp`, which backup `rbp` on the stack (because it's a callee-saved register)

#### Preserving all Registers
- In contrast to the function calls, exception can occur on any instruction
- In most cases, we don't even know at compile time if the generated code will cause an exception
- For example, the compiler can't know if an instruction causes a stack overflow or a page fault
- Since we don't know when an exception occurs, we can't backup any registers before
- This means we can't use a calling convention that relies on caller-saved register for exception handler
- Instead, we need a calling convention that preserves **all register**
- The `x86-interrupt` calling convention is such a calling convention
  - So it guarantees that all the register value are restored to their original value on function return
- Note that this doesn't means all registers are saved to the stack at function entry. Instead, the compiler only backups the register which are overwritten by function. 
  - This way, very efficient code can be generated for short function that only uses few register

#### The Interrupt Stack Frame
- On a normal function call (using `call` instruction), the CPU pushes the return address before calling the target function
- On function return (using the `ret` instruction), the CPU pops this return address and jump to it.
    ![Old and New Stack](./images/return.png)
- For exception and interrupt handler, however pushing a return address is not sufficient (Think why?)
  - Because since interrupt handler often run in different context (stack pointer, CPU flags, etc)
- Instead, the CPU performs the following step when an interrupt occurs:

0. **Saving the old stack pointer**: The CPU reads the stack pointer `rsp` and stack segment `ss` register value (base of stack) and remember them in an internal buffer.
1. **Aligning the Stack pointer**: Special casing - An interrupt can occur any intruction, so the stack pointer can have any value, too. However, some CPU instruction (e.g., some SSE instruction) requires that the stack pointer be aligned on a 16-byte boundary, so the CPU performs such an alignment right after the interrupt
2. **Switching Stack**(in some cases): A stack switch occurs when the CPU priviledge level changes, for example, when a CPU exception occurs in a user-mode program. It is also possible to configure stack switches for a specific interrupt using so called *Interrupt Stack Table* 
3. **Pushing the old stack pointer**: The CPU pushes the `rsp` and `ss` values from step 0 to the stack. This makes it possible to restore the original stack pointer when returning from an interrupt handler
4. **Pushing and updating the `RFLAG` register**: The `RFLAG` register contains various control and status bit. On interrupt entry, the CPU changes some bits and pushes the old value
5. **Pushing the IP - Instruction Pointer**: Before jumping to the interrupt handler function, the CPU pushes the instruction pointer (`rip`) and the code segment (`cs`). This is comparable to the return address push of a normal function call
6. **Pushing an error code**: For some specific exception, such as page fault, the CPU pushes an error code, which describes the cause of the exception
7. **Invoking the interrupt handler** The CPU reads the address and segment descriptor of the interrupt handler function from the corresponding field in the IDT. It then invokes the handler by loading the value into the `rip` and `cs` registers
    ![Interrupt Stack Frame](./images/interrupt-stack-frame.png)

- In `x86_64` crate, the interrupt stack frame is represented by the `InterruptStackFrame` struct
- It is passed to the interrupt handler as `&mut` and can be used to retrieve additional information about exception cause
- The struct contains no error code field, since only a few exception push an error code
  - These exception uses the separate `HandlerFuncWithErrCode` function type, which has an additional `error_code` segment

### Implementation

- Its time to handle CPU exception in our kernel, We'll start by creating a new interrupt module in `src/interrupt.rs`
    ```rust
    // src/lib.rs
    pub mod interrups;

    // src/interrupts.rs

    use x86_64::structures::idt::InterruptDescriptorTable;

    pub fn init_idt() {
        let mut idt = InterruptDescriptorTable::new();
    }

    ```
- Now we can add handler functions. We start by adding a handler for the breakpoint exception 
- The purpose is to temporarily pause a program when the breakpoint instruction `int3` is executed
- the breakpoint exception is commonly used in debuggers: When the user sets the breakpoint, the debugger overwrites the corresponding instruction with the `int3` instruction so that the CPU throws the breakpoint exception when it reach that line
- When the user wants to continue the program, the debugger replaces the `int3` instruction with the original instruction again and continue the program
- For our use case, we don't need to overwrite an instruction.
  - Instead, we just want to print a message when the breakpoint instruction in executed and then continue the program
  - So lets create a simple `breakpoint_handler` function and add it to our IDT:
    ```rust
    // src/interrupts.rs

    use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
    use crate::println;

    pub fn init_idt() {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler(breakpoint_handler);
    }

    extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
        println!("EXCEPTION BREAKPOINT\n {:#?}", stack_frame);
    }
    ```
- `x86-interrupt` calling convention is still unstable. To use it anyway, we have to explicitly enable it by adding `#![feature(abi_x86_interrupt)]` at the top of our `lib.rs`
- Loading the IDT
    ```rust
    // in src/interrupts.rs

    pub fn init_idt() {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler(breakpoint_handler);
        idt.load();
    }
    ```
- The `load` method expect a `&'static self`, that is, a reference valid for the complete runtime of the program
- The reason is that the CPU will access this table on every interrupt. so using a shorter lifetime than `'static` could lead to use-after-free bugs
- In order to fix this problem, we need to store our idt at a place where it has a `'static` lifetime. To achieve this, we could allocate our IDT on the heap using `Box` and then convert it to a `'static` reference, but we are writing an OS kernel and thus don’t have a heap (yet).
- As an alternative, we could try to store the IDT as a `static`:
    ```rust
    static IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

    pub fn init_idt() {
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.load();
    }
    ```
- However, there is a problem: Statics are immutable, so we can’t modify the breakpoint entry from our init function. We could solve this problem by using a `static mut`. But as soon as you add `mut` with the `static` it is unsafe for RUSt. Because the variable is available every where in the code and multiple threads can modify this. There is no borrowing system tracking usage. 
- So the compiler cannot prove: that two mutable accesses won't happen simultaneously
- Since rust can not verify you must use `unsafe` which means As a programmer, I guarantee this is safe
    ```rust
    static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

    pub fn init_idt() {
        unsafe {
            IDT.breakpoint.set_handler_fn(breakpoint_handler);
            IDT.load();
        }
    }
    ```
#### Lazy Static to Rescue
- The `lazy_static` uses the `unsafe` behind the scene, but it is abstracted away in a safe interface
    ```rust
    use lazy_static::lazy_static;

    lazy_static! {
        static ref IDT: InterruptDescriptorTable = {
            let mut idt = InterruptDescriptorTable::new();
            idt.breakpoint.set_handler_fn(breakpoint_handler);
            idt
        }
    }

    pub fn init_idt() {
        IDT.load();
    }
    ```

#### Test

```rust
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("hello world{}", "!");

    oxid_os::init();

    x86_64::instructions::interrupts::int3();

    #[cfg(test)]
    test_main();

    println!("it did not crash!");
    loop {}
}
```

## Chapter 6. Double Faults

### What is Double Fault ?
- An Exception that occurs when the CPU fails to invoke an exception handler
  - For example, it occurs when a page fault is triggered, but there is no page fault handler
- It is important to provide the double fault handler, becuase if a double fault is unhandled, then triple fault occurs. Triple fault can not be caught, and most hardware reacts with a system reset

#### Triggering a Double Fault

- Provoke a double fault handler 
    ```rust
    #[unsafe(no_mangle)]
    pub extern "C" fn _start() -> ! {
        println!("hello world!");

        oxid_os::init();

        // triggers page fault
        unsafe {
            *(0xsouravsh as *mut u8) = 42;
        };    

    }
    ```
- We used `unsafe` because we are trying to access an invalid address i.e., `souravsh`
  - Thus page fault will happen.
- We haven't registered a page fault handler in our IDT, so a double page fault occurs
  - The CPU on this fault will look for double fault handler, and that is also not defined, which will lead to triple fault
  - A triple fault is fatal, and QEMU reacts to it like most real hardware and issues a system reset

### A Double Fault Handler
- A double fault handler is a normal exception with an error code 
    ```rust
    lazy_static! {
        static ref IDT: InterruptDescriptorTable = {
            let mut idt = InterruptDescriptorTable::new();
            idt.breakpoint.set_handler_fn(breakpoint_handler);
            idt.double_fault.set_handler_fn(double_fault_handler);
            idt
        };
    }

    extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
        panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    }
    ```
- The error code of a double fault handler is always 0, so there's is no reason to print this
- One difference is you defined `double_fault_handler` as diverging function i.e., no return (`!`)
  - The reason is that `x86_64` architecture doesn't allow to return from a double fault exception
- Ok, then done ? No the current approach is not sufficient. It doesn't cover all the cases.

### Causes of Double Faults
- What causes double faults ?
  - A double fault is a special exception that occurs when the CPU **fails to invoke** an exception handler.
- What does "fail to invoke" means exactly ?
  - The handler not present ?
  - The handler is swapped out ?
  - And what happen if handler causes exception itself ?
- For example,
  - A breakpoint exception occurs, but the corresponding handler function is swapped out?
  - A page fault occurs, but the page fault handler is swapped out?
  - Our kernel overflows its stack and the guard page is hit?
- According to AMD64 manual 
  - A double fault exception **can** occur when a second exception occurs during the handling of a prior (first) exception handler
  - The “can” is important: Only very specific combinations of exceptions lead to a double fault. 
- Importantly, the CPU can itself fault while trying to deliver the exception. Whether this leads to normal handling or a double fault depends entirely on what kind of fault happens during this delivery process.
- The combinations are:

| First Exception              | Second Exception                                                                 |
|-----------------------------|----------------------------------------------------------------------------------|
| Divide-by-zero              | Invalid TSS, Segment Not Present, Stack-Segment Fault, General Protection Fault |
| Invalid TSS                 | Invalid TSS, Segment Not Present, Stack-Segment Fault, General Protection Fault |
| Segment Not Present         | Invalid TSS, Segment Not Present, Stack-Segment Fault, General Protection Fault |
| Stack-Segment Fault         | Invalid TSS, Segment Not Present, Stack-Segment Fault, General Protection Fault |
| General Protection Fault    | Invalid TSS, Segment Not Present, Stack-Segment Fault, General Protection Fault |
| Page Fault                  | Page Fault, Invalid TSS, Segment Not Present, Stack-Segment Fault, General Protection Fault |

- So, for example, a divide-by-zero fault followed by a page fault is fine (the page fault handler is invoked), but a divide-by-zero fault followed by a general-protection fault leads to a double fault.
- With the help of this table, we can answer the first three of the above questions:
  - If a breakpoint exception occurs and the corresponding handler function is swapped out, a page fault occurs and the page fault handler is invoked (not a double fault)
  - If a page fault occurs and the page fault handler is swapped out, a double fault occurs and the double fault handler is invoked.

#### Kernel Stack Overflow
- What happen if our kernel stack overflow its stack and the guard page is hit
- A guard page is a special memory page at the bottom (stack grows from top to down) of a stack 
- This makes it possible to detect stack overflow
- The page is not mapped to any physical frame
  - So accessing it cause page fault instead of silently corrupted the memory 
- The **bootloader** set up the guard page for our kernel stack
  - So stack overflow causes **page fault**
- When the page fault happen CPU looks for the page fault handler in the IDT and tries to push the IDT stack frame onto the stack
  - However current stack poninter points to the non-present guard page
  - Thus a second page fault occurs, which causes the double fault 
- So CPU tries to calls the double fault handler now
  - However in double fault CPU tries to push the exceptions stack frame too
  - The stack pointer still points to the guard page, so a third page fault occurs
  - Which causes triple fault - a system reboot
  - So our current double fault handler can not avoid triple fault
- We need to somehow ensure that stack is valid when double fault happen
- Fortunately x86_64 has solution for this problem

### Switching Stacks
- The `x86_64` architecture is able to switch to a **predefined, known-good stack when an exception occurs**
- This switch happens at **hardware level**, so it can be performed before CPU pushes the exception stack frame
- The switching mechanism is implemented as an Interrupt Stack Table (IST). 
- The IST table of 7 pointers to known-good stack. In Rust pseudocode:
    ```rust
    struct InterruptStackTable {
        stack_pointers: [Option<StackPointer>; 7],
    }
    ```
- For each exception we can choose a stack from the IST

#### The IST and TSS
- The IST is part of legacy TSS (Task State Segment)
- The TSS used to hold various pieces of information
  - Processor register state about a task in 32-bit mode
- On x86_64, the TSS no longer holds any task-specific information at all.
- Instead it holds, two stack table (the IST is one of them)
- The 64-bit TSS has 
  - Privilege Stack Table
    - Used by CPU when privilege level changes
    - For ex. if exception occurs while the CPU is in user mode (privilege level 3), the CPU normally switches to kernel mode (privilege level 0) before invoking the exception handler
    - We don't have any user level program so far, so we will this for next sections
  - Interrupt Stack Table
  - I/O Map based address

#### Creating a TSS
- Let's create a new TSS which contains the separate double fault stack in its Interrupt Stack table
- For that we need TSS struct - x86_64 provides that `TaskStateSegment struct`
    ```rust
    // in /src/lib.rs
    pub mod gdt;

    // in src/gdt.rs

    use x86_64::VirtAddr;
    use x86_64::structures::tss::TaskStateSegment;
    use lazy_static::lazy_static;

    pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

    lazy_static! {
        static ref TSS: TaskStateSegment {
            let mut tss = TaskStateSegment::new();
            tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
                const STACK_SIZE: usize = 4096 * 5;
                static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

                let stack_start = VirtAddr::from_ptr(&raw const STACK);
                let stack_end = stack_start + STACK_SIZE;
                stack_end
            };
            tss
        }
    }
    ```
- We are using `lazy_static` because Rust's const evaulator is not yet powerful enough to do this init at the compile time
- We define the $0^{th}$ entry is the double fault stack (we can pick any other index too)
- Then we write the top address of a double fault stack into the 0th entry
  - We write top because stack in x86_64 grows downwards, from high address to low address
- We haven't implemented memory management yet, so we dont' have proper way to allocate new stack
- Instead we will use `static mut` array as stack storage for now
- Note that `static mut` is not an immutable static, because otherwise the bootloader will map it to a read-only page 
  - Later we will implement stack and then we will replace this impl

#### Loading the TSS
- Now we need a way to tell the CPU that it should use the TSS (new which we created)
- Unfortunately, TSS uses the segmentation system (historical reason)
- Instead of loading the table directly, we need to add a new segment descriptor to the GDT
- Then we can load our TSS by invoking the `ltr` instruction with respective GDT index
- LTR is Load Task Register
  - Task Register is a special CPU register that 
    - Points to one TSS
    - The TSS holds the kernel stack pointer, I/O bitmap and some CPU state
    - Modern OS doesn't use this for task switching, instead only use for privilege transitions

#### The Global Descriptor Table
- GDT is relic that was used for memory segmentation before paging become the de facto standard
- However it is still need in 64 bit mode for various things, such as kernel/user mode configuration or TSS loading
- The GDT is a structure that contains the segments of a program
  - It was used in older architecutre to isolate the program memory 
- While segmentation is no more used in 64-bit, GDT still exists
- It is mostly used for two things
  - Switching between kernel space and user space
  - And loading TSS structure

##### Creating a GDT
- Let's create a GDT that includes a segment for our TSS static:
    ```rust
    use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};

    lazy_static! {
        static ref GDT: GlobalDescriptorTable = {
            let mut gdt = GlobalDescriptorTable::new();
            gdt.add_entry(Descriptor::kernel_code_segment);
            gdt.add_entry(Descriptor::tss_segment(&TSS));
            gdt
        }
    }
    ```
- Loading the GDT: to load our GDT, we create a `gdt::init` function that we call from our `init` method
    ```rust
    // in src/gdt.rs

    pub fn init() {
        GDT.load();
    }

    // in src/lib.rs
    pub fn init() {
        gdt::init();
        interrupts::init_idt();
    }

    ```

#### The final Steps
- The problem is that the GDT segments are not yet active because the segment and TSS register still contains the value from the old GDT
  - We also need to modify the double Fault IDT entry so that it uses the new stack
- In summary
  - We changed our GDT, so we should reload `cs`, the code segment register
    - This is required since the old segment selector could now point to different GDT
  - Load the TSS - We loaded the GDT that contains the TSS selector, but we still need to tell CPU that use TSS
  - Update the IDT entry - As soon as our TSS loaded, the CPU has access to the valid IST
    - Then we can tell CPU that it should use our new double fault stack by modifying our double fault IDT entry
- For first two steps
    ```rust
    use x86_64::structures::gdt::SegmentSelector;

    lazy_static! {
        static ref GDT: (GlobalDescriptorTable, Selectors) = {
            let mut gdt = GlobalDescriptorTable::new();
            let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
            let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
            (gdt, Selectors { code_selector, tss_selector })
        }
    }

    struct Selector {
        code_selector: SegmentSelector,
        tss_selector: SegmentSelector,
    }
    ```
- Now we can use the selector to reload the `cs` and load our `TSS`
    ```rust
    pub fn init() {
        use x86_64::instructions::tables::load_tss;
        use x86_64::instructions::segmentation::{CS, Segment};

        GDT.0.load();
        // Might possible to break memory safety by loading invalid selector, 
        // thus unsafe
        unsafe {
            // reload the code segment register using set_reg
            CS::set_reg(GDT.1.code_selector);
            // load the tss using load_tss
            load_tss(GDT.1.tss_selector);
        }
    }
    ```
- Now we have loaded TSS and interrupt stack table, we can set the stack index for our double fault handler in the IDT

    ```rust
    // in src/interrupts.rs

    use crate::gdt;
    lazy_static! {
        static ref IDT: InterruptDescriptorTable = {
            let mut idt = InterruptDescriptorTable::new();
            idt.breakpoint.set_handler_fn(breakpoint_handler);
            // set_stack_index is unsafe because of index is valid and not
            // used by any other exception
            unsafe {
                idt.double_fault.set_handler_fn(double_fault_handler)
                        .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            }
        }
    }
    ```
- From now we should never see triple fault again