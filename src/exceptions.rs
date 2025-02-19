use aarch64_cpu::registers::VBAR_EL1;
use core::arch::global_asm;
use tock_registers::interfaces::Writeable;

// NOTE: It's the symbol's address we are interested in, not the value stored there
extern "C" {
    static __exception_table: usize;
}

global_asm!(include_str!("exceptions.s"));

pub fn install_exception_table() {
    // SAFETY: Assume the symbol is properly aligned in assembly
    let exception_table_addr = unsafe { &__exception_table as *const usize as usize };

    VBAR_EL1.set(exception_table_addr as u64);
}

#[no_mangle]
extern "C" fn el1_sp0_sync_handler() {
    panic!("Unexpected synchronous exception from the current EL while using SP_EL0");
}

#[no_mangle]
extern "C" fn el1_sp0_irq_handler() {
    panic!("Unexpected IRQ from the current EL while using SP_EL0");
}

#[no_mangle]
extern "C" fn el1_sp0_fiq_handler() {
    panic!("Unexpected FIQ from the current EL while using SP_EL0");
}

#[no_mangle]
extern "C" fn el1_sp0_serror_handler() {
    panic!("Unexpected SError exception from the current EL while using SP_EL0");
}

#[no_mangle]
extern "C" fn el1_sp1_sync_handler() {
    panic!("Unexpected synchronous exception from the current EL while using SP_EL1");
}

#[no_mangle]
extern "C" fn el1_sp1_irq_handler() {
    panic!("Unexpected IRQ from the current EL while using SP_EL1");
}

#[no_mangle]
extern "C" fn el1_sp1_fiq_handler() {
    panic!("Unexpected FIQ from the current EL while using SP_EL1");
}

#[no_mangle]
extern "C" fn el1_sp1_serror_handler() {
    panic!("Unexpected SError exception from the current EL while using SP_EL1");
}

#[no_mangle]
extern "C" fn el0_64_sync_handler() {
    panic!("Unexpected synchronous exception from EL0 AArch64");
}

#[no_mangle]
extern "C" fn el0_64_irq_handler() {
    panic!("Unexpected IRQ from EL0 AArch64");
}

#[no_mangle]
extern "C" fn el0_64_fiq_handler() {
    panic!("Unexpected FIQ from EL0 AArch64");
}

#[no_mangle]
extern "C" fn el0_64_serror_handler() {
    panic!("Unexpected SError exception from EL0 AArch64");
}

#[no_mangle]
extern "C" fn el0_32_sync_handler() {
    panic!("Unexpected synchronous exception from EL0 AArch32");
}

#[no_mangle]
extern "C" fn el0_32_irq_handler() {
    panic!("Unexpected IRQ from EL0 AArch32");
}

#[no_mangle]
extern "C" fn el0_32_fiq_handler() {
    panic!("Unexpected FIQ from EL0 AArch32");
}

#[no_mangle]
extern "C" fn el0_32_serror_handler() {
    panic!("Unexpected SError exception from EL0 AArch32");
}
