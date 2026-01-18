
- [Chapter 1. A Freestanding Rust Binary](#chapter-1-a-freestanding-rust-binary)
  - [The no\_std Attribute](#the-no_std-attribute)
  - [Panic Implementation](#panic-implementation)
  - [Concrete things compiler MUST decide](#concrete-things-compiler-must-decide)
  - [What happens when you run a program ?](#what-happens-when-you-run-a-program-)
  - [Name mangling](#name-mangling)
  - [C ABI](#c-abi)
  - [Linker Errors](#linker-errors)


## Chapter 1. A Freestanding Rust Binary

[Philipp Oppermann's blog](https://os.phil-opp.com/freestanding-rust-binary/)

### The no_std Attribute
- Disabling the Standard Library: Rust crate link the standary library, which depends on the operating system for feature such as threads, files or networking. It also depends on C standard library `libc` which closely interacts with the OS services. 
- We are developing our own OS, so we would like to not do that
- Right now our crate implicitly links the standard library.
- To disable it we will use `no_std` 

### Panic Implementation
- The standard library provides its own panic handler function, but in a no_std environment we need to define it ourselves:

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
- Any type implementing the `trait` means it follows the contract, and thus compiler can decide what instruction to inject. Thus this becomes a **mandatory** thing for the rust compiler. The default comes from the Rust standard library, which internally depends on `libc` present in the `/usr/lib`. This is the core library which implements the memory allocation functions, and a lot more. This also includes utilities which setups the syscalls (kernel functions are not normal function calls, so they can't be resolve by *linker*, instead architecture-specific language thunks are used to call into a kernel) 
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

### C ABI 
- The OS understands a binary contract named as ABI (Application Binary Interface) and it understands only C ABI
- Bootloaders like GRUB already follow this 
    - So order is : Hardware -> Bootloader (often written for C ABI) -> your `_start` -> your kernel code 
- ABI is machine-level agreement about how code  humps into one other 
- Marking `extern "C"` to tell the compiler that **it should use the C calling convention for this function** (instead of unspecified Rust calling convention) 
    - As i explained earlier, this is required because the entry point is not called by any function, but invoked directly by the bootloader (or any other OS)

### Linker Errors











