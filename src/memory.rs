use crate::address::AddressVirtual;

#[allow(non_upper_case_globals)]
pub const MiB: u64 = 1 << 20;
#[allow(non_upper_case_globals)]
pub const GiB: u64 = 1 << 30;

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
pub fn dcache_clean_va_range(addr: AddressVirtual, size: u64) {
    dcache_operate_on_va_range!(addr, size, "cvac");
}

#[inline]
pub fn dcache_invalidate_va_range(addr: AddressVirtual, size: u64) {
    dcache_operate_on_va_range!(addr, size, "ivac");
}
