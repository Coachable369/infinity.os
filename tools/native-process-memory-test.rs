#[path = "../kernel/runtime/native_c_image.rs"] mod native_c_image;
#[path = "../kernel/runtime/native_memory.rs"] mod native_memory;
#[path = "../kernel/runtime/native_memory_arm.rs"] mod native_memory_arm;
#[path = "../kernel/runtime/native_mappings.rs"] mod native_mappings;
#[path = "../kernel/runtime/native_stdio.rs"] mod native_stdio;
#[path = "../kernel/runtime/native_compiler_launch.rs"] mod native_compiler_launch;
#[path = "../kernel/runtime/native_job_files.rs"] mod native_job_files;
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
    verify_job_files::<X86AddressSpace>();
    verify_job_files::<ArmAddressSpace>();
    verify_preparation::<X86AddressSpace>();
    verify_preparation::<ArmAddressSpace>();
}

// ------------------------=
// FUNC: verify_preparation
// DESC: Exercises bounded loading, dirty-frame clearing, exact data, W-X, RELRO, guards and cancellation reclamation on both table formats.
// ------------------=
fn verify_preparation<T: PageTable>() {
    use native_c_image::{Image, Segment, Region};
    let allocation_layout = std::alloc::Layout::new::<Arena>();
    let arena = unsafe { std::alloc::alloc(allocation_layout) };
    assert!(!arena.is_null());
    unsafe { core::ptr::write_bytes(arena, 0xa5, allocation_layout.size()); }
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, allocation_layout.size() as u64).unwrap();
    let initial = memory.available_pages();
    let mut segments = [Segment::default(); 8];
    segments[0] = Segment { source: 0, address: 0, file_size: 70000, memory_size: 131072, flags: 5 };
    segments[1] = Segment { source: 70000, address: 131072, file_size: 4097, memory_size: 65536, flags: 6 };
    let image = Image { entry: 0, virtual_base: 0x100000000, memory_size: 196608, segments,
        segment_count: 2, tls: None, relro: Some(Region { address: 131072, length: 4096 }), relative_relocations: None };
    let layout = CompilerVirtualLayout { image_base: 0x100000000, heap_base: 0x200000000,
        heap_pages: 32, stack_guard_base: 0x300000000, stack_pages: 16 };
    let source = vec![0x7b; 74097];
    for cancel_after in [0, 1, 4, 8, 12] {
        let mut job = ProcessPreparation::<T>::new(&mut memory, 1, image, layout).unwrap();
        for _ in 0..cancel_after { if job.poll(&mut memory, &source).unwrap() { break; } }
        job.release(&mut memory).unwrap();
        assert_eq!(memory.available_pages(), initial);
    }
    let mut job = ProcessPreparation::<T>::new(&mut memory, 1, image, layout).unwrap();
    job = match job.finish() { Err(job) => job, Ok(_) => panic!("Uninitialized space escaped") };
    let mut visits = 0;
    loop {
        let before = unsafe { core::slice::from_raw_parts(arena, allocation_layout.size()) }.to_vec();
        let ready = job.poll(&mut memory, &source).unwrap();
        let after = unsafe { core::slice::from_raw_parts(arena, allocation_layout.size()) };
        assert!(before.iter().zip(after).filter(|(a,b)| a != b).count() <= ProcessPreparation::<T>::CHUNK);
        visits += 1;
        assert!(visits < 100);
        if ready { break; }
    }
    assert!(visits > 10);
    let space = match job.finish() { Ok(space) => space, Err(_) => panic!("Ready space unavailable") };
    let bytes = unsafe { core::slice::from_raw_parts(space.image.physical_address as *const u8, image.memory_size) };
    assert!(bytes[..70000].iter().all(|b| *b == 0x7b));
    assert!(bytes[70000..131072].iter().all(|b| *b == 0));
    assert!(bytes[131072..135169].iter().all(|b| *b == 0x7b));
    assert!(bytes[135169..].iter().all(|b| *b == 0));
    for allocation in [space.heap, space.stack] {
        assert!(unsafe { core::slice::from_raw_parts(allocation.physical_address as *const u8,
            allocation.page_count as usize * 4096) }.iter().all(|b| *b == 0));
    }
    assert_eq!(space.page_tables.mapping(layout.image_base).unwrap().permissions, PagePermissions::USER_RX);
    assert_eq!(space.page_tables.mapping(layout.image_base+131072).unwrap().permissions, PagePermissions::USER_R);
    assert_eq!(space.page_tables.mapping(layout.image_base+135168).unwrap().permissions, PagePermissions::USER_RW);
    assert!(space.page_tables.mapping(layout.stack_guard_base).is_none());
    assert!(space.page_tables.mapping(space.stack_top).is_none());
    space.release(&mut memory).unwrap();
    assert_eq!(memory.available_pages(), initial);
    let mut job = ProcessPreparation::<T>::new(&mut memory, 1, image, layout).unwrap();
    assert!(job.poll(&mut memory, &source[..10]).is_err());
    job.release(&mut memory).unwrap();
    assert_eq!(memory.available_pages(), initial);
    unsafe { std::alloc::dealloc(arena, allocation_layout); }
}

// ------------------------=
// FUNC: verify_job_files
// DESC: Exercises exact grants, separate handle positions, bounded binary reads, invalid pointers and closed handles on both CPUs.
// ------------------=
fn verify_job_files<T: PageTable>() {
    use native_job_files::{File, Files};
    let layout = std::alloc::Layout::new::<Arena>();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!arena.is_null());
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let initial = memory.available_pages();
    let data = memory.allocate(2, 1).unwrap();
    let mut tables = T::new(&mut memory, 1).unwrap();
    let user = 0x1_0000_0000;
    tables.map_range(&mut memory, user, data.physical_address, 8192, PagePermissions::USER_RW).unwrap();
    let content: Vec<u8> = (0..5000).map(|n| (n % 251) as u8).collect();
    let files = [File { path: b"/input.o", bytes: &content }];
    let mut io = Files::new();
    copy_to_user(&mut tables, user, b"/input.o\0").unwrap();
    assert_eq!(io.syscall(&mut tables, &files, 0x120b, [user,2,user+128,0,0,0]), -13);
    assert_eq!(io.syscall(&mut tables, &files, 0x120b, [user,1,user+8192,0,0,0]), -14);
    assert_eq!(io.syscall(&mut tables, &files, 0x120b, [user,1,user+128,0,0,0]), 0);
    let mut value = [0;8];
    copy_from_user(&tables,user+128,&mut value).unwrap();
    assert_eq!(u64::from_le_bytes(value),3);
    assert_eq!(io.syscall(&mut tables,&files,0x120c,[3,user+256,5000,user+8192,0,0]),-14);
    assert_eq!(io.syscall(&mut tables,&files,0x120c,[3,user+256,5000,user+128,0,0]),0);
    copy_from_user(&tables,user+128,&mut value).unwrap();
    assert_eq!(u64::from_le_bytes(value),4096);
    let mut actual = [0;4096];
    copy_from_user(&tables,user+256,&mut actual).unwrap();
    assert_eq!(&actual, &content[..4096]);
    assert_eq!(io.syscall(&mut tables,&files,0x1210,[3,0,user+256,16,user+128,0]),0);
    assert_eq!(io.syscall(&mut tables,&files,0x120c,[3,user+256,4096,user+128,0,0]),0);
    copy_from_user(&tables,user+128,&mut value).unwrap();
    assert_eq!(u64::from_le_bytes(value),904);
    assert_eq!(io.syscall(&mut tables,&files,0x120e,[3,(-1i64) as u64,0,user+128,0,0]),-22);
    assert_eq!(io.syscall(&mut tables,&files,0x1211,[3,user+256,0,0,0,0]),0);
    copy_from_user(&tables,user+264,&mut value).unwrap();
    assert_eq!(u64::from_le_bytes(value),5000);
    assert_eq!(io.syscall(&mut tables,&files,0x120f,[3,0,0,0,0,0]),0);
    assert_eq!(io.syscall(&mut tables,&files,0x120c,[3,user+256,16,user+128,0,0]),-9);
    copy_to_user(&mut tables,user,b"/secret\0").unwrap();
    assert_eq!(io.syscall(&mut tables,&files,0x120b,[user,1,user+128,0,0,0]),-2);
    let headers=[File{path:b"/sdk/stdio.h",bytes:b"header"}];
    copy_to_user(&mut tables,user,b"/sdk\0").unwrap();
    assert_eq!(io.syscall(&mut tables,&headers,0x1212,[user,user+256,0,0,0,0]),0);
    let mut kind=[0;4];
    copy_from_user(&tables,user+284,&mut kind).unwrap();
    assert_eq!(u32::from_le_bytes(kind),2);
    copy_to_user(&mut tables,user,b"/sd\0").unwrap();
    assert_eq!(io.syscall(&mut tables,&headers,0x1212,[user,user+256,0,0,0,0]),-2);
    copy_to_user(&mut tables,user,b"\0").unwrap();
    assert_eq!(io.syscall(&mut tables,&headers,0x1212,[user,user+256,0,0,0,0]),-2);
    tables.release(&mut memory).unwrap();
    memory.release(data).unwrap();
    assert_eq!(memory.available_pages(),initial);
    unsafe {std::alloc::dealloc(arena,layout);}
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
    let saved = io.checkpoint();
    drop(io);
    let mut io = native_stdio::Streams::resume(&source,&mut output,&mut errors,saved).unwrap();
    assert_eq!(io.output(),binary);
    assert_eq!(io.diagnostics(),(&binary[..3],true));
    assert_eq!(io.syscall(&mut tables,0x121b,[0,user,10,user+7000,0,0]),0);
    copy_from_user(&tables,user+7000,&mut count).unwrap();
    assert_eq!(u64::from_le_bytes(count),0);
    drop(io);
    assert!(native_stdio::Streams::resume(&source,&mut output[..1],&mut errors,saved).is_none());
    // Stderr drain is explicit: the preceding unchanged compiler path retained
    // its prefix/truncation behavior. Drain must not move stdin or clear stdout.
    let mut drained = saved;
    drained.drain_errors();
    let mut io = native_stdio::Streams::resume(&source, &mut output, &mut errors, drained).unwrap();
    assert_eq!(io.diagnostics(), (&[][..], false));
    assert_eq!(io.output(), binary);
    assert_eq!(io.syscall(&mut tables, 0x121c, [2,user+4092,2,user+8192,0,0]), -14);
    assert_eq!(io.diagnostics(), (&[][..], false));
    assert_eq!(io.syscall(&mut tables, 0x121c, [2,user+4092,2,user+7000,0,0]), 0);
    assert_eq!(io.diagnostics(), (&binary[..2], false));
    drained = io.checkpoint();
    drop(io);
    let mut rolling_errors = [0; 8192];
    // More than one full lifetime buffer of genuine checked writes, followed
    // by a distinct final packet. Every consumed chunk is retained by caller.
    for packet in [&[0x11;4096][..], &[0x22;4096][..], &binary[..]] {
        drained.drain_errors();
        let mut io = native_stdio::Streams::resume(&source, &mut output, &mut rolling_errors, drained).unwrap();
        copy_to_user(&mut tables, user, packet).unwrap();
        assert_eq!(io.syscall(&mut tables, 0x121c, [2,user,packet.len() as u64,user+7000,0,0]), 0);
        assert_eq!(io.diagnostics(), (packet, false));
        assert_eq!(io.output(), binary);
        assert_eq!(io.syscall(&mut tables, 0x121b, [0,user,10,user+7000,0,0]), 0);
        copy_from_user(&tables, user+7000, &mut count).unwrap();
        assert_eq!(u64::from_le_bytes(count), 0);
        drained = io.checkpoint();
    }
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
    assert_eq!(mappings.sync(&tables,first,8192),Ok(()));
    assert_eq!(mappings.sync(&tables,first+4096,4096),Ok(()));
    assert_eq!(mappings.sync(&tables,first+1,4096),Err(MemoryError::AccessDenied));
    assert_eq!(mappings.sync(&tables,first,8193),Err(MemoryError::UnknownAllocation));
    assert_eq!(stranger.sync(&tables,first,4096),Err(MemoryError::UnknownAllocation));
    assert_eq!(mappings.sync(&tables,u64::MAX-4095,8192),Err(MemoryError::AddressOverflow));
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
    assert_eq!(mappings.sync(&tables,first,8192),Err(MemoryError::UnknownAllocation));
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
