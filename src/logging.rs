use crate::drivers::uart_mini;
use core::arch::asm;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct SerialConsole;

impl core::fmt::Write for SerialConsole {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            if c == '\n' {
                uart_mini::put_char('\r');
            }
            uart_mini::put_char(c);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut serial = $crate::logging::SerialConsole;
        serial.write_fmt(format_args!($($arg)*)).unwrap();
    }};
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        $crate::print!($($arg)*);
        $crate::print!("\n");
    }};
}

fn dump_registers() {
    let x0: u64;
    let x1: u64;
    let x2: u64;
    let x3: u64;
    let fp: u64;
    let lr: u64;
    let sp: u64;

    // SAFETY: Simply moving registers and then copying them to memory
    unsafe {
        asm!(
            "mov {0}, x0",
            "mov {1}, x1",
            "mov {2}, x2",
            "mov {3}, x3",
            "mov {4}, fp",
            "mov {5}, lr",
            "mov {6}, sp",
            out(reg) x0,
            out(reg) x1,
            out(reg) x2,
            out(reg) x3,
            out(reg) fp,
            out(reg) lr,
            out(reg) sp,
        );
    }

    println!(
        "Register dump:\n\
        x0: 0x{:016x} x1: 0x{:016x} x2: 0x{:016x} x3: 0x{:016x}\n\
        fp: 0x{:016x} lr: 0x{:016x} sp: 0x{:016x}",
        x0, x1, x2, x3, fp, lr, sp
    );
}

#[panic_handler]
#[cfg(not(test))]
pub fn panic(info: &PanicInfo) -> ! {
    // A nested panic could occur while we try to print our panic message.
    // In that case do not attempt to print anything and just loop forever.
    static IN_PROGRESS: AtomicBool = AtomicBool::new(false);
    if IN_PROGRESS.swap(true, Ordering::Relaxed) {
        loop {}
    }

    let (file, line, column) = match info.location() {
        Some(location) => (location.file(), location.line(), location.column()),
        _ => ("", 0, 0),
    };

    println!(
        "\nKERNEL PANIC - {} ('{}', line {}, column {})\n",
        info.message(),
        file,
        line,
        column,
    );

    dump_registers();

    loop {}
}
