#![allow(dead_code)]

#[path = "../kernel/storage/layout.rs"]
mod layout;

const BLOCKS_512_MIB: u64 = 512 * 1024 * 1024 / 512;
const REPROVISION_DISK_BLOCKS: u64 = 16 * 1024 * 1024 * 1024 / 512;

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
    let x86_esp_blocks = image_blocks("build/x86_64/installed-esp.img");
    let arm_esp_blocks = image_blocks("build/aarch64/installed-esp.img");
    let x86_kernel_blocks = image_blocks("build/x86_64/installed-kernel.elf");
    let arm_kernel_blocks = image_blocks("build/aarch64/installed-kernel.elf");
    let plan = layout::plan_entire_disk(BLOCKS_512_MIB, x86_esp_blocks, x86_kernel_blocks)
        .unwrap_or_else(|_| panic!("blank 512 MiB virtual disk must be installable"));
    let arm_plan = layout::plan_entire_disk(
        REPROVISION_DISK_BLOCKS,
        arm_esp_blocks,
        arm_kernel_blocks,
    )
    .unwrap_or_else(|_| panic!("the reprovisioned ARM64 VM disk must fit its real payload"));

    assert!(plan.esp_last < plan.container_first);
    assert!(plan.kernel_lba >= plan.container_first);
    assert!(
        plan.container_first + layout::STORE_RELATIVE_LBA < plan.container_last,
        "native object store must retain capacity after the kernel reservation"
    );
    assert!(arm_plan.esp_last < arm_plan.container_first);
    assert!(
        arm_plan.container_first + layout::STORE_RELATIVE_LBA < arm_plan.container_last,
        "ARM64 object storage must begin after the enlarged kernel reservation"
    );
    assert!(matches!(
        layout::plan_entire_disk(
            64 * 1024 * 1024 / 512,
            x86_esp_blocks,
            x86_kernel_blocks,
        ),
        Err(layout::LayoutError::InsufficientCapacity)
    ));
    assert!(matches!(
        layout::plan_entire_disk(
            REPROVISION_DISK_BLOCKS,
            arm_esp_blocks,
            layout::STORE_RELATIVE_LBA,
        ),
        Err(layout::LayoutError::InsufficientCapacity)
    ));
}
