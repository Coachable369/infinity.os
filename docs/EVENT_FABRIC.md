# Infinity Event Fabric (IEF)

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
