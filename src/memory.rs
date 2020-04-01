
// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use x86_64::{
    VirtAddr, PhysAddr,
    structures::paging::{
        PageTable, 
        OffsetPageTable, 
        Size4KiB,
        PhysFrame, 
        UnusedPhysFrame,
        FrameAllocator},
    structures::paging::page_table::{FrameError},
    registers::control::Cr3
};
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

// ---------------------------------------------------------------------------
// DATA STRUCTURES
// ---------------------------------------------------------------------------

/// A `FrameAllocator` that returns usable frames from the bootloader's memory
/// map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize
}

impl BootInfoFrameAllocator {

    /// Initialise the allocator.
    /// 
    /// NOTE: UNSAFE
    ///     This function is unsafe since the caller must ensure the memory map
    ///     is valid.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0
        }
    }

    /// Returns an iterator over the unused physical frames in the map.
    fn useable_frames(&self) -> impl Iterator<Item = UnusedPhysFrame> {
        // Get usable regions from the map
        let regions = self.memory_map.iter();
        let useable_regions = regions.filter(
            |r| r.region_type == MemoryRegionType::Usable);

        // Map each usable region to its address range
        let addr_ranges = useable_regions.map(
            |r| r.range.start_addr()..r.range.end_addr());

        // Transform into an iterator
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

        // Create physical frame types from the start addresses
        let frames = frame_addresses.map(
            |addr| PhysFrame::containing_address(PhysAddr::new(addr)));

        frames.map(|f| unsafe {UnusedPhysFrame::new(f)})
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame> {
        let frame = self.useable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

// ---------------------------------------------------------------------------
// PUBLIC FUNCTIONS
// ---------------------------------------------------------------------------

/// Initialise a new `OffsetPageTable`.
/// 
/// NOTE: UNSAFE
///     This function is unsafe because the caller must guarentee that the 
///     entire physical memory is mapped at the given `physical_mem_offset`.
/// 
///     This function must only be called once to avoid aliasing &mut 
///     references which is undefined behaviour.
pub unsafe fn init(phys_offset: VirtAddr) -> OffsetPageTable<'static> {
    let l4_table = active_l4_table(phys_offset);
    OffsetPageTable::new(l4_table, phys_offset)
}

/// Translate a virtual address into its mapped physical address, or `None` if
/// the address is not mapped.
/// 
/// NOTE: UNSAFE
///     This function is unsafe because the caller must guarentee that the 
///     entire physical memory is mapped at the given `physical_mem_offset`.
pub unsafe fn translate_addr(addr: VirtAddr, phys_offset: VirtAddr) 
    -> Option<PhysAddr> {
        
    translate_addr_inner(addr, phys_offset)
}

// ---------------------------------------------------------------------------
// PRIVATE FUNCTIONS
// ---------------------------------------------------------------------------

/// Get a mutable reference to the current active level 4 page table.
/// 
/// NOTE: UNSAFE
///     This function is unsafe because the caller must guarentee that the 
///     entire physical memory is mapped at the given `physical_mem_offset`.
/// 
///     This function must only be called once to avoid aliasing &mut 
///     references which is undefined behaviour.
unsafe fn active_l4_table(physical_mem_offset: VirtAddr) 
    -> &'static mut PageTable {

    // Get the frame physical address from the CR3 register.
    let (l4_table_frame, _) = Cr3::read();

    // Offset the physical address into the virtual space
    let phys = l4_table_frame.start_address();
    let virt = physical_mem_offset + phys.as_u64();

    // Get a mutable pointer to the table.
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}


fn translate_addr_inner(addr: VirtAddr, phys_offset: VirtAddr) 
    -> Option<PhysAddr> {

    // Read the active L4 table
    let (l4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];
    let mut frame = l4_table_frame;

    // Traverse the page table
    for &idx in &table_indexes {
        // Convert the frame to a page table reference
        let virt = phys_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        // Read the page table entry and update the frame variable
        let entry = &table[idx];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("Huge frames not supported")
        };
    }

    // Calculate the physical address by adding the page offset
    Some(frame.start_address() + u64::from(addr.page_offset()))
}