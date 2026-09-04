//! Typed, relational organization contracts above stable Object IDs. This
//! module contains no physical-path or block-address identity assumptions.

use super::object::{ObjectId, ObjectRef, Space};

pub const ORGANIZATION_SCHEMA_VERSION: u16 = 1;
pub const MAX_CATALOG_OBJECTS: usize = 32;
pub const MAX_CATALOG_RELATIONSHIPS: usize = 64;
pub const MAX_OBJECT_SET: usize = 32;
pub const MAX_TAGS: usize = 4;
pub const MAX_NAME: usize = 48;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectTypeId(pub u32);

pub const TYPE_CONTENT: ObjectTypeId = ObjectTypeId(0x1000);
pub const TYPE_DOCUMENT: ObjectTypeId = ObjectTypeId(0x1001);
pub const TYPE_IMAGE: ObjectTypeId = ObjectTypeId(0x1002);
pub const TYPE_AUDIO: ObjectTypeId = ObjectTypeId(0x1003);
pub const TYPE_VIDEO: ObjectTypeId = ObjectTypeId(0x1004);
pub const TYPE_SOURCE_CODE: ObjectTypeId = ObjectTypeId(0x1005);
pub const TYPE_DATASET: ObjectTypeId = ObjectTypeId(0x1006);
pub const TYPE_CONVERSATION: ObjectTypeId = ObjectTypeId(0x1007);
pub const TYPE_NOTE: ObjectTypeId = ObjectTypeId(0x1008);
pub const TYPE_COLLECTION: ObjectTypeId = ObjectTypeId(0x2000);
pub const TYPE_PROJECT: ObjectTypeId = ObjectTypeId(0x3000);
pub const TYPE_APPLICATION_DATA: ObjectTypeId = ObjectTypeId(0x4000);
pub const TYPE_SYSTEM_OBJECT: ObjectTypeId = ObjectTypeId(0x5000);
pub const TYPE_MODEL: ObjectTypeId = ObjectTypeId(0x6000);
pub const TYPE_IDENTITY_DATA: ObjectTypeId = ObjectTypeId(0x7000);
pub const TYPE_DEVICE_DATA: ObjectTypeId = ObjectTypeId(0x8000);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectTypeDescriptor {
    pub id: ObjectTypeId,
    pub version: u16,
    pub human_name: &'static [u8],
    pub schema_ref: u32,
    pub parent: Option<ObjectTypeId>,
}

pub static OBJECT_TYPES: &[ObjectTypeDescriptor] = &[
    object_type(TYPE_CONTENT, b"Content", 0x1000, None),
    object_type(TYPE_DOCUMENT, b"Document", 0x1001, Some(TYPE_CONTENT)),
    object_type(TYPE_IMAGE, b"Image", 0x1002, Some(TYPE_CONTENT)),
    object_type(TYPE_AUDIO, b"Audio", 0x1003, Some(TYPE_CONTENT)),
    object_type(TYPE_VIDEO, b"Video", 0x1004, Some(TYPE_CONTENT)),
    object_type(TYPE_SOURCE_CODE, b"SourceCode", 0x1005, Some(TYPE_CONTENT)),
    object_type(TYPE_DATASET, b"Dataset", 0x1006, Some(TYPE_CONTENT)),
    object_type(
        TYPE_CONVERSATION,
        b"Conversation",
        0x1007,
        Some(TYPE_CONTENT),
    ),
    object_type(TYPE_NOTE, b"Note", 0x1008, Some(TYPE_CONTENT)),
    object_type(TYPE_COLLECTION, b"Collection", 0x2000, None),
    object_type(TYPE_PROJECT, b"Project", 0x3000, None),
    object_type(TYPE_APPLICATION_DATA, b"ApplicationData", 0x4000, None),
    object_type(TYPE_SYSTEM_OBJECT, b"SystemObject", 0x5000, None),
    object_type(TYPE_MODEL, b"Model", 0x6000, None),
    object_type(TYPE_IDENTITY_DATA, b"IdentityData", 0x7000, None),
    object_type(TYPE_DEVICE_DATA, b"DeviceData", 0x8000, None),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MetadataValueType {
    Text,
    ObjectRef,
    ObjectType,
    Unsigned,
    Timestamp,
    TextSet,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MetadataFieldSchema {
    pub id: u32,
    pub schema_version: u16,
    pub value_type: MetadataValueType,
    pub required: bool,
    pub human_name: &'static [u8],
}

pub static CORE_METADATA_SCHEMA: &[MetadataFieldSchema] = &[
    field(1, MetadataValueType::Text, true, b"display_name"),
    field(2, MetadataValueType::ObjectType, true, b"object_type"),
    field(3, MetadataValueType::Timestamp, true, b"created"),
    field(4, MetadataValueType::Timestamp, true, b"modified"),
    field(5, MetadataValueType::ObjectRef, false, b"creator"),
    field(6, MetadataValueType::ObjectRef, false, b"owner"),
    field(7, MetadataValueType::Unsigned, false, b"logical_size"),
    field(8, MetadataValueType::Text, false, b"content_type"),
    field(9, MetadataValueType::TextSet, false, b"tags"),
    field(10, MetadataValueType::ObjectRef, false, b"source"),
    field(11, MetadataValueType::ObjectRef, false, b"generated_by"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum RelationshipType {
    MemberOf = 1,
    BelongsToProject = 2,
    ContainedBy = 3,
    References = 4,
    DerivedFrom = 5,
    GeneratedBy = 6,
    RelatedTo = 7,
    VersionOf = 8,
    OwnedBy = 9,
    SharedWith = 10,
    Favorite = 11,
    Pinned = 12,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectLocation {
    Local,
    Remote { node: [u8; 16] },
    Replicated { preferred_node: [u8; 16] },
    Mesh { authority: [u8; 16] },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StableObjectRef {
    pub object: ObjectRef,
    pub location: ObjectLocation,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tag {
    bytes: [u8; 24],
    length: u8,
}

impl Tag {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded UTF-8 tag used as lightweight metadata.
    // ------------------=
    pub fn new(value: &[u8]) -> Result<Self, OrganizationError> {
        if value.is_empty() || value.len() > 24 || core::str::from_utf8(value).is_err() {
            return Err(OrganizationError::InvalidMetadata);
        }
        let mut bytes = [0; 24];
        bytes[..value.len()].copy_from_slice(value);
        Ok(Self {
            bytes,
            length: value.len() as u8,
        })
    }

    // ------------------------=
    // FUNC: bytes
    // DESC: Returns the normalized tag bytes.
    // ------------------=
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.length as usize]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CoreMetadata {
    pub display_name: [u8; MAX_NAME],
    pub display_name_length: u8,
    pub object_type: ObjectTypeId,
    pub object_type_version: u16,
    pub created: u64,
    pub modified: u64,
    pub creator: Option<ObjectRef>,
    pub owner: Option<ObjectRef>,
    pub logical_size: u64,
    pub content_type_id: u32,
    pub tags: [Option<Tag>; MAX_TAGS],
}

impl CoreMetadata {
    // ------------------------=
    // FUNC: new
    // DESC: Creates validated required metadata for an object identity.
    // ------------------=
    pub fn new(
        name: &[u8],
        object_type: ObjectTypeId,
        now: u64,
    ) -> Result<Self, OrganizationError> {
        if name.is_empty()
            || name.len() > MAX_NAME
            || core::str::from_utf8(name).is_err()
            || type_descriptor(object_type).is_none()
        {
            return Err(OrganizationError::InvalidMetadata);
        }
        let mut display_name = [0; MAX_NAME];
        display_name[..name.len()].copy_from_slice(name);
        Ok(Self {
            display_name,
            display_name_length: name.len() as u8,
            object_type,
            object_type_version: 1,
            created: now,
            modified: now,
            creator: None,
            owner: None,
            logical_size: 0,
            content_type_id: 0,
            tags: [None; MAX_TAGS],
        })
    }

    // ------------------------=
    // FUNC: name
    // DESC: Returns the human display name without making it an identity.
    // ------------------=
    pub fn name(&self) -> &[u8] {
        &self.display_name[..self.display_name_length as usize]
    }

    // ------------------------=
    // FUNC: add_tag
    // DESC: Adds one validated tag without replacing typed relationships.
    // ------------------=
    pub fn add_tag(&mut self, tag: Tag) -> Result<(), OrganizationError> {
        if self.tags.iter().flatten().any(|existing| existing == &tag) {
            return Ok(());
        }
        let slot = self
            .tags
            .iter_mut()
            .find(|item| item.is_none())
            .ok_or(OrganizationError::Capacity)?;
        *slot = Some(tag);
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExtensionMetadata {
    pub schema_id: u32,
    pub schema_version: u16,
    pub payload_object: ObjectRef,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OrganizedObject {
    pub reference: StableObjectRef,
    pub core: CoreMetadata,
    pub extensions: [Option<ExtensionMetadata>; 4],
    pub space: Space,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Relationship {
    pub source: ObjectRef,
    pub kind: RelationshipType,
    pub target: ObjectRef,
    pub metadata: Option<ObjectRef>,
    pub created: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TemporalPredicate {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    Last30Days,
    Before(u64),
    After(u64),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectQuery {
    pub object_type: Option<ObjectTypeId>,
    pub name: Option<Tag>,
    pub tag: Option<Tag>,
    pub project: Option<ObjectRef>,
    pub collection: Option<ObjectRef>,
    pub created: Option<TemporalPredicate>,
    pub modified: Option<TemporalPredicate>,
    pub owner: Option<ObjectRef>,
    pub relationship: Option<RelationshipType>,
    pub space: Option<Space>,
}

impl ObjectQuery {
    // ------------------------=
    // FUNC: all
    // DESC: Creates an unfiltered typed object query.
    // ------------------=
    pub const fn all() -> Self {
        Self {
            object_type: None,
            name: None,
            tag: None,
            project: None,
            collection: None,
            created: None,
            modified: None,
            owner: None,
            relationship: None,
            space: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectSet {
    pub refs: [Option<ObjectRef>; MAX_OBJECT_SET],
    pub count: u8,
    pub query_schema_version: u16,
    pub ordered_by: SortOrder,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortOrder {
    Identity,
    Name,
    Created,
    Modified,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grouping {
    None,
    Type,
    Project,
    ModifiedDay,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Presentation {
    List,
    Grid,
    Timeline,
    Gallery,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct View {
    pub query: ObjectQuery,
    pub sort: SortOrder,
    pub grouping: Grouping,
    pub presentation: Presentation,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CollectionMode {
    Static,
    Dynamic,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CollectionDefinition {
    pub object: ObjectRef,
    pub mode: CollectionMode,
    pub query: Option<ObjectQuery>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrganizationError {
    InvalidMetadata,
    UnknownType,
    MissingObject,
    Duplicate,
    Capacity,
    InvalidRelationship,
}

pub struct OrganizationCatalog {
    objects: [Option<OrganizedObject>; MAX_CATALOG_OBJECTS],
    relationships_by_target: [Option<Relationship>; MAX_CATALOG_RELATIONSHIPS],
}

impl OrganizationCatalog {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded relational organization catalog.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            objects: [None; MAX_CATALOG_OBJECTS],
            relationships_by_target: [None; MAX_CATALOG_RELATIONSHIPS],
        }
    }

    // ------------------------=
    // FUNC: register
    // DESC: Registers typed metadata for an existing stable Object ID.
    // ------------------=
    pub fn register(&mut self, object: OrganizedObject) -> Result<(), OrganizationError> {
        if self
            .objects
            .iter()
            .flatten()
            .any(|item| item.reference.object == object.reference.object)
        {
            return Err(OrganizationError::Duplicate);
        }
        let slot = self
            .objects
            .iter_mut()
            .find(|item| item.is_none())
            .ok_or(OrganizationError::Capacity)?;
        *slot = Some(object);
        Ok(())
    }

    // ------------------------=
    // FUNC: attach
    // DESC: Persists a directional typed relationship in target-index order.
    // ------------------=
    pub fn attach(&mut self, relationship: Relationship) -> Result<(), OrganizationError> {
        if self.object(relationship.source).is_none() || self.object(relationship.target).is_none()
        {
            return Err(OrganizationError::MissingObject);
        }
        if self.relationships_by_target.iter().flatten().any(|item| {
            item.source == relationship.source
                && item.target == relationship.target
                && item.kind == relationship.kind
        }) {
            return Err(OrganizationError::Duplicate);
        }
        let count = self
            .relationships_by_target
            .iter()
            .take_while(|item| item.is_some())
            .count();
        if count == MAX_CATALOG_RELATIONSHIPS {
            return Err(OrganizationError::Capacity);
        }
        let key = relationship_key(&relationship);
        let at = self.relationships_by_target[..count]
            .binary_search_by(|item| relationship_key(item.as_ref().unwrap()).cmp(&key))
            .unwrap_or_else(|index| index);
        for index in (at..count).rev() {
            self.relationships_by_target[index + 1] = self.relationships_by_target[index];
        }
        self.relationships_by_target[at] = Some(relationship);
        Ok(())
    }

    // ------------------------=
    // FUNC: object
    // DESC: Resolves metadata by stable Object ID, never by display name alone.
    // ------------------=
    pub fn object(&self, reference: ObjectRef) -> Option<&OrganizedObject> {
        self.objects
            .iter()
            .flatten()
            .find(|item| item.reference.object == reference)
    }

    // ------------------------=
    // FUNC: reverse_relationship_nth
    // DESC: Uses the target-sorted relationship index for reverse membership queries.
    // ------------------=
    pub fn reverse_relationship_nth(
        &self,
        target: ObjectRef,
        index: usize,
    ) -> Option<Relationship> {
        let count = self
            .relationships_by_target
            .iter()
            .take_while(|item| item.is_some())
            .count();
        let first = lower_bound_target(&self.relationships_by_target[..count], target.id);
        self.relationships_by_target[first..count]
            .iter()
            .flatten()
            .take_while(|item| item.target == target)
            .nth(index)
            .copied()
    }

    // ------------------------=
    // FUNC: query
    // DESC: Evaluates typed metadata and indexed relationship predicates into an ObjectSet.
    // ------------------=
    pub fn query(&self, query: &ObjectQuery, now: u64) -> ObjectSet {
        let mut result = ObjectSet {
            refs: [None; MAX_OBJECT_SET],
            count: 0,
            query_schema_version: 1,
            ordered_by: SortOrder::Identity,
        };
        for item in self.objects.iter().flatten() {
            if !matches_query(self, item, query, now) {
                continue;
            }
            if result.count as usize == MAX_OBJECT_SET {
                break;
            }
            result.refs[result.count as usize] = Some(item.reference.object);
            result.count += 1;
        }
        result
    }
}

// ------------------------=
// FUNC: organization_schema_object
// DESC: Produces the versioned native schema object installed in System Space.
// ------------------=
pub fn organization_schema_object() -> [u8; 256] {
    let mut out = [0; 256];
    out[..8].copy_from_slice(b"INFOORG1");
    out[8..10].copy_from_slice(&ORGANIZATION_SCHEMA_VERSION.to_le_bytes());
    out[10..12].copy_from_slice(&(OBJECT_TYPES.len() as u16).to_le_bytes());
    out[12..14].copy_from_slice(&(CORE_METADATA_SCHEMA.len() as u16).to_le_bytes());
    out[14..16].copy_from_slice(&(12u16).to_le_bytes());
    for (index, descriptor) in OBJECT_TYPES.iter().enumerate() {
        let offset = 16 + index * 8;
        out[offset..offset + 4].copy_from_slice(&descriptor.id.0.to_le_bytes());
        out[offset + 4..offset + 6].copy_from_slice(&descriptor.version.to_le_bytes());
        out[offset + 6..offset + 8].copy_from_slice(&(descriptor.schema_ref as u16).to_le_bytes());
    }
    let checksum = checksum(&out[..252]);
    out[252..].copy_from_slice(&checksum.to_le_bytes());
    out
}

// ------------------------=
// FUNC: organization_schema_valid
// DESC: Validates the native organization schema object and its checksum.
// ------------------=
pub fn organization_schema_valid(value: &[u8]) -> bool {
    value.len() == 256
        && &value[..8] == b"INFOORG1"
        && u16::from_le_bytes([value[8], value[9]]) == ORGANIZATION_SCHEMA_VERSION
        && checksum(&value[..252])
            == u32::from_le_bytes([value[252], value[253], value[254], value[255]])
}

// ------------------------=
// FUNC: type_descriptor
// DESC: Looks up a stable object type descriptor by numeric identity.
// ------------------=
pub fn type_descriptor(id: ObjectTypeId) -> Option<&'static ObjectTypeDescriptor> {
    OBJECT_TYPES.iter().find(|item| item.id == id)
}

// ------------------------=
// FUNC: object_type
// DESC: Defines a versioned static object type descriptor.
// ------------------=
const fn object_type(
    id: ObjectTypeId,
    human_name: &'static [u8],
    schema_ref: u32,
    parent: Option<ObjectTypeId>,
) -> ObjectTypeDescriptor {
    ObjectTypeDescriptor {
        id,
        version: 1,
        human_name,
        schema_ref,
        parent,
    }
}

// ------------------------=
// FUNC: field
// DESC: Defines one typed core metadata field schema.
// ------------------=
const fn field(
    id: u32,
    value_type: MetadataValueType,
    required: bool,
    human_name: &'static [u8],
) -> MetadataFieldSchema {
    MetadataFieldSchema {
        id,
        schema_version: 1,
        value_type,
        required,
        human_name,
    }
}

// ------------------------=
// FUNC: relationship_key
// DESC: Produces the stable ordering key for the reverse relationship index.
// ------------------=
fn relationship_key(value: &Relationship) -> ([u8; 16], u16, [u8; 16]) {
    (value.target.id.0, value.kind as u16, value.source.id.0)
}

// ------------------------=
// FUNC: lower_bound_target
// DESC: Finds the first relationship for a target using binary search.
// ------------------=
fn lower_bound_target(values: &[Option<Relationship>], target: ObjectId) -> usize {
    let mut low = 0;
    let mut high = values.len();
    while low < high {
        let middle = (low + high) / 2;
        if values[middle].as_ref().unwrap().target.id < target {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    low
}

// ------------------------=
// FUNC: matches_query
// DESC: Applies typed query predicates to one organized object.
// ------------------=
fn matches_query(
    catalog: &OrganizationCatalog,
    item: &OrganizedObject,
    query: &ObjectQuery,
    now: u64,
) -> bool {
    if query
        .object_type
        .map(|value| value != item.core.object_type)
        .unwrap_or(false)
        || query
            .space
            .map(|value| value != item.space)
            .unwrap_or(false)
        || query
            .owner
            .map(|value| Some(value) != item.core.owner)
            .unwrap_or(false)
    {
        return false;
    }
    if query
        .name
        .map(|value| !contains_ascii(item.core.name(), value.bytes()))
        .unwrap_or(false)
        || query
            .tag
            .map(|value| !item.core.tags.iter().flatten().any(|tag| tag == &value))
            .unwrap_or(false)
    {
        return false;
    }
    if query
        .modified
        .map(|value| !temporal_matches(item.core.modified, value, now))
        .unwrap_or(false)
        || query
            .created
            .map(|value| !temporal_matches(item.core.created, value, now))
            .unwrap_or(false)
    {
        return false;
    }
    for (target, kind) in [
        (query.project, RelationshipType::BelongsToProject),
        (query.collection, RelationshipType::MemberOf),
    ] {
        if let Some(target) = target {
            let mut index = 0;
            let mut matched = false;
            while let Some(relation) = catalog.reverse_relationship_nth(target, index) {
                if relation.source == item.reference.object && relation.kind == kind {
                    matched = true;
                    break;
                }
                index += 1;
            }
            if !matched {
                return false;
            }
        }
    }
    if let Some(kind) = query.relationship {
        if !catalog
            .relationships_by_target
            .iter()
            .flatten()
            .any(|value| value.source == item.reference.object && value.kind == kind)
        {
            return false;
        }
    }
    true
}

// ------------------------=
// FUNC: temporal_matches
// DESC: Evaluates bounded temporal predicates using monotonic day units supplied by policy.
// ------------------=
fn temporal_matches(value: u64, predicate: TemporalPredicate, now: u64) -> bool {
    const DAY: u64 = 86_400;
    match predicate {
        TemporalPredicate::Today => value >= now.saturating_sub(DAY),
        TemporalPredicate::Yesterday => {
            value >= now.saturating_sub(DAY * 2) && value < now.saturating_sub(DAY)
        }
        TemporalPredicate::ThisWeek => value >= now.saturating_sub(DAY * 7),
        TemporalPredicate::LastWeek => {
            value >= now.saturating_sub(DAY * 14) && value < now.saturating_sub(DAY * 7)
        }
        TemporalPredicate::Last30Days => value >= now.saturating_sub(DAY * 30),
        TemporalPredicate::Before(limit) => value < limit,
        TemporalPredicate::After(limit) => value > limit,
    }
}

// ------------------------=
// FUNC: contains_ascii
// DESC: Performs bounded case-insensitive display-name matching.
// ------------------=
fn contains_ascii(value: &[u8], needle: &[u8]) -> bool {
    needle.len() <= value.len()
        && value.windows(needle.len()).any(|window| {
            window
                .iter()
                .zip(needle)
                .all(|(a, b)| a.eq_ignore_ascii_case(b))
        })
}

// ------------------------=
// FUNC: checksum
// DESC: Computes the native organization schema checksum.
// ------------------=
fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c9dc5u32, |hash, byte| {
        (hash ^ *byte as u32).wrapping_mul(0x01000193)
    })
}
