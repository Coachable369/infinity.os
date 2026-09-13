//! Physical-frame ownership and x86-64 address-space construction for native tasks.
//! The boot loader reserves the arena, so no page returned here can alias firmware,
//! kernel, framebuffer, or payload memory.

pub const PAGE_SIZE: u64 = 4096;
const MAX_POOL_PAGES: usize = 65_536;
const BITMAP_WORDS: usize = MAX_POOL_PAGES / 64;
const MAX_ALLOCATIONS: usize = 512;
const MAX_PAGE_TABLES: usize = 192;
const ENTRY_COUNT: usize = 512;
const ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const USER: u64 = 1 << 2;
const NO_EXECUTE: u64 = 1 << 63;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FrameAllocation {
    pub physical_address: u64,
    pub page_count: u32,
    pub owner: u32,
    pub(crate) allocation_id: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemoryError {
    InvalidPool,
    InvalidPageCount,
    OutOfMemory,
    AllocationTableFull,
    UnknownAllocation,
    InvalidAlignment,
    AddressOverflow,
    MappingConflict,
    MissingMapping,
    PageTableLimit,
}

pub struct NativeMemory {
    base: u64,
    page_count: usize,
    bitmap: [u64; BITMAP_WORDS],
    allocations: [Option<FrameAllocation>; MAX_ALLOCATIONS],
    next_allocation_id: u32,
}

impl NativeMemory {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates an unavailable allocator that can be initialized exactly once from the boot handoff.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            base: 0,
            page_count: 0,
            bitmap: [0; BITMAP_WORDS],
            allocations: [None; MAX_ALLOCATIONS],
            next_allocation_id: 1,
        }
    }

    // ------------------------=
    // FUNC: initialize
    // DESC: Claims a page-aligned loader-reserved physical arena as the native runtime frame pool.
    // ------------------=
    pub fn initialize(&mut self, base: u64, bytes: u64) -> Result<(), MemoryError> {
        if base == 0 || base & (PAGE_SIZE - 1) != 0 || bytes < PAGE_SIZE || bytes & (PAGE_SIZE - 1) != 0 {
            return Err(MemoryError::InvalidPool);
        }
        let pages = (bytes / PAGE_SIZE) as usize;
        if pages > MAX_POOL_PAGES {
            return Err(MemoryError::InvalidPool);
        }
        self.base = base;
        self.page_count = pages;
        self.bitmap.fill(0);
        self.allocations.fill(None);
        self.next_allocation_id = 1;
        Ok(())
    }

    // ------------------------=
    // FUNC: allocate
    // DESC: Allocates, owns, and zeroes one contiguous physical-page run for a native execution context.
    // ------------------=
    pub fn allocate(&mut self, page_count: u32, owner: u32) -> Result<FrameAllocation, MemoryError> {
        let requested = page_count as usize;
        if requested == 0 {
            return Err(MemoryError::InvalidPageCount);
        }
        let record = self.allocations.iter().position(Option::is_none)
            .ok_or(MemoryError::AllocationTableFull)?;
        let start = self.find_free_run(requested).ok_or(MemoryError::OutOfMemory)?;
        for page in start..start + requested {
            self.set_used(page, true);
        }
        let allocation = FrameAllocation {
            physical_address: self.base + start as u64 * PAGE_SIZE,
            page_count,
            owner,
            allocation_id: self.next_allocation_id,
        };
        self.next_allocation_id = self.next_allocation_id.wrapping_add(1).max(1);
        self.allocations[record] = Some(allocation);
        unsafe {
            core::ptr::write_bytes(allocation.physical_address as *mut u8, 0,
                requested * PAGE_SIZE as usize);
        }
        Ok(allocation)
    }

    // ------------------------=
    // FUNC: release
    // DESC: Releases only an exact live allocation token, preventing cross-context or partial frees.
    // ------------------=
    pub fn release(&mut self, allocation: FrameAllocation) -> Result<(), MemoryError> {
        let record = self.allocations.iter().position(|candidate| *candidate == Some(allocation))
            .ok_or(MemoryError::UnknownAllocation)?;
        let start = ((allocation.physical_address - self.base) / PAGE_SIZE) as usize;
        for page in start..start + allocation.page_count as usize {
            self.set_used(page, false);
        }
        self.allocations[record] = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: available_pages
    // DESC: Reports currently unowned pages in the reserved arena.
    // ------------------=
    pub fn available_pages(&self) -> usize {
        self.page_count - (0..self.page_count).filter(|page| self.is_used(*page)).count()
    }

    // ------------------------=
    // FUNC: find_free_run
    // DESC: Finds the first contiguous free run without allocating metadata from the pool itself.
    // ------------------=
    fn find_free_run(&self, requested: usize) -> Option<usize> {
        let mut run_start = 0;
        let mut run_length = 0;
        for page in 0..self.page_count {
            if self.is_used(page) {
                run_length = 0;
            } else {
                if run_length == 0 { run_start = page; }
                run_length += 1;
                if run_length == requested { return Some(run_start); }
            }
        }
        None
    }

    // ------------------------=
    // FUNC: is_used
    // DESC: Reads one physical-page ownership bit.
    // ------------------=
    fn is_used(&self, page: usize) -> bool {
        self.bitmap[page / 64] & (1u64 << (page % 64)) != 0
    }

    // ------------------------=
    // FUNC: set_used
    // DESC: Updates one physical-page ownership bit.
    // ------------------=
    fn set_used(&mut self, page: usize, used: bool) {
        let mask = 1u64 << (page % 64);
        if used { self.bitmap[page / 64] |= mask; }
        else { self.bitmap[page / 64] &= !mask; }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PagePermissions {
    pub user: bool,
    pub writable: bool,
    pub executable: bool,
}

impl PagePermissions {
    pub const KERNEL_RX: Self = Self { user: false, writable: false, executable: true };
    pub const KERNEL_R: Self = Self { user: false, writable: false, executable: false };
    pub const KERNEL_RW: Self = Self { user: false, writable: true, executable: false };
    pub const USER_RX: Self = Self { user: true, writable: false, executable: true };
    pub const USER_R: Self = Self { user: true, writable: false, executable: false };
    pub const USER_RW: Self = Self { user: true, writable: true, executable: false };
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MappedRange {
    pub virtual_address: u64,
    pub physical_address: u64,
    pub bytes: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KernelAccessMap {
    pub text: MappedRange,
    pub rodata: MappedRange,
    pub data: MappedRange,
    pub stack: MappedRange,
    pub gateway: MappedRange,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CompilerVirtualLayout {
    pub image_base: u64,
    pub heap_base: u64,
    pub heap_pages: u32,
    pub stack_guard_base: u64,
    pub stack_pages: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Mapping {
    pub physical_address: u64,
    pub permissions: PagePermissions,
}

pub struct X86AddressSpace {
    root: FrameAllocation,
    tables: [Option<FrameAllocation>; MAX_PAGE_TABLES],
    table_count: usize,
}

pub struct CompilerAddressSpace {
    pub page_tables: X86AddressSpace,
    pub image: FrameAllocation,
    pub heap: FrameAllocation,
    pub stack: FrameAllocation,
    pub entry: u64,
    pub stack_top: u64,
}

impl CompilerAddressSpace {
    // ------------------------=
    // FUNC: build
    // DESC: Allocates and maps a compiler image, heap, guarded stack, supervisor kernel regions, and the sole user-callable kernel gateway.
    // ------------------=
    pub fn build(memory: &mut NativeMemory, owner: u32,
        image: &super::native_c_image::Image, virtual_layout: CompilerVirtualLayout,
        kernel: KernelAccessMap) -> Result<Self, MemoryError> {
        let image_pages = pages_for(image.memory_size as u64)?;
        let image_allocation = memory.allocate(image_pages, owner)?;
        let heap = match memory.allocate(virtual_layout.heap_pages, owner) {
            Ok(value) => value,
            Err(error) => { let _ = memory.release(image_allocation); return Err(error); }
        };
        let stack = match memory.allocate(virtual_layout.stack_pages, owner) {
            Ok(value) => value,
            Err(error) => {
                let _ = memory.release(heap);
                let _ = memory.release(image_allocation);
                return Err(error);
            }
        };
        let mut page_tables = match X86AddressSpace::new(memory, owner) {
            Ok(value) => value,
            Err(error) => {
                let _ = memory.release(stack);
                let _ = memory.release(heap);
                let _ = memory.release(image_allocation);
                return Err(error);
            }
        };
        let mapped = Self::map_all(&mut page_tables, memory, image, virtual_layout,
            image_allocation, heap, stack, kernel);
        if let Err(error) = mapped {
            let _ = page_tables.release(memory);
            let _ = memory.release(stack);
            let _ = memory.release(heap);
            let _ = memory.release(image_allocation);
            return Err(error);
        }
        Ok(Self {
            page_tables,
            image: image_allocation,
            heap,
            stack,
            entry: virtual_layout.image_base + image.entry as u64,
            stack_top: virtual_layout.stack_guard_base +
                (virtual_layout.stack_pages as u64 + 1) * PAGE_SIZE,
        })
    }

    // ------------------------=
    // FUNC: image_bytes_mut
    // DESC: Exposes only the owned image frames so the checked ELF loader can populate them before execution.
    // ------------------=
    pub fn image_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.image.physical_address as *mut u8,
            self.image.page_count as usize * PAGE_SIZE as usize) }
    }

    // ------------------------=
    // FUNC: release
    // DESC: Atomically tears down page tables and all compiler-owned image, heap, and stack frames.
    // ------------------=
    pub fn release(self, memory: &mut NativeMemory) -> Result<(), MemoryError> {
        self.page_tables.release(memory)?;
        memory.release(self.stack)?;
        memory.release(self.heap)?;
        memory.release(self.image)
    }

    // ------------------------=
    // FUNC: map_all
    // DESC: Applies segment W-X, RELRO, guard, kernel-supervisor, and gateway policies to one address space.
    // ------------------=
    fn map_all(page_tables: &mut X86AddressSpace, memory: &mut NativeMemory,
        image: &super::native_c_image::Image, layout: CompilerVirtualLayout,
        image_allocation: FrameAllocation, heap: FrameAllocation, stack: FrameAllocation,
        kernel: KernelAccessMap) -> Result<(), MemoryError> {
        if layout.image_base & (PAGE_SIZE - 1) != 0 || layout.heap_pages == 0 ||
            layout.stack_pages == 0 {
            return Err(MemoryError::InvalidAlignment);
        }
        for segment in &image.segments[..image.segment_count] {
            let start = align_down(segment.address as u64);
            let end = align_up((segment.address + segment.memory_size) as u64)?;
            let permissions = if segment.flags & 1 != 0 { PagePermissions::USER_RX }
                else if segment.flags & 2 != 0 { PagePermissions::USER_RW }
                else { PagePermissions::USER_R };
            page_tables.map_range(memory, layout.image_base + start,
                image_allocation.physical_address + start, end - start, permissions)?;
        }
        page_tables.map_range(memory, layout.heap_base, heap.physical_address,
            heap.page_count as u64 * PAGE_SIZE, PagePermissions::USER_RW)?;
        page_tables.map_guarded_stack(memory, layout.stack_guard_base, stack)?;
        map_kernel_range(page_tables, memory, kernel.text, PagePermissions::KERNEL_RX)?;
        map_kernel_range(page_tables, memory, kernel.rodata, PagePermissions::KERNEL_R)?;
        map_kernel_range(page_tables, memory, kernel.data, PagePermissions::KERNEL_RW)?;
        map_kernel_range(page_tables, memory, kernel.stack, PagePermissions::KERNEL_RW)?;
        map_kernel_range(page_tables, memory, kernel.gateway, PagePermissions::USER_RX)?;
        if let Some(relro) = image.relro {
            let start = align_down(relro.address as u64);
            let end = align_up((relro.address + relro.length) as u64)?;
            page_tables.protect_range(layout.image_base + start, end - start,
                PagePermissions::USER_R)?;
        }
        Ok(())
    }
}

impl X86AddressSpace {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty four-level x86-64 page-table hierarchy owned by one execution context.
    // ------------------=
    pub fn new(memory: &mut NativeMemory, owner: u32) -> Result<Self, MemoryError> {
        let root = memory.allocate(1, owner)?;
        let mut tables = [None; MAX_PAGE_TABLES];
        tables[0] = Some(root);
        Ok(Self { root, tables, table_count: 1 })
    }

    // ------------------------=
    // FUNC: root_physical_address
    // DESC: Returns the CR3-compatible physical address of the context root table.
    // ------------------=
    pub fn root_physical_address(&self) -> u64 { self.root.physical_address }

    // ------------------------=
    // FUNC: map_range
    // DESC: Maps a page-aligned range with explicit user, write, and execute permissions.
    // ------------------=
    pub fn map_range(&mut self, memory: &mut NativeMemory, virtual_address: u64,
        physical_address: u64, bytes: u64, permissions: PagePermissions) -> Result<(), MemoryError> {
        validate_range(virtual_address, physical_address, bytes)?;
        let pages = bytes / PAGE_SIZE;
        for page in 0..pages {
            self.map_page(memory, virtual_address + page * PAGE_SIZE,
                physical_address + page * PAGE_SIZE, permissions)?;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: protect_range
    // DESC: Rewrites existing leaf permissions, including the post-relocation RELRO transition.
    // ------------------=
    pub fn protect_range(&mut self, virtual_address: u64, bytes: u64,
        permissions: PagePermissions) -> Result<(), MemoryError> {
        if virtual_address & (PAGE_SIZE - 1) != 0 || bytes == 0 || bytes & (PAGE_SIZE - 1) != 0 {
            return Err(MemoryError::InvalidAlignment);
        }
        for page in 0..bytes / PAGE_SIZE {
            let leaf = self.leaf_entry_mut(virtual_address + page * PAGE_SIZE)?;
            if *leaf & PRESENT == 0 { return Err(MemoryError::MissingMapping); }
            *leaf = (*leaf & ADDRESS_MASK) | leaf_flags(permissions);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: map_guarded_stack
    // DESC: Maps writable non-executable stack pages between deliberately absent lower and upper guard pages.
    // ------------------=
    pub fn map_guarded_stack(&mut self, memory: &mut NativeMemory, guard_base: u64,
        stack: FrameAllocation) -> Result<(), MemoryError> {
        self.map_range(memory, guard_base + PAGE_SIZE, stack.physical_address,
            stack.page_count as u64 * PAGE_SIZE, PagePermissions::USER_RW)
    }

    // ------------------------=
    // FUNC: mapping
    // DESC: Walks the live hierarchy and reports the effective leaf mapping for behavioral verification and fault handling.
    // ------------------=
    pub fn mapping(&self, virtual_address: u64) -> Option<Mapping> {
        let indices = page_indices(virtual_address);
        let mut table = self.root.physical_address;
        let mut effective_user = true;
        let mut effective_write = true;
        let mut effective_execute = true;
        for (level, index) in indices.iter().enumerate() {
            let entry = unsafe { *((table as *const u64).add(*index)) };
            if entry & PRESENT == 0 { return None; }
            effective_user &= entry & USER != 0;
            effective_write &= entry & WRITABLE != 0;
            effective_execute &= entry & NO_EXECUTE == 0;
            if level == 3 {
                return Some(Mapping {
                    physical_address: (entry & ADDRESS_MASK) | (virtual_address & (PAGE_SIZE - 1)),
                    permissions: PagePermissions {
                        user: effective_user,
                        writable: effective_write,
                        executable: effective_execute,
                    },
                });
            }
            table = entry & ADDRESS_MASK;
        }
        None
    }

    // ------------------------=
    // FUNC: release
    // DESC: Tears down every page-table frame belonging to the context.
    // ------------------=
    pub fn release(mut self, memory: &mut NativeMemory) -> Result<(), MemoryError> {
        while self.table_count > 0 {
            self.table_count -= 1;
            memory.release(self.tables[self.table_count].take().unwrap())?;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: map_page
    // DESC: Installs one 4 KiB leaf while allocating any missing intermediate tables.
    // ------------------=
    fn map_page(&mut self, memory: &mut NativeMemory, virtual_address: u64,
        physical_address: u64, permissions: PagePermissions) -> Result<(), MemoryError> {
        let indices = page_indices(virtual_address);
        let mut table = self.root.physical_address;
        for index in &indices[..3] {
            let entry = unsafe { &mut *((table as *mut u64).add(*index)) };
            if *entry & PRESENT == 0 {
                if self.table_count == MAX_PAGE_TABLES { return Err(MemoryError::PageTableLimit); }
                let child = memory.allocate(1, self.root.owner)?;
                self.tables[self.table_count] = Some(child);
                self.table_count += 1;
                *entry = child.physical_address | PRESENT | WRITABLE |
                    if permissions.user { USER } else { 0 };
            } else if permissions.user {
                *entry |= USER;
            }
            table = *entry & ADDRESS_MASK;
        }
        let leaf = unsafe { &mut *((table as *mut u64).add(indices[3])) };
        if *leaf & PRESENT != 0 { return Err(MemoryError::MappingConflict); }
        *leaf = physical_address | leaf_flags(permissions);
        Ok(())
    }

    // ------------------------=
    // FUNC: leaf_entry_mut
    // DESC: Resolves a mutable leaf entry without creating missing tables.
    // ------------------=
    fn leaf_entry_mut(&mut self, virtual_address: u64) -> Result<&mut u64, MemoryError> {
        let indices = page_indices(virtual_address);
        let mut table = self.root.physical_address;
        for index in &indices[..3] {
            let entry = unsafe { *((table as *const u64).add(*index)) };
            if entry & PRESENT == 0 { return Err(MemoryError::MissingMapping); }
            table = entry & ADDRESS_MASK;
        }
        Ok(unsafe { &mut *((table as *mut u64).add(indices[3])) })
    }
}

// ------------------------=
// FUNC: validate_range
// DESC: Rejects non-page-aligned, empty, or overflowing mappings before mutating page tables.
// ------------------=
fn validate_range(virtual_address: u64, physical_address: u64, bytes: u64) -> Result<(), MemoryError> {
    if virtual_address & (PAGE_SIZE - 1) != 0 || physical_address & (PAGE_SIZE - 1) != 0 ||
        bytes == 0 || bytes & (PAGE_SIZE - 1) != 0 {
        return Err(MemoryError::InvalidAlignment);
    }
    virtual_address.checked_add(bytes).ok_or(MemoryError::AddressOverflow)?;
    physical_address.checked_add(bytes).ok_or(MemoryError::AddressOverflow)?;
    Ok(())
}

// ------------------------=
// FUNC: map_kernel_range
// DESC: Maps a required kernel or gateway range while allowing explicitly empty optional sections.
// ------------------=
fn map_kernel_range(address_space: &mut X86AddressSpace, memory: &mut NativeMemory,
    range: MappedRange, permissions: PagePermissions) -> Result<(), MemoryError> {
    if range.bytes == 0 { return Ok(()); }
    address_space.map_range(memory, range.virtual_address, range.physical_address,
        range.bytes, permissions)
}

// ------------------------=
// FUNC: pages_for
// DESC: Rounds a non-empty byte count into a bounded physical-page count.
// ------------------=
fn pages_for(bytes: u64) -> Result<u32, MemoryError> {
    if bytes == 0 { return Err(MemoryError::InvalidPageCount); }
    let rounded = align_up(bytes)? / PAGE_SIZE;
    u32::try_from(rounded).map_err(|_| MemoryError::AddressOverflow)
}

// ------------------------=
// FUNC: align_down
// DESC: Rounds an address down to its containing physical page.
// ------------------=
fn align_down(value: u64) -> u64 { value & !(PAGE_SIZE - 1) }

// ------------------------=
// FUNC: align_up
// DESC: Rounds an address up to a page boundary while rejecting overflow.
// ------------------=
fn align_up(value: u64) -> Result<u64, MemoryError> {
    value.checked_add(PAGE_SIZE - 1).map(|next| next & !(PAGE_SIZE - 1))
        .ok_or(MemoryError::AddressOverflow)
}

// ------------------------=
// FUNC: page_indices
// DESC: Splits a canonical x86-64 virtual page number into four table indices.
// ------------------=
fn page_indices(address: u64) -> [usize; 4] {
    [
        ((address >> 39) & 0x1ff) as usize,
        ((address >> 30) & 0x1ff) as usize,
        ((address >> 21) & 0x1ff) as usize,
        ((address >> 12) & 0x1ff) as usize,
    ]
}

// ------------------------=
// FUNC: leaf_flags
// DESC: Encodes explicit W-X and privilege policy into an x86-64 leaf entry.
// ------------------=
fn leaf_flags(permissions: PagePermissions) -> u64 {
    PRESENT |
        if permissions.writable { WRITABLE } else { 0 } |
        if permissions.user { USER } else { 0 } |
        if permissions.executable { 0 } else { NO_EXECUTE }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
// ------------------------=
// FUNC: activate
// DESC: Enables write protection and NX enforcement before switching CR3 to a prepared context.
// ------------------=
pub unsafe fn activate(root_physical_address: u64) {
    let mut efer_low: u32;
    let mut efer_high: u32;
    core::arch::asm!("rdmsr", in("ecx") 0xc000_0080u32, out("eax") efer_low,
        out("edx") efer_high, options(nostack, preserves_flags));
    efer_low |= 1 << 11;
    core::arch::asm!("wrmsr", in("ecx") 0xc000_0080u32, in("eax") efer_low,
        in("edx") efer_high, options(nostack, preserves_flags));
    let mut cr0: u64;
    core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nostack, preserves_flags));
    cr0 |= 1 << 16;
    core::arch::asm!("mov cr0, {}", in(reg) cr0, options(nostack, preserves_flags));
    core::arch::asm!("mov cr3, {}", in(reg) root_physical_address,
        options(nostack, preserves_flags));
}

static mut NATIVE_MEMORY: NativeMemory = NativeMemory::empty();

// ------------------------=
// FUNC: initialize
// DESC: Initializes the process frame allocator from the versioned boot-time reservation.
// ------------------=
pub fn initialize(base: u64, bytes: u64) -> Result<(), MemoryError> {
    unsafe { (&mut *(&raw mut NATIVE_MEMORY)).initialize(base, bytes) }
}

// ------------------------=
// FUNC: memory_mut
// DESC: Provides kernel-internal access to the initialized native frame allocator.
// ------------------=
pub fn memory_mut() -> &'static mut NativeMemory {
    unsafe { &mut *(&raw mut NATIVE_MEMORY) }
}
