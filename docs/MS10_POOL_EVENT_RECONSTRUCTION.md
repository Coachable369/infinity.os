# Pool event reconstruction

Storage Settings observes four capability-scoped IEF topics through fixed
one-item LatestOnly queues. One topic is serviced per poll. Only the installed
storage service's correctly shaped event is accepted. Per-topic sequence skips
and explicit RECONSTRUCT hints mark the projection stale; event payloads never
become authoritative object state.

A complete typed IOP snapshot, including shared metadata quorums, clears stale
state only if no newer event arrived during its construction. Otherwise the
candidate is discarded and the next bounded scan starts again. Policy controls
cannot submit against a stale projection. Unchanged snapshots do not increment
the render revision. Periodic refresh remains a fallback for entirely lost hints.

Read-only installed diagnostic words in the main diagnostic snapshot:

- 507: Pool projection stale (0/1).
- 508: observed sequence/reconstruction gaps.
- 509: completed stale-to-authoritative reconstructions.
- 510: accepted Pool events.

The behavioral projection test consumes actual event sequence 1, overwrites the
one-entry queue with sequences 2 and 3, observes the gap and unchanged old
snapshot, then verifies that typed backend reads reconstruct generation and
policy before stale clears. This host test does not establish installed event
delivery; installed acceptance must exercise and inspect the same counters.
