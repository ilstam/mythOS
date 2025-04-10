pub mod gpio;
pub mod interrupt_controller;
pub mod uart_mini;

use crate::memory::{AddressPhysical, AddressVirtual};
use core::{marker::PhantomData, ops};

// Peripherals base address for BMC2837 / RPI3
pub(crate) const PERIPHERALS_BASE: AddressVirtual = AddressPhysical::new(0x3F00_0000).as_virtual();

pub(crate) struct MMIORegisters<T> {
    base: AddressVirtual,
    phantom: PhantomData<fn() -> T>,
}

impl<T> MMIORegisters<T> {
    // SAFETY: This is safe as long as there is a MMIO peripheral behind the address
    pub const unsafe fn new(base: AddressVirtual) -> Self {
        Self {
            base,
            phantom: PhantomData,
        }
    }

    pub const fn base_addr(self) -> AddressVirtual {
        self.base
    }
}

impl<T> ops::Deref for MMIORegisters<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: See MMIORegisters::new()'s safety section
        unsafe { &*(self.base.as_u64() as *const _) }
    }
}
