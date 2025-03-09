#![no_main]
#![no_std]

mod drivers;
mod exceptions;
mod irq;
mod locking;
mod logging;
mod paging;

use crate::locking::IRQSpinLock;
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

pub static PENDING_ACTIONS: IRQSpinLock<u64> = IRQSpinLock::new(0);

pub enum ACTIONS {
    UartAction = 0,
}

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
    paging::setup_paging();
    exceptions::install_exception_table();
    irq::enable_interrupts();

    uart_mini::init(115200);

    let a = 4;
    let b = 5;
    println!("Hello {} with some math: {a} + {b} = {}", "world", a + b);

    loop {
        loop {
            irq::disable_interrupts();

            let mut actions = PENDING_ACTIONS.lock();
            let pending = *actions;
            if pending == 0 {
                break;
            }
            *actions = 0;
            drop(actions);

            // Interrupts must be enabled while we process pending actions
            irq::enable_interrupts();

            if pending & (1 << (ACTIONS::UartAction as u64)) != 0 {
                uart_mini::process_pending_chars();
            }
        }

        // When we get here interrupts must be disabled, otherwise an interrupt
        // could arrive just before the WFI but after we checked for pending actions
        asm::wfi();
        irq::enable_interrupts();
    }
}
