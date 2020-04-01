// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use alloc::alloc::{Layout, GlobalAlloc};
use super::Locked;
use core::ptr;
use core::{mem, ptr::NonNull};

// ---------------------------------------------------------------------------
// CONSTANTS
// ---------------------------------------------------------------------------

/// Sizes of blocks to be used for the allocator.
/// 
/// Each size is a power of 2 to fit with block alignments.
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

// ---------------------------------------------------------------------------
// DATA STRUCTURES
// ---------------------------------------------------------------------------

/// The Fixed Size Block Allocator structure.
/// 
/// 
pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap
}

impl FixedSizeBlockAllocator {

    /// Create a new empty block allocator.
    pub const fn new() -> FixedSizeBlockAllocator {
        FixedSizeBlockAllocator {
            list_heads: [None; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty()
        }
    }

    /// Initiailise the allocator with the given heap bounds.
    /// 
    /// NOTE: UNSAFE
    ///     This function is unsafe because the caller must guarentee that the
    ///     given heap bounds are valid and the heap is unused. 
    /// 
    ///     This method must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_end: usize) {
        self.fallback_allocator.init(heap_start, heap_end);
    }

    /// Allocate using the fallback allocator.
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut()
        }
    }
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {

    /// Allocate memory using the fixed block allocator method.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Acquire the lock on ourselves
        let mut allocator = self.lock();

        // Determine which block size is required
        match list_index(&layout) {
            Some(index) => {
                // If the requested size can fit into a block attempt to get
                // the needed head item
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        // If a valid node is available move the head upto the
                        // next free block and return the found node.
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    },
                    None => {
                        // If no valid node we should create a new one using 
                        // the fallback allocator
                        let block_size = BLOCK_SIZES[index];

                        // Note: this only works if block sizes are powers of 
                        // two. No enforcement of this is made here since the 
                        // constant sizes are specifically set so.
                        let block_align = block_size;
                        let layout = Layout::from_size_align(
                            block_size, block_align).unwrap();
                        allocator.fallback_alloc(layout)
                    }
                }
            },
            None => allocator.fallback_alloc(layout)
        }
    }

    /// Deallocate memory previously assigned using an `alloc` call.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Lock the allocator reference
        let mut allocator = self.lock();

        // Find which block size the memory uses
        match list_index(&layout) {
            Some(index) => {
                // If the layout would fit into a block

                // Get a node pointing to the current head
                let new_node = ListNode {
                    next: allocator.list_heads[index].take()
                };

                // Verify that the block has the size and alignment required 
                // for storing the new node
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);

                let new_node_ptr = ptr as *mut ListNode;
                new_node_ptr.write(new_node);
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            },
            None => {
                // If the layout could not be fit into a block it would have
                // been allocated using the fallback allocator, so dealloc 
                // using that.
                let ptr = NonNull::new(ptr).unwrap();
                allocator.fallback_allocator.deallocate(ptr, layout);
            }
        }
    }
}

/// A node in the allocation list
struct ListNode {
    next: Option<&'static mut ListNode>
}


// ---------------------------------------------------------------------------
// PRIVATE FUNCTIONS
// ---------------------------------------------------------------------------

/// Get the index of the block size that this particular layout should fit in.
/// 
/// Will bin the layout into the first block size larger than or equal to the
/// required size.
fn list_index(layout: &Layout) -> Option<usize> {
    let required_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_size)
}

