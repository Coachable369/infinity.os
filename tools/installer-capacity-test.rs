#![allow(dead_code)]

#[path = "../kernel/storage/layout.rs"]
mod layout;

const BLOCKS_512_MIB: u64 = 512 * 1024 * 1024 / 512;

// ------------------------=
// FUNC: image_blocks
// DESC: Reads an installer payload artifact size as its required sector count.
// ------------------=
fn image_blocks(path: &str) -> u64 {
    let bytes = std::fs::metadata(path).expect("installer payload artifact").len();
    bytes.checked_add(511).expect("artifact size overflow") / 512
}

// ------------------------=
// FUNC: main
// DESC: Verifies production install geometry accepts a blank VM disk and bounds all payloads.
// ------------------=
fn main() {
    let esp_blocks = image_blocks("build/x86_64/installed-esp.img");
    let kernel_blocks = image_blocks("build/x86_64/installed-kernel.elf");
    let plan = layout::plan_entire_disk(BLOCKS_512_MIB, esp_blocks, kernel_blocks)
        .unwrap_or_else(|_| panic!("blank 512 MiB virtual disk must be installable"));

    assert!(plan.esp_last < plan.container_first);
    assert!(plan.kernel_lba >= plan.container_first);
    assert!(
        plan.container_first + layout::STORE_RELATIVE_LBA < plan.container_last,
        "native object store must retain capacity after the kernel reservation"
    );
    assert!(matches!(
        layout::plan_entire_disk(64 * 1024 * 1024 / 512, esp_blocks, kernel_blocks),
        Err(layout::LayoutError::InsufficientCapacity)
    ));
}
