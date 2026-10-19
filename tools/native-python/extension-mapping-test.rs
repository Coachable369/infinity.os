#[path = "../../kernel/runtime/native_c_image.rs"] mod native_c_image;
#[path = "../../kernel/runtime/native_memory.rs"] mod native_memory;
#[path = "../../kernel/runtime/native_memory_arm.rs"] mod native_memory_arm;
#[path = "../../kernel/runtime/native_job_files.rs"] mod native_job_files;
#[path = "../../kernel/runtime/native_extensions.rs"] mod native_extensions;
#[path = "../../kernel/runtime/native_extension_image.rs"] mod elf;
use native_memory::*;

// ------------------------=
// FUNC: verify
// DESC: Loads actual ELF bytes through production mapping code and verifies relocations, W^X, handle ownership and cleanup.
// ------------------=
fn verify<T: PageTable>(path: &str, machine: u16) {
    let bytes = std::fs::read(path).unwrap();
    let image = elf::Image::parse(&bytes, machine).unwrap();
    let layout = std::alloc::Layout::from_size_align(8 * 1024 * 1024, 4096).unwrap();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!arena.is_null());
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let initial = memory.available_pages();
    let mut tables = T::new(&mut memory, 7).unwrap();
    let mut libraries = native_extensions::Libraries::new(7);
    let files = [native_job_files::File { path: b"/library.so", bytes: &bytes }];
    let mut cleaned = 0;
    let handle = libraries.open(&mut memory, &mut tables, &[], &files, machine, 0, 2,
        |_, size| { assert!(size > 0); cleaned += size; }).unwrap();
    assert!(cleaned > 0);
    let free = memory.available_pages();
    assert_eq!(libraries.open(&mut memory, &mut tables, &[], &files, machine, 0, 2, |_, _| panic!()).unwrap(), handle);
    assert_eq!(memory.available_pages(), free);
    assert!(libraries.symbol(&files, machine, handle + 1, b"infinity_extension_value").is_err());
    assert!(libraries.symbol(&files, machine, handle, b"not_present").is_err());
    let address = libraries.symbol(&files, machine, handle, b"infinity_extension_value").unwrap();
    assert_eq!(tables.mapping(address).unwrap().permissions, PagePermissions::USER_RX);
    assert!(copy_to_user(&mut tables, address, &[0]).is_err());
    let relative = image.lookup(b"infinity_extension_value").unwrap().unwrap().value;
    let base = address - relative;
    let relro = image.relro.expect("fixture must contain a linker-authored RELRO interval");
    let readonly_address = libraries.symbol(&files, machine, handle, b"infinity_extension_readonly_address").unwrap();
    assert!(readonly_address >= base + relro.address as u64
        && readonly_address + 8 <= base + (relro.address + relro.length) as u64);
    for page in ((relro.address & !4095)..((relro.address + relro.length + 4095) & !4095)).step_by(4096) {
        assert_eq!(tables.mapping(base + page as u64).unwrap().permissions, PagePermissions::USER_R);
        assert!(copy_to_user(&mut tables, base + page as u64, &[0]).is_err());
    }
    let mut relocations = 0;
    let mut readonly_relocations = 0;
    let mut writable_relocations = 0;
    image.relocations(|r| {
        assert!(r.relative);
        let mut value = [0; 8];
        copy_from_user(&tables, base + r.address as u64, &mut value).unwrap();
        assert_eq!(u64::from_le_bytes(value), (base as i128 + r.addend as i128) as u64);
        let permissions = tables.mapping(base + r.address as u64).unwrap().permissions;
        assert!(!permissions.executable);
        let is_relro = r.address >= relro.address && r.address + 8 <= relro.address + relro.length;
        if is_relro {
            assert_eq!(permissions, PagePermissions::USER_R);
            assert!(copy_to_user(&mut tables, base + r.address as u64, &value).is_err());
            readonly_relocations += 1;
        } else {
            assert_eq!(permissions, PagePermissions::USER_RW);
            copy_to_user(&mut tables, base + r.address as u64, &value).unwrap();
            writable_relocations += 1;
        }
        relocations += 1;
        Ok(())
    }).unwrap();
    assert!(relocations > 0 && readonly_relocations > 0 && writable_relocations > 0);
    libraries.release_all(&mut tables, &mut memory).unwrap();
    assert!(tables.mapping(address).is_none());
    assert!(libraries.symbol(&files, machine, handle, b"infinity_extension_value").is_err());
    tables.release(&mut memory).unwrap();
    assert_eq!(memory.available_pages(), initial);
    unsafe { std::alloc::dealloc(arena, layout); }
}

// ------------------------=
// FUNC: main
// DESC: Runs real extension mapping behavior against both native page-table encodings.
// ------------------=
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    verify::<native_memory_arm::ArmAddressSpace>(&args[1], 183);
    verify::<X86AddressSpace>(&args[2], 62);
}
