#[path = "../kernel/runtime/native_c_image.rs"] mod native_c_image;
#[path = "../kernel/runtime/native_memory.rs"] mod native_memory;
#[path = "../kernel/runtime/native_memory_arm.rs"] mod native_memory_arm;
use native_memory::*;
use native_memory_arm::ArmAddressSpace;

#[repr(C, align(4096))]
struct Arena([u8; 8 * 1024 * 1024]);

// ------------------------=
// FUNC: verify
// DESC: Runs identical permission, cross-page copy, failure-atomicity, and ownership tests against both hardware encodings.
// ------------------=
fn verify<T: PageTable>() {
    let layout = std::alloc::Layout::new::<Arena>();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!arena.is_null());
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let initial = memory.available_pages();
    let data = memory.allocate(3, 1).unwrap();
    let mut tables = T::new(&mut memory, 1).unwrap();
    let user = 0x1_0000_0000;
    tables.map_range(&mut memory, user, data.physical_address, 8192, PagePermissions::USER_RW).unwrap();
    tables.map_range(&mut memory, user + 8192, data.physical_address + 8192, 4096,
        PagePermissions::USER_R).unwrap();
    tables.map_range(&mut memory, user + 0x10000, data.physical_address, 4096,
        PagePermissions::KERNEL_RW).unwrap();
    let input = [0x5au8; 5000];
    copy_to_user(&mut tables, user + 3000, &input).unwrap();
    let mut output = [0u8; 5000];
    copy_from_user(&tables, user + 3000, &mut output).unwrap();
    assert_eq!(input, output);
    assert_eq!(copy_to_user(&mut tables, user + 7000, &[0xff; 2000]), Err(MemoryError::AccessDenied));
    output.fill(0);
    copy_from_user(&tables, user + 3000, &mut output).unwrap();
    assert_eq!(input, output, "invalid partial writes must leave all pages untouched");
    assert_eq!(check_user_range(&tables, user + 0x10000, 1, false), Err(MemoryError::AccessDenied));
    assert_eq!(check_user_range(&tables, u64::MAX - 4, 16, false), Err(MemoryError::AddressOverflow));
    assert_eq!(check_user_range(&tables, user + 12000, 1000, false), Err(MemoryError::MissingMapping));
    assert_eq!(tables.protect_range(user + 8192, 8192, PagePermissions::USER_RW), Err(MemoryError::MissingMapping));
    assert!(!tables.mapping(user + 8192).unwrap().permissions.writable);
    assert_eq!(tables.protect_range(user + 0x10000, 4096, PagePermissions::USER_RW), Err(MemoryError::AccessDenied));
    assert_eq!(tables.map_range(&mut memory, user + 0x20000, data.physical_address, 4096,
        PagePermissions { user: true, writable: true, executable: true }), Err(MemoryError::InvalidPermissions));
    assert!(tables.mapping(user + 0x20000).is_none());
    tables.protect_range(user, 4096, PagePermissions::USER_RX).unwrap();
    assert_eq!(tables.mapping(user).unwrap().permissions, PagePermissions::USER_RX);
    assert_eq!(check_user_range(&tables, user, 1, true), Err(MemoryError::AccessDenied));
    assert!(tables.mapping(0x1000_0001_0000_0000).is_none());
    assert_eq!(tables.unmap_user_range(user + 0x10000, 4096), Err(MemoryError::AccessDenied));
    assert_eq!(tables.unmap_user_range(user + 8192, 8192), Err(MemoryError::MissingMapping));
    assert!(tables.mapping(user + 8192).is_some());
    tables.unmap_user_range(user, 12288).unwrap();
    assert_eq!(check_user_range(&tables, user, 1, false), Err(MemoryError::MissingMapping));
    assert!(tables.mapping(user + 8192).is_none());
    tables.map_range(&mut memory, user, data.physical_address, 4096, PagePermissions::USER_RW).unwrap();
    assert_eq!(tables.mapping(user).unwrap().physical_address, data.physical_address);
    tables.release(&mut memory).unwrap();
    memory.release(data).unwrap();
    assert_eq!(memory.available_pages(), initial);
    unsafe { std::alloc::dealloc(arena, layout); }
}

// ------------------------=
// FUNC: main
// DESC: Verifies architecture-neutral user-memory behavior using real x86 and ARM descriptor pages.
// ------------------=
fn main() {
    verify::<X86AddressSpace>();
    verify::<ArmAddressSpace>();
    verify_tls();
}

// ------------------------=
// FUNC: verify_tls
// DESC: Checks private virtual TLS pointers, initialized data, zero BSS, overflow rejection and failure atomicity.
// ------------------=
fn verify_tls() {
    use native_c_image::{tls::Template, ImageError};
    let template = Template { source: 0, address: 0, file_size: 4, memory_size: 32, alignment: 16 };
    for machine in [62, 183] {
        let mut storage = [0xa5; 128];
        let base = 0x400000003usize;
        let block = template.initialize_at(&[1, 2, 3, 4], machine, &mut storage, base).unwrap();
        let pointer = block.thread_pointer();
        assert_eq!(pointer % 16, 0);
        assert!(pointer >= base && pointer < base + 128);
        assert_eq!(&block.data()[..4], &[1, 2, 3, 4]);
        assert!(block.data()[4..].iter().all(|byte| *byte == 0));
        drop(block);
        if machine == 62 {
            let offset = pointer - base;
            assert_eq!(u64::from_le_bytes(storage[offset..offset + 8].try_into().unwrap()), pointer as u64);
        }
        storage.fill(0xa5);
        assert!(matches!(template.initialize_at(&[1, 2, 3, 4], machine, &mut storage, usize::MAX - 16), Err(ImageError::Bounds)));
        assert_eq!(storage, [0xa5; 128]);
        assert!(matches!(template.initialize_at(&[1, 2, 3, 4], machine, &mut storage[..8], base), Err(ImageError::Bounds)));
        assert_eq!(storage, [0xa5; 128]);
    }
}
