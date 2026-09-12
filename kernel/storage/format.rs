use super::component_manifest;
use super::{
    BlockDevice, CurrentLayout, DateTimeConfiguration, DestructiveConsequence, PoolPlan, SpacePlan,
    StorageDevice, StorageError, StorageProfile, StorageProvisioningPlan, StorageStrategy,
};

use super::payload::Image;
#[cfg(all(target_arch = "x86_64",not(feature="streamed-payload")))]
const ESP_IMAGE: Image = Image::embedded(include_bytes!("../../build/x86_64/installed-esp.img"));
#[cfg(all(target_arch = "x86_64",not(feature="streamed-payload")))]
const KERNEL_IMAGE: Image = Image::embedded(include_bytes!("../../build/x86_64/installed-kernel.elf"));
#[cfg(all(target_arch = "aarch64",not(feature="streamed-payload")))]
const ESP_IMAGE: Image = Image::embedded(include_bytes!("../../build/aarch64/installed-esp.img"));
#[cfg(all(target_arch = "aarch64",not(feature="streamed-payload")))]
const KERNEL_IMAGE: Image = Image::embedded(include_bytes!("../../build/aarch64/installed-kernel.elf"));
#[cfg(all(target_arch="aarch64",feature="streamed-payload"))]
include!("../../build/qwen/payload-manifest.rs");
#[cfg(all(not(target_arch="aarch64"),feature="streamed-payload"))]
compile_error!("streamed-payload currently requires the ARM64 firmware storage bridge");
const ESP_FIRST: u64 = 2048;
const ESP_BLOCKS: u64 = (ESP_IMAGE.len() / 512) as u64;
const ALIGNMENT_BLOCKS: u64 = 2048;
const KERNEL_RELATIVE_LBA: u64 = 2048;
const BOOT_CATALOG_RELATIVE_LBA: u64 = 3;
const SYSTEM_MANIFEST_RELATIVE_LBA: u64 = 4;
const COMPONENT_MANIFEST_RELATIVE_LBA: u64 = 5;
const GENERATION_ID: u64 = 1;
const GENERATION_INSTALLING: u32 = 1;
const GENERATION_READY: u32 = 2;
const GENERATION_ACTIVE: u32 = 3;
const INSTALL_CLASS_CORE: u32 = 1;
const INSTALL_CLASS_SYSTEM_OPTIONAL: u32 = 2;
const INSTALL_CLASS_POST_INSTALL: u32 = 3;

const MINIMUM_BLOCKS: u64 = 262_144;
const INFINITY_TYPE: [u8; 16] = [
    0x69, 0x66, 0x6e, 0x49, 0x69, 0x6e, 0x79, 0x74, 0x53, 0x54, 0x4f, 0x52, 0x41, 0x47, 0x45, 0x31,
];

// ------------------------=
// FUNC: plan_entire_disk
// DESC: Implements the plan entire disk operation.
// ------------------=
pub fn plan_entire_disk(
    device: StorageDevice,
    profile: StorageProfile,
    date_time: DateTimeConfiguration,
) -> Result<StorageProvisioningPlan, StorageError> {
    if device.logical_block_size != 512 || device.blocks < MINIMUM_BLOCKS || !date_time.is_valid() {
        return Err(StorageError::InsufficientCapacity);
    }
    let kernel_blocks = ((KERNEL_IMAGE.len() as u64)
        .checked_add(511)
        .ok_or(StorageError::Arithmetic)?)
        / 512;
    let layout = super::layout::plan_entire_disk(device.blocks, ESP_BLOCKS, kernel_blocks)
        .map_err(|error| match error {
            super::layout::LayoutError::InsufficientCapacity => StorageError::InsufficientCapacity,
            super::layout::LayoutError::Arithmetic => StorageError::Arithmetic,
        })?;
    let identities = super::install_identity::for_target(device.identity)
        .ok_or(StorageError::InstallationEntropyUnavailable)?;
    let mut spaces = profile_spaces(profile);
    for (space, id) in spaces.iter_mut().zip(identities.spaces) { space.uuid = id; }
    Ok(StorageProvisioningPlan {
        target: device,
        current_layout: if device.has_gpt {
            CurrentLayout::Gpt
        } else {
            CurrentLayout::Empty
        },
        strategy: StorageStrategy::EntireDisk,
        profile,
        date_time,
        consequence: DestructiveConsequence::EraseEntireDevice,
        destructive: true,
        requires_efi_region: true,
        alignment_blocks: ALIGNMENT_BLOCKS,
        container_format_version: 1,
        disk_uuid: identities.disk,
        esp_uuid: identities.esp,
        container_uuid: identities.container,
        pool: PoolPlan {
            uuid: identities.pool,
            member_count: 1,
            total_blocks: layout.container_last - layout.container_first + 1,
        },
        spaces,
        esp_first_lba: ESP_FIRST,
        esp_last_lba: layout.esp_last,
        container_first_lba: layout.container_first,
        container_last_lba: layout.container_last,
        kernel_lba: layout.kernel_lba,
        expected_pool_blocks: layout.container_last - layout.container_first + 1,
    })
}

// ------------------------=
// FUNC: provision
// DESC: Implements the provision operation.
// ------------------=
pub fn provision<D: BlockDevice, F: FnMut(u8, &[u8])>(
    device: &mut D,
    plan: &StorageProvisioningPlan,
    progress: &mut F,
) -> Result<(), StorageError> {
    validate_plan(device, plan)?;
    progress(3, b"INSTALLATION PLAN VALIDATED");
    crate::output_text(b"[provision] state=provisioning\n");
    write_gpt(device, plan).map_err(|_| StorageError::WriteGpt)?;
    progress(12, b"PARTITION TABLE WRITTEN");
    crate::output_text(b"[write] Partition table\n");
    progress(12, b"INSTALLING EFI BOOTLOADER AND BOOT ASSETS");
    write_esp(device, plan, progress).map_err(|_| StorageError::WriteBootRegion)?;
    progress(24, b"EFI BOOT ENVIRONMENT INSTALLED");
    crate::output_text(b"[write] EFI boot environment\n");
    write_native_metadata(device, plan, 1).map_err(|_| StorageError::WriteContainer)?;
    progress(35, b"INFINITY POOL CREATED");
    crate::output_text(b"[write] Infinity container, pool, and Spaces\n");
    write_system_generation(device, plan, GENERATION_INSTALLING, false)
        .map_err(|_| StorageError::WriteSystemGeneration)?;
    progress(43, b"SYSTEM GENERATION 1 CREATED");
    crate::output_text(b"[generation] Generation 1 state=INSTALLING\n");
    crate::output_text(b"[write] System component manifest (CORE)\n");
    progress(43, b"INSTALLING KERNEL, DRIVERS AND CORE COMPONENTS");
    write_kernel(device, plan, progress).map_err(|_| StorageError::WriteKernel)?;
    progress(56, b"KERNEL AND CORE COMPONENTS WRITTEN");
    let mut object_store = super::object::ObjectStore::format_with_progress(
        &mut *device,
        plan.container_first_lba,
        plan.expected_pool_blocks,
        plan.container_uuid,
        progress,
    )
    .map_err(|_| StorageError::WriteContainer)?;
    object_store
        .install_date_time_configuration(plan.date_time)
        .map_err(|_| StorageError::WriteContainer)?;
    crate::output_text(b"[write] Date and time configuration\n");
    if !object_store.runtime_bootstrap_valid() {
        return Err(StorageError::VerifyContainer);
    }
    progress(67, b"RUNTIME SERVICES INSTALLED");
    crate::output_text(b"[write] Infinity Object Store generation 1\n");
    crate::output_text(
        b"[write] Runtime, service registry, capability policy, native AI, and InfinityUI objects\n",
    );
    drop(object_store);
    write_container_header(device, plan, 2).map_err(|_| StorageError::WriteContainer)?;
    crate::output_text(b"[provision] state=installing\n[write] InfinityOS kernel\n");
    device.flush();
    write_container_header(device, plan, 3).map_err(|_| StorageError::WriteContainer)?;
    crate::output_text(b"[provision] state=verifying\n");
    progress(67, b"VERIFYING PARTITION TABLE AND BOOT ASSETS");
    verify(device, plan, progress)?;
    progress(79, b"STORAGE AND BOOT ASSETS VERIFIED");
    crate::output_text(b"[verify] Partition table\n[verify] EFI boot environment\n");
    crate::output_text(
        b"[verify] Infinity container\n[verify] Infinity pool\n[verify] System space\n",
    );
    crate::output_text(
        b"[verify] Personal space\n[verify] Applications space\n[verify] Recovery space\n",
    );
    crate::output_text(
        b"[verify] InfinityOS kernel\n[verify] CORE components\n[verify] Bootloader\n",
    );
    verify_runtime_bootstrap(device, plan)?;
    progress(87, b"RUNTIME POLICY VERIFIED");
    crate::output_text(b"[verify] Date and time configuration\n[verify] Runtime core\n[verify] Service manifests\n[verify] Capability policy\n[verify] Service registry\n[verify] Native AI model registry and policy\n[verify] InfinityUI runtime and skin packages\n[verify] Window Server and trusted UI policy\n");
    write_system_generation(device, plan, GENERATION_READY, false)
        .map_err(|_| StorageError::WriteSystemGeneration)?;
    verify_system_generation(device, plan, GENERATION_READY, false)?;
    progress(94, b"SYSTEM MANIFEST VERIFIED");
    crate::output_text(b"[generation] Generation 1 state=READY\n[verify] System manifest\n");
    write_system_generation(device, plan, GENERATION_ACTIVE, true)
        .map_err(|_| StorageError::ActivateGeneration)?;
    verify_system_generation(device, plan, GENERATION_ACTIVE, true)?;
    progress(98, b"GENERATION 1 ACTIVATED");
    crate::output_text(b"[generation] Generation 1 state=ACTIVE\n[verify] Generation activated\n");
    write_container_header(device, plan, 4).map_err(|_| StorageError::WriteContainer)?;
    device.flush();
    verify_complete(device, plan)?;
    progress(100, b"INFINITYOS INSTALLATION VERIFIED");
    crate::output_text(b"[provision] state=complete\n[install] complete\n");
    Ok(())
}

// ------------------------=
// FUNC: validate_plan
// DESC: Implements the validate plan operation.
// ------------------=
fn validate_plan<D: BlockDevice>(
    device: &D,
    plan: &StorageProvisioningPlan,
) -> Result<(), StorageError> {
    let expected = plan_entire_disk(plan.target, plan.profile, plan.date_time)?;
    if !plan.destructive
        || !plan.requires_efi_region
        || plan.strategy != StorageStrategy::EntireDisk
        || plan.consequence != DestructiveConsequence::EraseEntireDevice
        || plan.alignment_blocks != ALIGNMENT_BLOCKS
        || plan.container_format_version != 1
        || plan.pool.member_count != 1
        || plan.spaces.len() != 4
        || device.block_count() != plan.target.blocks
        || *plan != expected
    {
        return Err(StorageError::InvalidPlan);
    }
    Ok(())
}

// ------------------------=
// FUNC: write_gpt
// DESC: Writes or updates write gpt data.
// ------------------=
fn write_gpt<D: BlockDevice>(device: &mut D, plan: &StorageProvisioningPlan) -> Result<(), ()> {
    let last = plan.target.blocks - 1;
    let backup_entries = last - 32;
    let mut mbr = [0u8; 512];
    mbr[446 + 4] = 0xee;
    put_u32(&mut mbr, 446 + 8, 1);
    put_u32(&mut mbr, 446 + 12, (last.min(u32::MAX as u64)) as u32);
    mbr[510] = 0x55;
    mbr[511] = 0xaa;
    if !device.write_sector(0, &mbr) {
        return Err(());
    }

    let mut entries = [[0u8; 512]; 32];
    let esp_id = plan.esp_uuid;
    let container_id = plan.container_uuid;
    write_partition(
        &mut entries,
        0,
        &[
            0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e,
            0xc9, 0x3b,
        ],
        &esp_id,
        plan.esp_first_lba,
        plan.esp_last_lba,
        b"Infinity EFI",
    );
    write_partition(
        &mut entries,
        1,
        &INFINITY_TYPE,
        &container_id,
        plan.container_first_lba,
        plan.container_last_lba,
        b"Infinity Container",
    );
    let entries_crc = crc32_slices(&entries);
    for (index, sector) in entries.iter().enumerate() {
        if !device.write_sector(2 + index as u64, sector)
            || !device.write_sector(backup_entries + index as u64, sector)
        {
            return Err(());
        }
    }
    let primary = gpt_header(1, last, 34, last - 33, 2, entries_crc, plan.disk_uuid);
    let backup = gpt_header(
        last,
        1,
        34,
        last - 33,
        backup_entries,
        entries_crc,
        plan.disk_uuid,
    );
    if !device.write_sector(1, &primary) || !device.write_sector(last, &backup) {
        return Err(());
    }
    Ok(())
}

// ------------------------=
// FUNC: gpt_header
// DESC: Implements the gpt header operation.
// ------------------=
fn gpt_header(
    current: u64,
    alternate: u64,
    first: u64,
    last_usable: u64,
    entries_lba: u64,
    entries_crc: u32,
    guid: [u8; 16],
) -> [u8; 512] {
    let mut out = [0u8; 512];
    out[..8].copy_from_slice(b"EFI PART");
    put_u32(&mut out, 8, 0x0001_0000);
    put_u32(&mut out, 12, 92);
    put_u64(&mut out, 24, current);
    put_u64(&mut out, 32, alternate);
    put_u64(&mut out, 40, first);
    put_u64(&mut out, 48, last_usable);
    out[56..72].copy_from_slice(&guid);
    put_u64(&mut out, 72, entries_lba);
    put_u32(&mut out, 80, 128);
    put_u32(&mut out, 84, 128);
    put_u32(&mut out, 88, entries_crc);
    let crc = crc32(&out[..92]);
    put_u32(&mut out, 16, crc);
    out
}

// ------------------------=
// FUNC: write_partition
// DESC: Writes or updates write partition data.
// ------------------=
fn write_partition(
    entries: &mut [[u8; 512]; 32],
    index: usize,
    kind: &[u8; 16],
    id: &[u8; 16],
    first: u64,
    last: u64,
    name: &[u8],
) {
    let offset = index * 128;
    let sector = offset / 512;
    let within = offset % 512;
    entries[sector][within..within + 16].copy_from_slice(kind);
    entries[sector][within + 16..within + 32].copy_from_slice(id);
    put_u64(&mut entries[sector], within + 32, first);
    put_u64(&mut entries[sector], within + 40, last);
    for (i, byte) in name.iter().take(36).enumerate() {
        entries[sector][within + 56 + i * 2] = *byte;
    }
}

// ------------------------=
// FUNC: write_esp
// DESC: Writes or updates write esp data.
// ------------------=
fn write_esp<D: BlockDevice, F: FnMut(u8, &[u8])>(device: &mut D, plan: &StorageProvisioningPlan, progress: &mut F) -> Result<(), ()> {
    ESP_IMAGE.transfer(device,plan.esp_first_lba,false,|index| report_transfer(progress,index,ESP_IMAGE.len(),12,23,b"INSTALLING EFI BOOTLOADER AND BOOT ASSETS"))
}

// ------------------------=
// FUNC: write_native_metadata
// DESC: Writes or updates write native metadata data.
// ------------------=
fn write_native_metadata<D: BlockDevice>(
    device: &mut D,
    plan: &StorageProvisioningPlan,
    state: u32,
) -> Result<(), ()> {
    write_container_header(device, plan, state)?;
    let mut pool = [0u8; 512];
    pool[..8].copy_from_slice(b"INFPOOL1");
    put_u32(&mut pool, 8, 1);
    put_u32(&mut pool, 12, 512);
    pool[16..32].copy_from_slice(&plan.pool.uuid);
    pool[32..48].copy_from_slice(&plan.container_uuid);
    put_u64(&mut pool, 48, plan.pool.total_blocks);
    put_u64(&mut pool, 56, KERNEL_RELATIVE_LBA);
    put_u32(&mut pool, 64, plan.pool.member_count);
    finalize(&mut pool);
    if !device.write_sector(plan.container_first_lba + 1, &pool) {
        return Err(());
    }
    let mut spaces = [0u8; 512];
    spaces[..8].copy_from_slice(b"INFSPACE");
    put_u32(&mut spaces, 8, 1);
    put_u32(&mut spaces, 12, 512);
    put_u32(&mut spaces, 16, 4);
    for (index, space) in plan.spaces.iter().enumerate() {
        write_space(&mut spaces, 32 + index * 96, space);
    }
    finalize(&mut spaces);
    if !device.write_sector(plan.container_first_lba + 2, &spaces) {
        return Err(());
    }
    Ok(())
}

// ------------------------=
// FUNC: write_container_header
// DESC: Writes or updates write container header data.
// ------------------=
fn write_container_header<D: BlockDevice>(
    device: &mut D,
    plan: &StorageProvisioningPlan,
    state: u32,
) -> Result<(), ()> {
    let mut header = [0u8; 512];
    header[..8].copy_from_slice(b"INFCONT1");
    put_u32(&mut header, 8, plan.container_format_version);
    put_u32(&mut header, 12, 512);
    put_u32(&mut header, 16, state);
    put_u32(&mut header, 20, 512);
    put_u64(&mut header, 24, plan.expected_pool_blocks);
    put_u64(&mut header, 32, 1);
    put_u64(&mut header, 40, 2);
    put_u64(&mut header, 48, KERNEL_RELATIVE_LBA);
    put_u64(&mut header, 56, KERNEL_IMAGE.len() as u64);
    header[64..80].copy_from_slice(&plan.container_uuid);
    header[80..96].copy_from_slice(&plan.pool.uuid);
    put_u64(&mut header, 96, BOOT_CATALOG_RELATIVE_LBA);
    finalize(&mut header);
    if device.write_sector(plan.container_first_lba, &header) {
        Ok(())
    } else {
        Err(())
    }
}

// ------------------------=
// FUNC: write_space
// DESC: Writes or updates write space data.
// ------------------=
fn write_space(out: &mut [u8; 512], offset: usize, space: &SpacePlan) {
    out[offset..offset + 16].copy_from_slice(&space.uuid);
    let count = space.name.len().min(16);
    out[offset + 16..offset + 16 + count].copy_from_slice(&space.name[..count]);
    put_u32(out, offset + 32, space.kind);
    put_u32(out, offset + 36, space.policy);
    put_u64(out, offset + 40, space.minimum_blocks);
    put_u64(out, offset + 48, space.quota_blocks);
    put_u32(out, offset + 56, space.state);
    put_u64(out, offset + 64, space.content_lba);
    put_u64(out, offset + 72, space.content_bytes);
}

// ------------------------=
// FUNC: space
// DESC: Implements the space operation.
// ------------------=
fn space(
    _id: u8,
    name: &'static [u8],
    kind: u32,
    content_lba: u64,
    content_bytes: u64,
) -> SpacePlan {
    SpacePlan {
        uuid: [0; 16],
        name,
        kind,
        policy: 1,
        minimum_blocks: 0,
        quota_blocks: 0,
        state: 1,
        content_lba,
        content_bytes,
    }
}

// ------------------------=
// FUNC: profile_spaces
// DESC: Implements the profile spaces operation.
// ------------------=
fn profile_spaces(profile: StorageProfile) -> [SpacePlan; 4] {
    let mut spaces = [
        space(1, b"System", 1, BOOT_CATALOG_RELATIVE_LBA, 512),
        space(2, b"Personal", 2, 0, 0),
        space(3, b"Applications", 3, 0, 0),
        space(4, b"Recovery", 4, 0, 0),
    ];
    let priorities = match profile {
        StorageProfile::SharedDynamic => [1, 1, 1, 1],
    };
    for index in 0..spaces.len() {
        spaces[index].policy = priorities[index];
    }
    spaces
}

// ------------------------=
// FUNC: architecture
// DESC: Implements the architecture operation.
// ------------------=
fn architecture() -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        2
    }
    #[cfg(target_arch = "aarch64")]
    {
        3
    }
    #[cfg(target_arch = "x86")]
    {
        1
    }
}

// ------------------------=
// FUNC: write_system_generation
// DESC: Writes or updates write system generation data.
// ------------------=
fn write_system_generation<D: BlockDevice>(
    device: &mut D,
    plan: &StorageProvisioningPlan,
    state: u32,
    activate: bool,
) -> Result<(), ()> {
    let kernel_crc = KERNEL_IMAGE.checksum();
    let esp_crc = ESP_IMAGE.checksum();
    let components = component_manifest::encode(
        architecture(),
        kernel_crc,
        esp_crc,
        KERNEL_RELATIVE_LBA,
        KERNEL_IMAGE.len() as u64,
        plan.esp_first_lba,
        ESP_IMAGE.len() as u64,
    );
    for sector_index in 0..component_manifest::MANIFEST_SECTORS {
        let mut sector = [0u8; 512];
        let start = sector_index * 512;
        sector.copy_from_slice(&components[start..start + 512]);
        if !device.write_sector(
            plan.container_first_lba + COMPONENT_MANIFEST_RELATIVE_LBA + sector_index as u64,
            &sector,
        ) {
            return Err(());
        }
    }

    let mut manifest = [0u8; 512];
    manifest[..8].copy_from_slice(b"INFSYSM1");
    put_u32(&mut manifest, 8, 1);
    put_u32(&mut manifest, 12, 512);
    put_u32(&mut manifest, 16, state);
    put_u32(&mut manifest, 20, architecture());
    put_u64(&mut manifest, 24, GENERATION_ID);
    put_u64(&mut manifest, 32, 1);
    put_u64(&mut manifest, 40, KERNEL_RELATIVE_LBA);
    put_u64(&mut manifest, 48, KERNEL_IMAGE.len() as u64);
    put_u32(&mut manifest, 56, kernel_crc);
    put_u32(&mut manifest, 60, component_manifest::COMPONENT_COUNT);
    put_u64(&mut manifest, 64, COMPONENT_MANIFEST_RELATIVE_LBA);
    put_u32(&mut manifest, 72, 1);
    manifest[80..96].copy_from_slice(&plan.container_uuid);
    put_u64(&mut manifest, 96, 0);
    finalize(&mut manifest);
    if !device.write_sector(
        plan.container_first_lba + SYSTEM_MANIFEST_RELATIVE_LBA,
        &manifest,
    ) {
        return Err(());
    }

    let mut catalog = [0u8; 512];
    catalog[..8].copy_from_slice(b"INFBOOT1");
    put_u32(&mut catalog, 8, 1);
    put_u32(&mut catalog, 12, 512);
    put_u32(&mut catalog, 16, if activate { 1 } else { 0 });
    put_u32(&mut catalog, 20, architecture());
    put_u64(&mut catalog, 24, if activate { GENERATION_ID } else { 0 });
    put_u64(&mut catalog, 32, SYSTEM_MANIFEST_RELATIVE_LBA);
    put_u64(&mut catalog, 40, 0);
    put_u64(&mut catalog, 48, 0);
    put_u32(&mut catalog, 56, 1);
    catalog[64..80].copy_from_slice(&plan.container_uuid);
    finalize(&mut catalog);
    if !device.write_sector(
        plan.container_first_lba + BOOT_CATALOG_RELATIVE_LBA,
        &catalog,
    ) {
        return Err(());
    }
    device.flush();
    Ok(())
}

#[allow(dead_code)]
const INSTALL_CLASS_VALUES: [u32; 3] = [
    INSTALL_CLASS_CORE,
    INSTALL_CLASS_SYSTEM_OPTIONAL,
    INSTALL_CLASS_POST_INSTALL,
];

// ------------------------=
// FUNC: verify_system_generation
// DESC: Verifies verify system generation behavior.
// ------------------=
fn verify_system_generation<D: BlockDevice>(
    device: &mut D,
    plan: &StorageProvisioningPlan,
    state: u32,
    active: bool,
) -> Result<(), StorageError> {
    let mut s = [0u8; 512];
    if !device.read_sector(plan.container_first_lba + BOOT_CATALOG_RELATIVE_LBA, &mut s)
        || &s[..8] != b"INFBOOT1"
        || get_u32(&s, 8) != 1
        || get_u32(&s, 20) != architecture()
        || get_u64(&s, 32) != SYSTEM_MANIFEST_RELATIVE_LBA
        || get_u32(&s, 16) != (active as u32)
        || get_u64(&s, 24) != if active { GENERATION_ID } else { 0 }
        || !valid_record(&s, 512, 508)
    {
        return Err(StorageError::VerifySystemGeneration);
    }
    if !device.read_sector(
        plan.container_first_lba + SYSTEM_MANIFEST_RELATIVE_LBA,
        &mut s,
    ) || &s[..8] != b"INFSYSM1"
        || get_u32(&s, 8) != 1
        || get_u32(&s, 16) != state
        || get_u32(&s, 20) != architecture()
        || get_u64(&s, 24) != GENERATION_ID
        || get_u32(&s, 56) != KERNEL_IMAGE.checksum()
        || get_u32(&s, 60) != component_manifest::COMPONENT_COUNT
        || get_u64(&s, 64) != COMPONENT_MANIFEST_RELATIVE_LBA
        || !valid_record(&s, 512, 508)
    {
        return Err(StorageError::VerifySystemManifest);
    }
    let mut components = [0u8; component_manifest::MANIFEST_BYTES];
    for sector_index in 0..component_manifest::MANIFEST_SECTORS {
        if !device.read_sector(
            plan.container_first_lba + COMPONENT_MANIFEST_RELATIVE_LBA + sector_index as u64,
            &mut s,
        ) {
            return Err(StorageError::VerifySystemComponents);
        }
        let start = sector_index * 512;
        components[start..start + 512].copy_from_slice(&s);
    }
    if !component_manifest::validate(
        &components,
        architecture(),
        KERNEL_IMAGE.checksum(),
        KERNEL_RELATIVE_LBA,
        KERNEL_IMAGE.len() as u64,
    ) {
        return Err(StorageError::VerifySystemComponents);
    }
    Ok(())
}

// ------------------------=
// FUNC: report_transfer
// DESC: Reports completed sector work at bounded percentage boundaries, including actual KiB transferred.
// ------------------=
fn report_transfer<F: FnMut(u8, &[u8])>(progress: &mut F, index: usize, bytes: usize, start: u8, end: u8, label: &[u8]) {
    let total = bytes.div_ceil(512).max(1);
    let completed = (index + 1).min(total);
    let span = usize::from(end - start);
    let current = completed * span / total;
    if current == index * span / total && completed != total { return; }
    let mut detail = [0u8; 128];
    let mut length = label.len().min(72);
    detail[..length].copy_from_slice(&label[..length]);
    for byte in b" - " { detail[length] = *byte; length += 1; }
    for (value, suffix) in [(completed.saturating_mul(512).min(bytes).div_ceil(1024), b" / " as &[u8]), (bytes.div_ceil(1024), b" KiB" as &[u8])] {
        let mut digits = [0u8; 20];
        let mut count = 0;
        let mut value = value;
        loop { digits[count] = b'0' + (value % 10) as u8; count += 1; value /= 10; if value == 0 { break; } }
        for digit in digits[..count].iter().rev() { detail[length] = *digit; length += 1; }
        detail[length..length + suffix.len()].copy_from_slice(suffix); length += suffix.len();
    }
    progress(start + current as u8, &detail[..length]);
}

// ------------------------=
// FUNC: write_kernel
// DESC: Writes or updates write kernel data.
// ------------------=
fn write_kernel<D: BlockDevice, F: FnMut(u8, &[u8])>(device: &mut D, plan: &StorageProvisioningPlan, progress: &mut F) -> Result<(), ()> {
    KERNEL_IMAGE.transfer(device,plan.kernel_lba,false,|index| report_transfer(progress,index,KERNEL_IMAGE.len(),43,55,b"INSTALLING KERNEL, DRIVERS AND CORE COMPONENTS"))
}

// ------------------------=
// FUNC: verify
// DESC: Implements the verify operation.
// ------------------=
fn verify<D: BlockDevice, F: FnMut(u8, &[u8])>(
    device: &mut D,
    plan: &StorageProvisioningPlan,
    progress: &mut F,
) -> Result<(), StorageError> {
    let mut sector = [0u8; 512];
    if !device.read_sector(1, &mut sector) || &sector[..8] != b"EFI PART" {
        return Err(StorageError::VerifyGpt);
    }
    let entries_crc = get_u32(&sector, 88);
    if !valid_record(&sector, 92, 16) {
        return Err(StorageError::VerifyGpt);
    }
    let mut entries = [[0u8; 512]; 32];
    for index in 0..32 {
        if !device.read_sector(2 + index as u64, &mut entries[index]) {
            return Err(StorageError::VerifyGpt);
        }
    }
    if crc32_slices(&entries) != entries_crc {
        return Err(StorageError::VerifyGpt);
    }
    let backup_lba = plan.target.blocks - 1;
    if !device.read_sector(backup_lba, &mut sector)
        || &sector[..8] != b"EFI PART"
        || get_u64(&sector, 24) != backup_lba
        || get_u64(&sector, 32) != 1
        || get_u32(&sector, 88) != entries_crc
        || !valid_record(&sector, 92, 16)
    {
        return Err(StorageError::VerifyGpt);
    }
    let backup_entries = backup_lba - 32;
    for index in 0..32 {
        if !device.read_sector(backup_entries + index as u64, &mut entries[index]) {
            return Err(StorageError::VerifyGpt);
        }
    }
    if crc32_slices(&entries) != entries_crc {
        return Err(StorageError::VerifyGpt);
    }
    ESP_IMAGE.transfer(device,plan.esp_first_lba,true,|index| report_transfer(progress,index,ESP_IMAGE.len(),67,73,b"VERIFYING EFI BOOTLOADER AND BOOT ASSETS")).map_err(|_|StorageError::VerifyBootEnvironment)?;
    if !device.read_sector(plan.container_first_lba, &mut sector)
        || &sector[..8] != b"INFCONT1"
        || !valid_record(&sector, 512, 508)
    {
        return Err(StorageError::VerifyContainer);
    }
    if !device.read_sector(plan.container_first_lba + 1, &mut sector)
        || &sector[..8] != b"INFPOOL1"
        || !valid_record(&sector, 512, 508)
    {
        return Err(StorageError::VerifyPool);
    }
    if !device.read_sector(plan.container_first_lba + 2, &mut sector)
        || &sector[..8] != b"INFSPACE"
        || get_u32(&sector, 16) != 4
        || !valid_record(&sector, 512, 508)
    {
        return Err(StorageError::VerifySpaces);
    }
    KERNEL_IMAGE.transfer(device,plan.kernel_lba,true,|index| report_transfer(progress,index,KERNEL_IMAGE.len(),73,78,b"VERIFYING INSTALLED KERNEL AND CORE COMPONENTS")).map_err(|_|StorageError::VerifyKernel)?;
    Ok(())
}

// ------------------------=
// FUNC: verify_complete
// DESC: Verifies verify complete behavior.
// ------------------=
fn verify_complete<D: BlockDevice>(
    device: &mut D,
    plan: &StorageProvisioningPlan,
) -> Result<(), StorageError> {
    let mut s = [0u8; 512];
    if !device.read_sector(plan.container_first_lba, &mut s)
        || get_u32(&s, 16) != 4
        || !valid_record(&s, 512, 508)
    {
        Err(StorageError::VerifyContainer)
    } else {
        Ok(())
    }
}

#[inline(never)]
// ------------------------=
// FUNC: verify_runtime_bootstrap
// DESC: Verifies verify runtime bootstrap behavior.
// ------------------=
fn verify_runtime_bootstrap<D: BlockDevice>(
    device: &mut D,
    plan: &StorageProvisioningPlan,
) -> Result<(), StorageError> {
    let mut mounted = match super::object::ObjectStore::mount(device, plan.container_first_lba) {
        Ok(store) => store,
        Err(_) => {
            crate::output_text(b"[verify] native object metadata mount failed\n");
            return Err(StorageError::VerifyContainer);
        }
    };
    if mounted.runtime_bootstrap_valid()
        && mounted.date_time_configuration() == Some(plan.date_time)
    {
        Ok(())
    } else {
        crate::output_text(b"[verify] native runtime object validation failed\n");
        Err(StorageError::VerifyContainer)
    }
}

// ------------------------=
// FUNC: valid_record
// DESC: Reports whether valid record.
// ------------------=
fn valid_record(data: &[u8; 512], length: usize, crc_offset: usize) -> bool {
    let expected = get_u32(data, crc_offset);
    let mut copy = *data;
    put_u32(&mut copy, crc_offset, 0);
    crc32(&copy[..length]) == expected
}
// ------------------------=
// FUNC: finalize
// DESC: Implements the finalize operation.
// ------------------=
fn finalize(data: &mut [u8; 512]) {
    put_u32(data, 508, 0);
    let crc = crc32(data);
    put_u32(data, 508, crc);
}
// ------------------------=
// FUNC: crc32_slices
// DESC: Calculates and returns crc32 slices.
// ------------------=
fn crc32_slices(data: &[[u8; 512]; 32]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for sector in data {
        for byte in sector {
            crc = crc_step(crc, *byte);
        }
    }
    !crc
}
// ------------------------=
// FUNC: crc32
// DESC: Calculates and returns crc32.
// ------------------=
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc = crc_step(crc, *byte);
    }
    !crc
}
// ------------------------=
// FUNC: crc_step
// DESC: Calculates and returns crc step.
// ------------------=
fn crc_step(mut crc: u32, byte: u8) -> u32 {
    crc ^= byte as u32;
    for _ in 0..8 {
        crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
    }
    crc
}
// ------------------------=
// FUNC: put_u32
// DESC: Implements the put u32 operation.
// ------------------=
fn put_u32(out: &mut [u8], at: usize, value: u32) {
    out[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
// ------------------------=
// FUNC: put_u64
// DESC: Implements the put u64 operation.
// ------------------=
fn put_u64(out: &mut [u8], at: usize, value: u64) {
    out[at..at + 8].copy_from_slice(&value.to_le_bytes());
}
// ------------------------=
// FUNC: get_u32
// DESC: Reads get u32 data.
// ------------------=
fn get_u32(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}
// ------------------------=
// FUNC: get_u64
// DESC: Reads get u64 data.
// ------------------=
fn get_u64(data: &[u8], at: usize) -> u64 {
    u64::from_le_bytes([
        data[at],
        data[at + 1],
        data[at + 2],
        data[at + 3],
        data[at + 4],
        data[at + 5],
        data[at + 6],
        data[at + 7],
    ])
}
