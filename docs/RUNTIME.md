# Infinity Runtime

## Status

| Facility | Status | Boundary |
|---|---|---|
| Execution Context identity/lifecycle | TESTED | Stable 128-bit security identity is separate from a compact runtime handle. |
| Memory ownership | TESTED | A loader-reserved 256 MiB arena has exact allocation tokens, bounded metadata, zero-on-allocation, and deterministic release. |
| MMU address-space protection/switching | TESTED FOUNDATION | Independent x86-64 CR3 roots enforce RX, R/NX, RW/NX, RELRO, supervisor kernel mappings, a user RX gateway, and two stack guards. Ring-3 entry and the trap dispatcher remain required before native code is executed. |
| Cooperative task scheduling | TESTED | Weighted class rotation, runnable/waiting states, wake hooks, and idle accounting. |
| Preemptive context switching | PLANNED | Requires architecture timer and saved-register implementations. |
| Resource governance | TESTED | Memory and queue admission plus CPU/I/O observability. |
| Local AI execution resources | TESTED (host) | Per-request memory limit, workload class, deadline, cancellation, bounded queue, and counters. |

`ExecutionManager`, `Scheduler`, `CapabilityManager`, `IopRouter`, `EventFabric`, and `ServiceManager` are independent modules behind narrow typed interfaces. They are colocated in the kernel image for bring-up, but their contracts do not depend on one another's storage layout.

An Execution Context moves through `Defined`, `Runnable`, `Running`, `Waiting`, `Stopped`, or `Failed`. It owns one explicit message endpoint, one memory region, a CR3-sized address-space token bound only after mapping succeeds, a resource budget, and a security identity. There are no native PID, fork, exec, signal, file-descriptor, Unix-socket, root-user, or ambient filesystem primitives.

Current fixed capacities are 16 contexts, 64 capabilities, 16 IOP endpoints with 8 messages each, 16 subscriptions with 8 events each, and 16 services. Exhaustion is a typed failure; it cannot grow kernel memory without bound.

The Milestone 6 AI composition is intentionally modular: Model Registry/CPU
backend, Provider Router, Context Broker, Tool Broker, Voice Service, and Agent
Manager have independent typed interfaces. They are currently colocated with the
bring-up runtime, as are other services; this is not represented as MMU isolation.
See `AI.md` for the exact support boundary.

Lock order for future concurrency is service manager, execution manager, capability manager, then IOP/event queue. Current bring-up is single-core and cooperative. Code may not call a service while holding a manager lock.

See `assets/architecture/runtime-overview.svg`.
