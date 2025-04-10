pub struct AddressPhysical {
    addr: u64,
}

impl AddressPhysical {
    pub const fn new(addr: u64) -> Self {
        Self { addr }
    }

    pub const fn as_virtual(&self) -> AddressVirtual {
        // Identify mapping
        let addr = self.addr;
        AddressVirtual { addr }
    }
}

pub struct AddressVirtual {
    addr: u64,
}

impl AddressVirtual {
    pub const fn add(&self, offset: u64) -> Self {
        let addr = self.addr + offset;
        Self { addr }
    }

    pub const fn as_u64(&self) -> u64 {
        self.addr
    }
}
