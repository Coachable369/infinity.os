# Infinity Operation Protocol (IOP)

Milestone 6.5 reserves stable IDs for Object.Filter/Object.Destroy,
Namespace.Move, Project.List/Inspect/Create, Collection.List/Inspect/Create,
Storage.Usage, Device.Inspect, Capability.List, Event.Subscriptions, and
Voice.Status. Console schemas reference numeric IDs; machine callers submit
typed payloads directly.

IOP is the typed machine request/response path. Human console text translates to numeric operations; it is not the wire protocol.

The 80-byte version-1 little-endian header is encoded field-by-field: magic `IOP1`, protocol version, message type, schema version, numeric operation ID, request ID, 128-bit caller identity, capability reference, payload length, flags, monotonic deadline, correlation ID, causation ID, and checksum. Rust memory layout is never serialized.

Messages are Request, Response, Error, or Cancel. Every operation has a stable numeric ID and schema version. Inline payloads are limited to 192 bytes. Larger transfers use typed `ObjectRef` or `SharedBuffer { capability, offset, length }` references.

Every endpoint queue is bounded to eight messages. Full queues return `Backpressure`; expired requests return `DeadlineExceeded`; cancelled IDs return `Cancelled`; invalid or revoked authority returns `AccessDenied`. Cancellation is cooperative after delivery.

TESTED: header round trip/checksum, Service A to B echo, immediate revocation, deadline, cancellation hook, and saturation. Response routing and generated codecs beyond the echo schema are SCAFFOLDED.

Milestone 6 reserves typed operations for `Model.*`, `Intent.Resolve`,
`Context.Request`, `Tool.Invoke`, `Voice.*`, `Speech.*`, and `Agent.*`.
The local inference path uses the same bounded deadline/cancellation semantics.
Console text and model output are never transported as executable commands;
intent output is a closed typed plan. Full generated codecs for every AI schema
remain SCAFFOLDED.
# Milestone 7 operations

The operation registry now reserves stable IDs for Identity, Machine,
Credential, Authentication, Session, Profile, Personal Space, AI Profile,
Voice Profile, Settings, Onboarding, Shell, power-off, and restart. GUI and CLI
adapters select these IDs directly and never communicate through command text.
