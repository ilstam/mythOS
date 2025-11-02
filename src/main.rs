#![no_main]
#![no_std]

mod address;
mod allocator;
mod delay;
mod drivers;
mod exceptions;
mod irq;
mod locking;
mod logging;
mod memory;
mod paging;

use crate::address::{AddressPhysical, RangePhysical, KSTACK_GUARD_CPU0, KSTACK_TOP_CPU0};
use crate::delay::busy_wait;
use crate::locking::IRQSpinLock;
use crate::memory::PAGE_SIZE;
use aarch64_cpu::asm;
use aarch64_cpu::registers::{CurrentEL, ELR_EL2, HCR_EL2, SP, SPSR_EL2, SP_EL1};
use core::arch::global_asm;
use drivers::{mailbox, uart_mini};
use tock_registers::interfaces::{Readable, Writeable};

global_asm!(include_str!("boot.s"));

// NOTE: It's the symbol's address we are interested in, not the value stored there
extern "C" {
    static __kernel_size: usize;
}

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

    ELR_EL2.set(AddressPhysical::new(pre_main as usize as u64).as_u64());

    SP_EL1.set(KSTACK_TOP_CPU0.as_physical().as_u64());

    asm::eret();
}

// This is the Rust entry point to the kernel. The program counter is still
// a low address at this point.
#[no_mangle]
pub fn pre_main() {
    jump_to_el1();
    paging::setup_early_boot_paging();

    // Paging is now on, but the program counter and stack pointer are still
    // using low addresses. Time to update the SP and jump to a high address.
    asm::barrier::isb(asm::barrier::SY);
    let sp_low = AddressPhysical::new(SP.get());
    let sp_high = sp_low.as_virtual();
    SP.set(sp_high.as_u64());
    asm::barrier::isb(asm::barrier::SY);

    let main_addr = AddressPhysical::new(main as usize as u64).as_virtual();
    // SAFETY: We trust that paging has been setup correctly
    let main = unsafe { core::mem::transmute::<u64, fn()>(main_addr.as_u64()) };
    main();
}

fn blink_onboard_led() {
    #[cfg(feature = "qemu")]
    let wait_time = core::time::Duration::from_millis(1);
    #[cfg(not(feature = "qemu"))]
    let wait_time = core::time::Duration::from_millis(500);

    for _ in 0..3 {
        mailbox::set_onboard_led_status(
            mailbox::OnboardLEDPin::ActivityLED,
            mailbox::OnboardLEDStatus::High,
        )
        .unwrap();

        busy_wait(wait_time);

        mailbox::set_onboard_led_status(
            mailbox::OnboardLEDPin::ActivityLED,
            mailbox::OnboardLEDStatus::Low,
        )
        .unwrap();

        busy_wait(wait_time);
    }
}

// The firmware returns a single contiguous RAM region, but we need to account
// for the subregion where the binary has been loaded plus the stack pages plus
// stack guard pages. The stack and stack guard pages are located just before
// where the binary is loaded.  So essentially we should give 2 regions to the
// allocator, one from the start of RAM to the beginning of the stack guard
// area and then from the end of the binary to the end of RAM.
fn allocator_init(ram_range: RangePhysical, binary_size: usize) {
    if ram_range.base() < KSTACK_GUARD_CPU0.as_physical() {
        let size = KSTACK_GUARD_CPU0.as_physical().as_u64() - ram_range.base().as_u64();
        // SAFETY: We trust the math above is correct and the range returned by
        // the firmware is valid
        unsafe {
            allocator::add_region(&RangePhysical::new(AddressPhysical::new(0), size));
        }
    }

    let start = KSTACK_TOP_CPU0
        .add(binary_size as u64)
        .align_up(PAGE_SIZE)
        .as_physical();
    let size = ram_range.base().as_u64() + ram_range.size() - start.as_u64();
    // SAFETY: We trust the math above is correct and the range returned by the
    // firmware is valid
    unsafe {
        allocator::add_region(&RangePhysical::new(start, size));
    }
}

// When execution gets here the kernel is running from a high address
pub fn main() -> ! {
    exceptions::install_exception_table();
    irq::enable_interrupts();

    uart_mini::init(115200);

    blink_onboard_led();

    println!(
        "VideoCore Firmware Version: {:#x}",
        mailbox::get_vc_fw_version().unwrap()
    );
    println!(
        "Board Serial Number: {:#x}",
        mailbox::get_board_serial().unwrap()
    );

    let kernel_size = &raw const __kernel_size as usize;
    println!("Kernel binary size = {kernel_size:#x} bytes");

    let ram_range = mailbox::get_arm_memory().unwrap();
    println!(
        "ARM memory base={:#x} size={:#x}",
        ram_range.base().as_u64(),
        ram_range.size()
    );

    let vc_range = mailbox::get_videocore_memory().unwrap();
    println!(
        "VideoCore memory base={:#x} size={:#x}",
        vc_range.base().as_u64(),
        vc_range.size()
    );

    allocator_init(ram_range, kernel_size);

    paging::setup_runtime_paging();

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
