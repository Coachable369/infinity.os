# Reserved native System metadata

Internal metadata may use a stable native ObjectId without a human namespace entry. The bounded native object table resolves its exact UTF-8 reserved name (leading `@`, maximum 47 bytes), System space, Metadata type, and zero owner. This avoids consuming the 32 user namespace slots. It does not increase any object, namespace, or memory limit.

The public typed ObjectService rejects creation and access to internal System Metadata and other reserved System names. Namespace mutations beneath the exact `/system/storage` component are protected, including moving into or out of it; similarly prefixed Personal paths are unaffected. Native ObjectStore transaction helpers are internal trusted implementation APIs, not application capabilities. A Personal object with the same display name cannot satisfy the lookup. Duplicate reserved identities, invalid type/owner, or conflict with a legacy named identity fail closed.

Legacy named records retain their original ObjectId and bytes. New audit rings use `@pool-audit`; their first allocation, audit append, and associated lifecycle mutation commit under the same native transaction root. Lookup is a bounded in-memory object-table scan on metadata operations, not a renderer or per-packet operation.

The installed verifier reads either representation. Its explicit `--replay-upload-in-memory --vm-paused RAW` diagnostic opens the artifact read-only and directs all native transaction writes to an in-memory sector overlay; it does not repair or modify the installed disk. A successful replay is diagnostic evidence, not installed acceptance.
