#![allow(dead_code)]

#[path = "../kernel/storage/layout.rs"]
mod layout;

const BLOCKS_1_GIB: u64 = 1024 * 1024 * 1024 / 512;
const REPROVISION_DISK_BLOCKS: u64 = 16 * 1024 * 1024 * 1024 / 512;

// ------------------------=
// FUNC: image_blocks
// DESC: Reads an installer payload artifact size as its required sector count.
// ------------------=
fn image_blocks(path: &str) -> u64 {
    let bytes = std::fs::metadata(path)
        .expect("installer payload artifact")
        .len();
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
    // The loader's largest supported kernel must fit without touching the store.
    let maximum_kernel_blocks = layout::MAX_KERNEL_BLOCKS;
    assert_eq!(layout::store_relative_lba(256 * 2048), Ok(layout::STORE_RELATIVE_LBA));
    assert_eq!(layout::store_relative_lba(256 * 2048 + 1), Ok(layout::LARGE_STORE_RELATIVE_LBA));
    assert_eq!(layout::store_relative_lba(512 * 2048), Ok(layout::LARGE_STORE_RELATIVE_LBA));
    assert_eq!(layout::store_relative_lba(512 * 2048 + 1), Ok(layout::BROWSER_STORE_RELATIVE_LBA));
    let maximum = layout::plan_entire_disk(
        REPROVISION_DISK_BLOCKS,
        arm_esp_blocks,
        maximum_kernel_blocks,
    )
    .unwrap();
    assert_eq!(
        maximum.kernel_lba + maximum_kernel_blocks,
        maximum.container_first + layout::BROWSER_STORE_RELATIVE_LBA
    );
    assert_eq!(
        layout::plan_entire_disk(
            REPROVISION_DISK_BLOCKS,
            arm_esp_blocks,
            maximum_kernel_blocks + 1,
        ),
        Err(layout::LayoutError::InsufficientCapacity)
    );
    // A small historical payload still fits 1 GiB; production browser kernels
    // reserve more than that before object storage even begins.
    assert!(layout::plan_entire_disk(BLOCKS_1_GIB, 64 * 2048, 128 * 2048).is_ok());
    let plan = layout::plan_entire_disk(REPROVISION_DISK_BLOCKS, x86_esp_blocks, x86_kernel_blocks)
        .expect("the reprovisioned x86 VM disk must fit its real payload");
    let arm_plan =
        layout::plan_entire_disk(REPROVISION_DISK_BLOCKS, arm_esp_blocks, arm_kernel_blocks)
            .unwrap_or_else(|_| {
                panic!("the reprovisioned ARM64 VM disk must fit its real payload")
            });

    assert!(plan.esp_last < plan.container_first);
    assert!(plan.kernel_lba >= plan.container_first);
    assert!(
        plan.container_first + layout::store_relative_lba(x86_kernel_blocks).unwrap() < plan.container_last,
        "native object store must retain capacity after the kernel reservation"
    );
    assert!(arm_plan.esp_last < arm_plan.container_first);
    assert!(
        arm_plan.container_first + layout::store_relative_lba(arm_kernel_blocks).unwrap() < arm_plan.container_last,
        "ARM64 object storage must begin after the enlarged kernel reservation"
    );
    for (geometry, esp, kernel) in [(plan, x86_esp_blocks, x86_kernel_blocks),
                                  (arm_plan, arm_esp_blocks, arm_kernel_blocks)] {
        let minimum = geometry.container_first + layout::store_relative_lba(kernel).unwrap() + 35;
        assert!(layout::plan_entire_disk(minimum, esp, kernel).is_ok());
        assert_eq!(layout::plan_entire_disk(minimum - 1, esp, kernel),
                   Err(layout::LayoutError::InsufficientCapacity));
        assert!(geometry.kernel_lba + kernel <= geometry.container_first + layout::store_relative_lba(kernel).unwrap());
    }
    assert!(matches!(
        layout::plan_entire_disk(64 * 1024 * 1024 / 512, x86_esp_blocks, x86_kernel_blocks,),
        Err(layout::LayoutError::InsufficientCapacity)
    ));
    assert!(matches!(
        layout::plan_entire_disk(
            REPROVISION_DISK_BLOCKS,
            arm_esp_blocks,
            layout::MAX_KERNEL_BLOCKS + 1,
        ),
        Err(layout::LayoutError::InsufficientCapacity)
    ));
}
