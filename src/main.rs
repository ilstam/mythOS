#![no_main]
#![no_std]

mod logging;

use core::arch::global_asm;

global_asm!(include_str!("boot.s"));

#[no_mangle]
pub fn main() -> ! {
    let a = 4;
    let b = 5;
    println!("Hello {} with some math: {a} + {b} = {}", "world", a + b);

    panic!("Test panic");
}
