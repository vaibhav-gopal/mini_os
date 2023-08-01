use alloc::alloc::{GlobalAlloc, Layout}; // we have to implement GlobalAlloc and #[global_allocator] attribute for our heap allocator instance
use core::ptr::null_mut; // creates a null mutable pointer
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

// create a heap virtual memory region to use
pub const HEAP_START: usize = 0x_4444_4444_0000; // arbitrary start address (as long as it's not in use)
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

/// Map the heap virtual memory region to physical memory in order to use it
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64); // convert heap start to a virt addr
        let heap_end = heap_start + HEAP_SIZE - 1u64; // calculate the end of the heap into a virt addr (inclusive so subtract 1)
        let heap_start_page = Page::containing_address(heap_start); // get the page containing the start heap address
        let heap_end_page = Page::containing_address(heap_end); // get the page containing the end heap addresses
        Page::range_inclusive(heap_start_page, heap_end_page) // return a range of pages in between (inclusive)
    };

    for page in page_range { // iterate through the range of pages
        let frame = frame_allocator
            .allocate_frame() // get an unmapped physical frame
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE; // make sure the mapped frame is writable
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush() // create a new mapping using the map_to() method (recursively creates page tables for you) -> then refresh the cache (TLB)
        };
    }

    // external allocator for now...
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}


// External heap allocator for now...
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

