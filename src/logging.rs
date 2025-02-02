pub struct SerialConsole;

impl core::fmt::Write for SerialConsole {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            // SAFETY: We know that PL011 UART's data register is behind this address
            unsafe {
                core::ptr::write_volatile(0x3F201000 as *mut u8, byte);
            }
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
        print!($($arg)*);
        print!("\n");
    }};
}
