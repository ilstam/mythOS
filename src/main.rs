#![no_main]
#![no_std]

mod drivers;
mod exceptions;
mod irq;
mod logging;

use aarch64_cpu::asm;
use aarch64_cpu::registers::{CurrentEL, ELR_EL2, HCR_EL2, SPSR_EL2, SP_EL1};
use core::arch::global_asm;
use drivers::uart_mini;
use tock_registers::interfaces::{Readable, Writeable};

// NOTE: It's the symbol's address we are interested in, not the value stored there
extern "C" {
    static __kernel_load_addr: usize;
}

global_asm!(include_str!("boot.s"));

pub fn jump_to_el1() {
    match CurrentEL.read(CurrentEL::EL) {
        1 => return,
        2 => {}
        el => panic!("Unexpected EL: {}", el),
    }

    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

    // Also SPSR_EL2 bit 4 must be 0 to indicate that we'll return to the
    // AArch64 execution state. Unfortunately the aarch64_cpu crate doesn't
    // support that bit, but the write() function will make sure it's cleared.
    SPSR_EL2.write(
        SPSR_EL2::D::Masked
            + SPSR_EL2::A::Masked
            + SPSR_EL2::I::Masked
            + SPSR_EL2::F::Masked
            + SPSR_EL2::M::EL1h,
    );

    ELR_EL2.set(crate::main as *const () as u64);

    // SAFETY: Assume the symbol is defined correctly in the linker script
    let stack_top = unsafe { &__kernel_load_addr as *const usize as usize };
    SP_EL1.set(stack_top as u64);

    asm::eret();
}

#[no_mangle]
pub fn main() -> ! {
    jump_to_el1();
    exceptions::install_exception_table();
    irq::enable_interrupts();

    uart_mini::init(115200);

    let a = 4;
    let b = 5;
    println!("Hello {} with some math: {a} + {b} = {}", "world", a + b);

    loop {
        asm::wfi();
    }
}
