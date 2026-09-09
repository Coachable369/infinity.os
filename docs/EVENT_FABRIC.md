# Infinity Event Fabric (IEF)

9-B caution: router audit records are not IEF publication. Remote durable commit,
authoritative stream sequencing and subscriber IOP reconstruction are still
missing; installed remote writes remain disabled. See [the gap report](MILESTONE_9B_CONTROL_PLANE.md).

The Mesh routing domain defines typed Node discovered/lost, pairing, trust, session, capability, membership, health, policy, degraded, and recovered events. Security changes are Record class; high-frequency health is coalescible. Events announce committed state and are never authoritative.

IOP asks; IEF announces. Services remain authoritative state owners.

The API exposes System, Storage, Device, Session, Application, and Future AI routing domains. They share one fixed-capacity implementation initially; distributed routers are PLANNED without changing schemas.

- `Signal` is ephemeral and droppable/coalescible.
- `StateChange` is sequence tracked; gaps require authoritative IOP reconciliation.
- `Record` is append-before-publish audit data.

Each event carries numeric type, payload version, source identity, per-topic sequence, timestamp, priority, scope, correlation, causation, and a bounded 96-byte payload. Large content remains an object and the event carries a reference.

Subscriptions use typed filters, scope, leases, limits, and an overflow policy. Authority is checked at creation and again before delivery. Policies are drop-oldest, drop-newest, latest-only, coalesce, disconnect-slow-subscriber, and durable. Queues never exceed eight events. Publisher quotas use a per-second allowance plus burst; priority 200+ has protected capacity.

TESTED: filtering, delivery-time revocation, leases, coalescing, bounded slow subscribers, sequence gaps, IOP reconciliation, rate-limit mechanism, and in-memory Record ordering. Security bootstrap records persist in System Space. A general on-disk Record stream is SCAFFOLDED.

AI event types cover provider/model changes, inference start/completion/failure,
voice listening/transcript state, and agent lifecycle. Correlated inference
StateChange publication is IMPLEMENTED AND HOST-TESTED, including delivery-time
capability filtering. Voice/agent event publication is SCAFFOLDED until their
native providers execute real work. Events remain announcements; Model Registry,
Voice Service, and Agent Manager own their state.
# Identity state records

Milestone 7 publishes `Identity.StateChanged` as a capability-checked Record
event from the Identity Service only after the native identity object commits.
Its payload carries the new state generation, allowing subscribers to detect a
gap and reconcile through typed Identity operations. Secrets and verifier
material never enter the event payload or durable record log.

# Window lifecycle events

Stable IEF types cover Window.Created, Window.Destroyed, Window.Focused,
Window.Moved, Window.Resized, Window.StateChanged, Surface.Committed,
Display.Changed, Compositor.Degraded, Compositor.Recovered,
SecureInput.Started, and SecureInput.Stopped. The Window Server produces an
ordered bounded typed bridge queue for geometry, focus, state, capture,
creation, destruction, and context failure. Queue pressure drops the oldest
announcement and increments structured diagnostics; authoritative state remains
queryable. Capability-checked post-commit publication of creation, movement,
and surface commits, including correlation and causation, is **TESTED**. The
remaining reserved lifecycle publishers are **SCAFFOLDED**.

## Network routing domain

The Network domain defines interface, address, route, connectivity, connection,
policy, profile, resolver, discovery, degraded, and recovered types. Profile
activation is Record-class and is emitted after the native profile object
commits. Capability-filtered delivery, Record ordering, correlation/causation,
and delivery-time revocation are **TESTED**. Packet counters remain sampled
diagnostics rather than durable event spam.

## Pool committed transition hints

Pool retains the four bounded resource/replica/object/policy event topics. Their
40-byte payload carries ObjectId/resource ID (0..16), committed generation
(16..24), bytes (24..32), state (32), reserved zeros (33..36), and little-endian
transition flags (36..40). `storage_protocol::transition` defines independent
bits for resource availability/offline, create/update/policy/delete/reclamation,
transfer start/verified/available, degraded/healthy, healing start/resume/complete,
and stale detection/reconciliation. Multiple bits describe the same commit.
Transfer chunks and incomplete verification ticks produce no lifecycle event.

Coordinator publication retains one latest hint; same-object transitions merge.
Replacing a pending different-object hint sets `RECONSTRUCT`, explicitly requiring
an authoritative Pool IOP refresh rather than claiming delivery of lost lifecycle
events. Failed publication retains the hint and retries with bounded work; event
delivery never changes replica validity. Settings also periodically rebuilds its
bounded typed projection, including when no final event arrives. Healing-resume
is announced after a durable claim renewal or acknowledged nonzero recipient
checkpoint, not merely scheduling a retry.

Host behavioral tests cover typed flags, rejection/retry, bounded overflow hints
and projection recovery without events. Installed event-failure acceptance is
still required. These StateChange hints are not substitutes for durable audit.
