//! Bounded private anonymous mappings for isolated native applications.
//! The execution dispatcher owns this table; apps never supply its owner or budget.
use super::native_memory::{FrameAllocation, MemoryError, NativeMemory, PagePermissions, PageTable, PAGE_SIZE};

const MAX_MAPPINGS: usize = 32;
const ARENA_BASE: u64 = 0x400000000;
const ARENA_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy)]
struct Mapping { address: u64, allocation: FrameAllocation }

pub struct PrivateMappings {
    owner: u32,
    page_budget: u32,
    used_pages: u32,
    entries: [Option<Mapping>; MAX_MAPPINGS],
}

impl PrivateMappings {
    // ------------------------=
    // FUNC: new
    // DESC: Binds private mapping authority and a committed-page budget to one dispatcher-owned context.
    // ------------------=
    pub const fn new(owner: u32, page_budget: u32) -> Self {
        Self { owner, page_budget, used_pages: 0, entries: [None; MAX_MAPPINGS] }
    }

    // ------------------------=
    // FUNC: used_pages
    // DESC: Reports owned committed pages for resource accounting.
    // ------------------=
    pub fn used_pages(&self) -> u32 { self.used_pages }

    // ------------------------=
    // FUNC: map
    // DESC: Maps zeroed non-executable anonymous pages with bounded accounting and rollback of partial mapping failures.
    // ------------------=
    pub fn map<T: PageTable>(&mut self, tables: &mut T, memory: &mut NativeMemory,
        bytes: u64, writable: bool) -> Result<u64, MemoryError> {
        let pages = pages(bytes)?;
        if self.owner == 0 || pages > self.page_budget.saturating_sub(self.used_pages) {
            return Err(MemoryError::AccessDenied);
        }
        let slot = self.entries.iter().position(Option::is_none).ok_or(MemoryError::AllocationTableFull)?;
        let length = pages as u64 * PAGE_SIZE;
        if length > ARENA_BYTES { return Err(MemoryError::OutOfMemory); }
        let mut address = ARENA_BASE;
        while address + length <= ARENA_BASE + ARENA_BYTES {
            let conflict = (0..pages).find(|index| tables.mapping(address + *index as u64 * PAGE_SIZE).is_some());
            if let Some(index) = conflict { address += (index as u64 + 1) * PAGE_SIZE; }
            else { break; }
        }
        if address + length > ARENA_BASE + ARENA_BYTES { return Err(MemoryError::OutOfMemory); }
        let allocation = memory.allocate(pages, self.owner)?;
        let permissions = PagePermissions { user: true, writable, executable: false };
        for index in 0..pages {
            if let Err(error) = tables.map_range(memory, address + index as u64 * PAGE_SIZE,
                allocation.physical_address + index as u64 * PAGE_SIZE, PAGE_SIZE, permissions) {
                if index > 0 { tables.unmap_user_range(address, index as u64 * PAGE_SIZE)?; }
                memory.release(allocation)?;
                return Err(error);
            }
        }
        self.entries[slot] = Some(Mapping { address, allocation });
        self.used_pages += pages;
        Ok(address)
    }

    // ------------------------=
    // FUNC: owned
    // DESC: Requires an exact live owned mapping and validates every physical translation before mutation.
    // ------------------=
    fn owned<T: PageTable>(&self, tables: &T, address: u64, bytes: u64) -> Result<usize, MemoryError> {
        let pages = pages(bytes)?;
        let slot = self.entries.iter().position(|entry| entry.is_some_and(|entry|
            entry.address == address && entry.allocation.page_count == pages)).ok_or(MemoryError::UnknownAllocation)?;
        let entry = self.entries[slot].unwrap();
        for index in 0..pages {
            let mapping = tables.mapping(address + index as u64 * PAGE_SIZE).ok_or(MemoryError::MissingMapping)?;
            if !mapping.permissions.user || mapping.permissions.executable ||
                mapping.physical_address != entry.allocation.physical_address + index as u64 * PAGE_SIZE {
                return Err(MemoryError::AccessDenied);
            }
        }
        Ok(slot)
    }

    // ------------------------=
    // FUNC: unmap
    // DESC: Removes only a complete owned mapping and returns its frames to the allocator.
    // ------------------=
    pub fn unmap<T: PageTable>(&mut self, tables: &mut T, memory: &mut NativeMemory,
        address: u64, bytes: u64) -> Result<(), MemoryError> {
        let slot = self.owned(tables, address, bytes)?;
        let entry = self.entries[slot].unwrap();
        tables.unmap_user_range(address, entry.allocation.page_count as u64 * PAGE_SIZE)?;
        memory.release(entry.allocation)?;
        self.used_pages -= entry.allocation.page_count;
        self.entries[slot] = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: protect
    // DESC: Changes read/write rights of a complete owned mapping without granting executable memory.
    // ------------------=
    pub fn protect<T: PageTable>(&self, tables: &mut T, address: u64, bytes: u64,
        writable: bool) -> Result<(), MemoryError> {
        let slot = self.owned(tables, address, bytes)?;
        let entry = self.entries[slot].unwrap();
        tables.protect_range(address, entry.allocation.page_count as u64 * PAGE_SIZE,
            PagePermissions { user: true, writable, executable: false })
    }

    // ------------------------=
    // FUNC: release_all
    // DESC: Reclaims all dynamic mappings during normal exit, cancellation, or fatal process cleanup.
    // ------------------=
    pub fn release_all<T: PageTable>(&mut self, tables: &mut T, memory: &mut NativeMemory) -> Result<(), MemoryError> {
        for index in 0..MAX_MAPPINGS {
            if let Some(entry) = self.entries[index] {
                self.unmap(tables, memory, entry.address, entry.allocation.page_count as u64 * PAGE_SIZE)?;
            }
        }
        Ok(())
    }
}

// ------------------------=
// FUNC: pages
// DESC: Rounds a nonzero byte length while rejecting overflow before allocation or mutation.
// ------------------=
fn pages(bytes: u64) -> Result<u32, MemoryError> {
    if bytes == 0 { return Err(MemoryError::InvalidPageCount); }
    let rounded = bytes.checked_add(PAGE_SIZE - 1).ok_or(MemoryError::AddressOverflow)? / PAGE_SIZE;
    u32::try_from(rounded).map_err(|_| MemoryError::AddressOverflow)
}
