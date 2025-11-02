use crate::address::{AddressVirtual, RangePhysical};
use crate::locking::SpinLock;
use crate::memory::PAGE_SIZE;
use heapless::Vec;

static PAGE_ALLOCATOR: SpinLock<PageAllocator<PAGE_SIZE>> = SpinLock::new(PageAllocator::new());
const NUM_REGIONS: usize = 2;

#[derive(Debug)]
pub enum AllocError {
    OutOfMemory,
}

// The page allocator is implemented using a linked list. Each free page in its
// first 64 bytes stores a pointer (virtual address) to the next free page.
struct PageAllocator<const PAGE_SZ: u64> {
    regions: Vec<RangePhysical, NUM_REGIONS>,
    head: u64, // Pointer to a free page
    free_pages: usize,
}

impl<const PAGE_SZ: u64> PageAllocator<PAGE_SZ> {
    const fn new() -> Self {
        PageAllocator {
            regions: Vec::new(),
            head: 0,
            free_pages: 0,
        }
    }

    // SAFETY: The caller must ensure the entire region is available and free
    // and doesn't overlap with regions previously donated to the allocator
    unsafe fn add_region(&mut self, region: &RangePhysical) {
        for r in &self.regions {
            assert!(!region.overlaps(r));
        }
        self.regions.push(*region).unwrap();

        let mut addr = region.base().as_virtual().as_u64();
        let mut size = region.size();
        assert!((addr & (PAGE_SZ - 1)) == 0, "Address not page aligned");

        while size >= PAGE_SZ {
            let ptr: *mut u64 = addr as *mut u64;
            *ptr = self.head;
            self.head = addr;

            addr += PAGE_SZ;
            size -= PAGE_SZ;
            self.free_pages += 1;
        }
    }

    fn get_regions(&self) -> Vec<RangePhysical, NUM_REGIONS> {
        self.regions.clone()
    }

    fn allocate_page(&mut self) -> Result<AddressVirtual, AllocError> {
        if self.free_pages == 0 {
            return Err(AllocError::OutOfMemory);
        }
        assert!(self.head != 0);

        let page = self.head;
        // SAFETY: We trust that region added with add_region() is valid
        let next = unsafe { *(self.head as *mut u64) };
        self.head = next;
        self.free_pages -= 1;

        // SAFETY: We trust that region added with add_region() is valid
        unsafe {
            core::ptr::write_bytes(page as *mut u8, 0, PAGE_SZ as usize);
        }

        Ok(AddressVirtual::new(page))
    }

    // SAFETY: The vaddr must have been returned by a a previous call to
    // allocate_page() and must have not been previously freed.
    unsafe fn free_page(&mut self, vaddr: AddressVirtual) {
        let ptr = vaddr.as_u64() as *mut u64;
        *ptr = self.head;
        self.head = vaddr.as_u64();
        self.free_pages += 1;
    }
}

// SAFETY: The caller must ensure the entire region is available and free and
// doesn't overlap with regions previously donated to the allocator.
pub unsafe fn add_region(region: &RangePhysical) {
    let num_pages = region.size() / PAGE_SIZE;
    crate::println!("Adding {num_pages} pages to physical memory allocator: {region:#x?}");
    let mut p = PAGE_ALLOCATOR.lock();
    p.add_region(region);
}

pub fn get_regions() -> Vec<RangePhysical, NUM_REGIONS> {
    let p = PAGE_ALLOCATOR.lock();
    p.get_regions()
}

/// Allocates a 4KiB page with all bytes set to 0.
pub fn allocate_page() -> Result<AddressVirtual, AllocError> {
    let mut p = PAGE_ALLOCATOR.lock();
    p.allocate_page()
}

// SAFETY: The vaddr must have been returned by a a previous call to
// allocate_page() and must have not been previously freed.
#[allow(dead_code)]
pub unsafe fn free_page(vaddr: AddressVirtual) {
    let mut p = PAGE_ALLOCATOR.lock();
    p.free_page(vaddr);
}
