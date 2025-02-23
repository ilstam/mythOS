use crate::drivers::uart_mini;
use aarch64_cpu::registers::{ESR_EL1, FAR_EL1, VBAR_EL1};
use core::arch::global_asm;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::LocalRegisterCopy;

// NOTE: It's the symbol's address we are interested in, not the value stored there
extern "C" {
    static __exception_table: usize;
}

#[repr(C)]
// NOTE: The exception handler in exceptions.s expects this layout and size of
// the struct. If anything changes here the assembly routines will need to be
// updated too. The size assertion below just serves as a reminder in case the
// struct is extended.
struct ExceptionFrame {
    regs: [u64; 31],
    spsr_el1: u64,
    elr_el1: u64,
    esr_el1: u64,
}

// TODO: Define the size in assembly and write a build script that generates
// a Rust file with a const variable.
const _: () = {
    assert!(
        core::mem::size_of::<ExceptionFrame>() == 34 * 8,
        "Exception frame size is wrong"
    );
};

global_asm!(include_str!("exceptions.s"));

pub fn install_exception_table() {
    // SAFETY: Assume the symbol is properly aligned in assembly
    let exception_table_addr = unsafe { &__exception_table as *const usize as usize };

    VBAR_EL1.set(exception_table_addr as u64);
}

#[no_mangle]
extern "C" fn el1_sp0_sync_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected synchronous exception from the current EL while using SP_EL0");
}

#[no_mangle]
extern "C" fn el1_sp0_irq_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected IRQ from the current EL while using SP_EL0");
}

#[no_mangle]
extern "C" fn el1_sp0_fiq_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected FIQ from the current EL while using SP_EL0");
}

#[no_mangle]
extern "C" fn el1_sp0_serror_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected SError exception from the current EL while using SP_EL0");
}

#[no_mangle]
extern "C" fn el1_sp1_sync_handler(eframe: &mut ExceptionFrame) {
    let esr = LocalRegisterCopy::<u64, ESR_EL1::Register>::new(eframe.esr_el1);

    match esr.read_as_enum::<ESR_EL1::EC::Value>(ESR_EL1::EC) {
        Some(ESR_EL1::EC::Value::DataAbortCurrentEL) => {
            panic!("Data abort at address {:#x}", FAR_EL1.get())
        }
        Some(ESR_EL1::EC::Value::InstrAbortCurrentEL) => {
            panic!("Instruction abort at address {:#x}", FAR_EL1.get())
        }
        _ => panic!("Unexpected synchronous exception from the current EL while using SP_EL1, ESR_EL1.EC={:#b}", esr.read(ESR_EL1::EC)),
    };
}

#[no_mangle]
extern "C" fn el1_sp1_irq_handler(_eframe: &mut ExceptionFrame) {
    // NOTE: For now we assume that the interrupt was generated from the mini UART
    let c = uart_mini::get_char();
    uart_mini::put_char(c);
    if c == '\r' {
        uart_mini::put_char('\n');
    }
}

#[no_mangle]
extern "C" fn el1_sp1_fiq_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected FIQ from the current EL while using SP_EL1");
}

#[no_mangle]
extern "C" fn el1_sp1_serror_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected SError exception from the current EL while using SP_EL1");
}

#[no_mangle]
extern "C" fn el0_64_sync_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected synchronous exception from EL0 AArch64");
}

#[no_mangle]
extern "C" fn el0_64_irq_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected IRQ from EL0 AArch64");
}

#[no_mangle]
extern "C" fn el0_64_fiq_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected FIQ from EL0 AArch64");
}

#[no_mangle]
extern "C" fn el0_64_serror_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected SError exception from EL0 AArch64");
}

#[no_mangle]
extern "C" fn el0_32_sync_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected synchronous exception from EL0 AArch32");
}

#[no_mangle]
extern "C" fn el0_32_irq_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected IRQ from EL0 AArch32");
}

#[no_mangle]
extern "C" fn el0_32_fiq_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected FIQ from EL0 AArch32");
}

#[no_mangle]
extern "C" fn el0_32_serror_handler(_eframe: &mut ExceptionFrame) {
    panic!("Unexpected SError exception from EL0 AArch32");
}
