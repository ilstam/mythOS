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
    loop {}
}
