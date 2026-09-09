#[cfg(target_arch = "x86_64")]
mod ata;
#[cfg(feature = "installer")]
mod component_manifest;
#[cfg(feature = "installer")]
mod format;
#[cfg(feature = "installer")]
mod install_identity;
pub mod layout;
pub mod object;
pub(crate) mod fabric;
pub mod organization;
#[cfg(target_arch = "aarch64")]
mod uefi;

// ------------------------=
// FUNC: initialize_installation_identity
// DESC: Retains installation-only uniqueness during early boot independently of persistent networking identity; installed-only kernels retain no extra seed.
// ------------------=
pub fn initialize_installation_identity(entropy: &[u8; 32], valid: bool) {
    #[cfg(feature = "installer")]
    install_identity::initialize(entropy, valid);
    #[cfg(not(feature = "installer"))]
    let _ = (entropy, valid);
}

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
    pub disk_uuid: [u8; 16],
    pub esp_uuid: [u8; 16],
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
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static mut REPLICA_SERVICE: Option<fabric::service::ReplicaService> = None;

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
        let Ok((container, _, container_id)) = object::find_container(&mut device) else {
            return;
        };
        let disk_identity = object::device_identity(&mut device);
        match object::ObjectStore::mount(device, container) {
            Ok(mut store) => unsafe {
                crate::output_text(b"[storage] container valid\n[storage] pool online\n");
                crate::output_text(
                    b"[object] committed generation loaded\n[object] object index online\n",
                );
                crate::output_text(
                    b"[namespace] namespace online\n[storage] Infinity Object Store online.\n",
                );
                REPLICA_SERVICE = fabric::service::ReplicaService::mount(&mut store,
                    crate::runtime::fabric::resources::ResourceId(container_id), 1).ok();
                if let Some(service) = REPLICA_SERVICE.as_mut() { service.attach_device_identity(disk_identity); }
                OBJECT_STORE = Some(store);
                if REPLICA_SERVICE.is_some() {
                    crate::runtime::register_storage_backend(execute_replica_request);
                }
            },
            Err(_) => {
                crate::output_text(b"[storage] object store unavailable; disk was not modified\n")
            }
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: execute_replica_request
// DESC: Executes one authenticated request against mounted native state and emits a notice only for a committed generation change.
// ------------------=
fn execute_replica_request(request: crate::runtime::iop::remote::AuthenticatedStorageRequest)
    -> Result<(crate::runtime::iop::storage_protocol::StorageOperationV1,
        Option<crate::runtime::iop::storage_protocol::StorageCommit>), crate::runtime::iop::remote::RemoteError> {
    use crate::runtime::iop::{remote::RemoteError, storage_protocol::StorageCommit};
    with_store(|store| {
        let before = store.generation();
        let response = unsafe { REPLICA_SERVICE.as_mut().ok_or(RemoteError::ServiceUnavailable)
            .and_then(|service| service.execute(store, request)) };
        Ok(response.map(|response| {
            let notice = (store.generation() != before).then_some(StorageCommit {
                event: crate::runtime::iop::storage_protocol::EVENT_REPLICA_CHANGED,
                object: request.payload.object, generation: store.generation(), copied: response.offset,
                state: response.data[0], correlation: request.correlation, causation: request.request_id,
            });
            (response, notice)
        }))
    }).map_err(|_| RemoteError::ServiceUnavailable)?
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
// FUNC: network_state_load
// DESC: Loads authoritative typed networking state from System Space.
// ------------------=
pub fn network_state_load(out: &mut [u8]) -> Result<usize, object::ObjectError> {
    object_read_path(b"/system/network/state", None, out).map(|(_, length)| length)
}

// ------------------------=
// FUNC: network_state_commit
// DESC: Transactionally commits a new native networking-state object version.
// ------------------=
pub fn network_state_commit(content: &[u8]) -> Result<u32, object::ObjectError> {
    object_write_path(b"/system/network/state", content)
}

// ------------------------=
// FUNC: node_state_load
// DESC: Loads authoritative cryptographic node identity, trust, and mesh state from System Space.
// ------------------=
pub fn node_state_load(out: &mut [u8]) -> Result<usize, object::ObjectError> {
    object_read_path(b"/system/security/nodes/state", None, out).map(|(_, length)| length)
}

// ------------------------=
// FUNC: node_state_commit
// DESC: Transactionally commits the typed node trust object as a new native object version.
// ------------------=
pub fn node_state_commit(content: &[u8]) -> Result<u32, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| {
            let id = store.resolve(b"/system/security/nodes/state")?;
            store.replace_state(id, content)
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = content;
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: shell_profile_state_load
// DESC: Loads versioned declarative Shell Profile objects from authoritative System Space.
// ------------------=
pub fn shell_profile_state_load(out: &mut [u8]) -> Result<usize, object::ObjectError> {
    object_read_path(b"/system/settings/shell/profiles", None, out).map(|(_, length)| length)
}

// ------------------------=
// FUNC: shell_profile_state_commit
// DESC: Commits declarative Shell Profile state without dotfiles or executable startup code.
// ------------------=
pub fn shell_profile_state_commit(content: &[u8]) -> Result<u32, object::ObjectError> {
    object_write_path(b"/system/settings/shell/profiles", content)
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
// FUNC: namespace_create
// DESC: Creates a first-class Namespace Object and its human reference atomically.
// ------------------=
pub fn namespace_create(path: &[u8]) -> Result<object::ObjectId, object::ObjectError> {
    let name = path.rsplit(|byte| *byte == b'/').next().unwrap_or(&[]);
    if name.is_empty() {
        return Err(object::ObjectError::InvalidPath);
    }
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| {
            store.create_attached(
                name,
                object::ObjectType::NamespaceNode,
                object::Space::Personal,
                b"",
                path,
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
// FUNC: namespace_delete
// DESC: Deletes only an empty non-protected Namespace and rejects implicit recursion.
// ------------------=
pub fn namespace_delete(path: &[u8]) -> Result<object::ObjectId, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| store.delete_namespace(path))
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = path;
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: object_copy_path
// DESC: Creates a new logical Object identity while preserving source content and metadata.
// ------------------=
pub fn object_copy_path(
    source: &[u8],
    destination: &[u8],
) -> Result<object::ObjectId, object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| store.copy_path_attached(source, destination))
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (source, destination);
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: object_destroy_explicit
// DESC: Dispatches an explicit capability-confirmed underlying Object destruction request.
// ------------------=
pub fn object_destroy_explicit(
    reference: &[u8],
    authorized: bool,
) -> Result<(), object::ObjectError> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        with_store(|store| {
            let id = store.resolve(reference)?;
            store.destroy_explicit(id, authorized)
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (reference, authorized);
        Err(object::ObjectError::SpaceUnavailable)
    }
}

// ------------------------=
// FUNC: trash_move
// DESC: Moves a selected Namespace reference beneath Trash while preserving ObjectId and original path.
// ------------------=
pub fn trash_move(path: &[u8]) -> Result<object::ObjectId, object::ObjectError> {
    if path.starts_with(b"/trash/") || path == b"/trash" || path.len() + 6 > 95 {
        return Err(object::ObjectError::InvalidPath);
    }
    let id = namespace_resolve(path)?;
    let mut trash_path = [0u8; 95];
    trash_path[..6].copy_from_slice(b"/trash");
    trash_path[6..6 + path.len()].copy_from_slice(path);
    namespace_move(path, &trash_path[..6 + path.len()])?;
    Ok(id)
}

// ------------------------=
// FUNC: trash_restore
// DESC: Restores a Trash Namespace reference to its exact original human path.
// ------------------=
pub fn trash_restore(trash_path: &[u8]) -> Result<object::ObjectId, object::ObjectError> {
    if !trash_path.starts_with(b"/trash/") {
        return Err(object::ObjectError::InvalidPath);
    }
    let original = &trash_path[6..];
    let id = namespace_resolve(trash_path)?;
    namespace_move(trash_path, original)?;
    Ok(id)
}

// ------------------------=
// FUNC: trash_delete
// DESC: Permanently deletes one Trash entry through the selected reference lifecycle.
// ------------------=
pub fn trash_delete(trash_path: &[u8]) -> Result<object::ObjectId, object::ObjectError> {
    if !trash_path.starts_with(b"/trash/") {
        return Err(object::ObjectError::InvalidPath);
    }
    object_remove_path(trash_path)
}

// ------------------------=
// FUNC: trash_empty
// DESC: Permanently removes all visible Trash references with bounded repeated enumeration.
// ------------------=
pub fn trash_empty() -> Result<usize, object::ObjectError> {
    let mut removed = 0usize;
    loop {
        let Some(entry) = namespace_list_nth(b"/trash", 0)? else {
            return Ok(removed);
        };
        object_remove_path(&entry.path[..entry.path_len as usize])?;
        removed += 1;
    }
}

// ------------------------=
// FUNC: object_reference_nth
// DESC: Returns one Namespace reference attached to a stable Object identity.
// ------------------=
pub fn object_reference_nth(id: object::ObjectId, index: usize, out: &mut [u8]) -> Option<usize> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    unsafe {
        let store = OBJECT_STORE.as_ref()?;
        let path = store.namespace_ref_nth(id, index)?;
        let length = path.len().min(out.len());
        out[..length].copy_from_slice(&path[..length]);
        Some(length)
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (id, index, out);
        None
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
            s.inspect(id)
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
// FUNC: namespace_child_nth
// DESC: Returns one direct child projection without leaking deeper descendants into a navigator view.
// ------------------=
pub fn namespace_child_nth(
    parent: &[u8],
    requested: usize,
) -> Result<Option<object::NamespaceListResult>, object::ObjectError> {
    let mut visible = 0usize;
    for source_index in 0..256usize {
        let Some(entry) = namespace_list_nth(parent, source_index)? else {
            break;
        };
        let path = &entry.path[..entry.path_len as usize];
        if crate::runtime::object_navigation::is_immediate_namespace_child(parent, path) {
            if visible == requested {
                return Ok(Some(entry));
            }
            visible += 1;
        }
    }
    Ok(None)
}

// ------------------------=
// FUNC: namespace_child_count
// DESC: Counts direct children for native collection views and their status regions.
// ------------------=
pub fn namespace_child_count(parent: &[u8]) -> Result<usize, object::ObjectError> {
    let mut count = 0usize;
    for source_index in 0..256usize {
        let Some(entry) = namespace_list_nth(parent, source_index)? else {
            break;
        };
        if crate::runtime::object_navigation::is_immediate_namespace_child(
            parent,
            &entry.path[..entry.path_len as usize],
        ) {
            count += 1;
        }
    }
    Ok(count)
}

// ------------------------=
// FUNC: namespace_child_nth_sorted
// DESC: Returns one direct child in deterministic case-insensitive name order without heap allocation.
// ------------------=
pub fn namespace_child_nth_sorted(
    parent: &[u8],
    requested: usize,
    descending: bool,
) -> Result<Option<object::NamespaceListResult>, object::ObjectError> {
    let mut previous: Option<object::NamespaceListResult> = None;
    for _ in 0..=requested {
        let mut best: Option<object::NamespaceListResult> = None;
        for source_index in 0..256usize {
            let Some(candidate) = namespace_list_nth(parent, source_index)? else {
                break;
            };
            let candidate_path = &candidate.path[..candidate.path_len as usize];
            if !crate::runtime::object_navigation::is_immediate_namespace_child(
                parent,
                candidate_path,
            ) {
                continue;
            }
            if let Some(prior) = previous {
                let prior_path = &prior.path[..prior.path_len as usize];
                let ordering = namespace_name_order(candidate_path, prior_path);
                if (!descending && ordering != core::cmp::Ordering::Greater)
                    || (descending && ordering != core::cmp::Ordering::Less)
                {
                    continue;
                }
            }
            let replace = best
                .map(|current| {
                    let current_path = &current.path[..current.path_len as usize];
                    let ordering = namespace_name_order(candidate_path, current_path);
                    if descending {
                        ordering == core::cmp::Ordering::Greater
                    } else {
                        ordering == core::cmp::Ordering::Less
                    }
                })
                .unwrap_or(true);
            if replace {
                best = Some(candidate);
            }
        }
        let Some(next) = best else {
            return Ok(None);
        };
        previous = Some(next);
    }
    Ok(previous)
}

// ------------------------=
// FUNC: namespace_name_order
// DESC: Compares final NamespaceRef components using stable ASCII case folding.
// ------------------=
fn namespace_name_order(left: &[u8], right: &[u8]) -> core::cmp::Ordering {
    let left = crate::runtime::object_navigation::namespace_basename(left);
    let right = crate::runtime::object_navigation::namespace_basename(right);
    for index in 0..left.len().min(right.len()) {
        let a = left[index].to_ascii_lowercase();
        let b = right[index].to_ascii_lowercase();
        match a.cmp(&b) {
            core::cmp::Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    left.len().cmp(&right.len())
}

// ------------------------=
// FUNC: local_ai_model_object_refs
// DESC: Resolves all installed native model identities for Model Registry binding.
// ------------------=
pub fn local_ai_model_object_refs() -> Option<[[u8; 16]; 3]> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    unsafe {
        let store = OBJECT_STORE.as_ref()?;
        let conversation_pack = store.resolve(b"/system/ai/bootstrap").ok()?.0;
        Some([
            store.resolve(b"/system/models/local-intent-v1").ok()?.0,
            conversation_pack,
            conversation_pack,
        ])
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
