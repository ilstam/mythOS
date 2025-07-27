# My toy homebrew OS (mythOS)

This is a toy project to have fun and improve my Rust and ARM64 knowledge by building an OS kernel for Raspberry Pi 3.

Development is slow because I'm busy. :)

## What's done

* Booting and running the kernel from high addresses (link address != load address)
* Paging
* Exception handling and context switching
* Simple locking primitives
* Drivers for the interrupt controller, UART, GPIO module and mailbox interface.
* Physical memory allocator

## What's next

* Userspace processes and scheduling
* System calls and IPC
* SMP
* Block and filesystem drivers
* Graphics and/or a network stack if life allows
