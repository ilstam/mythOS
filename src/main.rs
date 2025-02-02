#![no_main]
#![no_std]

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("boot.s"));

#[panic_handler]
#[allow(clippy::empty_loop)]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
#[allow(clippy::empty_loop)]
pub fn main() -> ! {
    let string = "Hello, world!";
    for c in string.chars() {
        // SAFETY: We know that PL011 UART's data register is behind this address
        unsafe {
            core::ptr::write_volatile(0x3F201000 as *mut u8, c as u8);
        }
    }
    loop {}
}
