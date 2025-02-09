use crate::drivers::{AddressPhysical, PERIPHERALS_BASE};
use core;

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

impl GPIOPin {
    const NUM_GPIO_PINS: u8 = 54;
    const GPFSEL0_OFFSET: AddressPhysical = 0x20_0000;
    const GPFSEL0: AddressPhysical = PERIPHERALS_BASE + Self::GPFSEL0_OFFSET;

    pub fn new(pin: u8) -> Self {
        assert!(pin < Self::NUM_GPIO_PINS);
        GPIOPin { pin }
    }

    pub fn select_mode(self, mode: PinMode) {
        // Used to pick a register from GPFSEL0 to GPFSEL5
        let reg_index = self.pin as usize / 10;
        // Used to pick a field from FSEL0 to FSEL9 inside a GPFSELX register
        let field_index = self.pin as usize % 10;
        let gpfselx = Self::GPFSEL0 + reg_index * 4;

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
}
