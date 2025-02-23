// This is a driver for the interrupt controller included in BMC2837

use crate::drivers::{MMIORegisters, PERIPHERALS_BASE};
use crate::irq::IRQ;
use tock_registers::interfaces::Writeable;
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite};

// SAFETY: There should an interrupt controller behind that address as per BMC2837
const REGS: MMIORegisters<ICRegisters> =
    unsafe { MMIORegisters::<ICRegisters>::new(PERIPHERALS_BASE + 0xB000 + 0x200) };

register_structs! {
    #[allow(non_snake_case)]
    ICRegisters {
        (0x00 => IRQ_BASIC_PENDING: ReadOnly<u32>),
        (0x04 => IRQ_PENDING1: ReadOnly<u32>),
        (0x08 => IRQ_PENDING2: ReadOnly<u32>),
        (0x0c => FIQ_CONTROL: ReadWrite<u32>),
        (0x10 => ENABLE_IRQ1: ReadWrite<u32>),
        (0x14 => ENABLE_IRQ2: ReadWrite<u32>),
        (0x18 => ENABLE_IRQ_BASIC: ReadWrite<u32>),
        (0x1c => DISABLE_IRQ1: ReadWrite<u32>),
        (0x20 => DISABLE_IRQ2: ReadWrite<u32>),
        (0x24 => DISABLE_BASIC_IRQ: ReadWrite<u32>),
        (0x28 => @END),
    }
}

pub fn enable_irq(irq: IRQ) {
    match irq {
        IRQ::ARM(irq) => {
            REGS.ENABLE_IRQ_BASIC.set(1 << (irq as u32));
        }
        IRQ::GPU(irq) => {
            let irq = irq as u32;
            if irq < 32 {
                REGS.ENABLE_IRQ1.set(1 << irq);
            } else {
                let irq = irq - 32;
                REGS.ENABLE_IRQ2.set(1 << irq);
            }
        }
    }
}

#[allow(dead_code)]
pub fn disable_irq(irq: IRQ) {
    match irq {
        IRQ::ARM(irq) => {
            REGS.DISABLE_BASIC_IRQ.set(1 << (irq as u32));
        }
        IRQ::GPU(irq) => {
            let irq = irq as u32;
            if irq < 32 {
                REGS.DISABLE_IRQ1.set(1 << irq);
            } else {
                let irq = irq - 32;
                REGS.DISABLE_IRQ2.set(1 << irq);
            }
        }
    }
}
