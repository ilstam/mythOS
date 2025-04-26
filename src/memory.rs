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

#[derive(Clone, Copy)]
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

    #[allow(dead_code)]
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

#[inline]
pub fn get_cache_line_size() -> u32 {
    let ctr_el0: u64;

    // Unfortunately the aarch64-cpu crate doesn't support CTR_EL0 yet.
    // SAFETY: Executing a simple assembly instruction
    unsafe {
        core::arch::asm!(
            "mrs {0}, ctr_el0",
            out(reg) ctr_el0,
            options(nostack, preserves_flags),
        );
    }

    // DminLine, bits [19:16]: Log2 of the number of words in the smallest
    // cache line of all the data caches and unified caches
    let dminline = (ctr_el0 >> 16) & 0xF;

    const WORD_SIZE_IN_BYTES: u32 = 4;
    WORD_SIZE_IN_BYTES << dminline
}

macro_rules! dcache_operate_on_va_range {
    ($addr:expr, $size:expr, $dcache_op:literal) => {
        let cline_size = get_cache_line_size() as u64;

        // Start at the beginning of the cache line
        let mut start = AddressVirtual::new($addr.as_u64() & !(cline_size - 1));
        let end = $addr.add($size);

        while start < end {
            unsafe {
                // SAFETY: Executing a simple assembly instruction
                core::arch::asm!(
                    concat!("dc ", $dcache_op, ", {}"),
                    in(reg) start.as_u64(),
                    options(nostack, preserves_flags),
                );
            }
            start = start.add(cline_size);
        }
    }
}

#[inline]
#[allow(dead_code)]
pub fn dcache_clean_va_range(addr: AddressVirtual, size: u64) {
    dcache_operate_on_va_range!(addr, size, "cvac");
}

#[inline]
#[allow(dead_code)]
pub fn dcache_invalidate_va_range(addr: AddressVirtual, size: u64) {
    dcache_operate_on_va_range!(addr, size, "ivac");
}
