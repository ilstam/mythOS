pub mod gpio;
pub mod interrupt_controller;
pub mod uart_mini;

use crate::memory::{AddressVirtual, PERIPHERALS_BASE};
use aarch64_cpu::asm;
use core::{marker::PhantomData, ops};

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

#[inline]
fn peripheral_switch_in() {
    // See BCM2835-ARM-Peripherals.pdf section 1.3 for an explanation of why
    // this is necessary. That section suggests that we should place:
    // * A memory write barrier before the first write to a peripheral.
    // * A memory read barrier after the last read of a peripheral.
    //
    // This Linux patch suggests that perhaps only the read barrier might be
    // required: https://lore.kernel.org/all/1426213936-4139-1-git-send-email-eric@anholt.net/
    // However, that helper was dropped, see the full series history here:
    // https://lore.kernel.org/all/1430857666-18877-2-git-send-email-eric@anholt.net/
    //
    // In any case this is just a hobby kernel and we don't care that much
    // about performance so use a full SY barrier for peace of mind. This can
    // be optimized later if necessary.
    asm::barrier::dmb(asm::barrier::SY);
}
