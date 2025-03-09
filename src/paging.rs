use crate::locking::SpinLock;
use aarch64_cpu::asm::barrier;
use aarch64_cpu::registers::{ID_AA64MMFR0_EL1, MAIR_EL1, SCTLR_EL1, TCR_EL1, TTBR0_EL1};
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::register_bitfields;
use tock_registers::registers::InMemoryRegister;

#[allow(non_upper_case_globals)]
const MiB: u64 = 1 << 20;

register_bitfields! {
    u64,
    PTE [
        UXN OFFSET(54) NUMBITS(1) [],
        PXN OFFSET(53) NUMBITS(1) [],
        AF  OFFSET(10) NUMBITS(1) [],
        SH  OFFSET(8) NUMBITS(2) [
            OUTER_SHAREABLE = 0b10,
            INNER_SHAREABLE = 0b11
        ],
        AP OFFSET(6) NUMBITS(2) [
            RW_KERNEL = 0b00,
            RW_USER = 0b01,
            RO_KERNEL = 0b10,
            RO_USER = 0b11
        ],
        ATTR_INDEX OFFSET(2) NUMBITS(3) [],
        DESC_TYPE  OFFSET(1) NUMBITS(1) [
            BLOCK = 0,
            TABLE_OR_PAGE = 1
        ],
        VALID OFFSET(0) NUMBITS(1) []
    ]
}

#[repr(align(4096))]
struct PageTable {
    pte: [u64; 512],
}

static L2_PT: SpinLock<PageTable> = SpinLock::new(PageTable { pte: [0; 512] });

// Rpi3 has a physical address space of 1GiB. Here we identity map this 1GiB.
// We use 64KiB page granularity, and since we only need 30 bits to describe
// the physical address space, 2 level of pages tables are enough (L2 and L3).
// For now we pretend that the first 512 MiB are DRAM, and the following 512
// MiB are device memory (even though in reality only the top 16MiB are device
// memory). This lets us get a way with a single L2 page table with 2 block
// descriptor entries, each mapping 512 MiB and no L3 tables.
pub fn setup_paging() {
    let id_aa64mmfr0 = ID_AA64MMFR0_EL1.extract();
    if id_aa64mmfr0.read(ID_AA64MMFR0_EL1::TGran64) != ID_AA64MMFR0_EL1::TGran64::Supported.into() {
        panic!("The MMU doesn't support 64KiB translation granule");
    }

    MAIR_EL1.write(
        MAIR_EL1::Attr0_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc
            + MAIR_EL1::Attr0_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc
            + MAIR_EL1::Attr1_Device::nonGathering_nonReordering_EarlyWriteAck,
    );

    // These should match what we wrote to MAIR_EL1 just above
    enum MairType {
        Normal = 0,
        Device = 1,
    }

    // The first PTE in the L2 PT maps a 512MiB block of normal memory
    let physical_addr = 0;
    let entry0: InMemoryRegister<u64, PTE::Register> = InMemoryRegister::new(physical_addr);
    entry0.modify(
        PTE::VALID::SET
            + PTE::DESC_TYPE::BLOCK
            + PTE::ATTR_INDEX.val(MairType::Normal as u64)
            + PTE::SH::INNER_SHAREABLE
            + PTE::AF::SET
            + PTE::UXN::SET,
    );

    // The second PTE in the L2 PT maps a 512MiB block of device memory
    let physical_addr = 512 * MiB;
    let entry1: InMemoryRegister<u64, PTE::Register> = InMemoryRegister::new(physical_addr);
    entry1.modify(
        PTE::VALID::SET
            + PTE::DESC_TYPE::BLOCK
            + PTE::ATTR_INDEX.val(MairType::Device as u64)
            + PTE::SH::OUTER_SHAREABLE
            + PTE::AF::SET
            + PTE::PXN::SET
            + PTE::UXN::SET,
    );

    let mut l2_pt = L2_PT.lock();
    l2_pt.pte[0] = entry0.get();
    l2_pt.pte[1] = entry1.get();

    let ttbr0_baddr = &raw const *l2_pt as u64;
    TTBR0_EL1.write(TTBR0_EL1::BADDR.val(ttbr0_baddr >> 1) + TTBR0_EL1::CnP::SET);

    TCR_EL1.write(
        TCR_EL1::IPS.val(id_aa64mmfr0.read(ID_AA64MMFR0_EL1::PARange))
            + TCR_EL1::TG0::KiB_64
            + TCR_EL1::SH0::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::T0SZ.val(34), // 64-34=30 bits for addressing 1GiB
    );

    // Turn address translation on
    barrier::isb(barrier::SY);
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);
    barrier::isb(barrier::SY);
}
