# Infinity human namespace format v2

The namespace is a logical, UTF-8 path view over object identities. It is not a POSIX filesystem, inode table, mount tree, or symbolic-link implementation. Every entry directly relates one normalized absolute path to one 128-bit Object ID; multiple entries may target the same object.

Projects, Collections, Favorites, Recent, and other organizational dimensions
are typed objects, relationships, queries, or Views rather than directories.
The namespace remains one optional human-readable projection.

V1 rules:

- UTF-8 encoding; invalid sequences are rejected.
- Case-sensitive lookup is a temporary policy. Display case is preserved.
- Maximum encoded path is 95 bytes and each component is at most 63 encoded bytes; the future format will increase these without changing typed APIs.
- `/` is the separator. Empty components, trailing separators (except `/`), `.` and `..` are rejected.
- Only syntax and integrity constraints are prohibited; Windows/Unix historical filename restrictions are not imported.
- Rename and move replace one relationship and leave object identity/content unchanged.
- Detach removes one relationship. Object destruction is explicit and tombstones an unreferenced object.
- `/objects/<32 hexadecimal digits>` is a computed direct-object view, not a physical directory.

The initial Personal namespace uses the explicitly temporary identity `default` and creates `/home/default/{documents,pictures,media,projects,downloads,archive}`. It also creates `/apps`, `/shared`, `/devices`, `/system`, and `/objects`. NamespaceNode is an object type, but paths remain relationships rather than object identity.

Each persisted namespace record contains a used flag, path byte length, target Object ID, UTF-8 bytes, and reserved extension bytes. The containing `INFONSP2` sector is versioned and checksummed. V2 lookup uses a bounded reconstructed in-memory table; a persistent B+ tree is the planned scale replacement. Duplicate paths and dangling targets are rejected during mount validation.
