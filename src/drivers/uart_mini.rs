// As per BMC2837 section 2.2 the mini UART is NOT a 16550 compatible UART
// However as far as possible the first 8 control and status registers are laid
// out like a 16550 UART and the UART core is build to emulate 16550 behaviour.

use crate::drivers::{gpio, gpio::GPIOPin, peripheral_switch_in, MMIORegisters, PERIPHERALS_BASE};
use crate::irq::{enable_irq, GpuIrq, Irq};
use crate::locking::IRQSpinLock;
use crate::{ACTIONS, PENDING_ACTIONS};
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::registers::{Aliased, ReadOnly, ReadWrite};
use tock_registers::{register_bitfields, register_structs};

// SAFETY: There should a be a mini UART behind that address as per BMC2837
const REGS: MMIORegisters<AuxRegisters> =
    unsafe { MMIORegisters::<AuxRegisters>::new(PERIPHERALS_BASE.add(0x21_5000)) };

const RX_BUFFER_LEN: usize = 128;

#[derive(Copy, Clone)]
struct RxBuffer {
    // Circular buffer.
    // Old characters will be dropped if we can't process them fast enough.
    buffer: [char; RX_BUFFER_LEN],
    // Index pointing to the slot where the next character will go to
    tail: usize,
    // Number of characters in the buffer
    num_chars: usize,
}

static RX_BUFFER: IRQSpinLock<RxBuffer> = IRQSpinLock::new(RxBuffer {
    buffer: ['\0'; RX_BUFFER_LEN],
    tail: 0,
    num_chars: 0,
});

register_bitfields! {
    u32,

    AUX_IRQ [
        MINI_UART_IRQ 0,
        SPI_1_IRQ     1,
        SPI_2_IRQ     2,
    ],

    AUX_ENABLES [
        MINI_UART_ENABLE 0,
        SP1_ENABLE       1,
        SP2_ENABLE       2,
    ],

    AUX_MU_IER [
        ENABLE_RX_IRQ 0,
        ENABLE_TX_IRQ 1,
    ],

    AUX_MU_IIR_WRITE [
        CLEAR_RX_FIFO 1,
        CLEAR_TX_FIFO 2,
    ],

    AUX_MU_IIR_READ [
        IRQ_PENDING  OFFSET(0) NUMBITS(1) [],
        IRQ_ID       OFFSET(1) NUMBITS(2) [
            NO_IRQS = 0,
            TX_REGISTER_EMPTY = 1,
            RX_VALID = 2,
        ],
        FIFO_ENABLED OFFSET(6) NUMBITS(2) [],
    ],

    AUX_MU_LCR [
        DATA_SIZE OFFSET(0) NUMBITS(2) [
            SEVEN_BITS = 0,
            EIGHT_BITS = 3,
        ],
        BREAK     OFFSET(6) NUMBITS(1) [],
        DLAB      OFFSET(7) NUMBITS(1) [],
    ],

    AUX_MU_MCR [
        RTS 1,
    ],

    AUX_MU_LSR [
        DATA_READY 0,
        RX_OVERRUN 1,
        TX_READY   5,
        TX_IDLE    6,
    ],

    AUX_MU_MSR [
        CTS_STATUS 5,
    ],

    AUX_MU_SCRATCH [
        SCRATCH OFFSET(0) NUMBITS(8) [],
    ],

    AUX_MU_CNTL [
        RX_ENABLE          OFFSET(0) NUMBITS(1) [],
        TX_ENABLE          OFFSET(1) NUMBITS(1) [],
        RX_AUTOFLOW_ENABLE OFFSET(2) NUMBITS(1) [],
        TX_AUTOFLOW_ENABLE OFFSET(3) NUMBITS(1) [],
        RTS_AUTOFLOW_LEVEL OFFSET(4) NUMBITS(2) [
            SPACES_THREE = 0,
            SPACES_TWO = 1,
            SPACES_ONE = 2,
            SPACES_FOUR = 3,
        ],
        RTS_ASSERT_LEVEL   OFFSET(6) NUMBITS(1) [
            HIGH = 0,
            LOW = 1,
        ],
        CTS_ASSERT_LEVEL   OFFSET(7) NUMBITS(1) [
            HIGH = 0,
            LOW = 1,
        ],
    ],

    AUX_MU_STAT [
        RX_SYMBOL_AVAILABLE OFFSET(0)  NUMBITS(1) [],
        TX_SPACE_AVAILABLE  OFFSET(1)  NUMBITS(1) [],
        RX_IDLE             OFFSET(2)  NUMBITS(1) [],
        TX_IDLE             OFFSET(3)  NUMBITS(1) [],
        RX_OVERRUN          OFFSET(4)  NUMBITS(1) [],
        TX_FIFO_FULL        OFFSET(5)  NUMBITS(1) [],
        RTS_STATUS          OFFSET(6)  NUMBITS(1) [],
        CTS_STATUS          OFFSET(7)  NUMBITS(1) [],
        TX_FIFO_EMPTY       OFFSET(8)  NUMBITS(1) [],
        TX_DONE             OFFSET(9)  NUMBITS(1) [],
        RX_FIFO_FILL_LVL    OFFSET(16) NUMBITS(4) [],
        TX_FIFO_FILL_LVL    OFFSET(24) NUMBITS(4) [],
    ],
}

register_structs! {
    #[allow(non_snake_case)]
    AuxRegisters {
        (0x000 => AUX_IRQ: ReadOnly<u32, AUX_IRQ::Register>),
        (0x004 => AUX_ENABLES: ReadWrite<u32, AUX_ENABLES::Register>),
        (0x008 => _reserved0),
        (0x040 => AUX_MU_IO_DATA: ReadWrite<u8>),
        (0x041 => _reserved1),
        (0x044 => AUX_MU_IER: ReadWrite<u32, AUX_MU_IER::Register>),
        (0x048 => AUX_MU_IIR: Aliased<u32, AUX_MU_IIR_READ::Register, AUX_MU_IIR_WRITE::Register>),
        (0x04c => AUX_MU_LCR: ReadWrite<u32, AUX_MU_LCR::Register>),
        (0x050 => AUX_MU_MCR: ReadWrite<u32, AUX_MU_MCR::Register>),
        (0x054 => AUX_MU_LSR: ReadOnly<u32, AUX_MU_LSR::Register>),
        (0x058 => AUX_MU_MSR: ReadOnly<u32, AUX_MU_MSR::Register>),
        (0x05c => AUX_MU_SCRATCH: ReadWrite<u32, AUX_MU_SCRATCH::Register>),
        (0x060 => AUX_MU_CNTL: ReadWrite<u32, AUX_MU_CNTL::Register>),
        (0x064 => AUX_MU_STAT: ReadWrite<u32, AUX_MU_STAT::Register>),
        (0x068 => AUX_MU_BAUD: ReadWrite<u16>),
        (0x06a => _reserved2),
        (0x06c => @END),
    }
}

/// Configure UART for 8N1 (1 start bit, 8 data bits, no parity, 1 stop bit)
pub fn init(baud_rate: u32) {
    peripheral_switch_in();
    // The enable bit must be set first, otherwise we cannot even access the
    // rest of the registers.
    REGS.AUX_ENABLES.write(AUX_ENABLES::MINI_UART_ENABLE::SET);

    REGS.AUX_MU_CNTL
        .write(AUX_MU_CNTL::RX_ENABLE::CLEAR + AUX_MU_CNTL::TX_ENABLE::CLEAR);
    while !REGS.AUX_MU_STAT.is_set(AUX_MU_STAT::RX_IDLE) {
        // Wait until receiver is idle before proceeding
    }

    REGS.AUX_MU_LCR.write(AUX_MU_LCR::DATA_SIZE::EIGHT_BITS);

    // baudrate = (clock_freq) / (8 * (aux_mu_baud + 1))
    // TODO: Get the system clock frequency from the video core.
    // For now it's assumed to be 250 MHz.
    let reg_val: u32 = (250_000_000 / (8 * baud_rate)) - 1;
    REGS.AUX_MU_BAUD.set(reg_val as u16);

    // TXD1
    GPIOPin::new(14).select_mode(gpio::PinMode::Alt5);
    // RXD1
    GPIOPin::new(15).select_mode(gpio::PinMode::Alt5);

    REGS.AUX_MU_IER.modify(AUX_MU_IER::ENABLE_RX_IRQ::SET);
    enable_irq(Irq::Gpu(GpuIrq::Aux));

    // Setup is complete, enable RX/TX
    REGS.AUX_MU_CNTL
        .modify(AUX_MU_CNTL::RX_ENABLE::SET + AUX_MU_CNTL::TX_ENABLE::SET);
}

pub fn put_char(c: char) {
    peripheral_switch_in();
    while !REGS.AUX_MU_LSR.is_set(AUX_MU_LSR::TX_READY) {
        // Wait until we can transmit
    }
    REGS.AUX_MU_IO_DATA.set(c as u8);
}

fn get_char() -> char {
    REGS.AUX_MU_IO_DATA.get() as char
}

pub fn process_rx_irq() {
    peripheral_switch_in();
    let pending_rx_chars = REGS.AUX_MU_STAT.read(AUX_MU_STAT::RX_FIFO_FILL_LVL);
    let mut rx_buffer = RX_BUFFER.lock();

    for _ in 0..pending_rx_chars {
        let tail = rx_buffer.tail;
        rx_buffer.buffer[tail] = get_char();
        rx_buffer.tail = (tail + 1) % RX_BUFFER_LEN;
        rx_buffer.num_chars = (rx_buffer.num_chars + 1).min(RX_BUFFER_LEN);
    }

    drop(rx_buffer);

    let mut actions = PENDING_ACTIONS.lock();
    *actions |= 1 << (ACTIONS::UartAction as u64);
}

// This is the bottom half of the RX IRQ handler that runs outside interrupt context
pub fn process_pending_chars() {
    peripheral_switch_in();
    let mut rx_buffer = RX_BUFFER.lock();
    // Copy and reset the buffer
    let buffer = *rx_buffer;
    rx_buffer.num_chars = 0;
    // And release the lock before processing it so that interrupts are enabled again
    drop(rx_buffer);

    let mut start = if buffer.num_chars <= buffer.tail {
        buffer.tail - buffer.num_chars
    } else {
        RX_BUFFER_LEN - (buffer.num_chars - buffer.tail)
    };

    let mut num_chars = buffer.num_chars;
    while num_chars > 0 {
        let c = buffer.buffer[start];
        put_char(c);
        if c == '\r' {
            put_char('\n');
        }
        start = (start + 1) % RX_BUFFER_LEN;
        num_chars -= 1;
    }
}
