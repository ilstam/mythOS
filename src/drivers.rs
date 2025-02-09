pub mod gpio;
pub mod uart_mini;

use core::{marker::PhantomData, ops};

type AddressPhysical = usize;

// Peripherals base address for BMC2837 / RPI3
pub(crate) const PERIPHERALS_BASE: AddressPhysical = 0x3F00_0000;

pub(crate) struct MMIORegisters<T> {
    base: AddressPhysical,
    phantom: PhantomData<fn() -> T>,
}

impl<T> MMIORegisters<T> {
    // SAFETY: This is safe as long as there is a MMIO peripheral behind the address
    pub const unsafe fn new(base: AddressPhysical) -> Self {
        Self {
            base,
            phantom: PhantomData,
        }
    }

    pub const fn base_addr(self) -> AddressPhysical {
        self.base
    }
}

impl<T> ops::Deref for MMIORegisters<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: See MMIORegisters::new()'s safety section
        unsafe { &*(self.base as *const _) }
    }
}
