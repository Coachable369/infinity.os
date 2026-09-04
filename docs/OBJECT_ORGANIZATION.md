# Object Organization v1

Persistent information is identified by a 128-bit Object ID. Namespace path,
display name, Project, Collection, application, physical extent, device, and
future mesh location are attributes or references and never identity.

```text
Object ID
  |-- typed core metadata
  |-- versioned extension-schema references
  |-- directional typed relationships
  |-- zero or more Namespace references
  `-- local / remote / replicated / mesh location descriptor
```

The remote variants are identity-compatible contracts only; distributed
storage is not implemented in Milestone 6.5.

## Taxonomy and metadata

Stable numeric `ObjectTypeId` values define Content and Document, Image,
Audio, Video, SourceCode, Dataset, Conversation, and Note specializations,
plus Collection, Project, ApplicationData, SystemObject, Model, IdentityData,
and DeviceData. Each has a type version, human projection, schema reference,
and optional parent. The Object Store does not interpret application semantics.

Core metadata fields have stable IDs and declared value types. Extensions
contain a versioned schema ID and an ObjectRef to their typed payload; metadata
is not an uncontrolled string dictionary.

## Relationships and indexes

Initial directional relations are MemberOf, BelongsToProject, ContainedBy,
References, DerivedFrom, GeneratedBy, RelatedTo, VersionOf, OwnedBy,
SharedWith, Favorite, and Pinned. Favorites and pinning do not copy data.

The bounded catalog orders relationships by target Object ID, so reverse
membership starts with binary lower-bound lookup rather than an object scan.
The durable Object Store persists the same relationship identities. A scalable
persistent B+ tree is the planned replacement for the bounded table.

## Projects, Collections, Views, and queries

A Project is a first-class object, not a directory. Objects may belong to
multiple Projects. Static Collections use explicit MemberOf relationships;
dynamic Collections store a typed query. Neither duplicates content.

A View combines Object.Query with sort, grouping, and presentation metadata.
This is the future desktop contract for Projects, Documents, Pictures, Recent,
Shared, Favorites, and Collections.

Object.Query supports object type, name, tag, Project, Collection,
created/modified interval, owner, relationship, and Infinity Pool Space. It
returns an `ObjectSet`, never terminal text. Bring-up limits are 32 catalog
objects, 64 indexed relationships, four tags per object, and 32 ObjectRefs per
result.

## Persistent bootstrap and status

Installation writes `/system/organization/schema` as a checksummed 256-byte
`INFOORG1` native System object. The path is only a bootstrap Namespace
reference. Installation and mounted-system validation reject a missing or
corrupt schema.

- Taxonomy, typed metadata/extensions, relationships, Projects, Collections,
  Views, tags, location-neutral refs, query contract, and target index:
  **TESTED**.
- Durable base ObjectType and relationship encodings: **TESTED**.
- Native schema installation and detached remount: **TESTED on x86_64**.
- Large persistent indexes: **SCAFFOLDED**; native B+ tree is **PLANNED**.
- Distributed/replicated object transport: **PLANNED**.
