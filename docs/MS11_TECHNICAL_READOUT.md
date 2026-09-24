# MS11 technical readout — distributed compute

**Review date:** 2026-09-24

**Source checkpoint:** `31ed5b01b47114e81065fa1fbff6ca242b2037ec` (`mainline`)

**Status:** Distributed-compute infrastructure is implemented and the host behavioral gates below pass. Installed, three-node end-to-end acceptance is **not established by the evidence available for this readout**.

## Executive summary

MS11 adds a bounded compute service to InfinityOS's existing capability, Resource Fabric, Execution Context, IOP, event, and storage infrastructure. A caller can authorize and submit work, select eligible local or remote capacity, inspect its state, cancel it, and observe restartable work being re-placed after node loss. Remote work uses explicit peer-scoped request/cancel grants; it is not treated as extra local CPU capacity.

The important scope distinction is that the current executable workloads are `CpuChecksum`, `CpuBoundedCounter`, and `AcceleratorInferenceFixture`. Execution advances bounded accounting ticks and returns a deterministic digest derived from workload/input identifiers, epoch, and work count. This is real implementation of the coordination and execution-context lifecycle, **not evidence of arbitrary application execution, processing referenced object contents, GPU inference, or distributed Hermes/Mistral inference**.

### Implementation history

| Commit | Contribution |
| --- | --- |
| `2b9feb5` | Compute contracts/service, placement and reservations, capability/IOP integration, events, storage state, Task Manager projection, AI adapter, host milestone harness. |
| `98ab3fd` | Persistent remote execution contexts, outbound coordinator, remote completion/cancel/failover integration, coordinated AI submission, behavioral tests. |
| `31ed5b0` | Console authorization/submission, binary diagnostics, authority cleanup, resource publication/identity fixes, independent compute-policy enforcement, three-node installed acceptance runner. |

Commit titles describe development intent; they are not acceptance evidence. This report is based on inspected source and the executions listed below. Unrelated working-tree changes were present and were not modified.

## What was implemented

### 1. Versioned contracts and authoritative task state

[compute.rs](../kernel/runtime/compute.rs) defines schema version 1:

| Contract | Encoded size | Purpose |
| --- | ---: | --- |
| `ComputeRequestV1` | 192 bytes | Workload identity, input references, locality/privacy, durability, priority, allowed nodes/domains, resource budgets, deadline, correlation, capability, affinity and result contract. |
| `ComputeDispatchV1` | 144 bytes | Targeted remote request/cancel/result with task identity, epoch and bounded progress. |
| `ComputeResultV1` | 160 bytes | Authoritative task snapshot, placement, result digest and accounting. |
| Persisted compute state | 4,080 bytes | Versioned, checksummed snapshot archive and next task identity. |

Task states cover queued, placed, running, completed, failed, cancelled, and node-lost work. Public inspection uses the same authoritative snapshots as persistence and Task Manager.

### 2. Placement and resource governance

The service selects matching compute or accelerator resources together with memory on the same node. It filters capacity by health/availability, lease validity, locality/privacy and explicit node/domain constraints. Candidate scoring uses locality preference, observed load, latency, affinity and anti-affinity.

Compute and memory reservations are generation-bound. If the second reservation fails, the first is released. Terminal paths release reservations and destroy applicable contexts. Resource advertisements now carry compute, memory and accelerator kinds through the existing resource protocol; restored node identity retires stale local compute capacity.

Implemented bounds include 24 active task slots, 16 node observations, 64 notices, a 64 MiB per-request memory ceiling, eight coordinated outbound tasks, eight remote execution slots, and at most 64 accounting ticks per execution slice. Archived snapshots are additionally bounded; persistence retains at most 24 snapshots. These are implementation limits, not throughput or physical-memory measurements.

### 3. Authenticated remote coordination

[compute_operator.rs](../kernel/runtime/compute_operator.rs) tracks tasks with up to two explicit remote authorities. Its poll path advances bounded local work or an authenticated remote transition, consumes typed results, and retires temporary service capabilities. Per-hop deadlines are bounded to 30 clock units and do not exceed the task deadline.

[iop_remote.rs](../kernel/runtime/iop_remote.rs) and [iop_remote_payload.rs](../kernel/runtime/iop_remote_payload.rs) transport typed compute payloads through existing authenticated node sessions. Receiver execution rechecks authority, and compute policy is independent of storage-read policy. A storage permission is not sufficient to execute compute.

The remote executor keeps a context across slices, rejects a changed continuation contract, checks task/epoch identity, and destroys contexts on completion, cancellation or expiry. [runtime/mod.rs](../kernel/runtime/mod.rs) integrates execution, resource observations, node-loss handling, coordination and persistence into runtime polling.

### 4. Recovery, cancellation and events

- `Pinned` work fails when its execution node is lost.
- Eligible `Restartable` work can be placed elsewhere, incrementing its epoch and restart count. Old-epoch results are rejected.
- `CheckpointableScaffold` and `MigratableScaffold` are declared but rejected as unsupported durability; checkpoint migration is not delivered.
- Task cancellation requires task-scoped `ComputeCancel` authority. Remote cancellation uses a separate peer grant.
- Lifecycle notices include queue, placement, start, completion, failure, cancellation, restart and node loss. Runtime publication follows successful persistence. A subscriber detecting a sequence gap can inspect authoritative task state.
- Reboot restores audit snapshots; interrupted active work becomes terminal failure with `NodeLost`. It does **not** transparently resume a running checkpoint after reboot.

### 5. Console, Task Manager and diagnostics

[console.rs](../kernel/core/console.rs) exposes:

| Command | Function |
| --- | --- |
| `compute authorize scope=N seconds=N confirm=true` | Obtain explicit session-owned execution authority. |
| `compute authorize-cancel task=N seconds=N confirm=true` | Obtain cancellation authority for a specific task. |
| `compute revoke CAPABILITY` | Revoke an operator capability. |
| `compute submit ...` | Submit a typed workload with explicit budgets, locality, durability and remote grants; supports a fallback node. |
| `compute list` / `compute inspect TASK_ID` | Inspect authoritative task state and accounting. |
| `compute cancel TASK_ID COMPUTE_CANCEL_CAP` | Request authorized cancellation. |

[task_manager.rs](../kernel/runtime/task_manager.rs) exposes count/list/inspect projections over the same service, and Console's Task Manager renderer consumes those snapshots. Shared-state behavior is host-tested; this report does not certify visual layout or every GUI interaction.

[console_diagnostics.rs](../kernel/core/console_diagnostics.rs) publishes a fixed 256-word compute diagnostic snapshot with generation guards, task/context counts, resource reservations, task records, notices and operator capabilities. The installed runner reads binary fields rather than deciding success from Console prose. Diagnostic task records expose at most six tasks, even though the service supports more.

### 6. AI boundary and installed-state integration

[ai/mod.rs](../kernel/runtime/ai/mod.rs) provides direct and coordinated distributed-inference request adapters. They preserve the provider-neutral boundary and validate privacy/provider policy before translating a request into accelerator fixture work. This is an integration seam and scheduling fixture, not a remote model backend.

[storage/object.rs](../kernel/storage/object.rs) initializes compute state in the fresh System Space runtime object. [storage/mod.rs](../kernel/storage/mod.rs) loads and transactionally commits that state; runtime initialization restores it on installed boot. The milestone host harness formats, reads and remounts a fresh in-memory object store and verifies the initialized state. This proves initialization behavior, not a physical/virtual machine installation.

## Evidence rundown

The following commands were executed while preparing this document against the checkout at the stated source checkpoint. Assertions exercise typed state, binary serialization, permissions, context lifetime and reservations; diagnostic output is not the acceptance oracle.

| Evidence | Observed result | What it establishes |
| --- | --- | --- |
| `make milestone-11-test` | Exit 0 | Integrated host scenario covering fresh-store initialization/remount, wire round trips, remote placement, bounded execution/accounting, denied/revoked authority, cancellation, deadline failure, pinned failure, restart/fencing, Task Manager projection, event-gap inspection, accelerator fixture, AI privacy denial, persistence/restore and unsupported durability. |
| `cargo test --quiet --manifest-path tools/milestone9-harness/Cargo.toml compute` | 5 passed; 0 failed; 68 filtered | Stale local compute identity retirement; registered request/cancel grant restrictions; compute frame round trip; receiver execution and revocation recheck; independent compute-policy denial. |
| `cargo test --quiet --manifest-path tools/milestone9-harness/Cargo.toml coordinator_binds_placed_task_to_remote_iop_result_and_releases_reservations` | 1 passed; 0 failed; 72 filtered | Production coordinator completion, cancellation, hop-timeout recovery, epoch fencing and reservation/context cleanup through a host fixture. |

The milestone executable is one integrated assertion-driven program, not a reported count of individual test cases. Compiler warnings remain, including unused/deprecated items and a host-harness feature-configuration warning; passing does not mean warning-free. No VM was restarted or reinstalled for this documentation task.

### Installed-system runner: implemented, successful receipt not verified

[tools/ms11-installed-compute.py](../tools/ms11-installed-compute.py) implements a three-node x86_64 QEMU acceptance sequence:

1. Copy and hash installer/kernel artifacts; independently install three nodes.
2. Boot without installer media, authenticate and verify independent persistent identities.
3. Configure network links, pair peers, open secure sessions and authorize resource advertisements and scope-specific compute.
4. Submit work from A to B and check completion, digest, 4,096 accounted ticks and reservation release.
5. Exercise revoked authority, remote cancellation and deadline cleanup.
6. Stop B during restartable work; require epoch-2 completion on C; return B and check result stability.
7. Capture Console/Task Manager screenshots and compare authoritative state.
8. Cold-reboot A and compare persisted terminal snapshots.

Its intended outputs are `result.json` (install identities/media state), `artifacts/sha256.json`, screenshots and `ms11-result.json` (runtime outcomes and Ethernet metadata). No successful `ms11-result.json` was located in the repository, `/private/tmp`, or the inspected user temporary directory during this review. Therefore these are **implemented acceptance scenarios, not verified run results** here.

Two qualifications apply even to a future successful run: the current GUI comparison reads shared diagnostic state rather than validating rendered rows; the returned-node check establishes final-state stability, while deliberate stale-result rejection is directly exercised by the host harness. Also, the revoked-submit installed check waits for an unchanged task count, which alone does not prove the command has finished processing; a stronger completion barrier would improve that evidence.

## Release and completion boundaries

- The existing [MS10 completion record](MS10_DISTRIBUTED_COMPLETION.md) still says MS10 is incomplete and explicitly prohibits inferring that MS11 is unblocked from host tests. This readout does not supersede that gate or prove its missing installed lifecycle.
- Current ISO presence/timestamps are not source-to-artifact parity proof. At review, the x86_64 installer was dated Sep 24 16:43 and AArch64 Sep 24 14:27; the latest MS11 commit was 16:37. In particular, the AArch64 timestamp predates that commit. No claim is made that both published ISOs contain the final MS11 changes.
- No installed AArch64 multi-node execution result, physical-machine proof, throughput benchmark, desktop-responsiveness measurement under distributed load, or real accelerator inference measurement is established here.
- The result digest is a deterministic fixture identity, not a cryptographic content-integrity proof or a checksum over fetched object bytes. Input references are carried by contract; arbitrary input-object processing is not demonstrated.
- Remote compute is bounded fixture execution, not a general binary deployment/runtime system. Checkpoint transfer, transparent migration and distributed local-LLM acceleration remain outside the delivered path.

**Bottom line:** MS11 has substantial implemented and host-verified distributed-compute mechanics. An unconditional installed-system completion claim requires a retained, artifact-pinned successful three-node receipt, resolution of the stated prerequisite gate, and architecture-specific packaging verification.

## Reproduction

From the repository root, rerun the three host commands in the evidence table. The installed acceptance runner is separate from `make milestone-11-test`:

```sh
python3 tools/ms11-installed-compute.py --output /tmp/infinity-ms11-acceptance-NEW
```

Use a new output directory. The runner requires QEMU, its configured x86_64 EDK2 firmware (override with `--firmware`), `builds/InfinityOS-x86_64.iso`, `build/x86_64/kernel.elf`, and `build/x86_64/installed-kernel.elf`, plus the prerequisites of its shared installed-guest helper. It creates test guests/disks; it is not a read-only check of an existing desktop VM. `--prepare-only` and `--run-only` split installation from runtime acceptance. Retain artifact hashes, structured receipts and screenshots together, and record the exact source revision used to build the artifacts.
