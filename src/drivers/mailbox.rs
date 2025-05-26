// This seems to be the best documentation for the VideoCore firmware and the
// BCM2837 mailbox protocol: https://github.com/raspberrypi/firmware/wiki/Mailboxes

use crate::address::AddressVirtual;
use crate::drivers::{peripheral_switch_in, MMIORegisters, PERIPHERALS_BASE};
use crate::locking::SpinLock;
use crate::memory::{dcache_clean_va_range, dcache_invalidate_va_range};
use aarch64_cpu::asm::barrier;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::registers::{ReadOnly, WriteOnly};
use tock_registers::{register_bitfields, register_structs};

// SAFETY: The mailbox 0 & 1 registers are mapped there on RPi3
const MB0: MMIORegisters<Mailbox0Registers> =
    unsafe { MMIORegisters::<Mailbox0Registers>::new(PERIPHERALS_BASE.add(0xB880)) };
const MB1: MMIORegisters<Mailbox1Registers> =
    unsafe { MMIORegisters::<Mailbox1Registers>::new(PERIPHERALS_BASE.add(0xB880 + 0x20)) };

static MAILBOX_SPINLOCK: SpinLock<()> = SpinLock::new(());

const CHANNEL_BITMASK: u32 = 0xf;
const TAGS_CHANNEL: u32 = 8;

register_bitfields! {
    u32,

    MB0_STATUS [
        EMPTY 30,
    ],

    MB1_STATUS [
        FULL 31,
    ],
}

register_structs! {
    #[allow(non_snake_case)]
    Mailbox0Registers {
        (0x00 => DATA: ReadOnly<u32>),
        (0x04 => _reserved0),
        (0x18 => STATUS: ReadOnly<u32, MB0_STATUS::Register>),
        (0x1c => _reserved1),
        (0x20 => @END),
    }
}

register_structs! {
    #[allow(non_snake_case)]
    Mailbox1Registers {
        (0x00 => DATA: WriteOnly<u32>),
        (0x04 => _reserved0),
        (0x18 => STATUS: ReadOnly<u32, MB1_STATUS::Register>),
        (0x1c => _reserved1),
        (0x20 => @END),
    }
}

fn mailbox_send(channel: u32, buffer_addr: AddressVirtual, buffer_size: u32) {
    peripheral_switch_in();

    let mut bus_addr = buffer_addr.as_bus().as_u32();
    assert!(bus_addr & CHANNEL_BITMASK == 0);
    bus_addr |= TAGS_CHANNEL;

    let _lock = MAILBOX_SPINLOCK.lock();
    while MB1.STATUS.is_set(MB1_STATUS::FULL) {}

    // Since the mbox buffer lives in the kernel stack in normal cacheable
    // memory which is non-DMA coherent we need to flush the cache lines so
    // that the mbox controller can see what we wrote there.
    dcache_clean_va_range(buffer_addr, buffer_size.into());
    // Use a DMB so that the write to MB1.DATA isn't re-ordered before the read
    // from MB1.STATUS OR before the cache flushing
    barrier::dmb(barrier::SY);
    MB1.DATA.set(bus_addr);

    loop {
        while MB0.STATUS.is_set(MB0_STATUS::EMPTY) {}
        // Invalidate the cache lines so that the reads come from RAM which is
        // the Point-of-Coherency with the mbox controller
        dcache_invalidate_va_range(buffer_addr, buffer_size.into());
        // Use a DMB so that the read from MB0.DATA isn't re-ordered before
        // the read from MB0.STATUS OR before the cache invalidation
        barrier::dmb(barrier::SY);

        let data = MB0.DATA.get();
        if data & CHANNEL_BITMASK == channel {
            break;
        }
    }
}

#[derive(PartialEq)]
#[repr(u32)]
enum MailboxBufferCode {
    // Request Codes
    ProcessRequest = 0,
    // Response Codes
    Success = 0x80000000,
    #[allow(dead_code)]
    Error = 0x80000001,
}

#[repr(C)]
struct MailboxMsgHeader {
    size: u32, // buffer size in bytes (including the header values, the end tag and padding)
    code: MailboxBufferCode, // buffer request / response code
}

#[repr(C)]
struct MailboxMsgFooter {
    end: u32, // must be 0
}

// --------------------------------------------------------------------------
// Property Tags Channel
// --------------------------------------------------------------------------

#[repr(u32)]
enum PropertyTag {
    GetFwVersion = 0x00000001,
    GetBoardSerial = 0x00010004,
    SetOnboardLedStatus = 0x00038041,
}

#[repr(C)]
struct PropertyMsgHeader {
    mbox_header: MailboxMsgHeader,
    id: PropertyTag,
    value_size: u32, // value buffer size in bytes
    value_code: u32, // request / response code
}

macro_rules! define_and_init_property_msg {
    ($struct_name:ident, $var_name:ident, $tag_id:expr, $( $field:ident : $ty:ty = $val:expr),* $(,)?) => {
        #[repr(C, align(16))]
        struct $struct_name {
            tag_header: PropertyMsgHeader,
            $( $field: $ty, )*
            tag_footer: MailboxMsgFooter,
        }

        let value_size = 0 $( + core::mem::size_of::<$ty>() as u32 )*;
        let full_size = core::mem::size_of::<PropertyMsgHeader>() as u32
            + core::mem::size_of::<MailboxMsgFooter>() as u32
            + value_size;

        let mut uninit = core::mem::MaybeUninit::<$struct_name>::uninit();
        let p = uninit.as_mut_ptr();
        // SAFETY: The pointers satisfy the requirements set by write_volatile()
        // and assume_init() is called after the memory is initialised.
        let $var_name = unsafe {
            core::ptr::write_volatile(&raw mut (*p).tag_header.mbox_header.size, full_size);
            core::ptr::write_volatile(
                &raw mut (*p).tag_header.mbox_header.code,
                MailboxBufferCode::ProcessRequest,
            );
            core::ptr::write_volatile(&raw mut (*p).tag_header.id, $tag_id);
            core::ptr::write_volatile(&raw mut (*p).tag_header.value_size, value_size);
            core::ptr::write_volatile(&raw mut (*p).tag_header.value_code, 0);
            $(
                core::ptr::write_volatile(&raw mut (*p).$field, $val);
            )*
            core::ptr::write_volatile(&raw mut (*p).tag_footer.end, 0);
            uninit.assume_init()
        };
    };
}

macro_rules! mailbox_send_ptag_and_handle_error {
    ($msg:ident) => {
        let addr = AddressVirtual::new(&$msg as *const _ as u64);
        mailbox_send(TAGS_CHANNEL, addr, $msg.tag_header.mbox_header.size);

        // SAFETY: The pointer satisfies the requirements set by read_volatile()
        let code = unsafe { core::ptr::read_volatile(&$msg.tag_header.mbox_header.code) };
        if code != MailboxBufferCode::Success {
            return Err(code as u32);
        }
    };
}

pub fn get_vc_fw_version() -> Result<u32, u32> {
    define_and_init_property_msg!(
        PropertyMsgFwVersion,
        msg,
        PropertyTag::GetFwVersion,
        fw_version: u32 = 0,
    );

    mailbox_send_ptag_and_handle_error!(msg);

    // SAFETY: The pointer satisfies the requirements set by read_volatile()
    unsafe { Ok(core::ptr::read_volatile(&msg.fw_version)) }
}

pub fn get_board_serial() -> Result<u64, u32> {
    define_and_init_property_msg!(
        PropertyMsgBoardSerial,
        msg,
        PropertyTag::GetBoardSerial,
        serial_low: u32 = 0,
        serial_high: u32 = 0,
    );

    mailbox_send_ptag_and_handle_error!(msg);

    // SAFETY: The pointers satisfy the requirements set by read_volatile()
    let serial_low = unsafe { core::ptr::read_volatile(&msg.serial_low) };
    let serial_high = unsafe { core::ptr::read_volatile(&msg.serial_high) };
    let serial_num = ((serial_high as u64) << 32) | serial_low as u64;

    Ok(serial_num)
}

// The documentation in https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface
// says that it's status=42, power=130. However it seems that pin 130 controls
// the activity LED (ACT not PWR) on RPi3b.
#[repr(u32)]
pub enum OnboardLEDPin {
    ActivityLED = 130,
}

#[repr(u32)]
pub enum OnboardLEDStatus {
    Low = 0,
    High = 1,
}

pub fn set_onboard_led_status(pin: OnboardLEDPin, status: OnboardLEDStatus) -> Result<(), u32> {
    define_and_init_property_msg!(
        PropertyMsgSetOnboardLedStatus,
        msg,
        PropertyTag::SetOnboardLedStatus,
        pin_num: OnboardLEDPin = pin,
        status: OnboardLEDStatus = status,
    );

    mailbox_send_ptag_and_handle_error!(msg);

    Ok(())
}
