pub const STORE_RELATIVE_LBA: u64 = 98_304;
const MINIMUM_BLOCKS: u64 = 262_144;
const ESP_FIRST: u64 = 2_048;
const ALIGNMENT_BLOCKS: u64 = 2_048;
const KERNEL_RELATIVE_LBA: u64 = 2_048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    InsufficientCapacity,
    Arithmetic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntireDiskLayout {
    pub esp_last: u64,
    pub container_first: u64,
    pub container_last: u64,
    pub kernel_lba: u64,
}

// ------------------------=
// FUNC: plan_entire_disk
// DESC: Calculates install geometry and rejects disks or kernel images that cannot fit safely.
// ------------------=
pub fn plan_entire_disk(
    device_blocks: u64,
    esp_blocks: u64,
    kernel_blocks: u64,
) -> Result<EntireDiskLayout, LayoutError> {
    if device_blocks < MINIMUM_BLOCKS {
        return Err(LayoutError::InsufficientCapacity);
    }
    let esp_last = ESP_FIRST
        .checked_add(esp_blocks)
        .and_then(|value| value.checked_sub(1))
        .ok_or(LayoutError::Arithmetic)?;
    let container_first = align_up(
        esp_last.checked_add(1).ok_or(LayoutError::Arithmetic)?,
        ALIGNMENT_BLOCKS,
    )?;
    let container_last = device_blocks
        .checked_sub(34)
        .ok_or(LayoutError::Arithmetic)?;
    let kernel_lba = container_first
        .checked_add(KERNEL_RELATIVE_LBA)
        .ok_or(LayoutError::Arithmetic)?;
    let kernel_end = KERNEL_RELATIVE_LBA
        .checked_add(kernel_blocks)
        .ok_or(LayoutError::Arithmetic)?;
    let store_lba = container_first
        .checked_add(STORE_RELATIVE_LBA)
        .ok_or(LayoutError::Arithmetic)?;
    if kernel_end > STORE_RELATIVE_LBA
        || kernel_lba
            .checked_add(kernel_blocks)
            .ok_or(LayoutError::Arithmetic)?
            > container_last
        || store_lba >= container_last
    {
        return Err(LayoutError::InsufficientCapacity);
    }
    Ok(EntireDiskLayout {
        esp_last,
        container_first,
        container_last,
        kernel_lba,
    })
}

// ------------------------=
// FUNC: align_up
// DESC: Aligns one block address upward without permitting arithmetic overflow.
// ------------------=
fn align_up(value: u64, alignment: u64) -> Result<u64, LayoutError> {
    value
        .checked_add(alignment - 1)
        .map(|aligned| aligned / alignment * alignment)
        .ok_or(LayoutError::Arithmetic)
}
