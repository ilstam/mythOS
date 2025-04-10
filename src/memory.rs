#[allow(non_upper_case_globals)]
pub const MiB: u64 = 1 << 20;
#[allow(non_upper_case_globals)]
pub const GiB: u64 = 1 << 30;

// The kernel virtual address range [0xffffffffc0000000, 0xffffffffffffffff]
// maps to the physical address range [0, 0x3fffffff]
const ADDRESS_SPACE_SIZE: u64 = GiB;
const _HIGH_MEMORY_START: u64 = 0xFFFF_FFFF_C000_0000;
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
        Self { addr }
    }

    pub const fn as_physical(&self) -> AddressPhysical {
        let addr = self.addr - HIGH_MEMORY_START.as_u64();
        AddressPhysical { addr }
    }

    pub const fn as_u64(&self) -> u64 {
        self.addr
    }
}
