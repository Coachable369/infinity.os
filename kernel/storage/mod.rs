#[cfg(target_arch = "x86_64")]
mod ata;
#[cfg(feature = "installer")]
mod format;
pub mod layout;
pub mod object;
pub mod organization;
#[cfg(target_arch = "aarch64")]
mod uefi;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StorageDevice {
    pub identity: &'static [u8],
    pub model: [u8; 40],
    pub model_length: usize,
    pub blocks: u64,
    pub logical_block_size: u32,
    pub physical_block_size: u32,
    pub removable: bool,
    pub bus: &'static [u8],
    pub has_gpt: bool,
}

impl StorageDevice {
    // ------------------------=
    // FUNC: capacity_mib
    // DESC: Calculates and returns capacity mib.
    // ------------------=
    pub fn capacity_mib(&self) -> u64 {
        self.blocks.saturating_mul(self.logical_block_size as u64) / (1024 * 1024)
    }

    // ------------------------=
    // FUNC: model
    // DESC: Implements the model operation.
    // ------------------=
    pub fn model(&self) -> &[u8] {
        &self.model[..self.model_length]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CurrentLayout {
    Empty,
    Gpt,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StorageStrategy {
    EntireDisk,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StorageProfile {
    SharedDynamic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTimeConfiguration {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub time_zone_id: u16,
    pub utc_offset_minutes: i16,
}

impl DateTimeConfiguration {
    // ------------------------=
    // FUNC: utc_default
    // DESC: Supplies a valid UTC fallback when firmware does not expose a usable wall clock.
    // ------------------=
    pub const fn utc_default() -> Self {
        Self {
            year: 2026,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            time_zone_id: 6,
            utc_offset_minutes: 0,
        }
    }

    // ------------------------=
    // FUNC: is_valid
    // DESC: Validates the bounded calendar and UTC-offset fields stored by the installer.
    // ------------------=
    pub fn is_valid(&self) -> bool {
        (2020..=2199).contains(&self.year)
            && (1..=12).contains(&self.month)
            && self.day >= 1
            && self.day <= days_in_month(self.year, self.month)
            && self.hour <= 23
            && self.minute <= 59
            && self.second <= 59
            && self.time_zone_id >= 1
            && (-14 * 60..=14 * 60).contains(&self.utc_offset_minutes)
    }
}

// ------------------------=
// FUNC: days_in_month
// DESC: Returns the Gregorian day count for a validated year and month.
// ------------------=
pub const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DestructiveConsequence {
    EraseEntireDevice,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PoolPlan {
    pub uuid: [u8; 16],
    pub member_count: u32,
    pub total_blocks: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpacePlan {
    pub uuid: [u8; 16],
    pub name: &'static [u8],
    pub kind: u32,
    pub policy: u32,
    pub minimum_blocks: u64,
    pub quota_blocks: u64,
    pub state: u32,
    pub content_lba: u64,
    pub content_bytes: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StorageProvisioningPlan {
    pub target: StorageDevice,
    pub current_layout: CurrentLayout,
    pub strategy: StorageStrategy,
    pub profile: StorageProfile,
    pub date_time: DateTimeConfiguration,
    pub consequence: DestructiveConsequence,
    pub destructive: bool,
    pub requires_efi_region: bool,
    pub alignment_blocks: u64,
    pub container_format_version: u32,
    pub container_uuid: [u8; 16],
    pub pool: PoolPlan,
    pub spaces: [SpacePlan; 4],
    pub esp_first_lba: u64,
    pub esp_last_lba: u64,
    pub container_first_lba: u64,
    pub container_last_lba: u64,
    pub kernel_lba: u64,
    pub expected_pool_blocks: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    NoDevice,
    InvalidPlan,
    InsufficientCapacity,
    Arithmetic,
    WriteGpt,
    WriteBootRegion,
    WriteContainer,
    WriteKernel,
    VerifyGpt,
    VerifyBootEnvironment,
    VerifyContainer,
    VerifyPool,
    VerifySpaces,
    VerifyKernel,
    WriteSystemGeneration,
    VerifySystemGeneration,
    VerifySystemManifest,
    VerifySystemComponents,
    ActivateGeneration,
}

pub struct StorageManager;

impl StorageManager {
    // ------------------------=
    // FUNC: discover
    // DESC: Implements the discover operation.
    // ------------------=
    pub fn discover() -> Option<StorageDevice> {
        #[cfg(all(target_arch = "x86_64", feature = "installer"))]
        {
            ata::discover()
        }
        #[cfg(all(target_arch = "aarch64", feature = "installer"))]
        {
            uefi::discover()
        }
        #[cfg(not(any(
            all(target_arch = "x86_64", feature = "installer"),
            all(target_arch = "aarch64", feature = "installer")
        )))]
        {
            None
        }
    }

    // ------------------------=
    // FUNC: plan_entire_disk
    // DESC: Implements the plan entire disk operation.
    // ------------------=
    pub fn plan_entire_disk(
        device: StorageDevice,
        profile: StorageProfile,
        date_time: DateTimeConfiguration,
    ) -> Result<StorageProvisioningPlan, StorageError> {
        #[cfg(feature = "installer")]
        {
            format::plan_entire_disk(device, profile, date_time)
        }
        #[cfg(not(feature = "installer"))]
        {
            let _ = (device, profile, date_time);
            Err(StorageError::NoDevice)
        }
    }

    // ------------------------=
    // FUNC: provision
    // DESC: Implements the provision operation.
    // ------------------=
    pub fn provision(plan: &StorageProvisioningPlan) -> Result<(), StorageError> {
        let mut ignore_progress = |_percent: u8, _label: &[u8]| {};
        Self::provision_with_progress(plan, &mut ignore_progress)
    }

    // ------------------------=
    // FUNC: provision_with_progress
    // DESC: Provisions a target while reporting only successfully reached installation checkpoints.
    // ------------------=
    pub fn provision_with_progress<F>(
        plan: &StorageProvisioningPlan,
        progress: &mut F,
    ) -> Result<(), StorageError>
    where
        F: FnMut(u8, &[u8]),
    {
        #[cfg(all(target_arch = "x86_64", feature = "installer"))]
        {
            let mut device = ata::AtaDevice::open().ok_or(StorageError::NoDevice)?;
            format::provision(&mut device, plan, progress)
        }
        #[cfg(all(target_arch = "aarch64", feature = "installer"))]
        {
            let mut device = uefi::UefiBlockDevice::open().ok_or(StorageError::NoDevice)?;
            format::provision(&mut device, plan, progress)
        }
        #[cfg(not(any(
            all(target_arch = "x86_64", feature = "installer"),
            all(target_arch = "aarch64", feature = "installer")
        )))]
        {
            let _ = (plan, progress);
            Err(StorageError::NoDevice)
        }
    }
}

#[cfg(target_arch = "x86_64")]
type NativeObjectStore = object::ObjectStore<ata::AtaDevice>;
#[cfg(target_arch = "aarch64")]
type NativeObjectStore = object::ObjectStore<uefi::UefiBlockDevice>;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static mut OBJECT_STORE: Option<NativeObjectStore> = None;

// ------------------------=
// FUNC: initialize_object_store
// DESC: Initializes initialize object store state.
// ------------------=
pub fn initialize_object_store() {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        let device = {
            #[cfg(target_arch = "x86_64")]
            {
                ata::AtaDevice::open()
            }
            #[cfg(target_arch = "aarch64")]
            {
                uefi::UefiBlockDevice::open()
            }
        };
        let Some(mut device) = device else {
            return;
        };
        let Ok((container, _, _)) = object::find_container(&mut device) else {
            return;
        };
        match object::ObjectStore::mount(device, container) {
            Ok(store) => unsafe {
                crate::output_text(b"[storage] container valid\n[storage] pool online\n");
                crate::output_text(
                    b"[object] committed generation loaded\n[object] object index online\n",
                );
                crate::output_text(
                    b"[namespace] namespace online\n[storage] Infinity Object Store online.\n",
                );
                OBJECT_STORE = Some(store);
            },
            Err(_) => {
                crate::output_text(b"[storage] object store unavailable; disk was not modified\n")
            }
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: with_store
// DESC: Implements the with store operation.
// ------------------=
fn with_store<T>(
    f: impl FnOnce(&mut NativeObjectStore) -> Result<T, object::ObjectError>,
) -> Result<T, object::ObjectError> {
    unsafe {
        OBJECT_STORE
            .as_mut()
            .ok_or(object::ObjectError::SpaceUnavailable)
            .and_then(f)
    }
}

// ------------------------=
// FUNC: installed_date_time_configuration
// DESC: Exposes the validated installed date-and-time settings to system services without revealing Object Store internals.
// ------------------=
pub fn installed_date_time_configuration() -> Option<DateTimeConfiguration> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| Ok(store.date_time_configuration()))
            .ok()
            .flatten()
    }
    #[cfg(target_arch = "x86")]
    {
        None
    }
}

// ------------------------=
// FUNC: object_query_nth
// DESC: Returns one member of a typed native Object.Query result without rendering text.
// ------------------=
pub fn object_query_nth(
    kind: Option<object::ObjectType>,
    space: Option<object::Space>,
    index: usize,
) -> Result<Option<object::ObjectQueryResult>, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| {
            Ok(store.query_nth(
                object::ObjectQueryRequest {
                    kind,
                    space,
                    include_tombstones: false,
                },
                index,
            ))
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (kind, space, index);
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: organization_create
// DESC: Creates a first-class Project or Collection object and one optional namespace projection.
// ------------------=
pub fn organization_create(
    kind: object::ObjectType,
    name: &[u8],
) -> Result<object::ObjectId, object::ObjectError> {
    if !matches!(
        kind,
        object::ObjectType::Project | object::ObjectType::Collection
    ) {
        return Err(object::ObjectError::InvalidObject);
    }
    let base = if kind == object::ObjectType::Project {
        b"/projects/".as_slice()
    } else {
        b"/collections/".as_slice()
    };
    let mut path = [0u8; 95];
    if name.is_empty() || base.len() + name.len() > path.len() {
        return Err(object::ObjectError::InvalidPath);
    }
    path[..base.len()].copy_from_slice(base);
    path[base.len()..base.len() + name.len()].copy_from_slice(name);
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| {
            store.create_attached(
                name,
                kind,
                object::Space::Personal,
                b"",
                &path[..base.len() + name.len()],
            )
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = path;
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: organization_inspect
// DESC: Resolves a Project or Collection namespace projection to stable typed metadata.
// ------------------=
pub fn organization_inspect(
    kind: object::ObjectType,
    name: &[u8],
) -> Result<object::ObjectMetadata, object::ObjectError> {
    let base = if kind == object::ObjectType::Project {
        b"/projects/".as_slice()
    } else if kind == object::ObjectType::Collection {
        b"/collections/".as_slice()
    } else {
        return Err(object::ObjectError::InvalidObject);
    };
    let mut path = [0u8; 95];
    if name.is_empty() || base.len() + name.len() > path.len() {
        return Err(object::ObjectError::InvalidPath);
    }
    path[..base.len()].copy_from_slice(base);
    path[base.len()..base.len() + name.len()].copy_from_slice(name);
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| {
            let id = store.resolve(&path[..base.len() + name.len()])?;
            let metadata = store.metadata(id)?;
            if metadata.kind != kind {
                return Err(object::ObjectError::InvalidObject);
            }
            Ok(metadata)
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = path;
        Err(object::ObjectError::SpaceUnavailable)
    }
}
#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: with_store
// DESC: Implements the with store operation.
// ------------------=
fn with_store<T>(
    _f: impl FnOnce(&mut ()) -> Result<T, object::ObjectError>,
) -> Result<T, object::ObjectError> {
    Err(object::ObjectError::SpaceUnavailable)
}

// ------------------------=
// FUNC: object_create_note
// DESC: Implements the object create note operation.
// ------------------=
pub fn object_create_note(
    name: &[u8],
    content: &[u8],
) -> Result<object::ObjectId, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            s.create(
                name,
                object::ObjectType::Text,
                object::Space::Personal,
                content,
            )
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (name, content);
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: object_create_note_at
// DESC: Implements the object create note at operation.
// ------------------=
pub fn object_create_note_at(
    name: &[u8],
    content: &[u8],
    path: &[u8],
) -> Result<object::ObjectId, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            s.create_attached(
                name,
                object::ObjectType::Text,
                object::Space::Personal,
                content,
                path,
            )
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (name, content, path);
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: object_write_path
// DESC: Implements the object write path operation.
// ------------------=
pub fn object_write_path(path: &[u8], content: &[u8]) -> Result<u32, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            let id = s.resolve(path)?;
            s.write(id, content)
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (path, content);
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: object_read_path
// DESC: Implements the object read path operation.
// ------------------=
pub fn object_read_path(
    path: &[u8],
    version: Option<u32>,
    out: &mut [u8],
) -> Result<(object::ObjectId, usize), object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            let id = s.resolve(path)?;
            let n = s.read(id, version, out)?;
            Ok((id, n))
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (path, version, out);
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: identity_state_load
// DESC: Loads the native identity object without exposing filesystem semantics.
// ------------------=
pub fn identity_state_load(out: &mut [u8]) -> Result<usize, object::ObjectError> {
    object_read_path(b"/system/identity/state", None, out).map(|(_, length)| length)
}

// ------------------------=
// FUNC: identity_state_commit
// DESC: Transactionally commits the next version of authoritative identity state.
// ------------------=
pub fn identity_state_commit(content: &[u8]) -> Result<u32, object::ObjectError> {
    object_write_path(b"/system/identity/state", content)
}
// ------------------------=
// FUNC: namespace_attach
// DESC: Implements the namespace attach operation.
// ------------------=
pub fn namespace_attach(path: &[u8], id: object::ObjectId) -> Result<(), object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| s.attach(path, id))
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (path, id);
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: namespace_link
// DESC: Implements the namespace link operation.
// ------------------=
pub fn namespace_link(from: &[u8], to: &[u8]) -> Result<object::ObjectId, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            let id = s.resolve(from)?;
            s.attach(to, id)?;
            Ok(id)
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (from, to);
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: namespace_resolve
// DESC: Resolves a human namespace projection to its stable native Object ID.
// ------------------=
pub fn namespace_resolve(path: &[u8]) -> Result<object::ObjectId, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| s.resolve(path))
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = path;
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: namespace_ensure_link
// DESC: Idempotently persists another human namespace reference to an existing object.
// ------------------=
pub fn namespace_ensure_link(
    from: &[u8],
    to: &[u8],
) -> Result<object::ObjectId, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            let id = s.resolve(from)?;
            match s.resolve(to) {
                Ok(existing) if existing == id => Ok(id),
                Ok(_) => Err(object::ObjectError::NameConflict),
                Err(object::ObjectError::NamespaceNotFound) => {
                    s.attach(to, id)?;
                    Ok(id)
                }
                Err(error) => Err(error),
            }
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (from, to);
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: namespace_move
// DESC: Implements the namespace move operation.
// ------------------=
pub fn namespace_move(from: &[u8], to: &[u8]) -> Result<(), object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| s.move_entry(from, to))
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (from, to);
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: namespace_detach
// DESC: Implements the namespace detach operation.
// ------------------=
pub fn namespace_detach(path: &[u8]) -> Result<(), object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| s.detach(path))
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = path;
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: object_history
// DESC: Implements the object history operation.
// ------------------=
pub fn object_history(path: &[u8]) -> Result<(object::ObjectId, usize, u32), object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            let id = s.resolve(path)?;
            Ok((id, s.history_count(id), s.current_version(id)?))
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = path;
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: object_history_version
// DESC: Implements the object history version operation.
// ------------------=
pub fn object_history_version(
    path: &[u8],
    index: usize,
) -> Result<(u32, bool), object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            let id = s.resolve(path)?;
            s.history_version_nth(id, index)
                .ok_or(object::ObjectError::InvalidVersion)
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (path, index);
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: object_inspect_path
// DESC: Implements the object inspect path operation.
// ------------------=
pub fn object_inspect_path(
    path: &[u8],
) -> Result<(object::ObjectMetadata, usize), object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            let id = s.resolve(path)?;
            let metadata = s.metadata(id)?;
            let mut content = [0u8; object::MAX_CONTENT];
            s.read(id, None, &mut content)?;
            Ok((metadata, s.namespace_refs(id)))
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = path;
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: object_restore
// DESC: Implements the object restore operation.
// ------------------=
pub fn object_restore(path: &[u8], version: u32) -> Result<u32, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            let id = s.resolve(path)?;
            s.restore(id, version)
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (path, version);
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: object_remove_path
// DESC: Implements the object remove path operation.
// ------------------=
pub fn object_remove_path(path: &[u8]) -> Result<object::ObjectId, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| s.remove_path(path))
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = path;
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: storage_usage
// DESC: Implements the storage usage operation.
// ------------------=
pub fn storage_usage() -> Result<(u32, u32, u64), object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| Ok((s.usage_blocks(), s.total_blocks(), s.generation())))
    }
    #[cfg(target_arch = "x86")]
    {
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: storage_usage_by_space
// DESC: Implements the storage usage by space operation.
// ------------------=
pub fn storage_usage_by_space() -> Result<[u32; 4], object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| {
            Ok([
                s.usage_by_space(object::Space::System),
                s.usage_by_space(object::Space::Personal),
                s.usage_by_space(object::Space::Applications),
                s.usage_by_space(object::Space::Recovery),
            ])
        })
    }
    #[cfg(target_arch = "x86")]
    {
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: object_collect
// DESC: Implements the object collect operation.
// ------------------=
pub fn object_collect() -> Result<u32, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|s| s.collect())
    }
    #[cfg(target_arch = "x86")]
    {
        Err(object::ObjectError::SpaceUnavailable)
    }
}
// ------------------------=
// FUNC: namespace_entry
// DESC: Implements the namespace entry operation.
// ------------------=
pub fn namespace_entry(index: usize, out: &mut [u8]) -> Option<(usize, object::ObjectId)> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    unsafe {
        let store = OBJECT_STORE.as_ref()?;
        let (path, id) = store.namespace_entry(index)?;
        let n = path.len().min(out.len());
        out[..n].copy_from_slice(&path[..n]);
        Some((n, id))
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (index, out);
        None
    }
}

// ------------------------=
// FUNC: namespace_list_nth
// DESC: Returns one typed namespace projection below a prefix for native object pickers.
// ------------------=
pub fn namespace_list_nth(
    prefix: &[u8],
    index: usize,
) -> Result<Option<object::NamespaceListResult>, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| Ok(store.namespace_list_nth(prefix, index)))
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (prefix, index);
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: local_ai_model_object_ref
// DESC: Resolves the installed native model identity for Model Registry binding.
// ------------------=
pub fn local_ai_model_object_ref() -> Option<[u8; 16]> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    unsafe {
        OBJECT_STORE
            .as_ref()?
            .resolve(b"/system/models/local-intent-v1")
            .ok()
            .map(|identity| identity.0)
    }
    #[cfg(target_arch = "x86")]
    {
        None
    }
}

pub trait BlockDevice {
    // ------------------------=
    // FUNC: block_count
    // DESC: Implements the block count operation.
    // ------------------=
    fn block_count(&self) -> u64;
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads read sector data.
    // ------------------=
    fn read_sector(&mut self, lba: u64, sector: &mut [u8; 512]) -> bool;
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes or updates write sector data.
    // ------------------=
    fn write_sector(&mut self, lba: u64, sector: &[u8; 512]) -> bool;
    // ------------------------=
    // FUNC: flush
    // DESC: Implements the flush operation.
    // ------------------=
    fn flush(&mut self) -> bool;
}

impl<T: BlockDevice + ?Sized> BlockDevice for &mut T {
    // ------------------------=
    // FUNC: block_count
    // DESC: Implements the block count operation.
    // ------------------=
    fn block_count(&self) -> u64 {
        (**self).block_count()
    }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads read sector data.
    // ------------------=
    fn read_sector(&mut self, lba: u64, sector: &mut [u8; 512]) -> bool {
        (**self).read_sector(lba, sector)
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes or updates write sector data.
    // ------------------=
    fn write_sector(&mut self, lba: u64, sector: &[u8; 512]) -> bool {
        (**self).write_sector(lba, sector)
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Implements the flush operation.
    // ------------------=
    fn flush(&mut self) -> bool {
        (**self).flush()
    }
}
