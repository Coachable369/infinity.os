//! Native package ELF admission and stable loader-grant behavior, not installed execution.
#![allow(dead_code)]
#[path = "../../kernel/runtime/native_c_image.rs"] mod native_c_image;
#[path = "../../kernel/runtime/native_memory.rs"] mod native_memory;
#[path = "../../kernel/runtime/native_job_files.rs"] mod native_job_files;
#[path = "../../kernel/runtime/native_extensions.rs"] mod native_extensions;
#[path = "../../kernel/runtime/native_extension_image.rs"] mod image;
#[path = "../../kernel/runtime/native_python_packages.rs"] mod native_python_packages;
#[path = "../../kernel/core/console_python_extensions.rs"] mod snapshots;

static mut FILESYSTEM: native_python_packages::Vfs = native_python_packages::Vfs::new();
static mut SNAPSHOTS: snapshots::Snapshots = snapshots::Snapshots::new();

// ------------------------=
// FUNC: verify
// DESC: Validates actual architecture-specific CPython imports and proves later mutable package changes cannot alter an admitted extension's bytes.
// ------------------=
fn verify(main: &str, library: &str, machine: u16) {
    let executable = std::fs::read(main).unwrap();
    let binary = std::fs::read(library).unwrap();
    assert!(native_extensions::validate_python_extension(&executable, &binary, machine).is_ok());
    assert!(native_extensions::validate_python_extension(&executable, &binary, if machine == 183 { 62 } else { 183 }).is_err());
    assert!(native_extensions::validate_python_extension(&[], &binary, machine).is_err());
    let parsed = image::Image::parse(&binary, machine).unwrap();
    let mut imported = None;
    parsed.relocations(|relocation| {
        if !relocation.relative {
            let symbol = parsed.symbol(relocation.symbol)?;
            if !symbol.defined && !symbol.weak && imported.is_none() { imported = Some(symbol.name.to_vec()); }
        }
        Ok(())
    }).unwrap();
    let imported = imported.unwrap();
    let mut invalid = binary.clone();
    for at in 0..=binary.len() - imported.len() {
        if &binary[at..at + imported.len()] == imported.as_slice() { invalid[at] = b'!'; }
    }
    assert!(native_extensions::validate_python_extension(&executable, &invalid, machine).is_err());
    let filesystem = unsafe { &mut *(&raw mut FILESYSTEM) };
    let snapshots = unsafe { &mut *(&raw mut SNAPSHOTS) };
    filesystem.reset(true);
    filesystem.put(b"/packages/infinity_native_test.so", &binary).unwrap();
    snapshots.capture(filesystem, &executable, machine).unwrap();
    assert_eq!(snapshots.count(), 1);
    assert_eq!(snapshots.file(0).unwrap().path, b"/packages/infinity_native_test.so");
    filesystem.remove(b"/packages/infinity_native_test.so", false).unwrap();
    filesystem.put(b"/packages/other.py", b"answer = 42").unwrap();
    assert_eq!(snapshots.file(0).unwrap().bytes, binary);
    filesystem.put(b"/packages/broken.so", &invalid).unwrap();
    assert!(!snapshots::Snapshots::validate(filesystem, &executable, machine));
    assert!(snapshots.capture(filesystem, &executable, machine).is_err());
    assert_eq!(snapshots.count(), 0);
    assert!(snapshots.file(0).is_none());
    filesystem.reset(true);
    for index in 0..=snapshots::COUNT {
        filesystem.put(format!("/packages/module{index}.so").as_bytes(), &binary).unwrap();
    }
    assert!(!snapshots::Snapshots::validate(filesystem, &executable, machine));
}

// ------------------------=
// FUNC: main
// DESC: Checks native wheel admission and immutable snapshots against both real built architecture payloads.
// ------------------=
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 5);
    verify(&args[1], &args[2], 183);
    verify(&args[3], &args[4], 62);
}
