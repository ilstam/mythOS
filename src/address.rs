use crate::memory::{GiB, PAGE_SIZE};

// The kernel virtual address range [0xffffffffc0000000, 0xffffffffffffffff]
// maps to the physical address range [0, 0x3fffffff]
const ADDRESS_SPACE_SIZE: u64 = GiB;
const _HIGH_MEMORY_START: u64 = 0xFFFF_FFFF_C000_0000;
const VC_MMU_RAM_RANGE: core::ops::RangeInclusive<u32> = 0xC000_0000..=0xFEFF_FFFF;
const VC_MMU_PERIPHERALS_RANGE: core::ops::RangeInclusive<u32> = 0x7E00_0000..=0x7EFF_FFFF;

pub const PERIPHERALS_BASE: AddressVirtual = AddressPhysical::new(0x3F00_0000).as_virtual();
pub const HIGH_MEMORY_START: AddressVirtual = AddressVirtual::new(_HIGH_MEMORY_START);
pub const KSTACKTOP_CPU0: AddressVirtual = AddressVirtual::new(0xFFFF_FFFF_C008_0000);
pub const KSTACKGUARD_CPU0: AddressVirtual = KSTACKTOP_CPU0.subtract(PAGE_SIZE * 2);

#[derive(Clone, Copy, Debug)]
pub struct AddressPhysical {
    addr: u64,
}

impl AddressPhysical {
    pub const fn new(addr: u64) -> Self {
        assert!(addr < ADDRESS_SPACE_SIZE);
        Self { addr }
    }

    pub const fn add(&self, offset: u64) -> Self {
        let addr = self.addr + offset;
        Self::new(addr)
    }

    pub const fn as_virtual(&self) -> AddressVirtual {
        HIGH_MEMORY_START.add(self.addr)
    }

    pub const fn as_bus(&self) -> AddressBus {
        let peripherals_base = PERIPHERALS_BASE.as_physical().as_u64();
        let addr = if self.addr < peripherals_base {
            self.addr as u32 + *VC_MMU_RAM_RANGE.start()
        } else {
            self.addr as u32 + (*VC_MMU_PERIPHERALS_RANGE.start() - peripherals_base as u32)
        };
        AddressBus::new(addr)
    }

    pub const fn as_u64(&self) -> u64 {
        self.addr
    }
}

impl Eq for AddressPhysical {}

impl PartialEq for AddressPhysical {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl PartialOrd for AddressPhysical {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.addr.partial_cmp(&other.addr)
    }
}

#[derive(Clone, Copy)]
pub struct AddressVirtual {
    addr: u64,
}

impl AddressVirtual {
    // Clippy complains that the second check in the assert!() is always true,
    // however ADDRESS_SPACE_SIZE might change in the future
    #[allow(clippy::absurd_extreme_comparisons)]
    pub const fn new(addr: u64) -> Self {
        assert!(
            (addr >= _HIGH_MEMORY_START) && (addr <= _HIGH_MEMORY_START + (ADDRESS_SPACE_SIZE - 1))
        );
        Self { addr }
    }

    pub const fn add(&self, offset: u64) -> Self {
        let addr = self.addr + offset;
        Self::new(addr)
    }

    pub const fn subtract(&self, offset: u64) -> Self {
        let addr = self.addr - offset;
        Self::new(addr)
    }

    pub const fn as_physical(&self) -> AddressPhysical {
        let addr = self.addr - HIGH_MEMORY_START.as_u64();
        AddressPhysical::new(addr)
    }

    pub const fn as_bus(&self) -> AddressBus {
        self.as_physical().as_bus()
    }

    pub const fn as_u64(&self) -> u64 {
        self.addr
    }
}

impl Eq for AddressVirtual {}

impl PartialEq for AddressVirtual {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl PartialOrd for AddressVirtual {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.addr.partial_cmp(&other.addr)
    }
}

// See BCM2835-ARM-Peripherals.pdf section 1.2.4 Bus Addresses
// The VC MMU mapping are taken from here (should be the same for 2836 and 2837):
// https://lists.denx.de/pipermail/u-boot/2015-March/208201.html
#[derive(Clone, Copy)]
pub struct AddressBus {
    addr: u32,
}

impl AddressBus {
    pub const fn new(addr: u32) -> Self {
        let in_ram_range = addr >= *VC_MMU_RAM_RANGE.start() && addr <= *VC_MMU_RAM_RANGE.end();
        let in_peripherals_range =
            addr >= *VC_MMU_PERIPHERALS_RANGE.start() && addr <= *VC_MMU_PERIPHERALS_RANGE.end();
        assert!(in_ram_range || in_peripherals_range);
        Self { addr }
    }

    pub const fn as_u32(&self) -> u32 {
        self.addr
    }
}

impl Eq for AddressBus {}

impl PartialEq for AddressBus {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl PartialOrd for AddressBus {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.addr.partial_cmp(&other.addr)
    }
}

// NOTE: Normally we'd define an AddressRange generic type and define a trait
// that both AddressPhysical and AddressVirtual implement. Unfortunately, Rust
// doesn't support const functions in trait implementation so that doesn't
// work. For this reason if we need ranges for virtual addresses we'll end up
// duplicating code.
#[derive(Clone, Copy, Debug)]
pub struct RangePhysical {
    base: AddressPhysical,
    size: u64,
}

impl RangePhysical {
    pub const fn new(base: AddressPhysical, size: u64) -> Self {
        assert!(size > 0);
        // Check that the end address is valid
        base.add(size - 1);
        Self { base, size }
    }

    pub const fn base(&self) -> AddressPhysical {
        self.base
    }

    pub const fn size(&self) -> u64 {
        self.size
    }

    pub const fn overlaps(&self, other: &Self) -> bool {
        let this_start = self.base().as_u64();
        let this_end = this_start + self.size - 1;
        let other_start = other.base().as_u64();
        let other_end = other_start + other.size - 1;

        (other_start >= this_start && other_start <= this_end)
            || (other_end >= this_start && other_end <= this_end)
    }
}
