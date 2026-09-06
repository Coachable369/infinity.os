//! InfinityOS native object store. All encodings are explicit little-endian;
//! no Rust layout is ever written to persistent media.

use super::organization;
use super::{BlockDevice, DateTimeConfiguration};

#[path = "../runtime/ai/generation.rs"]
mod ai_generation_asset;
#[path = "../ai_model_asset.rs"]
mod ai_model_asset;

pub const ALLOCATION_BLOCK_SECTORS: u64 = 8; // 4 KiB independent of media sectors
                                             // The first 48 MiB of the Infinity Container is reserved for the installed
                                             // kernel and future boot-critical growth. Native object metadata begins after
                                             // that explicit boundary rather than relying on the current kernel size.
pub use super::layout::STORE_RELATIVE_LBA;
pub const ROOT_A: u64 = 0;
pub const ROOT_B: u64 = 1;
pub const BANK_A: u64 = 8;
pub const BANK_B: u64 = 40;
const CONTENT: u64 = 80;
pub const BOOTSTRAP_CONTENT_OBJECTS: u64 = 14;
// Eight object-table sectors fit in each 32-sector metadata bank. Keeping the
// table capacity derived from its serialized geometry prevents bootstrap
// System objects from consuming the user-visible object budget by accident.
const OBJECT_TABLE_SECTORS: usize = 8;
const OBJECTS_PER_SECTOR: usize = 4;
const MAX_OBJECTS: usize = OBJECT_TABLE_SECTORS * OBJECTS_PER_SECTOR;
const MAX_VERSIONS: usize = 32;
const MAX_ENTRIES: usize = 32;
const MAX_RELATIONSHIPS: usize = 16;
const MAX_PATH: usize = 95;
const MAX_COMPONENT: usize = 63;
pub const MAX_CONTENT: usize = 16 * 1024;
const ALLOCATION_BYTES: usize = 1968;
pub const FORMAT_VERSION: u32 = 3;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct ObjectId(pub [u8; 16]);
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectRef {
    pub id: ObjectId,
}
#[derive(Clone, Copy)]
pub struct ObjectCreateRequest<'a> {
    pub name: &'a [u8],
    pub kind: ObjectType,
    pub space: Space,
    pub content: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct ObjectReadRequest {
    pub object: ObjectRef,
    pub version: Option<u32>,
    pub offset: u32,
    pub length: u32,
}
#[derive(Clone, Copy)]
pub struct ObjectReadResult {
    pub object: ObjectRef,
    pub version: u32,
    pub logical_size: u32,
    pub content_crc: u32,
}
#[derive(Clone, Copy)]
pub struct ObjectUpdateRequest<'a> {
    pub object: ObjectRef,
    pub content: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct ObjectHistoryRequest {
    pub object: ObjectRef,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ObjectHistoryResult {
    pub object: ObjectRef,
    pub versions: u32,
    pub current_version: u32,
}
#[derive(Clone, Copy)]
pub struct ObjectDeleteRequest {
    pub object: ObjectRef,
}
#[derive(Clone, Copy)]
pub struct NamespaceResolveRequest<'a> {
    pub path: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct NamespaceAttachRequest<'a> {
    pub path: &'a [u8],
    pub object: ObjectRef,
}
#[derive(Clone, Copy)]
pub struct NamespaceDetachRequest<'a> {
    pub path: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct NamespaceMoveRequest<'a> {
    pub from: &'a [u8],
    pub to: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct NamespaceListRequest<'a> {
    pub prefix: &'a [u8],
    pub index: usize,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NamespaceListResult {
    pub path: [u8; MAX_PATH],
    pub path_len: u8,
    pub object: ObjectRef,
}
#[derive(Clone, Copy)]
pub struct ObjectQueryRequest {
    pub kind: Option<ObjectType>,
    pub space: Option<Space>,
    pub include_tombstones: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ObjectQueryResult {
    pub object: ObjectRef,
    pub kind: ObjectType,
    pub space: Space,
    pub display_name: [u8; 47],
    pub display_name_length: u8,
    pub current_version: u32,
    pub logical_size: u32,
}
#[derive(Clone, Copy)]
pub struct ObjectMetadataUpdateRequest<'a> {
    pub object: ObjectRef,
    pub owner: ObjectId,
    pub content_type: ContentType,
    pub tags: &'a [u8],
    pub flags: u32,
}
#[derive(Clone, Copy)]
pub struct RelationshipAttachRequest {
    pub source: ObjectRef,
    pub kind: RelationshipType,
    pub target: ObjectRef,
    pub flags: u32,
}
#[derive(Clone, Copy)]
pub struct RelationshipDetachRequest {
    pub source: ObjectRef,
    pub kind: RelationshipType,
    pub target: ObjectRef,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RelationshipResult {
    pub source: ObjectRef,
    pub kind: RelationshipType,
    pub target: ObjectRef,
    pub flags: u32,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpaceUsage {
    pub space: Space,
    pub allocated_blocks: u32,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ObjectMetadata {
    pub object: ObjectRef,
    pub kind: ObjectType,
    pub space: Space,
    pub owner: ObjectId,
    pub content_type: ContentType,
    pub flags: u32,
    pub current_version: u32,
    pub logical_size: u32,
    pub created: u64,
    pub modified: u64,
    pub tags: [u8; 8],
    pub tags_len: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContentType {
    Binary = 1,
    Utf8Text = 2,
    Namespace = 3,
    System = 4,
    Application = 5,
    Metadata = 6,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RelationshipType {
    MemberOf = 1,
    DerivedFrom = 2,
    References = 3,
    GeneratedBy = 4,
    BelongsToProject = 5,
    ContainedBy = 6,
    RelatedTo = 7,
    VersionOf = 8,
    OwnedBy = 9,
    SharedWith = 10,
    Favorite = 11,
    Pinned = 12,
}

/// Authorization is deliberately ahead of the service. Callers must cross
/// this typed boundary even while the milestone policy remains permissive.
pub trait ObjectCapabilityPolicy {
    // ------------------------=
    // FUNC: authorize
    // DESC: Implements the authorize operation.
    // ------------------=
    fn authorize(&self, operation: ObjectOperation, object: Option<ObjectRef>) -> bool;
}
#[derive(Clone, Copy)]
pub enum ObjectOperation {
    Create,
    Read,
    Update,
    Query,
    Delete,
    History,
    NamespaceResolve,
    NamespaceList,
    NamespaceAttach,
    NamespaceDetach,
    NamespaceMove,
    RelationshipAttach,
    RelationshipDetach,
    RelationshipQuery,
    Collect,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectType {
    Blob = 1,
    Text = 2,
    NamespaceNode = 3,
    SystemComponent = 4,
    ApplicationData = 5,
    Metadata = 6,
    Collection = 7,
    Project = 8,
    Model = 9,
    IdentityData = 10,
    DeviceData = 11,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Space {
    System = 1,
    Personal = 2,
    Applications = 3,
    Recovery = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectError {
    NotFound,
    NamespaceNotFound,
    NameConflict,
    InvalidObject,
    InvalidPath,
    InsufficientCapacity,
    CorruptMetadata,
    CorruptContent,
    UnsupportedFormat,
    TransactionFailed,
    ChecksumMismatch,
    InvalidVersion,
    SpaceUnavailable,
    Busy,
    Unauthorized,
}

#[derive(Clone, Copy)]
struct ObjectRecord {
    used: bool,
    tombstone: bool,
    id: ObjectId,
    kind: u8,
    space: u8,
    current_version: u32,
    created: u64,
    modified: u64,
    name_len: u8,
    name: [u8; 47],
    owner: ObjectId,
    flags: u32,
    content_type: u16,
    tags_len: u8,
    tags: [u8; 8],
}
impl Default for ObjectRecord {
    // ------------------------=
    // FUNC: default
    // DESC: Returns the default initialized value.
    // ------------------=
    fn default() -> Self {
        Self {
            used: false,
            tombstone: false,
            id: ObjectId::default(),
            kind: 0,
            space: 0,
            current_version: 0,
            created: 0,
            modified: 0,
            name_len: 0,
            name: [0; 47],
            owner: ObjectId([0; 16]),
            flags: 0,
            content_type: 0,
            tags_len: 0,
            tags: [0; 8],
        }
    }
}

#[derive(Clone, Copy, Default)]
struct VersionRecord {
    used: bool,
    object: ObjectId,
    number: u32,
    extent: u32,
    blocks: u16,
    size: u32,
    content_crc: u32,
    parent: u32,
    generation: u64,
}

#[derive(Clone, Copy)]
struct NamespaceRecord {
    used: bool,
    target: ObjectId,
    path_len: u8,
    path: [u8; MAX_PATH],
}
impl Default for NamespaceRecord {
    // ------------------------=
    // FUNC: default
    // DESC: Returns the default initialized value.
    // ------------------=
    fn default() -> Self {
        Self {
            used: false,
            target: ObjectId::default(),
            path_len: 0,
            path: [0; MAX_PATH],
        }
    }
}

#[derive(Clone, Copy, Default)]
struct RelationshipRecord {
    used: bool,
    source: ObjectId,
    target: ObjectId,
    kind: u16,
    flags: u32,
    created: u64,
}

#[derive(Clone, Copy)]
struct State {
    generation: u64,
    next_identity: u64,
    total_blocks: u32,
    allocation: [u8; ALLOCATION_BYTES],
    objects: [ObjectRecord; MAX_OBJECTS],
    versions: [VersionRecord; MAX_VERSIONS],
    entries: [NamespaceRecord; MAX_ENTRIES],
    relationships: [RelationshipRecord; MAX_RELATIONSHIPS],
}

impl State {
    // ------------------------=
    // FUNC: empty
    // DESC: Implements the empty operation.
    // ------------------=
    const fn empty(total_blocks: u32) -> Self {
        Self {
            generation: 0,
            next_identity: 1,
            total_blocks,
            allocation: [0; ALLOCATION_BYTES],
            objects: [ObjectRecord {
                used: false,
                tombstone: false,
                id: ObjectId([0; 16]),
                kind: 0,
                space: 0,
                current_version: 0,
                created: 0,
                modified: 0,
                name_len: 0,
                name: [0; 47],
                owner: ObjectId([0; 16]),
                flags: 0,
                content_type: 0,
                tags_len: 0,
                tags: [0; 8],
            }; MAX_OBJECTS],
            versions: [VersionRecord {
                used: false,
                object: ObjectId([0; 16]),
                number: 0,
                extent: 0,
                blocks: 0,
                size: 0,
                content_crc: 0,
                parent: 0,
                generation: 0,
            }; MAX_VERSIONS],
            entries: [NamespaceRecord {
                used: false,
                target: ObjectId([0; 16]),
                path_len: 0,
                path: [0; MAX_PATH],
            }; MAX_ENTRIES],
            relationships: [RelationshipRecord {
                used: false,
                source: ObjectId([0; 16]),
                target: ObjectId([0; 16]),
                kind: 0,
                flags: 0,
                created: 0,
            }; MAX_RELATIONSHIPS],
        }
    }
}

pub struct ObjectStore<D: BlockDevice> {
    device: D,
    container_lba: u64,
    state: State,
    mounted_root: u8,
    in_transaction: bool,
}

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: format
    // DESC: Implements the format operation.
    // ------------------=
    pub fn format(
        device: D,
        container_lba: u64,
        container_blocks: u64,
        seed: [u8; 16],
    ) -> Result<Self, ObjectError> {
        let available = container_blocks
            .checked_sub(STORE_RELATIVE_LBA + CONTENT)
            .ok_or(ObjectError::InsufficientCapacity)?
            / ALLOCATION_BLOCK_SECTORS;
        let mut store = Self {
            device,
            container_lba,
            state: State::empty(available.min((ALLOCATION_BYTES * 8) as u64) as u32),
            mounted_root: 0,
            in_transaction: false,
        };
        store.state.next_identity =
            u64::from_le_bytes(seed[..8].try_into().unwrap_or([1; 8])).max(1);
        store.initialize_namespace()?;
        store.commit()?;
        Ok(store)
    }

    // ------------------------=
    // FUNC: mount
    // DESC: Implements the mount operation.
    // ------------------=
    pub fn mount(mut device: D, container_lba: u64) -> Result<Self, ObjectError> {
        let first = read_root(&mut device, container_lba, ROOT_A);
        let second = read_root(&mut device, container_lba, ROOT_B);
        let unsupported = matches!(first, Err(ObjectError::UnsupportedFormat))
            || matches!(second, Err(ObjectError::UnsupportedFormat));
        let first = first.unwrap_or(None);
        let second = second.unwrap_or(None);
        let mut candidates = [(first, 0u8), (second, 1u8)];
        if candidates[1].0.map(|x| x.0).unwrap_or(0) > candidates[0].0.map(|x| x.0).unwrap_or(0) {
            candidates.swap(0, 1);
        }
        for (root, slot) in candidates {
            if let Some((generation, bank)) = root {
                if let Ok(state) = read_bank(&mut device, container_lba, bank, generation) {
                    return Ok(Self {
                        device,
                        container_lba,
                        state,
                        mounted_root: slot,
                        in_transaction: false,
                    });
                }
            }
        }
        if unsupported {
            Err(ObjectError::UnsupportedFormat)
        } else {
            Err(ObjectError::CorruptMetadata)
        }
    }

    // ------------------------=
    // FUNC: initialize_namespace
    // DESC: Initializes initialize namespace state.
    // ------------------=
    fn initialize_namespace(&mut self) -> Result<(), ObjectError> {
        let paths: [&[u8]; 15] = [
            b"/",
            b"/home",
            b"/home/default",
            b"/home/default/documents",
            b"/home/default/pictures",
            b"/home/default/media",
            b"/home/default/projects",
            b"/home/default/downloads",
            b"/home/default/archive",
            b"/apps",
            b"/shared",
            b"/devices",
            b"/system",
            b"/projects",
            b"/trash",
        ];
        for path in paths {
            let name = path.rsplit(|c| *c == b'/').next().unwrap_or(b"root");
            let space =
                if path == b"/" || path == b"/system" || path == b"/trash" || path == b"/devices" {
                    Space::System
                } else if path == b"/apps" {
                    Space::Applications
                } else {
                    Space::Personal
                };
            let id = self.create_record(name, ObjectType::NamespaceNode, space)?;
            self.attach_record(path, id)?;
        }
        let system = self.create_record(
            b"installed-kernel",
            ObjectType::SystemComponent,
            Space::System,
        )?;
        self.write_record(system, b"Milestone 3A boot-region compatibility")?;
        let recovery =
            self.create_record(b"recovery-state", ObjectType::Metadata, Space::Recovery)?;
        self.write_record(recovery, b"native object-store recovery metadata")?;
        // Versioned native binary bootstrap objects. These are deliberately
        // small enough to load before the human namespace/service database.
        let runtime =
            self.create_record(b"runtime-core", ObjectType::SystemComponent, Space::System)?;
        self.write_record(
            runtime,
            &[
                b'I', b'N', b'F', b'R', b'U', b'N', b'1', 0, 1, 0, 0, 0, 1, 0, 0, 0,
            ],
        )?;
        self.attach_record(b"/system/runtime", runtime)?;
        let registry =
            self.create_record(b"service-registry", ObjectType::Metadata, Space::System)?;
        let mut service_registry = [0u8; 128];
        service_registry[..8].copy_from_slice(b"INFSVC1\0");
        service_registry[8..10].copy_from_slice(&1u16.to_le_bytes());
        service_registry[10..12].copy_from_slice(&28u16.to_le_bytes());
        for id in 1..=28u32 {
            let at = 12 + (id as usize - 1) * 4;
            service_registry[at..at + 4].copy_from_slice(&id.to_le_bytes());
        }
        self.write_record(registry, &service_registry)?;
        self.attach_record(b"/system/service-registry", registry)?;
        let policy =
            self.create_record(b"capability-policy", ObjectType::Metadata, Space::System)?;
        // INFCAP1, schema v1, deny-ambient, followed by the first durable
        // security Record: System.ServiceInstalled for the bootstrap registry.
        self.write_record(
            policy,
            &[
                b'I', b'N', b'F', b'C', b'A', b'P', b'1', 0, 1, 0, 1, 0, b'R', b'E', b'C', b'1', 1,
                0, 0, 0, 1, 0, 0, 0,
            ],
        )?;
        self.attach_record(b"/system/capability-policy", policy)?;
        let model = self.create_record(
            b"local-intent-v1",
            ObjectType::SystemComponent,
            Space::System,
        )?;
        self.write_record(model, &ai_model_asset::model_object_bytes())?;
        self.attach_record(b"/system/models/local-intent-v1", model)?;
        let ai_service =
            self.create_record(b"ai-bootstrap", ObjectType::SystemComponent, Space::System)?;
        let dialogue = ai_service;
        let creative = ai_service;
        let mut ai_bootstrap = [0u8; 96 + ai_generation_asset::CONVERSATION_MODEL_OBJECT_BYTES * 2];
        ai_bootstrap[..8].copy_from_slice(b"INFAI1\0\0");
        ai_bootstrap[8..12].copy_from_slice(&2u32.to_le_bytes());
        ai_bootstrap[12..16].copy_from_slice(&3u32.to_le_bytes());
        ai_bootstrap[16..32].copy_from_slice(&model.0);
        ai_bootstrap[32..36].copy_from_slice(&ai_model_asset::LOCAL_INTENT_MODEL_ID.to_le_bytes());
        ai_bootstrap[36..40].copy_from_slice(&ai_model_asset::local_model_checksum().to_le_bytes());
        ai_bootstrap[40..56].copy_from_slice(&dialogue.0);
        ai_bootstrap[56..60].copy_from_slice(&ai_generation_asset::DIALOGUE_MODEL_ID.to_le_bytes());
        ai_bootstrap[60..64].copy_from_slice(
            &ai_generation_asset::model_checksum(ai_generation_asset::DIALOGUE_MODEL_ID)
                .to_le_bytes(),
        );
        ai_bootstrap[64..80].copy_from_slice(&creative.0);
        ai_bootstrap[80..84].copy_from_slice(&ai_generation_asset::CREATIVE_MODEL_ID.to_le_bytes());
        ai_bootstrap[84..88].copy_from_slice(
            &ai_generation_asset::model_checksum(ai_generation_asset::CREATIVE_MODEL_ID)
                .to_le_bytes(),
        );
        let dialogue_at = 96;
        let creative_at = dialogue_at + ai_generation_asset::CONVERSATION_MODEL_OBJECT_BYTES;
        ai_bootstrap[dialogue_at..creative_at].copy_from_slice(
            &ai_generation_asset::model_object_bytes(ai_generation_asset::DIALOGUE_MODEL_ID),
        );
        ai_bootstrap[creative_at..].copy_from_slice(&ai_generation_asset::model_object_bytes(
            ai_generation_asset::CREATIVE_MODEL_ID,
        ));
        self.write_record(ai_service, &ai_bootstrap)?;
        self.attach_record(b"/system/ai/bootstrap", ai_service)?;
        let voice = self.create_record(
            b"voice-framework",
            ObjectType::SystemComponent,
            Space::System,
        )?;
        self.write_record(
            voice,
            b"INFVOICE\x01\0push-to-talk\0capability-required\0speech-provider-unavailable",
        )?;
        self.attach_record(b"/system/voice/runtime", voice)?;
        let agents = self.create_record(b"agent-policy", ObjectType::Metadata, Space::System)?;
        self.write_record(
            agents,
            b"INFAGENT\x01\0explicit-tools\0bounded-tasks\0no-inherited-user-authority",
        )?;
        self.attach_record(b"/system/agents/policy", agents)?;
        let organization =
            self.create_record(b"organization-schema", ObjectType::Metadata, Space::System)?;
        self.write_record(organization, &organization::organization_schema_object())?;
        self.attach_record(b"/system/organization/schema", organization)?;
        let identity =
            self.create_record(b"identity-state", ObjectType::IdentityData, Space::System)?;
        self.write_record(identity, b"INFIDN1\0FIRST-BOOT-REQUIRED")?;
        self.attach_record(b"/system/identity/state", identity)?;
        let network_state =
            self.create_record(b"network-state", ObjectType::Metadata, Space::System)?;
        let mut state = [0u8; 32];
        state[..8].copy_from_slice(b"INFNET01");
        state[8..10].copy_from_slice(&1u16.to_le_bytes());
        state[12..16].copy_from_slice(&1u32.to_le_bytes());
        state[16..24].copy_from_slice(&1u64.to_le_bytes());
        self.write_record(network_state, &state)?;
        self.attach_record(b"/system/network/state", network_state)?;
        let shell_profiles =
            self.create_record(b"shell-profile-state", ObjectType::Metadata, Space::System)?;
        self.write_record(shell_profiles, b"INFSHL01\x01\0")?;
        self.attach_record(b"/system/settings/shell/profiles", shell_profiles)?;
        Ok(())
    }

    // ------------------------=
    // FUNC: runtime_bootstrap_valid
    // DESC: Implements the runtime bootstrap valid operation.
    // ------------------=
    pub fn runtime_bootstrap_valid(&mut self) -> bool {
        for (path, magic) in [
            (b"/system/runtime".as_slice(), b"INFRUN1".as_slice()),
            (
                b"/system/service-registry".as_slice(),
                b"INFSVC1".as_slice(),
            ),
            (
                b"/system/capability-policy".as_slice(),
                b"INFCAP1".as_slice(),
            ),
            (b"/system/ai/bootstrap".as_slice(), b"INFAI1".as_slice()),
            (
                b"/system/models/local-intent-v1".as_slice(),
                b"INFMLM1".as_slice(),
            ),
            (b"/system/voice/runtime".as_slice(), b"INFVOICE".as_slice()),
            (b"/system/agents/policy".as_slice(), b"INFAGENT".as_slice()),
            (
                b"/system/organization/schema".as_slice(),
                b"INFOORG1".as_slice(),
            ),
            (b"/system/identity/state".as_slice(), b"INFIDN1".as_slice()),
            (b"/system/network/state".as_slice(), b"INFNET01".as_slice()),
            (
                b"/system/settings/shell/profiles".as_slice(),
                b"INFSHL01".as_slice(),
            ),
        ] {
            let Ok(id) = self.resolve(path) else {
                return false;
            };
            // Bootstrap System objects evolve after first boot. In particular, the
            // native identity-state object grows beyond the compact model image
            // size once onboarding commits the machine and user identities.
            // Validate against the Object Store's real content ceiling so a valid
            // installed generation is not rejected after that transition.
            let mut content = [0u8; MAX_CONTENT];
            let Ok(length) = self.read(id, None, &mut content) else {
                return false;
            };
            if length < magic.len() || &content[..magic.len()] != magic {
                return false;
            }
            if path == b"/system/models/local-intent-v1"
                && !ai_model_asset::model_object_valid(&content[..length])
            {
                return false;
            }
            if path == b"/system/organization/schema"
                && !organization::organization_schema_valid(&content[..length])
            {
                return false;
            }
        }
        let Ok(bootstrap_id) = self.resolve(b"/system/ai/bootstrap") else {
            return false;
        };
        let Ok(model_id) = self.resolve(b"/system/models/local-intent-v1") else {
            return false;
        };
        let dialogue_id = bootstrap_id;
        let creative_id = bootstrap_id;
        let mut bootstrap = [0u8; MAX_CONTENT];
        let Ok(length) = self.read(bootstrap_id, None, &mut bootstrap) else {
            return false;
        };
        let dialogue_valid = ai_generation_asset::model_object_valid(
            ai_generation_asset::DIALOGUE_MODEL_ID,
            bootstrap
                .get(96..96 + ai_generation_asset::CONVERSATION_MODEL_OBJECT_BYTES)
                .unwrap_or(&[]),
        );
        let creative_valid = ai_generation_asset::model_object_valid(
            ai_generation_asset::CREATIVE_MODEL_ID,
            bootstrap
                .get(
                    96 + ai_generation_asset::CONVERSATION_MODEL_OBJECT_BYTES
                        ..96 + ai_generation_asset::CONVERSATION_MODEL_OBJECT_BYTES * 2,
                )
                .unwrap_or(&[]),
        );
        if length < 88
            || !dialogue_valid
            || !creative_valid
            || bootstrap[16..32] != model_id.0
            || u32::from_le_bytes(bootstrap[36..40].try_into().unwrap_or([0; 4]))
                != ai_model_asset::local_model_checksum()
            || bootstrap[40..56] != dialogue_id.0
            || u32::from_le_bytes(bootstrap[56..60].try_into().unwrap_or([0; 4]))
                != ai_generation_asset::DIALOGUE_MODEL_ID
            || u32::from_le_bytes(bootstrap[60..64].try_into().unwrap_or([0; 4]))
                != ai_generation_asset::model_checksum(ai_generation_asset::DIALOGUE_MODEL_ID)
            || bootstrap[64..80] != creative_id.0
            || u32::from_le_bytes(bootstrap[80..84].try_into().unwrap_or([0; 4]))
                != ai_generation_asset::CREATIVE_MODEL_ID
            || u32::from_le_bytes(bootstrap[84..88].try_into().unwrap_or([0; 4]))
                != ai_generation_asset::model_checksum(ai_generation_asset::CREATIVE_MODEL_ID)
        {
            return false;
        }
        true
    }

    // ------------------------=
    // FUNC: install_date_time_configuration
    // DESC: Persists the installer-selected local wall time and typed time-zone identity as a native System object.
    // ------------------=
    pub fn install_date_time_configuration(
        &mut self,
        configuration: DateTimeConfiguration,
    ) -> Result<ObjectId, ObjectError> {
        if !configuration.is_valid() {
            return Err(ObjectError::InvalidObject);
        }
        let mut content = [0u8; 32];
        content[..8].copy_from_slice(b"INFTIME1");
        content[8..10].copy_from_slice(&1u16.to_le_bytes());
        content[10..12].copy_from_slice(&32u16.to_le_bytes());
        content[12..14].copy_from_slice(&configuration.year.to_le_bytes());
        content[14] = configuration.month;
        content[15] = configuration.day;
        content[16] = configuration.hour;
        content[17] = configuration.minute;
        content[18] = configuration.second;
        content[19] = 1; // Installer-local wall-clock source.
        content[20..22].copy_from_slice(&configuration.time_zone_id.to_le_bytes());
        content[22..24].copy_from_slice(&configuration.utc_offset_minutes.to_le_bytes());
        self.create_attached(
            b"date-time-settings",
            ObjectType::Metadata,
            Space::System,
            &content,
            b"/system/settings/date-time",
        )
    }

    // ------------------------=
    // FUNC: date_time_configuration
    // DESC: Reads and validates the installed native date-and-time configuration object.
    // ------------------=
    pub fn date_time_configuration(&mut self) -> Option<DateTimeConfiguration> {
        let object = self.resolve(b"/system/settings/date-time").ok()?;
        let mut content = [0u8; 32];
        let length = self.read(object, None, &mut content).ok()?;
        if length != content.len()
            || &content[..8] != b"INFTIME1"
            || u16::from_le_bytes(content[8..10].try_into().ok()?) != 1
            || u16::from_le_bytes(content[10..12].try_into().ok()?) as usize != content.len()
        {
            return None;
        }
        let configuration = DateTimeConfiguration {
            year: u16::from_le_bytes(content[12..14].try_into().ok()?),
            month: content[14],
            day: content[15],
            hour: content[16],
            minute: content[17],
            second: content[18],
            time_zone_id: u16::from_le_bytes(content[20..22].try_into().ok()?),
            utc_offset_minutes: i16::from_le_bytes(content[22..24].try_into().ok()?),
        };
        configuration.is_valid().then_some(configuration)
    }

    // ------------------------=
    // FUNC: next_id
    // DESC: Calculates and returns next id.
    // ------------------=
    fn next_id(&mut self) -> ObjectId {
        let mut x = self.state.next_identity
            ^ self.state.generation.rotate_left(17)
            ^ 0x9e37_79b9_7f4a_7c15;
        let mut out = [0u8; 16];
        for byte in &mut out {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *byte = x as u8;
        }
        out[6] = (out[6] & 0x0f) | 0x40;
        out[8] = (out[8] & 0x3f) | 0x80;
        self.state.next_identity = self.state.next_identity.wrapping_add(0x9e37_79b9).max(1);
        ObjectId(out)
    }

    // ------------------------=
    // FUNC: create_record
    // DESC: Writes or updates create record data.
    // ------------------=
    fn create_record(
        &mut self,
        name: &[u8],
        kind: ObjectType,
        space: Space,
    ) -> Result<ObjectId, ObjectError> {
        if name.len() > 47 || core::str::from_utf8(name).is_err() {
            return Err(ObjectError::InvalidObject);
        }
        let count = self.state.objects.iter().take_while(|x| x.used).count();
        if count == MAX_OBJECTS {
            return Err(ObjectError::InsufficientCapacity);
        }
        let id = self.next_id();
        let mut stored = [0u8; 47];
        stored[..name.len()].copy_from_slice(name);
        let slot = self.state.objects[..count]
            .binary_search_by_key(&id, |x| x.id)
            .unwrap_or_else(|x| x);
        for i in (slot..count).rev() {
            self.state.objects[i + 1] = self.state.objects[i];
        }
        self.state.objects[slot] = ObjectRecord {
            used: true,
            tombstone: false,
            id,
            kind: kind as u8,
            space: space as u8,
            current_version: 0,
            created: self.state.generation + 1,
            modified: self.state.generation + 1,
            name_len: name.len() as u8,
            name: stored,
            owner: ObjectId([0; 16]),
            flags: 0,
            content_type: default_content_type(kind) as u16,
            tags_len: 0,
            tags: [0; 8],
        };
        Ok(id)
    }

    // ------------------------=
    // FUNC: create
    // DESC: Implements the create operation.
    // ------------------=
    pub fn create(
        &mut self,
        name: &[u8],
        kind: ObjectType,
        space: Space,
        content: &[u8],
    ) -> Result<ObjectId, ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            let id = self.create_record(name, kind, space)?;
            self.write_record(id, content)?;
            Ok(id)
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: create_attached
    // DESC: Writes or updates create attached data.
    // ------------------=
    pub fn create_attached(
        &mut self,
        name: &[u8],
        kind: ObjectType,
        space: Space,
        content: &[u8],
        path: &[u8],
    ) -> Result<ObjectId, ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            validate_path(path)?;
            if self
                .state
                .entries
                .iter()
                .any(|entry| entry.used && entry.path() == path)
            {
                return Err(ObjectError::NameConflict);
            }
            let id = self.create_record(name, kind, space)?;
            self.write_record(id, content)?;
            self.attach_record(path, id)?;
            Ok(id)
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: copy_attached
    // DESC: Copies content and metadata into a distinct ObjectId and atomically attaches its destination reference.
    // ------------------=
    pub fn copy_attached(
        &mut self,
        source: ObjectId,
        destination: &[u8],
    ) -> Result<ObjectId, ObjectError> {
        let before = self.begin()?;
        let result = self.copy_record_attached(source, destination);
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: copy_path_attached
    // DESC: Atomically duplicates one namespace entry and every descendant beneath a new path.
    // ------------------=
    pub fn copy_path_attached(
        &mut self,
        source_path: &[u8],
        destination: &[u8],
    ) -> Result<ObjectId, ObjectError> {
        validate_path(source_path)?;
        validate_path(destination)?;
        if source_path == destination
            || (destination.starts_with(source_path)
                && destination.get(source_path.len()) == Some(&b'/'))
        {
            return Err(ObjectError::InvalidPath);
        }
        let source = self.resolve(source_path)?;
        for entry in self.state.entries.iter().filter(|entry| entry.used) {
            let path = entry.path();
            if path != source_path
                && !(path.starts_with(source_path) && path.get(source_path.len()) == Some(&b'/'))
            {
                continue;
            }
            let suffix = &path[source_path.len()..];
            if destination.len() + suffix.len() > MAX_PATH {
                return Err(ObjectError::InvalidPath);
            }
            let mut candidate = [0u8; MAX_PATH];
            candidate[..destination.len()].copy_from_slice(destination);
            candidate[destination.len()..destination.len() + suffix.len()].copy_from_slice(suffix);
            let candidate = &candidate[..destination.len() + suffix.len()];
            if self
                .state
                .entries
                .iter()
                .any(|current| current.used && current.path() == candidate)
            {
                return Err(ObjectError::NameConflict);
            }
        }

        let before = self.begin()?;
        let result = (|| {
            let root = self.copy_record_attached(source, destination)?;
            for index in 0..MAX_ENTRIES {
                let entry = self.state.entries[index];
                if !entry.used {
                    continue;
                }
                let path = entry.path();
                if path == source_path
                    || !path.starts_with(source_path)
                    || path.get(source_path.len()) != Some(&b'/')
                {
                    continue;
                }
                let suffix = &path[source_path.len()..];
                let mut target = [0u8; MAX_PATH];
                target[..destination.len()].copy_from_slice(destination);
                target[destination.len()..destination.len() + suffix.len()].copy_from_slice(suffix);
                self.copy_record_attached(
                    entry.target,
                    &target[..destination.len() + suffix.len()],
                )?;
            }
            Ok(root)
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: copy_record_attached
    // DESC: Copies one object record inside an existing transaction, including versionless namespace nodes.
    // ------------------=
    fn copy_record_attached(
        &mut self,
        source: ObjectId,
        destination: &[u8],
    ) -> Result<ObjectId, ObjectError> {
        let source_index = self.object_index(source)?;
        let source_record = self.state.objects[source_index];
        if source_record.tombstone {
            return Err(ObjectError::NotFound);
        }
        let mut content = [0u8; MAX_CONTENT];
        let content_length = if source_record.current_version == 0 {
            0
        } else {
            self.read(source, None, &mut content)?
        };
        let name = destination
            .rsplit(|byte| *byte == b'/')
            .next()
            .unwrap_or(&[]);
        let id = self.create_record(
            name,
            type_from_u8(source_record.kind)?,
            space_from_u8(source_record.space)?,
        )?;
        let target_index = self.object_index(id)?;
        self.state.objects[target_index].owner = source_record.owner;
        self.state.objects[target_index].flags = source_record.flags;
        self.state.objects[target_index].content_type = source_record.content_type;
        self.state.objects[target_index].tags_len = source_record.tags_len;
        self.state.objects[target_index].tags = source_record.tags;
        if source_record.current_version != 0 {
            self.write_record(id, &content[..content_length])?;
        }
        self.attach_record(destination, id)?;
        Ok(id)
    }

    // ------------------------=
    // FUNC: delete_namespace
    // DESC: Atomically removes an empty Namespace reference and rejects non-empty containers by default.
    // ------------------=
    pub fn delete_namespace(&mut self, path: &[u8]) -> Result<ObjectId, ObjectError> {
        if matches!(
            path,
            b"/" | b"/system" | b"/home" | b"/apps" | b"/shared" | b"/recovery" | b"/trash"
        ) {
            return Err(ObjectError::Unauthorized);
        }
        let id = self.resolve(path)?;
        let metadata = self.metadata(id)?;
        if metadata.kind != ObjectType::NamespaceNode {
            return Err(ObjectError::InvalidObject);
        }
        let has_child = self.state.entries.iter().any(|entry| {
            entry.used
                && entry.path().len() > path.len()
                && entry.path().starts_with(path)
                && entry.path().get(path.len()) == Some(&b'/')
        });
        if has_child {
            return Err(ObjectError::Busy);
        }
        self.remove_path(path)
    }

    // ------------------------=
    // FUNC: destroy_explicit
    // DESC: Permanently tombstones a non-system ObjectId only through an explicit authorized path.
    // ------------------=
    pub fn destroy_explicit(&mut self, id: ObjectId, authorized: bool) -> Result<(), ObjectError> {
        if !authorized {
            return Err(ObjectError::Unauthorized);
        }
        let before = self.begin()?;
        let result = (|| {
            let object = self.object_index(id)?;
            if self.state.objects[object].space == Space::System as u8
                || self.state.objects[object].flags & 1 != 0
                || self.state.relationships.iter().any(|relationship| {
                    relationship.used && (relationship.source == id || relationship.target == id)
                })
            {
                return Err(ObjectError::Unauthorized);
            }
            for entry in self
                .state
                .entries
                .iter_mut()
                .filter(|entry| entry.used && entry.target == id)
            {
                entry.used = false;
            }
            self.state.objects[object].tombstone = true;
            self.state.objects[object].modified = self.state.generation + 1;
            Ok(())
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: write
    // DESC: Implements the write operation.
    // ------------------=
    pub fn write(&mut self, id: ObjectId, content: &[u8]) -> Result<u32, ObjectError> {
        let before = self.begin()?;
        let result = self.write_record(id, content);
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: write_record
    // DESC: Writes or updates write record data.
    // ------------------=
    fn write_record(&mut self, id: ObjectId, content: &[u8]) -> Result<u32, ObjectError> {
        if content.len() > MAX_CONTENT {
            return Err(ObjectError::InsufficientCapacity);
        }
        let object = self.object_index(id)?;
        if self.state.objects[object].tombstone {
            return Err(ObjectError::NotFound);
        }
        let version = self.state.objects[object]
            .current_version
            .checked_add(1)
            .ok_or(ObjectError::InvalidVersion)?;
        let vslot = self
            .state
            .versions
            .iter()
            .position(|x| !x.used)
            .ok_or(ObjectError::InsufficientCapacity)?;
        let blocks = ((content.len() + 4095) / 4096).max(1) as u16;
        let space = space_from_u8(self.state.objects[object].space)?;
        let extent = self.allocate(space, blocks)?;
        self.write_content(extent, blocks, content)?;
        let crc = crc32(content);
        self.state.versions[vslot] = VersionRecord {
            used: true,
            object: id,
            number: version,
            extent,
            blocks,
            size: content.len() as u32,
            content_crc: crc,
            parent: self.state.objects[object].current_version,
            generation: self.state.generation + 1,
        };
        self.state.objects[object].current_version = version;
        self.state.objects[object].modified = self.state.generation + 1;
        Ok(version)
    }

    // ------------------------=
    // FUNC: read
    // DESC: Implements the read operation.
    // ------------------=
    pub fn read(
        &mut self,
        id: ObjectId,
        version: Option<u32>,
        out: &mut [u8],
    ) -> Result<usize, ObjectError> {
        let oi = self.object_index(id)?;
        if self.state.objects[oi].tombstone {
            return Err(ObjectError::NotFound);
        }
        let number = version.unwrap_or(self.state.objects[oi].current_version);
        let v = self
            .state
            .versions
            .iter()
            .find(|v| v.used && v.object == id && v.number == number)
            .copied()
            .ok_or(ObjectError::InvalidVersion)?;
        if v.size as usize > out.len() {
            return Err(ObjectError::InsufficientCapacity);
        }
        self.read_content(v.extent, v.blocks, &mut out[..v.size as usize])?;
        if crc32(&out[..v.size as usize]) != v.content_crc {
            return Err(ObjectError::CorruptContent);
        }
        Ok(v.size as usize)
    }

    // ------------------------=
    // FUNC: restore
    // DESC: Implements the restore operation.
    // ------------------=
    pub fn restore(&mut self, id: ObjectId, version: u32) -> Result<u32, ObjectError> {
        let mut data = [0u8; MAX_CONTENT];
        let size = self.read(id, Some(version), &mut data)?;
        self.write(id, &data[..size])
    }

    // ------------------------=
    // FUNC: attach
    // DESC: Implements the attach operation.
    // ------------------=
    pub fn attach(&mut self, path: &[u8], id: ObjectId) -> Result<(), ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            self.object_index(id)?;
            self.attach_record(path, id)
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: attach_record
    // DESC: Writes or updates attach record data.
    // ------------------=
    fn attach_record(&mut self, path: &[u8], id: ObjectId) -> Result<(), ObjectError> {
        validate_path(path)?;
        if self
            .state
            .entries
            .iter()
            .any(|x| x.used && x.path() == path)
        {
            return Err(ObjectError::NameConflict);
        }
        let slot = self
            .state
            .entries
            .iter()
            .position(|x| !x.used)
            .ok_or(ObjectError::InsufficientCapacity)?;
        let mut stored = [0u8; MAX_PATH];
        stored[..path.len()].copy_from_slice(path);
        self.state.entries[slot] = NamespaceRecord {
            used: true,
            target: id,
            path_len: path.len() as u8,
            path: stored,
        };
        Ok(())
    }

    // ------------------------=
    // FUNC: resolve
    // DESC: Implements the resolve operation.
    // ------------------=
    pub fn resolve(&self, path: &[u8]) -> Result<ObjectId, ObjectError> {
        if path.starts_with(b"/objects/") {
            let id = parse_id(&path[9..]).ok_or(ObjectError::InvalidPath)?;
            let slot = self.object_index(id)?;
            if self.state.objects[slot].tombstone {
                return Err(ObjectError::NotFound);
            }
            return Ok(id);
        }
        self.state
            .entries
            .iter()
            .find(|x| x.used && x.path() == path)
            .map(|x| x.target)
            .ok_or(ObjectError::NamespaceNotFound)
    }

    // ------------------------=
    // FUNC: move_entry
    // DESC: Implements the move entry operation.
    // ------------------=
    pub fn move_entry(&mut self, from: &[u8], to: &[u8]) -> Result<(), ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            validate_path(from)?;
            validate_path(to)?;
            if from == to || (to.starts_with(from) && to.get(from.len()) == Some(&b'/')) {
                return Err(ObjectError::InvalidPath);
            }
            if !self
                .state
                .entries
                .iter()
                .any(|entry| entry.used && entry.path() == from)
            {
                return Err(ObjectError::NamespaceNotFound);
            }
            for entry in self.state.entries.iter().filter(|entry| entry.used) {
                let path = entry.path();
                if path != from && !(path.starts_with(from) && path.get(from.len()) == Some(&b'/'))
                {
                    continue;
                }
                let suffix = &path[from.len()..];
                if to.len() + suffix.len() > MAX_PATH {
                    return Err(ObjectError::InvalidPath);
                }
                let mut candidate = [0u8; MAX_PATH];
                candidate[..to.len()].copy_from_slice(to);
                candidate[to.len()..to.len() + suffix.len()].copy_from_slice(suffix);
                let candidate = &candidate[..to.len() + suffix.len()];
                if self.state.entries.iter().any(|current| {
                    current.used
                        && current.path() != path
                        && !(current.path().starts_with(from)
                            && current.path().get(from.len()) == Some(&b'/'))
                        && current.path() == candidate
                }) {
                    return Err(ObjectError::NameConflict);
                }
            }
            for entry in self.state.entries.iter_mut().filter(|entry| entry.used) {
                let path = entry.path();
                if path != from && !(path.starts_with(from) && path.get(from.len()) == Some(&b'/'))
                {
                    continue;
                }
                let suffix_length = path.len() - from.len();
                let mut moved = [0u8; MAX_PATH];
                moved[..to.len()].copy_from_slice(to);
                moved[to.len()..to.len() + suffix_length].copy_from_slice(&path[from.len()..]);
                entry.path = moved;
                entry.path_len = (to.len() + suffix_length) as u8;
            }
            Ok(())
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: detach
    // DESC: Implements the detach operation.
    // ------------------=
    pub fn detach(&mut self, path: &[u8]) -> Result<(), ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            let slot = self
                .state
                .entries
                .iter()
                .position(|x| x.used && x.path() == path)
                .ok_or(ObjectError::NamespaceNotFound)?;
            self.state.entries[slot].used = false;
            Ok(())
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: remove
    // DESC: Implements the remove operation.
    // ------------------=
    pub fn remove(&mut self, id: ObjectId) -> Result<(), ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            if self.state.entries.iter().any(|x| x.used && x.target == id) {
                return Err(ObjectError::NameConflict);
            }
            let slot = self.object_index(id)?;
            self.state.objects[slot].tombstone = true;
            self.state.objects[slot].modified = self.state.generation + 1;
            Ok(())
        })();
        self.finish(before, result)
    }

    /// Removes the final namespace reference and tombstones its object in one
    /// generation. If another name still references the object, only this path
    /// is detached and the object remains live.
    // ------------------------=
    // FUNC: remove_path
    // DESC: Removes or invalidates remove path state.
    // ------------------=
    pub fn remove_path(&mut self, path: &[u8]) -> Result<ObjectId, ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            let slot = self
                .state
                .entries
                .iter()
                .position(|x| x.used && x.path() == path)
                .ok_or(ObjectError::NamespaceNotFound)?;
            let id = self.state.entries[slot].target;
            self.state.entries[slot].used = false;
            if !self.state.entries.iter().any(|x| x.used && x.target == id) {
                let object = self.object_index(id)?;
                self.state.objects[object].tombstone = true;
                self.state.objects[object].modified = self.state.generation + 1;
            }
            Ok(id)
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: update_metadata
    // DESC: Writes or updates update metadata data.
    // ------------------=
    pub fn update_metadata(
        &mut self,
        request: ObjectMetadataUpdateRequest<'_>,
    ) -> Result<(), ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            if request.tags.len() > 8 || core::str::from_utf8(request.tags).is_err() {
                return Err(ObjectError::InvalidObject);
            }
            let i = self.object_index(request.object.id)?;
            let mut tags = [0u8; 8];
            tags[..request.tags.len()].copy_from_slice(request.tags);
            let record = &mut self.state.objects[i];
            record.owner = request.owner;
            record.content_type = request.content_type as u16;
            record.flags = request.flags;
            record.tags = tags;
            record.tags_len = request.tags.len() as u8;
            record.modified = self.state.generation + 1;
            Ok(())
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: metadata
    // DESC: Implements the metadata operation.
    // ------------------=
    pub fn metadata(&self, id: ObjectId) -> Result<ObjectMetadata, ObjectError> {
        let i = self.object_index(id)?;
        let o = self.state.objects[i];
        Ok(ObjectMetadata {
            object: ObjectRef { id },
            kind: type_from_u8(o.kind)?,
            space: space_from_u8(o.space)?,
            owner: o.owner,
            content_type: content_type_from_u16(o.content_type)?,
            flags: o.flags,
            current_version: o.current_version,
            logical_size: self.current_size(id).unwrap_or(0),
            created: o.created,
            modified: o.modified,
            tags: o.tags,
            tags_len: o.tags_len,
        })
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns metadata and reference count without requiring a content version.
    // ------------------=
    pub fn inspect(&self, id: ObjectId) -> Result<(ObjectMetadata, usize), ObjectError> {
        Ok((self.metadata(id)?, self.namespace_refs(id)))
    }

    // ------------------------=
    // FUNC: relationship_attach
    // DESC: Implements the relationship attach operation.
    // ------------------=
    pub fn relationship_attach(
        &mut self,
        request: RelationshipAttachRequest,
    ) -> Result<(), ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            self.object_index(request.source.id)?;
            self.object_index(request.target.id)?;
            if self.state.relationships.iter().any(|r| {
                r.used
                    && r.source == request.source.id
                    && r.target == request.target.id
                    && r.kind == request.kind as u16
            }) {
                return Err(ObjectError::NameConflict);
            }
            let slot = self
                .state
                .relationships
                .iter()
                .position(|r| !r.used)
                .ok_or(ObjectError::InsufficientCapacity)?;
            self.state.relationships[slot] = RelationshipRecord {
                used: true,
                source: request.source.id,
                target: request.target.id,
                kind: request.kind as u16,
                flags: request.flags,
                created: self.state.generation + 1,
            };
            Ok(())
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: relationship_detach
    // DESC: Implements the relationship detach operation.
    // ------------------=
    pub fn relationship_detach(
        &mut self,
        request: RelationshipDetachRequest,
    ) -> Result<(), ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            let slot = self
                .state
                .relationships
                .iter()
                .position(|r| {
                    r.used
                        && r.source == request.source.id
                        && r.target == request.target.id
                        && r.kind == request.kind as u16
                })
                .ok_or(ObjectError::NotFound)?;
            self.state.relationships[slot].used = false;
            Ok(())
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: relationship_nth
    // DESC: Implements the relationship nth operation.
    // ------------------=
    pub fn relationship_nth(&self, source: ObjectId, index: usize) -> Option<RelationshipResult> {
        self.state
            .relationships
            .iter()
            .filter(|r| r.used && r.source == source)
            .nth(index)
            .and_then(|r| {
                Some(RelationshipResult {
                    source: ObjectRef { id: r.source },
                    kind: relationship_from_u16(r.kind).ok()?,
                    target: ObjectRef { id: r.target },
                    flags: r.flags,
                })
            })
    }

    // ------------------------=
    // FUNC: query_nth
    // DESC: Reads query nth data.
    // ------------------=
    pub fn query_nth(
        &self,
        request: ObjectQueryRequest,
        index: usize,
    ) -> Option<ObjectQueryResult> {
        self.state
            .objects
            .iter()
            .filter(|o| o.used && (request.include_tombstones || !o.tombstone))
            .filter(|o| request.kind.map(|v| v as u8 == o.kind).unwrap_or(true))
            .filter(|o| request.space.map(|v| v as u8 == o.space).unwrap_or(true))
            .nth(index)
            .and_then(|o| {
                let kind = type_from_u8(o.kind).ok()?;
                let space = space_from_u8(o.space).ok()?;
                Some(ObjectQueryResult {
                    object: ObjectRef { id: o.id },
                    kind,
                    space,
                    display_name: o.name,
                    display_name_length: o.name_len,
                    current_version: o.current_version,
                    logical_size: self.current_size(o.id).unwrap_or(0),
                })
            })
    }

    /// Deterministic, explicitly invoked reclamation. Only tombstoned objects
    /// with no namespace or relationship references are eligible.
    // ------------------------=
    // FUNC: collect
    // DESC: Implements the collect operation.
    // ------------------=
    pub fn collect(&mut self) -> Result<u32, ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            let mut reclaimed = 0u32;
            for oi in 0..MAX_OBJECTS {
                let o = self.state.objects[oi];
                if !o.used
                    || !o.tombstone
                    || self
                        .state
                        .entries
                        .iter()
                        .any(|e| e.used && e.target == o.id)
                    || self
                        .state
                        .relationships
                        .iter()
                        .any(|r| r.used && (r.source == o.id || r.target == o.id))
                {
                    continue;
                }
                for vi in 0..MAX_VERSIONS {
                    let v = self.state.versions[vi];
                    if v.used && v.object == o.id {
                        for b in v.extent as usize..v.extent as usize + v.blocks as usize {
                            set_bit(&mut self.state.allocation, b, false);
                            reclaimed += 1;
                        }
                        self.state.versions[vi].used = false;
                    }
                }
                self.state.objects[oi].used = false;
            }
            self.compact_objects();
            Ok(reclaimed)
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: generation
    // DESC: Implements the generation operation.
    // ------------------=
    pub fn generation(&self) -> u64 {
        self.state.generation
    }
    // ------------------------=
    // FUNC: history_count
    // DESC: Implements the history count operation.
    // ------------------=
    pub fn history_count(&self, id: ObjectId) -> usize {
        self.state
            .versions
            .iter()
            .filter(|v| v.used && v.object == id)
            .count()
    }
    // ------------------------=
    // FUNC: history_version_nth
    // DESC: Implements the history version nth operation.
    // ------------------=
    pub fn history_version_nth(&self, id: ObjectId, index: usize) -> Option<(u32, bool)> {
        let current = self.current_version(id).ok()?;
        self.state
            .versions
            .iter()
            .filter(|v| v.used && v.object == id)
            .nth(index)
            .map(|v| (v.number, v.number == current))
    }
    // ------------------------=
    // FUNC: namespace_refs
    // DESC: Implements the namespace refs operation.
    // ------------------=
    pub fn namespace_refs(&self, id: ObjectId) -> usize {
        self.state
            .entries
            .iter()
            .filter(|e| e.used && e.target == id)
            .count()
    }

    // ------------------------=
    // FUNC: namespace_ref_nth
    // DESC: Returns one authorized human Namespace reference for a stable ObjectId.
    // ------------------=
    pub fn namespace_ref_nth(&self, id: ObjectId, index: usize) -> Option<&[u8]> {
        self.state
            .entries
            .iter()
            .filter(|entry| entry.used && entry.target == id)
            .nth(index)
            .map(NamespaceRecord::path)
    }
    // ------------------------=
    // FUNC: usage_blocks
    // DESC: Implements the usage blocks operation.
    // ------------------=
    pub fn usage_blocks(&self) -> u32 {
        self.state.allocation.iter().map(|b| b.count_ones()).sum()
    }
    // ------------------------=
    // FUNC: usage_by_space
    // DESC: Implements the usage by space operation.
    // ------------------=
    pub fn usage_by_space(&self, space: Space) -> u32 {
        self.state
            .versions
            .iter()
            .filter(|v| v.used)
            .map(|v| {
                self.object_index(v.object)
                    .ok()
                    .filter(|i| self.state.objects[*i].space == space as u8)
                    .map(|_| v.blocks as u32)
                    .unwrap_or(0)
            })
            .sum()
    }
    // ------------------------=
    // FUNC: total_blocks
    // DESC: Implements the total blocks operation.
    // ------------------=
    pub fn total_blocks(&self) -> u32 {
        self.state.total_blocks
    }
    // ------------------------=
    // FUNC: object_exists
    // DESC: Implements the object exists operation.
    // ------------------=
    pub fn object_exists(&self, id: ObjectId) -> bool {
        self.object_index(id)
            .map(|slot| !self.state.objects[slot].tombstone)
            .unwrap_or(false)
    }
    // ------------------------=
    // FUNC: current_version
    // DESC: Implements the current version operation.
    // ------------------=
    pub fn current_version(&self, id: ObjectId) -> Result<u32, ObjectError> {
        Ok(self.state.objects[self.object_index(id)?].current_version)
    }
    // ------------------------=
    // FUNC: current_size
    // DESC: Implements the current size operation.
    // ------------------=
    pub fn current_size(&self, id: ObjectId) -> Result<u32, ObjectError> {
        let version = self.current_version(id)?;
        self.state
            .versions
            .iter()
            .find(|v| v.used && v.object == id && v.number == version)
            .map(|v| v.size)
            .ok_or(ObjectError::InvalidVersion)
    }
    // ------------------------=
    // FUNC: namespace_entry
    // DESC: Implements the namespace entry operation.
    // ------------------=
    pub fn namespace_entry(&self, index: usize) -> Option<(&[u8], ObjectId)> {
        self.state
            .entries
            .iter()
            .filter(|e| e.used)
            .nth(index)
            .map(|e| (e.path(), e.target))
    }
    // ------------------------=
    // FUNC: namespace_list_nth
    // DESC: Implements the namespace list nth operation.
    // ------------------=
    pub fn namespace_list_nth(&self, prefix: &[u8], index: usize) -> Option<NamespaceListResult> {
        self.state
            .entries
            .iter()
            .filter(|e| e.used && e.path().starts_with(prefix))
            .nth(index)
            .map(|e| NamespaceListResult {
                path: e.path,
                path_len: e.path_len,
                object: ObjectRef { id: e.target },
            })
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Implements the begin operation.
    // ------------------=
    fn begin(&mut self) -> Result<State, ObjectError> {
        if self.in_transaction {
            return Err(ObjectError::Busy);
        }
        self.in_transaction = true;
        Ok(self.state)
    }
    // ------------------------=
    // FUNC: finish
    // DESC: Implements the finish operation.
    // ------------------=
    fn finish<T>(
        &mut self,
        before: State,
        result: Result<T, ObjectError>,
    ) -> Result<T, ObjectError> {
        match result {
            Ok(value) => match self.commit() {
                Ok(()) => {
                    self.in_transaction = false;
                    Ok(value)
                }
                Err(e) => {
                    self.state = before;
                    self.in_transaction = false;
                    Err(e)
                }
            },
            Err(e) => {
                self.state = before;
                self.in_transaction = false;
                Err(e)
            }
        }
    }

    // ------------------------=
    // FUNC: object_index
    // DESC: Implements the object index operation.
    // ------------------=
    fn object_index(&self, id: ObjectId) -> Result<usize, ObjectError> {
        let count = self.state.objects.iter().take_while(|x| x.used).count();
        self.state.objects[..count]
            .binary_search_by_key(&id, |x| x.id)
            .map_err(|_| ObjectError::NotFound)
    }
    // ------------------------=
    // FUNC: compact_objects
    // DESC: Implements the compact objects operation.
    // ------------------=
    fn compact_objects(&mut self) {
        let mut to = 0;
        for from in 0..MAX_OBJECTS {
            if self.state.objects[from].used {
                if from != to {
                    self.state.objects[to] = self.state.objects[from];
                    self.state.objects[from] = ObjectRecord::default();
                }
                to += 1;
            }
        }
    }
    // ------------------------=
    // FUNC: allocate
    // DESC: Implements the allocate operation.
    // ------------------=
    fn allocate(&mut self, _space: Space, count: u16) -> Result<u32, ObjectError> {
        let total = self.state.total_blocks as usize;
        let need = count as usize;
        for start in 0..=total.saturating_sub(need) {
            if (start..start + need).all(|i| !bit(&self.state.allocation, i)) {
                for i in start..start + need {
                    set_bit(&mut self.state.allocation, i, true);
                }
                return Ok(start as u32);
            }
        }
        Err(ObjectError::InsufficientCapacity)
    }
    // ------------------------=
    // FUNC: write_content
    // DESC: Writes or updates write content data.
    // ------------------=
    fn write_content(&mut self, extent: u32, blocks: u16, data: &[u8]) -> Result<(), ObjectError> {
        let base = self.container_lba
            + STORE_RELATIVE_LBA
            + CONTENT
            + extent as u64 * ALLOCATION_BLOCK_SECTORS;
        for sector in 0..blocks as u64 * ALLOCATION_BLOCK_SECTORS {
            let mut bytes = [0u8; 512];
            let at = sector as usize * 512;
            if at < data.len() {
                let n = (data.len() - at).min(512);
                bytes[..n].copy_from_slice(&data[at..at + n]);
            }
            if !self.device.write_sector(base + sector, &bytes) {
                return Err(ObjectError::TransactionFailed);
            }
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: read_content
    // DESC: Reads read content data.
    // ------------------=
    fn read_content(
        &mut self,
        extent: u32,
        blocks: u16,
        out: &mut [u8],
    ) -> Result<(), ObjectError> {
        let base = self.container_lba
            + STORE_RELATIVE_LBA
            + CONTENT
            + extent as u64 * ALLOCATION_BLOCK_SECTORS;
        for sector in 0..blocks as u64 * ALLOCATION_BLOCK_SECTORS {
            let at = sector as usize * 512;
            if at >= out.len() {
                break;
            }
            let mut bytes = [0u8; 512];
            if !self.device.read_sector(base + sector, &mut bytes) {
                return Err(ObjectError::CorruptContent);
            }
            let n = (out.len() - at).min(512);
            out[at..at + n].copy_from_slice(&bytes[..n]);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: commit
    // DESC: Implements the commit operation.
    // ------------------=
    fn commit(&mut self) -> Result<(), ObjectError> {
        let generation = self.state.generation + 1;
        let bank = if self.mounted_root == 0 {
            BANK_B
        } else {
            BANK_A
        };
        self.state.generation = generation;
        write_bank(&mut self.device, self.container_lba, bank, &self.state)?;
        if !self.device.flush() {
            return Err(ObjectError::TransactionFailed);
        }
        let root = if self.mounted_root == 0 {
            ROOT_B
        } else {
            ROOT_A
        };
        write_root(&mut self.device, self.container_lba, root, generation, bank)?;
        if !self.device.flush() {
            return Err(ObjectError::TransactionFailed);
        }
        self.mounted_root ^= 1;
        Ok(())
    }
}

/// Typed machine boundary. Policy validation occurs before every object-store
/// operation; callers never parse console text or receive block addresses.
pub struct ObjectService<'a, D: BlockDevice, P: ObjectCapabilityPolicy> {
    store: &'a mut ObjectStore<D>,
    policy: &'a P,
}
impl<'a, D: BlockDevice, P: ObjectCapabilityPolicy> ObjectService<'a, D, P> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub fn new(store: &'a mut ObjectStore<D>, policy: &'a P) -> Self {
        Self { store, policy }
    }
    // ------------------------=
    // FUNC: allow
    // DESC: Implements the allow operation.
    // ------------------=
    fn allow(
        &self,
        operation: ObjectOperation,
        object: Option<ObjectRef>,
    ) -> Result<(), ObjectError> {
        if self.policy.authorize(operation, object) {
            Ok(())
        } else {
            Err(ObjectError::Unauthorized)
        }
    }
    // ------------------------=
    // FUNC: create
    // DESC: Implements the create operation.
    // ------------------=
    pub fn create(&mut self, r: ObjectCreateRequest<'_>) -> Result<ObjectRef, ObjectError> {
        self.allow(ObjectOperation::Create, None)?;
        self.store
            .create(r.name, r.kind, r.space, r.content)
            .map(|id| ObjectRef { id })
    }
    // ------------------------=
    // FUNC: read
    // DESC: Implements the read operation.
    // ------------------=
    pub fn read(
        &mut self,
        r: ObjectReadRequest,
        out: &mut [u8],
    ) -> Result<ObjectReadResult, ObjectError> {
        self.allow(ObjectOperation::Read, Some(r.object))?;
        let mut full = [0u8; MAX_CONTENT];
        let size = self.store.read(r.object.id, r.version, &mut full)?;
        let start = r.offset as usize;
        if start > size {
            return Err(ObjectError::InvalidObject);
        }
        let requested = if r.length == 0 {
            size - start
        } else {
            r.length as usize
        };
        let count = requested.min(size - start);
        if count > out.len() {
            return Err(ObjectError::InsufficientCapacity);
        }
        out[..count].copy_from_slice(&full[start..start + count]);
        Ok(ObjectReadResult {
            object: r.object,
            version: r
                .version
                .unwrap_or(self.store.current_version(r.object.id)?),
            logical_size: size as u32,
            content_crc: crc32(&full[..size]),
        })
    }
    // ------------------------=
    // FUNC: update
    // DESC: Implements the update operation.
    // ------------------=
    pub fn update(&mut self, r: ObjectUpdateRequest<'_>) -> Result<u32, ObjectError> {
        self.allow(ObjectOperation::Update, Some(r.object))?;
        self.store.write(r.object.id, r.content)
    }
    // ------------------------=
    // FUNC: query
    // DESC: Implements the query operation.
    // ------------------=
    pub fn query(
        &self,
        r: ObjectQueryRequest,
        index: usize,
    ) -> Result<Option<ObjectQueryResult>, ObjectError> {
        self.allow(ObjectOperation::Query, None)?;
        Ok(self.store.query_nth(r, index))
    }
    // ------------------------=
    // FUNC: delete
    // DESC: Implements the delete operation.
    // ------------------=
    pub fn delete(&mut self, r: ObjectDeleteRequest) -> Result<(), ObjectError> {
        self.allow(ObjectOperation::Delete, Some(r.object))?;
        self.store.remove(r.object.id)
    }
    // ------------------------=
    // FUNC: history
    // DESC: Implements the history operation.
    // ------------------=
    pub fn history(&self, r: ObjectHistoryRequest) -> Result<ObjectHistoryResult, ObjectError> {
        self.allow(ObjectOperation::History, Some(r.object))?;
        Ok(ObjectHistoryResult {
            object: r.object,
            versions: self.store.history_count(r.object.id) as u32,
            current_version: self.store.current_version(r.object.id)?,
        })
    }
    // ------------------------=
    // FUNC: resolve
    // DESC: Implements the resolve operation.
    // ------------------=
    pub fn resolve(&self, r: NamespaceResolveRequest<'_>) -> Result<ObjectRef, ObjectError> {
        self.allow(ObjectOperation::NamespaceResolve, None)?;
        self.store.resolve(r.path).map(|id| ObjectRef { id })
    }
    // ------------------------=
    // FUNC: list
    // DESC: Implements the list operation.
    // ------------------=
    pub fn list(
        &self,
        r: NamespaceListRequest<'_>,
    ) -> Result<Option<NamespaceListResult>, ObjectError> {
        self.allow(ObjectOperation::NamespaceList, None)?;
        Ok(self.store.namespace_list_nth(r.prefix, r.index))
    }
    // ------------------------=
    // FUNC: attach
    // DESC: Implements the attach operation.
    // ------------------=
    pub fn attach(&mut self, r: NamespaceAttachRequest<'_>) -> Result<(), ObjectError> {
        self.allow(ObjectOperation::NamespaceAttach, Some(r.object))?;
        self.store.attach(r.path, r.object.id)
    }
    // ------------------------=
    // FUNC: detach
    // DESC: Implements the detach operation.
    // ------------------=
    pub fn detach(&mut self, r: NamespaceDetachRequest<'_>) -> Result<(), ObjectError> {
        self.allow(ObjectOperation::NamespaceDetach, None)?;
        self.store.detach(r.path)
    }
    // ------------------------=
    // FUNC: move_entry
    // DESC: Implements the move entry operation.
    // ------------------=
    pub fn move_entry(&mut self, r: NamespaceMoveRequest<'_>) -> Result<(), ObjectError> {
        self.allow(ObjectOperation::NamespaceMove, None)?;
        self.store.move_entry(r.from, r.to)
    }
    // ------------------------=
    // FUNC: relationship_attach
    // DESC: Implements the relationship attach operation.
    // ------------------=
    pub fn relationship_attach(&mut self, r: RelationshipAttachRequest) -> Result<(), ObjectError> {
        self.allow(ObjectOperation::RelationshipAttach, Some(r.source))?;
        self.store.relationship_attach(r)
    }
    // ------------------------=
    // FUNC: relationship_detach
    // DESC: Implements the relationship detach operation.
    // ------------------=
    pub fn relationship_detach(&mut self, r: RelationshipDetachRequest) -> Result<(), ObjectError> {
        self.allow(ObjectOperation::RelationshipDetach, Some(r.source))?;
        self.store.relationship_detach(r)
    }
    // ------------------------=
    // FUNC: relationship
    // DESC: Implements the relationship operation.
    // ------------------=
    pub fn relationship(
        &self,
        source: ObjectRef,
        index: usize,
    ) -> Result<Option<RelationshipResult>, ObjectError> {
        self.allow(ObjectOperation::RelationshipQuery, Some(source))?;
        Ok(self.store.relationship_nth(source.id, index))
    }
    // ------------------------=
    // FUNC: collect
    // DESC: Implements the collect operation.
    // ------------------=
    pub fn collect(&mut self) -> Result<u32, ObjectError> {
        self.allow(ObjectOperation::Collect, None)?;
        self.store.collect()
    }
}

impl NamespaceRecord {
    // ------------------------=
    // FUNC: path
    // DESC: Implements the path operation.
    // ------------------=
    fn path(&self) -> &[u8] {
        &self.path[..self.path_len as usize]
    }
}

// ------------------------=
// FUNC: validate_path
// DESC: Implements the validate path operation.
// ------------------=
fn validate_path(path: &[u8]) -> Result<(), ObjectError> {
    if path.is_empty()
        || path.len() > MAX_PATH
        || path[0] != b'/'
        || core::str::from_utf8(path).is_err()
        || path.windows(2).any(|x| x == b"//")
        || path.ends_with(b"/") && path != b"/"
        || path
            .split(|c| *c == b'/')
            .any(|x| x == b".." || x == b"." || x.len() > MAX_COMPONENT)
    {
        Err(ObjectError::InvalidPath)
    } else {
        Ok(())
    }
}

// ------------------------=
// FUNC: default_content_type
// DESC: Implements the default content type operation.
// ------------------=
fn default_content_type(kind: ObjectType) -> ContentType {
    match kind {
        ObjectType::Blob => ContentType::Binary,
        ObjectType::Text => ContentType::Utf8Text,
        ObjectType::NamespaceNode => ContentType::Namespace,
        ObjectType::SystemComponent => ContentType::System,
        ObjectType::ApplicationData => ContentType::Application,
        ObjectType::Metadata
        | ObjectType::Collection
        | ObjectType::Project
        | ObjectType::IdentityData
        | ObjectType::DeviceData => ContentType::Metadata,
        ObjectType::Model => ContentType::System,
    }
}
// ------------------------=
// FUNC: type_from_u8
// DESC: Implements the type from u8 operation.
// ------------------=
fn type_from_u8(value: u8) -> Result<ObjectType, ObjectError> {
    match value {
        1 => Ok(ObjectType::Blob),
        2 => Ok(ObjectType::Text),
        3 => Ok(ObjectType::NamespaceNode),
        4 => Ok(ObjectType::SystemComponent),
        5 => Ok(ObjectType::ApplicationData),
        6 => Ok(ObjectType::Metadata),
        7 => Ok(ObjectType::Collection),
        8 => Ok(ObjectType::Project),
        9 => Ok(ObjectType::Model),
        10 => Ok(ObjectType::IdentityData),
        11 => Ok(ObjectType::DeviceData),
        _ => Err(ObjectError::CorruptMetadata),
    }
}
// ------------------------=
// FUNC: space_from_u8
// DESC: Implements the space from u8 operation.
// ------------------=
fn space_from_u8(value: u8) -> Result<Space, ObjectError> {
    match value {
        1 => Ok(Space::System),
        2 => Ok(Space::Personal),
        3 => Ok(Space::Applications),
        4 => Ok(Space::Recovery),
        _ => Err(ObjectError::CorruptMetadata),
    }
}
// ------------------------=
// FUNC: content_type_from_u16
// DESC: Implements the content type from u16 operation.
// ------------------=
fn content_type_from_u16(value: u16) -> Result<ContentType, ObjectError> {
    match value {
        1 => Ok(ContentType::Binary),
        2 => Ok(ContentType::Utf8Text),
        3 => Ok(ContentType::Namespace),
        4 => Ok(ContentType::System),
        5 => Ok(ContentType::Application),
        6 => Ok(ContentType::Metadata),
        _ => Err(ObjectError::CorruptMetadata),
    }
}
// ------------------------=
// FUNC: relationship_from_u16
// DESC: Implements the relationship from u16 operation.
// ------------------=
fn relationship_from_u16(value: u16) -> Result<RelationshipType, ObjectError> {
    match value {
        1 => Ok(RelationshipType::MemberOf),
        2 => Ok(RelationshipType::DerivedFrom),
        3 => Ok(RelationshipType::References),
        4 => Ok(RelationshipType::GeneratedBy),
        5 => Ok(RelationshipType::BelongsToProject),
        6 => Ok(RelationshipType::ContainedBy),
        7 => Ok(RelationshipType::RelatedTo),
        8 => Ok(RelationshipType::VersionOf),
        9 => Ok(RelationshipType::OwnedBy),
        10 => Ok(RelationshipType::SharedWith),
        11 => Ok(RelationshipType::Favorite),
        12 => Ok(RelationshipType::Pinned),
        _ => Err(ObjectError::CorruptMetadata),
    }
}

// ------------------------=
// FUNC: bit
// DESC: Implements the bit operation.
// ------------------=
fn bit(map: &[u8; ALLOCATION_BYTES], index: usize) -> bool {
    map[index / 8] & (1 << (index % 8)) != 0
}
// ------------------------=
// FUNC: set_bit
// DESC: Writes or updates set bit data.
// ------------------=
fn set_bit(map: &mut [u8; ALLOCATION_BYTES], index: usize, value: bool) {
    if value {
        map[index / 8] |= 1 << (index % 8)
    } else {
        map[index / 8] &= !(1 << (index % 8))
    }
}

// ------------------------=
// FUNC: write_root
// DESC: Writes or updates write root data.
// ------------------=
fn write_root<D: BlockDevice>(
    d: &mut D,
    c: u64,
    slot: u64,
    g: u64,
    bank: u64,
) -> Result<(), ObjectError> {
    let mut s = [0u8; 512];
    s[..8].copy_from_slice(b"INFOROOT");
    put32(&mut s, 8, FORMAT_VERSION);
    put64(&mut s, 16, g);
    put64(&mut s, 24, bank);
    finish_sector(&mut s);
    if d.write_sector(c + STORE_RELATIVE_LBA + slot, &s) {
        Ok(())
    } else {
        Err(ObjectError::TransactionFailed)
    }
}
// ------------------------=
// FUNC: read_root
// DESC: Reads read root data.
// ------------------=
fn read_root<D: BlockDevice>(
    d: &mut D,
    c: u64,
    slot: u64,
) -> Result<Option<(u64, u64)>, ObjectError> {
    let mut s = [0u8; 512];
    if !d.read_sector(c + STORE_RELATIVE_LBA + slot, &mut s)
        || &s[..8] != b"INFOROOT"
        || !valid_sector(&s)
    {
        return Ok(None);
    }
    if get32(&s, 8) != FORMAT_VERSION {
        return Err(ObjectError::UnsupportedFormat);
    }
    let bank = get64(&s, 24);
    if bank != BANK_A && bank != BANK_B {
        return Ok(None);
    }
    Ok(Some((get64(&s, 16), bank)))
}

// ------------------------=
// FUNC: write_bank
// DESC: Writes or updates write bank data.
// ------------------=
fn write_bank<D: BlockDevice>(
    d: &mut D,
    c: u64,
    bank: u64,
    state: &State,
) -> Result<(), ObjectError> {
    let base = c + STORE_RELATIVE_LBA + bank;
    let mut h = [0u8; 512];
    h[..8].copy_from_slice(b"INFOSTAT");
    put32(&mut h, 8, FORMAT_VERSION);
    put64(&mut h, 16, state.generation);
    put64(&mut h, 24, state.next_identity);
    put32(&mut h, 32, state.total_blocks);
    put32(&mut h, 36, ALLOCATION_BYTES as u32);
    finish_sector(&mut h);
    write(d, base, &h)?;
    for sector in 0..4 {
        let mut s = [0u8; 512];
        s[..8].copy_from_slice(b"INFOALC2");
        put32(&mut s, 8, FORMAT_VERSION);
        let start = sector * 492;
        let end = (start + 492).min(ALLOCATION_BYTES);
        s[16..16 + end - start].copy_from_slice(&state.allocation[start..end]);
        finish_sector(&mut s);
        write(d, base + 1 + sector as u64, &s)?;
    }
    for sector in 0..OBJECT_TABLE_SECTORS {
        let mut s = [0u8; 512];
        s[..8].copy_from_slice(b"INFOOBJ2");
        put32(&mut s, 8, FORMAT_VERSION);
        for n in 0..OBJECTS_PER_SECTOR {
            encode_object(
                &state.objects[sector * OBJECTS_PER_SECTOR + n],
                &mut s,
                16 + n * 120,
            );
        }
        finish_sector(&mut s);
        // The first six sectors retain their v2 locations. The two expansion
        // sectors use previously reserved bank space after relationships.
        let offset = if sector < 6 {
            5 + sector as u64
        } else {
            25 + (sector - 6) as u64
        };
        write(d, base + offset, &s)?;
    }
    for sector in 0..4 {
        let mut s = [0u8; 512];
        s[..8].copy_from_slice(b"INFOVER2");
        put32(&mut s, 8, FORMAT_VERSION);
        for n in 0..8 {
            encode_version(&state.versions[sector * 8 + n], &mut s, 16 + n * 60);
        }
        finish_sector(&mut s);
        write(d, base + 11 + sector as u64, &s)?;
    }
    for sector in 0..8 {
        let mut s = [0u8; 512];
        s[..8].copy_from_slice(b"INFONSP2");
        put32(&mut s, 8, FORMAT_VERSION);
        for n in 0..4 {
            encode_entry(&state.entries[sector * 4 + n], &mut s, 16 + n * 120);
        }
        finish_sector(&mut s);
        write(d, base + 15 + sector as u64, &s)?;
    }
    for sector in 0..2 {
        let mut s = [0u8; 512];
        s[..8].copy_from_slice(b"INFOREL2");
        put32(&mut s, 8, FORMAT_VERSION);
        for n in 0..8 {
            encode_relationship(&state.relationships[sector * 8 + n], &mut s, 16 + n * 60);
        }
        finish_sector(&mut s);
        write(d, base + 23 + sector as u64, &s)?;
    }
    Ok(())
}

// ------------------------=
// FUNC: read_bank
// DESC: Reads read bank data.
// ------------------=
fn read_bank<D: BlockDevice>(d: &mut D, c: u64, bank: u64, g: u64) -> Result<State, ObjectError> {
    let base = c + STORE_RELATIVE_LBA + bank;
    let h = read(d, base)?;
    if &h[..8] != b"INFOSTAT"
        || get32(&h, 8) != FORMAT_VERSION
        || get64(&h, 16) != g
        || !valid_sector(&h)
    {
        return Err(ObjectError::CorruptMetadata);
    }
    let total = get32(&h, 32);
    if total as usize > ALLOCATION_BYTES * 8 || get32(&h, 36) != ALLOCATION_BYTES as u32 {
        return Err(ObjectError::CorruptMetadata);
    }
    let mut state = State::empty(total);
    state.generation = g;
    state.next_identity = get64(&h, 24);
    for sector in 0..4 {
        let s = read(d, base + 1 + sector as u64)?;
        check(&s, b"INFOALC2")?;
        let start = sector * 492;
        let end = (start + 492).min(ALLOCATION_BYTES);
        state.allocation[start..end].copy_from_slice(&s[16..16 + end - start]);
    }
    for sector in 0..OBJECT_TABLE_SECTORS {
        let offset = if sector < 6 {
            5 + sector as u64
        } else {
            25 + (sector - 6) as u64
        };
        let s = read(d, base + offset)?;
        check(&s, b"INFOOBJ2")?;
        for n in 0..OBJECTS_PER_SECTOR {
            state.objects[sector * OBJECTS_PER_SECTOR + n] = decode_object(&s, 16 + n * 120)?;
        }
    }
    for sector in 0..4 {
        let s = read(d, base + 11 + sector as u64)?;
        check(&s, b"INFOVER2")?;
        for n in 0..8 {
            state.versions[sector * 8 + n] = decode_version(&s, 16 + n * 60)?;
        }
    }
    for sector in 0..8 {
        let s = read(d, base + 15 + sector as u64)?;
        check(&s, b"INFONSP2")?;
        for n in 0..4 {
            state.entries[sector * 4 + n] = decode_entry(&s, 16 + n * 120)?;
        }
    }
    for sector in 0..2 {
        let s = read(d, base + 23 + sector as u64)?;
        check(&s, b"INFOREL2")?;
        for n in 0..8 {
            state.relationships[sector * 8 + n] = decode_relationship(&s, 16 + n * 60)?;
        }
    }
    validate_state(&state)?;
    Ok(state)
}

// ------------------------=
// FUNC: encode_object
// DESC: Implements the encode object operation.
// ------------------=
fn encode_object(x: &ObjectRecord, s: &mut [u8], o: usize) {
    s[o] = x.used as u8;
    s[o + 1] = x.tombstone as u8;
    s[o + 2] = x.kind;
    s[o + 3] = x.space;
    s[o + 4..o + 20].copy_from_slice(&x.id.0);
    put32(s, o + 20, x.current_version);
    put64(s, o + 24, x.created);
    put64(s, o + 32, x.modified);
    s[o + 40] = x.name_len;
    s[o + 41..o + 88].copy_from_slice(&x.name);
    s[o + 88..o + 104].copy_from_slice(&x.owner.0);
    put32(s, o + 104, x.flags);
    put16(s, o + 108, x.content_type);
    s[o + 110] = x.tags_len;
    s[o + 111..o + 119].copy_from_slice(&x.tags)
}
// ------------------------=
// FUNC: decode_object
// DESC: Implements the decode object operation.
// ------------------=
fn decode_object(s: &[u8], o: usize) -> Result<ObjectRecord, ObjectError> {
    let len = s[o + 40] as usize;
    if len > 47 {
        return Err(ObjectError::CorruptMetadata);
    };
    let tags_len = s[o + 110] as usize;
    if tags_len > 8 {
        return Err(ObjectError::CorruptMetadata);
    }
    let mut id = [0; 16];
    id.copy_from_slice(&s[o + 4..o + 20]);
    let mut name = [0; 47];
    name.copy_from_slice(&s[o + 41..o + 88]);
    let mut owner = [0; 16];
    owner.copy_from_slice(&s[o + 88..o + 104]);
    let mut tags = [0; 8];
    tags.copy_from_slice(&s[o + 111..o + 119]);
    if core::str::from_utf8(&name[..len]).is_err() {
        return Err(ObjectError::CorruptMetadata);
    }
    Ok(ObjectRecord {
        used: s[o] != 0,
        tombstone: s[o + 1] != 0,
        id: ObjectId(id),
        kind: s[o + 2],
        space: s[o + 3],
        current_version: get32(s, o + 20),
        created: get64(s, o + 24),
        modified: get64(s, o + 32),
        name_len: len as u8,
        name,
        owner: ObjectId(owner),
        flags: get32(s, o + 104),
        content_type: get16(s, o + 108),
        tags_len: tags_len as u8,
        tags,
    })
}
// ------------------------=
// FUNC: encode_version
// DESC: Implements the encode version operation.
// ------------------=
fn encode_version(x: &VersionRecord, s: &mut [u8], o: usize) {
    s[o] = x.used as u8;
    s[o + 4..o + 20].copy_from_slice(&x.object.0);
    put32(s, o + 20, x.number);
    put32(s, o + 24, x.extent);
    put16(s, o + 28, x.blocks);
    put32(s, o + 32, x.size);
    put32(s, o + 36, x.content_crc);
    put32(s, o + 40, x.parent);
    put64(s, o + 44, x.generation)
}
// ------------------------=
// FUNC: decode_version
// DESC: Implements the decode version operation.
// ------------------=
fn decode_version(s: &[u8], o: usize) -> Result<VersionRecord, ObjectError> {
    let mut id = [0; 16];
    id.copy_from_slice(&s[o + 4..o + 20]);
    let blocks = get16(s, o + 28);
    if blocks as usize > (MAX_CONTENT / 4096) || get32(s, o + 32) as usize > MAX_CONTENT {
        return Err(ObjectError::CorruptMetadata);
    }
    Ok(VersionRecord {
        used: s[o] != 0,
        object: ObjectId(id),
        number: get32(s, o + 20),
        extent: get32(s, o + 24),
        blocks,
        size: get32(s, o + 32),
        content_crc: get32(s, o + 36),
        parent: get32(s, o + 40),
        generation: get64(s, o + 44),
    })
}
// ------------------------=
// FUNC: encode_entry
// DESC: Implements the encode entry operation.
// ------------------=
fn encode_entry(x: &NamespaceRecord, s: &mut [u8], o: usize) {
    s[o] = x.used as u8;
    s[o + 1] = x.path_len;
    s[o + 4..o + 20].copy_from_slice(&x.target.0);
    s[o + 20..o + 115].copy_from_slice(&x.path)
}
// ------------------------=
// FUNC: decode_entry
// DESC: Implements the decode entry operation.
// ------------------=
fn decode_entry(s: &[u8], o: usize) -> Result<NamespaceRecord, ObjectError> {
    let len = s[o + 1] as usize;
    if len > MAX_PATH {
        return Err(ObjectError::CorruptMetadata);
    };
    let mut id = [0; 16];
    id.copy_from_slice(&s[o + 4..o + 20]);
    let mut path = [0; MAX_PATH];
    path.copy_from_slice(&s[o + 20..o + 115]);
    if s[o] != 0 {
        validate_path(&path[..len]).map_err(|_| ObjectError::CorruptMetadata)?;
    }
    Ok(NamespaceRecord {
        used: s[o] != 0,
        target: ObjectId(id),
        path_len: len as u8,
        path,
    })
}

// ------------------------=
// FUNC: encode_relationship
// DESC: Implements the encode relationship operation.
// ------------------=
fn encode_relationship(x: &RelationshipRecord, s: &mut [u8], o: usize) {
    s[o] = x.used as u8;
    put16(s, o + 2, x.kind);
    s[o + 4..o + 20].copy_from_slice(&x.source.0);
    s[o + 20..o + 36].copy_from_slice(&x.target.0);
    put32(s, o + 36, x.flags);
    put64(s, o + 40, x.created)
}
// ------------------------=
// FUNC: decode_relationship
// DESC: Implements the decode relationship operation.
// ------------------=
fn decode_relationship(s: &[u8], o: usize) -> Result<RelationshipRecord, ObjectError> {
    let mut source = [0; 16];
    source.copy_from_slice(&s[o + 4..o + 20]);
    let mut target = [0; 16];
    target.copy_from_slice(&s[o + 20..o + 36]);
    let record = RelationshipRecord {
        used: s[o] != 0,
        kind: get16(s, o + 2),
        source: ObjectId(source),
        target: ObjectId(target),
        flags: get32(s, o + 36),
        created: get64(s, o + 40),
    };
    if record.used {
        relationship_from_u16(record.kind)?;
    }
    Ok(record)
}

// ------------------------=
// FUNC: validate_state
// DESC: Implements the validate state operation.
// ------------------=
fn validate_state(state: &State) -> Result<(), ObjectError> {
    let mut saw_unused = false;
    let mut prior = None;
    for o in &state.objects {
        if !o.used {
            saw_unused = true;
            continue;
        }
        if saw_unused {
            return Err(ObjectError::CorruptMetadata);
        }
        type_from_u8(o.kind)?;
        space_from_u8(o.space)?;
        content_type_from_u16(o.content_type)?;
        if let Some(id) = prior {
            if id >= o.id {
                return Err(ObjectError::CorruptMetadata);
            }
        }
        prior = Some(o.id);
        if o.current_version > 0
            && !state
                .versions
                .iter()
                .any(|v| v.used && v.object == o.id && v.number == o.current_version)
        {
            return Err(ObjectError::CorruptMetadata);
        }
    }
    for v in &state.versions {
        if !v.used {
            continue;
        }
        if v.blocks == 0
            || v.size as usize > (v.blocks as usize * 4096)
            || (v.extent as u64 + v.blocks as u64) > state.total_blocks as u64
        {
            return Err(ObjectError::CorruptMetadata);
        }
        if !state.objects.iter().any(|o| o.used && o.id == v.object) {
            return Err(ObjectError::CorruptMetadata);
        }
        for b in v.extent as usize..v.extent as usize + v.blocks as usize {
            if !bit(&state.allocation, b) {
                return Err(ObjectError::CorruptMetadata);
            }
        }
    }
    for (i, e) in state.entries.iter().enumerate() {
        if !e.used {
            continue;
        }
        if !state.objects.iter().any(|o| o.used && o.id == e.target) {
            return Err(ObjectError::CorruptMetadata);
        }
        if state.entries[..i]
            .iter()
            .any(|other| other.used && other.path() == e.path())
        {
            return Err(ObjectError::CorruptMetadata);
        }
    }
    for r in &state.relationships {
        if !r.used {
            continue;
        }
        relationship_from_u16(r.kind)?;
        if !state.objects.iter().any(|o| o.used && o.id == r.source)
            || !state.objects.iter().any(|o| o.used && o.id == r.target)
        {
            return Err(ObjectError::CorruptMetadata);
        }
    }
    Ok(())
}

// ------------------------=
// FUNC: parse_id
// DESC: Implements the parse id operation.
// ------------------=
fn parse_id(bytes: &[u8]) -> Option<ObjectId> {
    if bytes.len() != 32 {
        return None;
    }
    let mut out = [0; 16];
    for i in 0..16 {
        out[i] = (hex(bytes[i * 2])? << 4) | hex(bytes[i * 2 + 1])?;
    }
    Some(ObjectId(out))
}
// ------------------------=
// FUNC: hex
// DESC: Implements the hex operation.
// ------------------=
fn hex(x: u8) -> Option<u8> {
    match x {
        b'0'..=b'9' => Some(x - b'0'),
        b'a'..=b'f' => Some(x - b'a' + 10),
        b'A'..=b'F' => Some(x - b'A' + 10),
        _ => None,
    }
}
// ------------------------=
// FUNC: write
// DESC: Implements the write operation.
// ------------------=
fn write<D: BlockDevice>(d: &mut D, l: u64, s: &[u8; 512]) -> Result<(), ObjectError> {
    if d.write_sector(l, s) {
        Ok(())
    } else {
        Err(ObjectError::TransactionFailed)
    }
}
// ------------------------=
// FUNC: read
// DESC: Implements the read operation.
// ------------------=
fn read<D: BlockDevice>(d: &mut D, l: u64) -> Result<[u8; 512], ObjectError> {
    let mut s = [0; 512];
    if d.read_sector(l, &mut s) {
        Ok(s)
    } else {
        Err(ObjectError::CorruptMetadata)
    }
}
// ------------------------=
// FUNC: check
// DESC: Implements the check operation.
// ------------------=
fn check(s: &[u8; 512], magic: &[u8; 8]) -> Result<(), ObjectError> {
    if &s[..8] == magic && get32(s, 8) == FORMAT_VERSION && valid_sector(s) {
        Ok(())
    } else {
        Err(ObjectError::CorruptMetadata)
    }
}
// ------------------------=
// FUNC: finish_sector
// DESC: Implements the finish sector operation.
// ------------------=
fn finish_sector(s: &mut [u8; 512]) {
    put32(s, 508, 0);
    let c = crc32(s);
    put32(s, 508, c)
}
// ------------------------=
// FUNC: valid_sector
// DESC: Reports whether valid sector.
// ------------------=
fn valid_sector(s: &[u8; 512]) -> bool {
    let want = get32(s, 508);
    let mut copy = *s;
    put32(&mut copy, 508, 0);
    crc32(&copy) == want
}
// ------------------------=
// FUNC: crc32
// DESC: Calculates and returns crc32.
// ------------------=
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 }
        }
    }
    !crc
}
// ------------------------=
// FUNC: put16
// DESC: Implements the put16 operation.
// ------------------=
fn put16(s: &mut [u8], o: usize, v: u16) {
    s[o..o + 2].copy_from_slice(&v.to_le_bytes())
}
// ------------------------=
// FUNC: put32
// DESC: Implements the put32 operation.
// ------------------=
fn put32(s: &mut [u8], o: usize, v: u32) {
    s[o..o + 4].copy_from_slice(&v.to_le_bytes())
}
// ------------------------=
// FUNC: put64
// DESC: Implements the put64 operation.
// ------------------=
fn put64(s: &mut [u8], o: usize, v: u64) {
    s[o..o + 8].copy_from_slice(&v.to_le_bytes())
}
// ------------------------=
// FUNC: get16
// DESC: Implements the get16 operation.
// ------------------=
fn get16(s: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([s[o], s[o + 1]])
}
// ------------------------=
// FUNC: get32
// DESC: Implements the get32 operation.
// ------------------=
fn get32(s: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]])
}
// ------------------------=
// FUNC: get64
// DESC: Implements the get64 operation.
// ------------------=
fn get64(s: &[u8], o: usize) -> u64 {
    u64::from_le_bytes([
        s[o],
        s[o + 1],
        s[o + 2],
        s[o + 3],
        s[o + 4],
        s[o + 5],
        s[o + 6],
        s[o + 7],
    ])
}

// ------------------------=
// FUNC: find_container
// DESC: Reads find container data.
// ------------------=
pub fn find_container<D: BlockDevice>(d: &mut D) -> Result<(u64, u64, [u8; 16]), ObjectError> {
    let mut h = [0; 512];
    if !d.read_sector(1, &mut h) || &h[..8] != b"EFI PART" {
        return Err(ObjectError::SpaceUnavailable);
    }
    let entries = get64(&h, 72);
    const KIND: [u8; 16] = [
        0x69, 0x66, 0x6e, 0x49, 0x69, 0x6e, 0x79, 0x74, 0x53, 0x54, 0x4f, 0x52, 0x41, 0x47, 0x45,
        0x31,
    ];
    for sector in 0..32 {
        let mut s = [0; 512];
        if !d.read_sector(entries + sector, &mut s) {
            return Err(ObjectError::CorruptMetadata);
        }
        for n in 0..4 {
            let o = n * 128;
            if s[o..o + 16] == KIND {
                let first = get64(&s, o + 32);
                let last = get64(&s, o + 40);
                let mut id = [0; 16];
                id.copy_from_slice(&s[o + 16..o + 32]);
                return Ok((first, last - first + 1, id));
            }
        }
    }
    Err(ObjectError::SpaceUnavailable)
}
