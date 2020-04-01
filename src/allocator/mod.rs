// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use alloc::alloc::Layout;
use x86_64::{
    structures::paging::{
        mapper::MapToError, 
        FrameAllocator, 
        Mapper, 
        Page, 
        PageTableFlags, 
        Size4KiB
    },
    VirtAddr
};

// ---------------------------------------------------------------------------
// MODULES
// ---------------------------------------------------------------------------

pub mod fixed_size_block;
use fixed_size_block::FixedSizeBlockAllocator;

// ---------------------------------------------------------------------------
// STATICS AND CONSTNATS
// ---------------------------------------------------------------------------

pub const HEAP_START: usize = 0x4444_4444_0000;
pub const HEAP_SIZE: usize = 10240;

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(
    FixedSizeBlockAllocator::new());

// ---------------------------------------------------------------------------
// DATA STRUCTURES
// ---------------------------------------------------------------------------

/// A wrapper around `spin::Mutex` to allow trait implementations for locked
/// global attrs.
pub struct Locked<A> {
    inner: spin::Mutex<A>
}

impl<A> Locked<A> {

    /// Create a new instance of the `Locked` wrapper with the given member.
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner)
        }
    }

    /// Lock the mutex.
    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

/// Contains information about the kernel heap.
#[derive(Debug)]
pub struct HeapInfo {
    start_virt_addr: VirtAddr,
    start_phys_addr: Page,
    size: usize
}

// ---------------------------------------------------------------------------
// PUBLIC FUNCTIONS
// ---------------------------------------------------------------------------

/// Initialise the kernel heap.
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>
) -> Result<HeapInfo, MapToError<Size4KiB>> {
    // Get the page range required for the heap
    let heap_start = VirtAddr::new(HEAP_START as u64);
    let page_range = {
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // For each page required allocate a frame and map it.
    for page in page_range {
        let frame = frame_allocator.allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        mapper.map_to(page, frame, flags, frame_allocator)?.flush();
    }

    // TODO: remove
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    // Return the heap information
    Ok(HeapInfo {
        start_virt_addr: heap_start,
        start_phys_addr: page_range.start,
        size: HEAP_SIZE
    })
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("[ALLOC-ERROR] Failed to allocate: {:?}", layout);
}