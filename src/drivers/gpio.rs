use crate::drivers::{MMIORegisters, PERIPHERALS_BASE};
use core;
use tock_registers::interfaces::Writeable;
use tock_registers::register_structs;
use tock_registers::registers::{ReadWrite, WriteOnly};

#[allow(dead_code)]
pub(crate) enum PinMode {
    INPUT = 0b000,
    OUTPUT = 0b001,
    ALT0 = 0b100,
    ALT1 = 0b101,
    ALT2 = 0b110,
    ALT3 = 0b111,
    ALT4 = 0b011,
    ALT5 = 0b010,
}

pub(crate) struct GPIOPin {
    pin: u8,
}

// SAFETY: There should a be a GPIO module behind that address as per BMC2837
const REGS: MMIORegisters<GPIORegisters> =
    unsafe { MMIORegisters::<GPIORegisters>::new(PERIPHERALS_BASE + 0x20_0000) };

register_structs! {
    #[allow(non_snake_case)]
    GPIORegisters {
        (0x000 => GPFSEL0: ReadWrite<u32>),
        (0x004 => _reserved0),
        (0x01c => GPSET0: WriteOnly<u32>),
        (0x020 => GPSET1: WriteOnly<u32>),
        (0x024 => _reserved1),
        (0x028 => GPCLR0: WriteOnly<u32>),
        (0x02c => GPCLR1: WriteOnly<u32>),
        (0x030 => _reserved2),
        (0x0b4 => @END),
    }
}

impl GPIOPin {
    const NUM_GPIO_PINS: u8 = 54;

    pub fn new(pin: u8) -> Self {
        assert!(pin < Self::NUM_GPIO_PINS);
        GPIOPin { pin }
    }

    pub fn select_mode(self, mode: PinMode) {
        // Used to pick a register from GPFSEL0 to GPFSEL5
        let reg_index = self.pin as usize / 10;
        // Used to pick a field from FSEL0 to FSEL9 inside a GPFSELX register
        let field_index = self.pin as usize % 10;
        let gpfselx = REGS.base_addr() + reg_index * 4;

        // SAFETY: We trust there's a GPFEL register behind that address and
        // that the calculation above is correct.
        // TODO: Guard this read/write operation with a mutex
        let mut value = unsafe { core::ptr::read_volatile(gpfselx as *const u32) };

        // Clear FSELx
        value &= !(0b111 << (field_index * 3));
        // Set the mode
        value |= (mode as u32) << (field_index * 3);

        // SAFETY: Same as above
        unsafe {
            core::ptr::write_volatile(gpfselx as *mut u32, value);
        };
    }

    #[allow(dead_code)]
    pub fn set_high(self) {
        if self.pin < 32 {
            REGS.GPSET0.set(1 << self.pin);
        } else {
            REGS.GPSET1.set(1 << (self.pin - 32));
        }
    }

    #[allow(dead_code)]
    pub fn set_low(self) {
        if self.pin < 32 {
            REGS.GPCLR0.set(1 << self.pin);
        } else {
            REGS.GPCLR1.set(1 << (self.pin - 32));
        }
    }
}
