use core::time::Duration;

/// Busy-wait for the specified duration. The highest precision that can be
/// used is in milliseconds (anything else will be rounded to the closest
/// millisecond value).
pub fn busy_wait(duration: Duration) {
    let frequency: u64;
    // SAFETY: Executing a simple assembly instruction
    unsafe {
        core::arch::asm!("mrs {0}, CNTFRQ_EL0", out(reg) frequency);
    }

    let start: u64;
    // SAFETY: Executing a simple assembly instruction
    unsafe {
        core::arch::asm!("mrs {0}, CNTPCT_EL0", out(reg) start);
    }

    let ticks_needed = (frequency * duration.as_millis() as u64) / 1000;
    let deadline = start.wrapping_add(ticks_needed);

    loop {
        let now: u64;
        // SAFETY: Executing a simple assembly instruction
        unsafe {
            core::arch::asm!("mrs {0}, CNTPCT_EL0", out(reg) now);
        }

        if now >= deadline {
            break;
        }
    }
}
