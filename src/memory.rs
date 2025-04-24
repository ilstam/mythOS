#[allow(non_upper_case_globals)]
pub const MiB: u64 = 1 << 20;
#[allow(non_upper_case_globals)]
pub const GiB: u64 = 1 << 30;

// The kernel virtual address range [0xffffffffc0000000, 0xffffffffffffffff]
// maps to the physical address range [0, 0x3fffffff]
const ADDRESS_SPACE_SIZE: u64 = GiB;
const _HIGH_MEMORY_START: u64 = 0xFFFF_FFFF_C000_0000;
const VC_MMU_RAM_RANGE: core::ops::RangeInclusive<u32> = 0xC000_0000..=0xFEFF_FFFF;
const VC_MMU_PERIPHERALS_RANGE: core::ops::RangeInclusive<u32> = 0x7E00_0000..=0x7EFF_FFFF;

pub const PERIPHERALS_BASE: AddressVirtual = AddressPhysical::new(0x3F00_0000).as_virtual();
pub const HIGH_MEMORY_START: AddressVirtual = AddressVirtual::new(_HIGH_MEMORY_START);
pub const KSTACKTOP_CPU0: AddressVirtual = AddressVirtual::new(0xFFFF_FFFF_C008_0000);

pub struct AddressPhysical {
    addr: u64,
}

impl AddressPhysical {
    pub const fn new(addr: u64) -> Self {
        assert!(addr < ADDRESS_SPACE_SIZE);
        Self { addr }
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

pub struct AddressVirtual {
    addr: u64,
}

impl AddressVirtual {
    pub const fn new(addr: u64) -> Self {
        assert!(addr >= _HIGH_MEMORY_START);
        Self { addr }
    }

    pub const fn add(&self, offset: u64) -> Self {
        let addr = self.addr + offset;
        Self::new(addr)
    }

    pub const fn as_physical(&self) -> AddressPhysical {
        let addr = self.addr - HIGH_MEMORY_START.as_u64();
        AddressPhysical::new(addr)
    }

    #[allow(dead_code)]
    pub const fn as_bus(&self) -> AddressBus {
        self.as_physical().as_bus()
    }

    pub const fn as_u64(&self) -> u64 {
        self.addr
    }
}

// See BCM2835-ARM-Peripherals.pdf section 1.2.4 Bus Addresses
// The VC MMU mapping are taken from here (should be the same for 2836 and 2837):
// https://lists.denx.de/pipermail/u-boot/2015-March/208201.html
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

    #[allow(dead_code)]
    pub const fn as_u32(&self) -> u32 {
        self.addr
    }
}
