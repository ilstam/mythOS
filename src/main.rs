#![no_main]
#![no_std]

mod address;
mod delay;
mod drivers;
mod exceptions;
mod irq;
mod locking;
mod logging;
mod memory;
mod paging;

use crate::address::{AddressPhysical, KSTACKTOP_CPU0};
use crate::delay::busy_wait;
use crate::locking::IRQSpinLock;
use aarch64_cpu::asm;
use aarch64_cpu::registers::{CurrentEL, ELR_EL2, HCR_EL2, SP, SPSR_EL2, SP_EL1};
use core::arch::global_asm;
use drivers::{mailbox, uart_mini};
use tock_registers::interfaces::{Readable, Writeable};

global_asm!(include_str!("boot.s"));

pub static PENDING_ACTIONS: IRQSpinLock<u64> = IRQSpinLock::new(0);

pub enum ACTIONS {
    UartAction = 0,
}

pub fn jump_to_el1() {
    match CurrentEL.read(CurrentEL::EL) {
        1 => return,
        2 => {}
        el => panic!("Unexpected EL: {}", el),
    }

    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

    // Also SPSR_EL2 bit 4 must be 0 to indicate that we'll return to the
    // AArch64 execution state. Unfortunately the aarch64_cpu crate doesn't
    // support that bit, but the write() function will make sure it's cleared.
    SPSR_EL2.write(
        SPSR_EL2::D::Masked
            + SPSR_EL2::A::Masked
            + SPSR_EL2::I::Masked
            + SPSR_EL2::F::Masked
            + SPSR_EL2::M::EL1h,
    );

    ELR_EL2.set(AddressPhysical::new(crate::pre_main as *const () as u64).as_u64());

    SP_EL1.set(KSTACKTOP_CPU0.as_physical().as_u64());

    asm::eret();
}

// This is the Rust entry point to the kernel. The program counter is still
// a low address at this point.
#[no_mangle]
pub fn pre_main() {
    jump_to_el1();
    paging::setup_paging();

    // Paging is now on, but the program counter and stack pointer are still
    // using low addresses. Time to update the SP and jump to a high address.
    asm::barrier::isb(asm::barrier::SY);
    let sp_low = AddressPhysical::new(SP.get());
    let sp_high = sp_low.as_virtual();
    SP.set(sp_high.as_u64());
    asm::barrier::isb(asm::barrier::SY);

    let main_addr = AddressPhysical::new(crate::main as *const () as u64).as_virtual();
    // SAFETY: We trust that paging has been setup correctly
    let main: fn() -> () = unsafe { core::mem::transmute(main_addr.as_u64() as *const ()) };
    main();
}

fn blink_onboard_led() {
    let times = 3;
    println!("Will now blink the LED {times} times");

    for _ in 0..times {
        mailbox::set_onboard_led_status(
            mailbox::OnboardLEDPin::ActivityLED,
            mailbox::OnboardLEDStatus::High,
        )
        .unwrap();

        busy_wait(core::time::Duration::from_millis(500));

        mailbox::set_onboard_led_status(
            mailbox::OnboardLEDPin::ActivityLED,
            mailbox::OnboardLEDStatus::Low,
        )
        .unwrap();

        busy_wait(core::time::Duration::from_millis(500));
    }

    println!("LED blinking over");
}

pub fn main() -> ! {
    // At this point we are running the kernel at a high address but low
    // addresses are still mapped in the page tables. Disable TTBR0 so that we
    // can only access memory using high addresses. After we do that attempting
    // to access anything using a low address will result in a page fault.
    paging::disable_ttbr0();

    exceptions::install_exception_table();
    irq::enable_interrupts();

    uart_mini::init(115200);

    println!(
        "VideoCore Firmware Version: {:#x}",
        mailbox::get_vc_fw_version().unwrap()
    );
    println!(
        "Board Serial Number: {:#x}",
        mailbox::get_board_serial().unwrap()
    );

    blink_onboard_led();
    print!("Everything you type will be echoed: ");

    loop {
        loop {
            irq::disable_interrupts();

            let mut actions = PENDING_ACTIONS.lock();
            let pending = *actions;
            if pending == 0 {
                break;
            }
            *actions = 0;
            drop(actions);

            // Interrupts must be enabled while we process pending actions
            irq::enable_interrupts();

            if pending & (1 << (ACTIONS::UartAction as u64)) != 0 {
                uart_mini::process_pending_chars();
            }
        }

        // When we get here interrupts must be disabled, otherwise an interrupt
        // could arrive just before the WFI but after we checked for pending actions
        asm::wfi();
        irq::enable_interrupts();
    }
}
