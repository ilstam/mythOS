pub mod gpio;

type AddressPhysical = usize;

// Peripherals base address for BMC2837 / RPI3
pub(crate) const PERIPHERALS_BASE: AddressPhysical = 0x3F00_0000;
