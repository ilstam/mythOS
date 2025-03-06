use crate::drivers::uart_mini;
use crate::drivers::{interrupt_controller, interrupt_controller::PendingIrqs};
use aarch64_cpu::registers::DAIF;
use core::sync::atomic::{compiler_fence, Ordering};
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

impl TryFrom<u32> for GpuIrq {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            29 => Ok(GpuIrq::Aux),
            _ => Err(()),
        }
    }
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
    // No H/W barriers are needed when writing PSTATE fields, but compiler
    // barriers are still required.
    compiler_fence(Ordering::SeqCst);
    DAIF.modify(DAIF::D::Unmasked + DAIF::A::Unmasked + DAIF::I::Unmasked + DAIF::F::Unmasked);
    compiler_fence(Ordering::SeqCst);
}

#[inline]
pub fn disable_interrupts() {
    // No H/W barriers are needed when writing PSTATE fields, but compiler
    // barriers are still required.
    compiler_fence(Ordering::SeqCst);
    DAIF.modify(DAIF::D::Masked + DAIF::A::Masked + DAIF::I::Masked + DAIF::F::Masked);
    compiler_fence(Ordering::SeqCst);
}

pub fn process_irqs() {
    let PendingIrqs { mut gpu, arm } = interrupt_controller::pending_irqs();

    // TODO: Allow drivers to register handlers dynamically
    while gpu != 0 {
        let lowest_set_bit = gpu.trailing_zeros();
        match GpuIrq::try_from(lowest_set_bit) {
            Ok(GpuIrq::Aux) => {
                uart_mini::process_rx_irq();
            }
            _ => {
                panic!("Unexpected GPU IRQ {lowest_set_bit}")
            }
        }

        // Clear the lowest set bit
        gpu &= !(1 << lowest_set_bit);
    }

    if arm != 0 {
        panic!("Unexpected ARM IRQ(s), pending_mask={arm}");
    }
}
