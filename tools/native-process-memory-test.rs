#[path = "../kernel/runtime/native_c_image.rs"] mod native_c_image;
#[path = "../kernel/runtime/native_memory.rs"] mod native_memory;
#[path = "../kernel/runtime/native_memory_arm.rs"] mod native_memory_arm;
#[path = "../kernel/runtime/native_mappings.rs"] mod native_mappings;
#[path = "../kernel/runtime/native_stdio.rs"] mod native_stdio;
#[path = "../kernel/runtime/native_compiler_launch.rs"] mod native_compiler_launch;
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
    verify_mappings::<X86AddressSpace>();
    verify_mappings::<ArmAddressSpace>();
    verify_streams::<X86AddressSpace>();
    verify_streams::<ArmAddressSpace>();
    verify_launch::<X86AddressSpace>();
    verify_launch::<ArmAddressSpace>();
}

// ------------------------=
// FUNC: verify_launch
// DESC: Validates encoded launch pointers, IDs, heap boundaries and failure atomicity using both native page table encodings.
// ------------------=
fn verify_launch<T: PageTable>() {
    use native_compiler_launch::{write, Invocation};
    let layout = std::alloc::Layout::new::<Arena>();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!arena.is_null());
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let initial = memory.available_pages();
    let data = memory.allocate(3, 1).unwrap();
    let mut tables = T::new(&mut memory, 1).unwrap();
    let base = 0x200000000;
    tables.map_range(&mut memory, base, data.physical_address, 12288, PagePermissions::USER_RW).unwrap();
    let arguments: &[&[u8]] = &[b"clang", b"-cc1", b"-"];
    let invocation = || Invocation { arguments, executable: b"/system/clang", home: b"/home/default",
        node: b"test", process: 7, session: 11, heap: base+8192, heap_bytes: 4096 };
    write(&mut tables, base, invocation()).unwrap();
    let mut bytes = [0u8;4096];
    copy_from_user(&tables, base, &mut bytes).unwrap();
    let word = |offset| u64::from_le_bytes(bytes[offset..offset+8].try_into().unwrap());
    assert_eq!(word(0), 1 | (72u64 << 32));
    assert_eq!(word(8), 3 | (7u64 << 32));
    assert_eq!(word(16), 11);
    assert_eq!(word(24), base+72);
    assert_eq!((word(32),word(40)), (base+8192,4096));
    assert_eq!(word(96), 0);
    for (index, value) in arguments.iter().enumerate() {
        let offset = (word(72+index*8)-base) as usize;
        assert_eq!(&bytes[offset..offset+value.len()], *value);
        assert_eq!(bytes[offset+value.len()], 0);
    }
    let mut bad = invocation(); bad.arguments = &[b"bad\0argument"];
    assert_eq!(write(&mut tables, base, bad), Err(MemoryError::InvalidAddress));
    let mut bad = invocation(); bad.heap = base;
    assert_eq!(write(&mut tables, base, bad), Err(MemoryError::InvalidAddress));
    let mut bad = invocation(); bad.process = 0;
    assert_eq!(write(&mut tables, base, bad), Err(MemoryError::InvalidAddress));
    let too_many: [&[u8]; 33] = [b"x"; 33];
    let mut bad = invocation(); bad.arguments = &too_many;
    assert_eq!(write(&mut tables, base, bad), Err(MemoryError::InvalidAddress));
    let large = [b'x'; 4096]; let large_args: &[&[u8]] = &[&large];
    let mut bad = invocation(); bad.arguments = large_args;
    assert_eq!(write(&mut tables, base, bad), Err(MemoryError::InvalidAddress));
    let mut bad = invocation(); bad.heap_bytes = 8192;
    assert_eq!(write(&mut tables, base, bad), Err(MemoryError::MissingMapping));
    let mut bad = invocation(); bad.heap = u64::MAX - 15; bad.heap_bytes = 32;
    assert_eq!(write(&mut tables, base, bad), Err(MemoryError::AddressOverflow));
    let mut after = [0;4096]; copy_from_user(&tables, base, &mut after).unwrap();
    assert_eq!(bytes, after);
    tables.release(&mut memory).unwrap(); memory.release(data).unwrap();
    assert_eq!(memory.available_pages(), initial);
    unsafe { std::alloc::dealloc(arena, layout); }
}

// ------------------------=
// FUNC: verify_streams
// DESC: Exercises real checked compiler streams on both page-table implementations, including invalid result pointers and binary separation.
// ------------------=
fn verify_streams<T: PageTable>() {
    let layout = std::alloc::Layout::new::<Arena>();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!arena.is_null());
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let data = memory.allocate(2, 1).unwrap();
    let mut tables = T::new(&mut memory, 1).unwrap();
    let user = 0x1_0000_0000;
    tables.map_range(&mut memory, user, data.physical_address, 8192, PagePermissions::USER_RW).unwrap();
    let source = [7; 5000];
    let mut output = [0; 8];
    let mut errors = [0; 3];
    let mut io = native_stdio::Streams::new(&source, &mut output, &mut errors);
    assert_eq!(io.syscall(&mut tables, 0x121b, [0,user,10,user+8192,0,0]), -14);
    assert_eq!(io.syscall(&mut tables, 0x121b, [0,user,5000,user+7000,0,0]), 0);
    let mut count = [0;8];
    copy_from_user(&tables,user+7000,&mut count).unwrap();
    assert_eq!(u64::from_le_bytes(count),4096);
    assert_eq!(io.syscall(&mut tables, 0x121b, [0,user,5000,user+7000,0,0]), 0);
    copy_from_user(&tables,user+7000,&mut count).unwrap();
    assert_eq!(u64::from_le_bytes(count),904);
    assert_eq!(io.syscall(&mut tables, 0x121b, [0,0,5000,user+7000,0,0]), 0);
    copy_from_user(&tables,user+7000,&mut count).unwrap();
    assert_eq!(u64::from_le_bytes(count),0);
    let binary = [0x7f,0,255,1,2,3,4,5];
    copy_to_user(&mut tables,user+4092,&binary).unwrap();
    assert_eq!(io.syscall(&mut tables,0x121c,[1,user+4092,8,user+8192,0,0]),-14);
    assert!(io.output().is_empty());
    assert_eq!(io.syscall(&mut tables,0x121c,[1,user+4092,8,user+7000,0,0]),0);
    assert_eq!(io.output(),binary);
    assert_eq!(io.syscall(&mut tables,0x121c,[1,user+4092,1,user+7000,0,0]),-28);
    assert_eq!(io.output(),binary);
    assert_eq!(io.syscall(&mut tables,0x121c,[2,user+4092,8,user+7000,0,0]),0);
    assert_eq!(io.diagnostics(),(&binary[..3],true));
    assert_eq!(io.output(),binary);
    assert_eq!(io.syscall(&mut tables,0x121c,[0x100000001,user,1,user+7000,0,0]),-9);
    assert_eq!(io.syscall(&mut tables,0x121d,[1,user+7000,0,0,0,0]),0);
    copy_from_user(&tables,user+7000,&mut count[..4]).unwrap();
    assert_eq!(&count[..4], &[0;4]);
    tables.release(&mut memory).unwrap();
    memory.release(data).unwrap();
    unsafe {std::alloc::dealloc(arena,layout);}
}

// ------------------------=
// FUNC: verify_mappings
// DESC: Verifies budget, zero-fill, non-executable protection, cross-owner denial, reuse and complete frame reclamation on both architectures.
// ------------------=
fn verify_mappings<T: PageTable>() {
    let layout = std::alloc::Layout::new::<Arena>();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!arena.is_null());
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let initial = memory.available_pages();
    let mut tables = T::new(&mut memory, 7).unwrap();
    let mut mappings = native_mappings::PrivateMappings::new(7, 3);
    let mut stranger = native_mappings::PrivateMappings::new(8, 3);
    let first = mappings.map(&mut tables, &mut memory, 4097, true).unwrap();
    assert_eq!(mappings.used_pages(), 2);
    let mut bytes = [0xa5; 8192];
    copy_from_user(&tables, first, &mut bytes).unwrap();
    assert_eq!(bytes, [0; 8192]);
    copy_to_user(&mut tables, first, &[42; 8192]).unwrap();
    assert_eq!(mappings.map(&mut tables, &mut memory, 8192, true), Err(MemoryError::AccessDenied));
    assert_eq!(stranger.unmap(&mut tables, &mut memory, first, 8192), Err(MemoryError::UnknownAllocation));
    assert_eq!(mappings.unmap(&mut tables, &mut memory, first, 4096), Err(MemoryError::UnknownAllocation));
    mappings.protect(&mut tables, first, 8192, false).unwrap();
    assert_eq!(copy_to_user(&mut tables, first, &[1]), Err(MemoryError::AccessDenied));
    assert!(!tables.mapping(first).unwrap().permissions.executable);
    mappings.protect(&mut tables, first, 8192, true).unwrap();
    copy_to_user(&mut tables, first, &[43]).unwrap();
    let second = mappings.map(&mut tables, &mut memory, 4096, false).unwrap();
    assert_eq!(second, first + 8192);
    mappings.unmap(&mut tables, &mut memory, first, 8192).unwrap();
    assert!(tables.mapping(first).is_none());
    assert_eq!(mappings.map(&mut tables, &mut memory, 8192, true).unwrap(), first);
    copy_from_user(&tables, first, &mut bytes).unwrap();
    assert_eq!(bytes, [0; 8192]);
    assert_eq!(mappings.map(&mut tables, &mut memory, u64::MAX, true), Err(MemoryError::AddressOverflow));
    mappings.release_all(&mut tables, &mut memory).unwrap();
    assert_eq!(mappings.used_pages(), 0);
    assert!(tables.mapping(second).is_none());
    tables.release(&mut memory).unwrap();
    assert_eq!(memory.available_pages(), initial);

    // Leave too few frames to build the page-table chain after data allocation.
    let mut tables = T::new(&mut memory, 7).unwrap();
    let reserved = memory.allocate((memory.available_pages() - 4) as u32, 9).unwrap();
    assert!(mappings.map(&mut tables, &mut memory, 12288, true).is_err());
    assert_eq!(mappings.used_pages(), 0);
    assert!(tables.mapping(first).is_none());
    tables.release(&mut memory).unwrap();
    memory.release(reserved).unwrap();
    assert_eq!(memory.available_pages(), initial);
    unsafe { std::alloc::dealloc(arena, layout); }
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
