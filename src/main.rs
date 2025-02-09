#![no_main]
#![no_std]

mod drivers;
mod logging;

use core::arch::global_asm;
use drivers::uart_mini;

global_asm!(include_str!("boot.s"));

#[no_mangle]
pub fn main() -> ! {
    uart_mini::init(115200);

    let a = 4;
    let b = 5;
    println!("Hello {} with some math: {a} + {b} = {}", "world", a + b);

    loop {
        let c = uart_mini::get_char();
        uart_mini::put_char(c);
        if c == '\r' {
            uart_mini::put_char('\n');
        }
    }
}
