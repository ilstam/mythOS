use crate::drivers::interrupt_controller;
use aarch64_cpu::registers::DAIF;
use tock_registers::interfaces::ReadWriteable;

#[allow(dead_code)]
pub enum IRQ {
    ARM(ARM_IRQ),
    GPU(GPU_IRQ),
}

#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum ARM_IRQ {
    Timer = 0,
    Mailbox = 1,
    Doorbell0 = 2,
    Doorbell1 = 3,
    GpuHalted0 = 4,
    GpuHalted1 = 5,
    AccessErrorType1 = 6,
    AccessErrorType0 = 7,
}

#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum GPU_IRQ {
    SystemTimer1 = 1,
    SystemTimer3 = 3,
    USB = 9,
    AUX = 29,
    I2CSPISLV = 43,
    PWA0 = 45,
    PWA1 = 46,
    SMI = 48,
    GPIO0 = 49,
    GPIO1 = 50,
    GPIO2 = 51,
    GPIO3 = 52,
    I2C = 53,
    SPI = 54,
    PCM = 55,
    UART = 57,
}

#[inline]
pub fn enable_irq(irq: IRQ) {
    interrupt_controller::enable_irq(irq);
}

#[allow(dead_code)]
#[inline]
pub fn disable_irq(irq: IRQ) {
    interrupt_controller::disable_irq(irq);
}

#[inline]
pub fn enable_interrupts() {
    DAIF.modify(DAIF::D::Unmasked + DAIF::A::Unmasked + DAIF::I::Unmasked + DAIF::F::Unmasked);
}

#[allow(dead_code)]
#[inline]
pub fn disable_interrupts() {
    DAIF.modify(DAIF::D::Masked + DAIF::A::Masked + DAIF::I::Masked + DAIF::F::Masked);
}
