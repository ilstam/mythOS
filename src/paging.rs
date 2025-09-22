use crate::address::{
    AddressPhysical, AddressVirtual, RangePhysical, PERIPHERALS_BASE, PERIPHERALS_SIZE,
};
use crate::allocator::allocate_page;
use crate::locking::SpinLock;
use crate::memory::{MiB, PAGE_SIZE};
use aarch64_cpu::asm::barrier;
use aarch64_cpu::registers::{
    ID_AA64MMFR0_EL1, MAIR_EL1, SCTLR_EL1, SP, TCR_EL1, TTBR0_EL1, TTBR1_EL1,
};
use tock_registers::fields::FieldValue;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::register_bitfields;
use tock_registers::registers::InMemoryRegister;

register_bitfields! {
    u64,
    PTE [
        UXN OFFSET(54) NUMBITS(1) [],
        PXN OFFSET(53) NUMBITS(1) [],
        ADDRESS OFFSET(12) NUMBITS(18) [],
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

enum MairType {
    Normal = 0,
    Device = 1,
}

#[repr(align(4096))]
struct PageTable {
    pte: [u64; 512],
}

// Root L2 page table used after early boot. Uses a 4KiB translation granule.
static L2_PT: SpinLock<PageTable> = SpinLock::new(PageTable { pte: [0; 512] });

// Root L2 page table used during early boot. Uses a 64KiB translation granule.
//
// Normally we would use a SpinLock here, but on ARM64 you can't reliably use
// atomics before enabling the MMU. Since we need to update this data structure
// before actually enabling the MMU let's use a static mut here. We know that
// at that point there is a single thread of execution so there's no potential
// for race conditions.
static mut L2_PT_EARLY: PageTable = PageTable { pte: [0; 512] };

// Rpi3 has a physical address space of 1GiB. Here we identity map this 1GiB.
// We use 64KiB page granularity, and since we only need 30 bits to describe
// the physical address space, 2 level of pages tables are enough (L2 and L3).
// For now we pretend that the first 512 MiB are DRAM, and the following 512
// MiB are device memory (even though in reality only the top 16MiB are device
// memory). This lets us get a way with a single L2 page table with 2 block
// descriptor entries, each mapping 512 MiB and no L3 tables.
//
// The same root page table is used by both TTBR0 and TTBR1 which means that
// the same physical memory can be accessed using either low virtual addresses
// [0, 0x3fffffff] or high addresses [0xffffffffc0000000, 0xffffffffffffffff].
pub fn setup_early_boot_paging() {
    let id_aa64mmfr0 = ID_AA64MMFR0_EL1.extract();
    if id_aa64mmfr0.read(ID_AA64MMFR0_EL1::TGran64) != ID_AA64MMFR0_EL1::TGran64::Supported.into() {
        panic!("The MMU doesn't support 64KiB translation granule");
    }

    // These should match enum MairType
    MAIR_EL1.write(
        MAIR_EL1::Attr0_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc
            + MAIR_EL1::Attr0_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc
            + MAIR_EL1::Attr1_Device::nonGathering_nonReordering_EarlyWriteAck,
    );

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

    // SAFETY: This function is called at early boot from a single thread of
    // execution so accessing this static mut it's not racy. Check the variable
    // declaration for an explanation of why we can't use a SpinLock here.
    unsafe {
        L2_PT_EARLY.pte[0] = entry0.get();
        L2_PT_EARLY.pte[1] = entry1.get();
    }

    let ttbr0_baddr = &raw const L2_PT_EARLY as u64;
    TTBR0_EL1.write(TTBR0_EL1::BADDR.val(ttbr0_baddr >> 1) + TTBR0_EL1::CnP::SET);

    let ttbr1_baddr = ttbr0_baddr;
    TTBR1_EL1.write(TTBR1_EL1::BADDR.val(ttbr1_baddr >> 1) + TTBR1_EL1::CnP::SET);

    TCR_EL1.write(
        TCR_EL1::IPS.val(id_aa64mmfr0.read(ID_AA64MMFR0_EL1::PARange))
            + TCR_EL1::TG1::KiB_64
            + TCR_EL1::SH1::Inner
            + TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD1::EnableTTBR1Walks
            + TCR_EL1::T1SZ.val(34) // 64-34=30 bits for addressing 1GiB
            + TCR_EL1::TG0::KiB_64
            + TCR_EL1::SH0::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::T0SZ.val(34), // 64-34=30 bits for addressing 1GiB
    );

    // Flush the TLB just in case
    flush_tlb_all();

    // Turn address translation on
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);
    barrier::isb(barrier::SY);
}

fn l2_idx(va: AddressVirtual) -> usize {
    ((va.as_u64() >> 21) & 0x1ff) as usize
}

fn l3_idx(va: AddressVirtual) -> usize {
    ((va.as_u64() >> 12) & 0x1ff) as usize
}

fn map_page(va: AddressVirtual, pa: AddressPhysical, attributes: FieldValue<u64, PTE::Register>) {
    let mut l2_pt = L2_PT.lock();
    let l2_pte =
        &mut l2_pt.pte[l2_idx(va)] as *mut u64 as *mut InMemoryRegister<u64, PTE::Register>;
    // SAFETY: The pointer points to memory allocated for l2_pt on the heap
    let l2_pte = unsafe { &*l2_pte };

    let l3_pt;
    if l2_pte.is_set(PTE::VALID) {
        // There already is a L3 PT for the 4KiB in question
        let l3_pt_addr = AddressPhysical::new(l2_pte.read(PTE::ADDRESS) << 12);
        l3_pt = unsafe { &mut *(l3_pt_addr.as_virtual().as_u64() as *mut PageTable) };
    } else {
        // We need to allocate a new page to be used as the L3 PT, and we need
        // to update the L2 PTE accordingly
        let page = allocate_page().unwrap();
        l3_pt = unsafe { &mut *(page.as_u64() as *mut PageTable) };

        l2_pte.set(page.as_physical().as_u64());
        l2_pte.modify(PTE::VALID::SET + PTE::DESC_TYPE::TABLE_OR_PAGE);
    }

    let l3_pte =
        &mut l3_pt.pte[l3_idx(va)] as *mut u64 as *mut InMemoryRegister<u64, PTE::Register>;
    // SAFETY: The pointer points to memory previously allocated with allocate_page()
    let l3_pte = unsafe { &*l3_pte };

    l3_pte.set(pa.as_u64());
    l3_pte.modify(PTE::VALID::SET + PTE::DESC_TYPE::TABLE_OR_PAGE + attributes);
}

pub fn map_range(
    mut va: AddressVirtual,
    mut pa: AddressPhysical,
    mut size: u64,
    attributes: FieldValue<u64, PTE::Register>,
) {
    if size == 0 {
        return;
    }

    loop {
        map_page(va, pa, attributes);
        size = size.saturating_sub(PAGE_SIZE);
        // Break early to avoid creating invalid PAs or VAs
        if size == 0 {
            break;
        }
        va = va.add(PAGE_SIZE);
        pa = pa.add(PAGE_SIZE);
    }
}

/// Setup the runtime page tables used by the kernel after early boot and after
/// initialising the page allocator. The runtime page tables use a 4KiB
/// translation granule and identity map all RAM and the peripherals space
/// using TTBR1_EL1. This function disables TTBR0_EL1 walks.
pub fn setup_runtime_paging(ram_range: RangePhysical) {
    let id_aa64mmfr0 = ID_AA64MMFR0_EL1.extract();
    if id_aa64mmfr0.read(ID_AA64MMFR0_EL1::TGran4) != ID_AA64MMFR0_EL1::TGran4::Supported.into() {
        panic!("The MMU doesn't support 4KiB translation granule");
    }

    // TODO: Use more fine grained regions with the appropriate attributes for
    // rodata, code, etc. Also unmap the stack guard pages.

    // Map all RAM
    let attributes = PTE::ATTR_INDEX.val(MairType::Normal as u64)
        + PTE::SH::INNER_SHAREABLE
        + PTE::AF::SET
        + PTE::UXN::SET;
    map_range(
        ram_range.base().as_virtual(),
        ram_range.base(),
        ram_range.size(),
        attributes,
    );

    // Map the peripherals space
    let attributes = PTE::ATTR_INDEX.val(MairType::Device as u64)
        + PTE::SH::OUTER_SHAREABLE
        + PTE::AF::SET
        + PTE::PXN::SET
        + PTE::UXN::SET;
    map_range(
        PERIPHERALS_BASE,
        PERIPHERALS_BASE.as_physical(),
        PERIPHERALS_SIZE,
        attributes,
    );

    // To change the translation granule of TTBR1_EL1 we're going to jump to a
    // low address so that we can use TTBR1_EL0 temporarily.

    // First move the stack pointer to its low address equivalent
    let sp_high = AddressVirtual::new(SP.get());
    let sp_low = sp_high.as_physical();
    SP.set(sp_low.as_u64());
    barrier::dsb(barrier::SY);

    // And then jump to a low address
    let func_addr =
        AddressVirtual::new(switch_to_runtime_page_tables as usize as u64).as_physical();
    // SAFETY: TTBR0_EL1 is active so it's safe to jump to a low address
    let switch_to_runtime_page_tables =
        unsafe { core::mem::transmute::<u64, fn()>(func_addr.as_u64()) };
    switch_to_runtime_page_tables();

    // We're now back here running from a high address using the new page
    // tables! Let's update the SP again.
    let sp_low = AddressPhysical::new(SP.get());
    let sp_high = sp_low.as_virtual();
    SP.set(sp_high.as_u64());
    barrier::dsb(barrier::SY);

    // Low addresses are still mapped. Disable TTBR0 so that we can only access
    // memory using high addresses. After we do that attempting to access
    // anything using a low address will result in a page fault.
    TCR_EL1.modify(TCR_EL1::EPD0::DisableTTBR0Walks + TCR_EL1::T0SZ.val(64));
    flush_tlb_all();
}

// This function must be run from a low address
#[inline(never)]
fn switch_to_runtime_page_tables() {
    // Change the translation granule of TTBR1_EL1 to 4KiB
    TCR_EL1.modify(TCR_EL1::TG1::KiB_4);
    barrier::dsb(barrier::SY);

    // And update the root page table
    let ttbr1_baddr = AddressPhysical::new(&raw const *L2_PT.lock() as u64).as_u64();
    TTBR1_EL1.write(TTBR1_EL1::BADDR.val(ttbr1_baddr >> 1) + TTBR1_EL1::CnP::SET);
    barrier::dsb(barrier::SY);

    flush_tlb_all();

    // Jump back to a high address using the address stored in the LR!
}

#[inline]
fn flush_tlb_all() {
    // SAFETY: The inline assembly flushes the TLB
    unsafe {
        core::arch::asm!("tlbi vmalle1");
    }

    barrier::dsb(barrier::SY);
    barrier::isb(barrier::SY);
}
