use crate::drivers::interrupt_controller;
use aarch64_cpu::registers::DAIF;
use tock_registers::interfaces::ReadWriteable;

#[allow(dead_code)]
pub enum Irq {
    Arm(ArmIrq),
    Gpu(GpuIrq),
}

#[allow(dead_code)]
pub enum ArmIrq {
    Timer = 0,
    Mailbox = 1,
    Doorbell0 = 2,
    Doorbell1 = 3,
    GpuHalted0 = 4,
    GpuHalted1 = 5,
    AccessErrorType1 = 6,
    AccessErrorType0 = 7,
}

#[allow(dead_code)]
pub enum GpuIrq {
    SystemTimer1 = 1,
    SystemTimer3 = 3,
    Usb = 9,
    Aux = 29,
    I2cSpiSlv = 43,
    Pwa0 = 45,
    Pwa1 = 46,
    Smi = 48,
    Gpio0 = 49,
    Gpio1 = 50,
    Gpio2 = 51,
    Gpio3 = 52,
    I2c = 53,
    Spi = 54,
    Pcm = 55,
    Uart = 57,
}

#[inline]
pub fn enable_irq(irq: Irq) {
    interrupt_controller::enable_irq(irq);
}

#[allow(dead_code)]
#[inline]
pub fn disable_irq(irq: Irq) {
    interrupt_controller::disable_irq(irq);
}

#[inline]
pub fn enable_interrupts() {
    DAIF.modify(DAIF::D::Unmasked + DAIF::A::Unmasked + DAIF::I::Unmasked + DAIF::F::Unmasked);
}

#[inline]
pub fn disable_interrupts() {
    DAIF.modify(DAIF::D::Masked + DAIF::A::Masked + DAIF::I::Masked + DAIF::F::Masked);
}
