//! ARMv8-A stage-one 4 KiB / 48-bit tables. Native RAM uses MAIR slot zero
//! (normal write-back, inner-shareable); devices are deliberately not user mapped.
use super::native_memory::{FrameAllocation, Mapping, MemoryError, NativeMemory,
    PagePermissions, PageTable, PAGE_SIZE, validate_permissions};

const ADDRESS: u64 = 0x0000_ffff_ffff_f000;
const TABLE_LIMIT: usize = 768;
const VALID: u64 = 3;
const AP_USER: u64 = 1 << 6;
const AP_RO: u64 = 1 << 7;
const PXN: u64 = 1 << 53;
const UXN: u64 = 1 << 54;

pub struct ArmAddressSpace {
    root: FrameAllocation,
    tables: [Option<FrameAllocation>; TABLE_LIMIT],
    count: usize,
}

impl ArmAddressSpace {
    // ------------------------=
    // FUNC: root_physical_address
    // DESC: Returns the TTBR0-compatible physical address of the private lower-half hierarchy.
    // ------------------=
    pub fn root_physical_address(&self) -> u64 { self.root.physical_address }

    // ------------------------=
    // FUNC: leaf
    // DESC: Walks owned table descriptors without creating tables or accepting block mappings.
    // ------------------=
    fn leaf(&self, address: u64) -> Result<*mut u64, MemoryError> {
        if address >> 48 != 0 { return Err(MemoryError::InvalidAddress); }
        let mut table = self.root.physical_address;
        for shift in [39, 30, 21] {
            let entry = unsafe { *((table as *const u64).add(((address >> shift) & 511) as usize)) };
            if entry & 3 != VALID { return Err(MemoryError::MissingMapping); }
            table = entry & ADDRESS;
        }
        Ok(unsafe { (table as *mut u64).add(((address >> 12) & 511) as usize) })
    }

    // ------------------------=
    // FUNC: map_page
    // DESC: Allocates only missing intermediate tables and rejects overwriting an existing page.
    // ------------------=
    fn map_page(&mut self, memory: &mut NativeMemory, address: u64, physical: u64,
        permissions: PagePermissions) -> Result<(), MemoryError> {
        let mut table = self.root.physical_address;
        for shift in [39, 30, 21] {
            let entry = unsafe { &mut *((table as *mut u64).add(((address >> shift) & 511) as usize)) };
            if *entry & 3 == 0 {
                if self.count == TABLE_LIMIT { return Err(MemoryError::PageTableLimit); }
                let child = memory.allocate(1, self.root.owner)?;
                self.tables[self.count] = Some(child);
                self.count += 1;
                *entry = child.physical_address | VALID;
            }
            if *entry & 3 != VALID { return Err(MemoryError::MappingConflict); }
            table = *entry & ADDRESS;
        }
        let leaf = unsafe { &mut *((table as *mut u64).add(((address >> 12) & 511) as usize)) };
        if *leaf & 1 != 0 { return Err(MemoryError::MappingConflict); }
        *leaf = physical | flags(permissions);
        Ok(())
    }
}

impl PageTable for ArmAddressSpace {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty private hierarchy using the same physical ownership allocator as x86.
    // ------------------=
    fn new(memory: &mut NativeMemory, owner: u32) -> Result<Self, MemoryError> {
        let root = memory.allocate(1, owner)?;
        if root.physical_address >> 48 != 0 {
            memory.release(root)?;
            return Err(MemoryError::InvalidAddress);
        }
        let mut tables = [None; TABLE_LIMIT];
        tables[0] = Some(root);
        Ok(Self { root, tables, count: 1 })
    }

    // ------------------------=
    // FUNC: map_range
    // DESC: Maps normal RAM with AP, PXN and UXN hardware access restrictions.
    // ------------------=
    fn map_range(&mut self, memory: &mut NativeMemory, address: u64, physical: u64,
        bytes: u64, permissions: PagePermissions) -> Result<(), MemoryError> {
        validate(address, physical, bytes)?;
        validate_permissions(permissions)?;
        for offset in (0..bytes).step_by(PAGE_SIZE as usize) {
            self.map_page(memory, address + offset, physical + offset, permissions)?;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: protect_range
    // DESC: Preflights the whole permission change and never promotes kernel pages to user pages.
    // ------------------=
    fn protect_range(&mut self, address: u64, bytes: u64,
        permissions: PagePermissions) -> Result<(), MemoryError> {
        validate(address, 0, bytes)?;
        validate_permissions(permissions)?;
        for offset in (0..bytes).step_by(PAGE_SIZE as usize) {
            let current = self.mapping(address + offset).ok_or(MemoryError::MissingMapping)?;
            if current.permissions.user != permissions.user { return Err(MemoryError::AccessDenied); }
        }
        for offset in (0..bytes).step_by(PAGE_SIZE as usize) {
            let leaf = self.leaf(address + offset)?;
            unsafe { *leaf = (*leaf & ADDRESS) | flags(permissions); }
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: mapping
    // DESC: Decodes effective access from a private hierarchy containing no block or table restrictions.
    // ------------------=
    fn mapping(&self, address: u64) -> Option<Mapping> {
        let leaf = unsafe { *self.leaf(address).ok()? };
        if leaf & 3 != VALID { return None; }
        let user = leaf & AP_USER != 0;
        Some(Mapping { physical_address: (leaf & ADDRESS) | (address & 4095),
            permissions: PagePermissions { user, writable: leaf & AP_RO == 0,
                executable: leaf & if user { UXN } else { PXN } == 0 } })
    }

    // ------------------------=
    // FUNC: unmap_user_range
    // DESC: Validates the whole user interval before removing ARM page descriptors from an inactive root.
    // ------------------=
    fn unmap_user_range(&mut self, address: u64, bytes: u64) -> Result<(), MemoryError> {
        validate(address, 0, bytes)?;
        super::native_memory::check_user_range(self, address, bytes as usize, false)?;
        for offset in (0..bytes).step_by(PAGE_SIZE as usize) {
            unsafe { *self.leaf(address + offset)? = 0; }
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: release
    // DESC: Returns every owned table frame after the caller has stopped using TTBR0.
    // ------------------=
    fn release(mut self, memory: &mut NativeMemory) -> Result<(), MemoryError> {
        while self.count > 0 {
            self.count -= 1;
            memory.release(self.tables[self.count].take().unwrap())?;
        }
        Ok(())
    }
}

// ------------------------=
// FUNC: flags
// DESC: Encodes AF, inner-shareable normal memory, nG, AP, and separate user/kernel execute permissions.
// ------------------=
fn flags(permissions: PagePermissions) -> u64 {
    VALID | (1 << 10) | (3 << 8) | (1 << 11) |
        if permissions.user { AP_USER } else { 0 } |
        if permissions.writable { 0 } else { AP_RO } |
        if permissions.user || !permissions.executable { PXN } else { 0 } |
        if !permissions.user || !permissions.executable { UXN } else { 0 }
}

// ------------------------=
// FUNC: validate
// DESC: Rejects noncanonical, misaligned, empty, or overflowing ARM lower-half mappings.
// ------------------=
fn validate(address: u64, physical: u64, bytes: u64) -> Result<(), MemoryError> {
    if (address | physical | bytes) & 4095 != 0 || bytes == 0 {
        return Err(MemoryError::InvalidAlignment);
    }
    let end = address.checked_add(bytes - 1).ok_or(MemoryError::AddressOverflow)?;
    let physical_end = physical.checked_add(bytes - 1).ok_or(MemoryError::AddressOverflow)?;
    if end >> 48 != 0 || physical_end >> 48 != 0 { return Err(MemoryError::InvalidAddress); }
    Ok(())
}
