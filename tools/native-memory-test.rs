#[path = "../kernel/runtime/native_c_image.rs"]
mod native_c_image;
#[path = "../kernel/runtime/native_memory.rs"]
mod native_memory;
#[path = "../kernel/runtime/execution.rs"]
mod execution;

use native_c_image::{Image, Region, Segment, MAX_SEGMENTS};
use native_memory::{CompilerAddressSpace, CompilerVirtualLayout, FrameAllocation,
    KernelAccessMap, MappedRange, MemoryError, NativeMemory, PagePermissions,
    X86AddressSpace, PAGE_SIZE};
use std::alloc::{alloc_zeroed, dealloc, Layout};
use execution::{AddressSpaceToken, ExecutionError, ExecutionManager, MemoryRegion,
    PriorityClass, ResourceBudget};

struct AlignedArena {
    pointer: *mut u8,
    layout: Layout,
}

impl AlignedArena {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates page-aligned host memory that behaves like the loader-reserved identity-mapped physical arena.
    // ------------------=
    fn new(bytes: usize) -> Self {
        let layout = Layout::from_size_align(bytes, PAGE_SIZE as usize).unwrap();
        let pointer = unsafe { alloc_zeroed(layout) };
        assert!(!pointer.is_null());
        Self { pointer, layout }
    }

    // ------------------------=
    // FUNC: base
    // DESC: Returns the synthetic physical base address used by the production allocator and page-table walker.
    // ------------------=
    fn base(&self) -> u64 { self.pointer as u64 }
}

impl Drop for AlignedArena {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases the page-aligned host fixture after every allocator and mapper assertion has completed.
    // ------------------=
    fn drop(&mut self) { unsafe { dealloc(self.pointer, self.layout); } }
}

// ------------------------=
// FUNC: range
// DESC: Creates one page-aligned identity-style kernel mapping fixture.
// ------------------=
fn range(virtual_address: u64, physical_address: u64, pages: u64) -> MappedRange {
    MappedRange { virtual_address, physical_address, bytes: pages * PAGE_SIZE }
}

// ------------------------=
// FUNC: sample_image
// DESC: Constructs a parsed-image equivalent with RX text, read-only data, writable data, and one RELRO page.
// ------------------=
fn sample_image() -> Image {
    let mut segments = [Segment::default(); MAX_SEGMENTS];
    segments[0] = Segment { source: 0, address: 0, file_size: 0x1800,
        memory_size: 0x2000, flags: 5 };
    segments[1] = Segment { source: 0, address: 0x2000, file_size: 0x1000,
        memory_size: 0x1000, flags: 4 };
    segments[2] = Segment { source: 0, address: 0x3000, file_size: 0x1800,
        memory_size: 0x3000, flags: 6 };
    Image { entry: 0x100, memory_size: 0x6000, virtual_base: 0,
        segments, segment_count: 3, tls: None,
        relro: Some(Region { address: 0x3000, length: 0x1000 }) }
}

// ------------------------=
// FUNC: assert_permissions
// DESC: Verifies both translation and effective x86 privilege, write, and execute permissions.
// ------------------=
fn assert_permissions(process: &CompilerAddressSpace, address: u64,
    expected: PagePermissions) {
    let mapping = process.page_tables.mapping(address).expect("mapping must exist");
    assert_eq!(mapping.permissions, expected);
}

// ------------------------=
// FUNC: main
// DESC: Behaviorally verifies exact frame ownership, independent CR3 roots, W-X, RELRO, stack guards, and kernel gateway isolation.
// ------------------=
fn main() {
    let arena = AlignedArena::new(32 * 1024 * 1024);
    let mut memory = NativeMemory::empty();
    memory.initialize(arena.base(), arena.layout.size() as u64).unwrap();
    let initial_pages = memory.available_pages();

    let first = memory.allocate(3, 11).unwrap();
    let second = memory.allocate(2, 12).unwrap();
    assert!(first.physical_address + 3 * PAGE_SIZE <= second.physical_address ||
        second.physical_address + 2 * PAGE_SIZE <= first.physical_address);
    let forged = FrameAllocation { owner: 13, ..first };
    assert_eq!(memory.release(forged), Err(MemoryError::UnknownAllocation));
    memory.release(first).unwrap();
    memory.release(second).unwrap();
    assert_eq!(memory.available_pages(), initial_pages);

    let image = sample_image();
    let layout = CompilerVirtualLayout {
        image_base: 0x0000_0001_0000_0000,
        heap_base: 0x0000_0002_0000_0000,
        heap_pages: 8,
        stack_guard_base: 0x0000_0003_0000_0000,
        stack_pages: 4,
    };
    let kernel = KernelAccessMap {
        text: range(0x0400_0000, 0x0400_0000, 2),
        rodata: range(0x0400_2000, 0x0400_2000, 1),
        data: range(0x0400_3000, 0x0400_3000, 2),
        stack: range(0x0500_0000, 0x0500_0000, 2),
        gateway: range(0x0000_007f_ffff_0000, 0x0600_0000, 1),
    };
    let process = CompilerAddressSpace::build(&mut memory, 41, &image, layout, kernel).unwrap();
    assert_eq!(process.entry, layout.image_base + 0x100);
    assert_eq!(process.stack_top, layout.stack_guard_base + 5 * PAGE_SIZE);
    assert_permissions(&process, layout.image_base, PagePermissions::USER_RX);
    assert_permissions(&process, layout.image_base + 0x2000, PagePermissions::USER_R);
    assert_permissions(&process, layout.image_base + 0x3000, PagePermissions::USER_R);
    assert_permissions(&process, layout.image_base + 0x4000, PagePermissions::USER_RW);
    assert_permissions(&process, layout.heap_base, PagePermissions::USER_RW);
    assert!(process.page_tables.mapping(layout.stack_guard_base).is_none());
    assert_permissions(&process, layout.stack_guard_base + PAGE_SIZE, PagePermissions::USER_RW);
    assert!(process.page_tables.mapping(layout.stack_guard_base + 5 * PAGE_SIZE).is_none());
    assert_permissions(&process, kernel.text.virtual_address, PagePermissions::KERNEL_RX);
    assert_permissions(&process, kernel.rodata.virtual_address, PagePermissions::KERNEL_R);
    assert_permissions(&process, kernel.data.virtual_address, PagePermissions::KERNEL_RW);
    assert_permissions(&process, kernel.gateway.virtual_address, PagePermissions::USER_RX);
    assert!(process.page_tables.mapping(0x0700_0000).is_none());

    let second_process = CompilerAddressSpace::build(&mut memory, 42, &image, layout, kernel).unwrap();
    assert_ne!(process.page_tables.root_physical_address(),
        second_process.page_tables.root_physical_address());
    let mut executions = ExecutionManager::new();
    let handle = executions.create(9, 12, MemoryRegion { base: layout.image_base, length: 0x6000 },
        1, PriorityClass::Normal, ResourceBudget { memory_limit: 64 * 1024 * 1024,
            cpu_weight: 100, message_queue_limit: 32, io_priority: 1 }).unwrap();
    assert_eq!(executions.get(handle).unwrap().address_space, AddressSpaceToken(0));
    assert_eq!(executions.bind_address_space(handle, 7), Err(ExecutionError::InvalidAddressSpace));
    executions.bind_address_space(handle, process.page_tables.root_physical_address()).unwrap();
    assert_eq!(executions.get(handle).unwrap().address_space,
        AddressSpaceToken(process.page_tables.root_physical_address()));

    let mut large_map = X86AddressSpace::new(&mut memory, 43).unwrap();
    large_map.map_range(&mut memory, 0x0000_0004_0000_0000, 0x0800_0000,
        768 * 1024 * 1024, PagePermissions::KERNEL_R).unwrap();
    assert_eq!(large_map.mapping(0x0000_0004_2fff_f000).unwrap().physical_address,
        0x0800_0000 + 0x2fff_f000);
    large_map.release(&mut memory).unwrap();
    second_process.release(&mut memory).unwrap();
    process.release(&mut memory).unwrap();
    assert_eq!(memory.available_pages(), initial_pages);
}
