#![allow(dead_code)]

#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "storage.rs"]
mod storage;
#[path = "../kernel/ui/mod.rs"]
mod ui;

mod console {
    #[derive(Clone, Copy)]
    pub enum ConsoleKey { Character(u8), Backspace, Delete, Left, Right, Home, End, Enter }
}

// ------------------------=
// FUNC: output_text
// DESC: Provides the kernel diagnostic sink required by the host behavior harness.
// ------------------=
fn output_text(_: &[u8]) {}

use runtime::capability::{CapabilityManager, CapabilityType};
use runtime::ai::{AiRuntime, provider::ProviderDescriptor, types::{AiError, InferenceOptions, ModelExecutionRequest, PrivacyPolicy, ProviderPolicy, ResourcePolicy, WorkloadClass, CAP_VISION}};
use runtime::compute::{
    execute_remote_dispatch, publish_notice, ComputeDispatchV1, ComputeDurability, ComputeError, ComputeLocality, ComputeRequestV1,
    ComputeResultV1, ComputeService, ComputeState, NodeComputeObservation, WorkloadKind,
    COMPUTE_RESULT_CONTRACT_V1, COMPUTE_SCHEMA_VERSION,
};
use runtime::event::{EventFabric, EventFilter, OverflowPolicy};
use runtime::execution::{ExecutionManager, SecurityIdentity};
use runtime::fabric::resources::{Directory, Health, Resource, ResourceId, ResourceKind};
use runtime::iop::{IopMessage, IopRouter, OperationId};
use runtime::node::types::NodeId;
use runtime::task_manager::TaskManager;
use std::{cell::RefCell, rc::Rc};
use storage::{BlockDevice, object::{ObjectStore, STORE_RELATIVE_LBA}};

#[derive(Clone)]
struct MemoryDisk(Rc<RefCell<Vec<[u8; 512]>>>);

impl BlockDevice for MemoryDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Reports the bounded fresh-install fixture capacity.
    // ------------------=
    fn block_count(&self) -> u64 { self.0.borrow().len() as u64 }

    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one sector from the behavioral fresh-install fixture.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool { self.0.borrow().get(lba as usize).copied().map(|sector| *out = sector).is_some() }

    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes one sector to the behavioral fresh-install fixture.
    // ------------------=
    fn write_sector(&mut self, lba: u64, input: &[u8; 512]) -> bool { self.0.borrow_mut().get_mut(lba as usize).map(|sector| *sector = *input).is_some() }

    // ------------------------=
    // FUNC: flush
    // DESC: Completes the in-memory durable boundary synchronously.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}

// ------------------------=
// FUNC: node
// DESC: Creates one deterministic independent installed-node identity for the native fixture.
// ------------------=
fn node(value: u8) -> NodeId { NodeId([value; 32]) }

// ------------------------=
// FUNC: resource
// DESC: Creates one leased healthy resource observation with explicit kind and capacity.
// ------------------=
fn resource(owner: NodeId, id: u8, kind: ResourceKind, capacity: u64) -> Resource {
    Resource { id: ResourceId([id; 16]), owner, kind, device: [id.wrapping_add(32); 16], capacity, available: capacity, reserved: 0, health: Health::Healthy, online: true, capabilities: 1, generation: 1, sequence: 1, expires: 10_000 }
}

// ------------------------=
// FUNC: add_node_resources
// DESC: Adds independently identified compute, memory, and optional accelerator capacity to the real Resource Directory.
// ------------------=
fn add_node_resources(directory: &mut Directory, owner: NodeId, id: u8, accelerator: bool) {
    directory.observe_local(resource(owner, id, ResourceKind::Compute, 1_000), owner, 1).unwrap();
    directory.observe_local(resource(owner, id + 1, ResourceKind::Memory, 512 * 1024 * 1024), owner, 1).unwrap();
    if accelerator { directory.observe_local(resource(owner, id + 2, ResourceKind::Accelerator, 1_000), owner, 1).unwrap(); }
}

// ------------------------=
// FUNC: request
// DESC: Builds a bounded version-one request whose policy fields can be changed by each behavioral scenario.
// ------------------=
fn request(kind: WorkloadKind, durability: ComputeDurability, capability_ref: u64, workload: u8) -> ComputeRequestV1 {
    ComputeRequestV1 { schema_version: COMPUTE_SCHEMA_VERSION, workload_kind: kind, locality: ComputeLocality::RequireRemote, durability, priority: 128, privacy_local_only: false, workload_id: [workload; 16], input_refs: [[3; 16], [7; 16]], allowed_nodes: [node(2), node(3)], allowed_node_count: 2, allowed_domains: [20, 30], allowed_domain_count: 2, memory_bytes: 2 * 1024 * 1024, deadline: 1_000, correlation_id: workload as u64 * 100, capability_ref, affinity: 0, anti_affinity: 0, work_units: 10, cpu_units: 25, restart_eligible: durability == ComputeDurability::Restartable, result_contract: COMPUTE_RESULT_CONTRACT_V1 }
}

// ------------------------=
// FUNC: grant_compute
// DESC: Grants exact workload-scoped Compute.Use authority to the fixture caller.
// ------------------=
fn grant_compute(capabilities: &mut CapabilityManager, caller: SecurityIdentity, workload: u8) -> u64 {
    capabilities.grant(CapabilityType::ComputeUse, u64::from_le_bytes([workload; 8]), 1, 0, caller, caller, Some(2_000), 0).unwrap()
}

// ------------------------=
// FUNC: main
// DESC: Behaviorally verifies typed IOP, placement, authority, cancellation, deadlines, failover fencing, event recovery, accounting, UI parity, and accelerator scheduling.
// ------------------=
fn main() {
    let sectors = STORE_RELATIVE_LBA as usize + 65_536; let disk = MemoryDisk(Rc::new(RefCell::new(vec![[0; 512]; sectors])));
    let mut installed = ObjectStore::format(disk.clone(), 0, sectors as u64, [0x44; 16]).unwrap();
    let state_id = installed.resolve(b"/system/runtime").unwrap(); let mut installed_state = [0; runtime::compute::COMPUTE_STATE_BYTES];
    let mut runtime_state = [0; 16 + runtime::compute::COMPUTE_STATE_BYTES]; assert_eq!(installed.read(state_id, None, &mut runtime_state).unwrap(), runtime_state.len()); installed_state.copy_from_slice(&runtime_state[16..]);
    let mut fresh_compute = ComputeService::new(); fresh_compute.restore_state(&installed_state).unwrap(); assert_eq!(fresh_compute.task_count(), 0);
    drop(installed); let remounted = ObjectStore::mount(disk, 0).unwrap(); assert_eq!(remounted.resolve(b"/system/runtime").unwrap(), state_id);

    let local = node(1); let peer_b = node(2); let peer_c = node(3); let caller = SecurityIdentity([9; 16]);
    let mut directory = Directory::new(); add_node_resources(&mut directory, local, 10, false); add_node_resources(&mut directory, peer_b, 20, false); add_node_resources(&mut directory, peer_c, 30, true);
    for kind in [ResourceKind::Compute, ResourceKind::Memory, ResourceKind::Accelerator] {
        let advertised = resource(peer_b, 90 + kind as u8, kind, 4_096);
        let wire = runtime::fabric::resource_protocol::encode(advertised, 60).unwrap();
        let decoded = runtime::fabric::resource_protocol::decode(wire, peer_b, 10).unwrap();
        assert_eq!(decoded.kind, kind); assert_eq!(decoded.capacity, 4_096); assert_eq!(decoded.expires, 70);
    }
    let mut compute = ComputeService::new();
    compute.observe_node(&directory, NodeComputeObservation { node: local, trust_domain: 10, latency_us: 20, latency_known: true, load_percent: 5, generation: 1 }).unwrap();
    compute.observe_node(&directory, NodeComputeObservation { node: peer_b, trust_domain: 20, latency_us: 100, latency_known: true, load_percent: 10, generation: 1 }).unwrap();
    compute.observe_node(&directory, NodeComputeObservation { node: peer_c, trust_domain: 30, latency_us: 300, latency_known: true, load_percent: 20, generation: 1 }).unwrap();
    let mut capabilities = CapabilityManager::new();

    let use_cap = grant_compute(&mut capabilities, caller, 11); let original = request(WorkloadKind::CpuChecksum, ComputeDurability::Pinned, use_cap, 11);
    let decoded = ComputeRequestV1::decode(&original.encode()).unwrap(); assert_eq!(decoded, original);
    let service_cap = capabilities.grant(CapabilityType::ServiceCall, OperationId::ComputeRequest as u64, 1, 0, caller, caller, Some(2_000), 0).unwrap();
    let mut router = IopRouter::new(); router.register_endpoint(11, caller).unwrap();
    let message = IopMessage::request(OperationId::ComputeRequest, 1, caller, service_cap, 1_000, original.correlation_id, &original.encode()).unwrap();
    router.send(11, message, &capabilities, 10).unwrap(); let received = router.receive(11, 10).unwrap();
    let wire_request = ComputeRequestV1::decode(&received.payload[..received.header.payload_length as usize]).unwrap();
    let remote_task = compute.request(wire_request, caller, &capabilities, &mut directory, local, 10).unwrap();
    assert_eq!(compute.inspect(remote_task).unwrap().node, peer_b);
    let mut execution_b = ExecutionManager::new(); let mut execution_c = ExecutionManager::new();
    compute.start(remote_task, 1, caller, &capabilities, &mut execution_b, &mut directory, 11).unwrap();
    let completed = compute.run_slice(remote_task, 1, caller, &capabilities, &mut execution_b, 100, &mut directory, 12).unwrap();
    assert_eq!(completed.state, ComputeState::Completed); assert_eq!(completed.accounting.used_cpu_ticks, 10); assert_eq!(execution_b.count(), 0);
    let result = ComputeResultV1 { schema_version: COMPUTE_SCHEMA_VERSION, snapshot: completed }; assert_eq!(ComputeResultV1::decode(&result.encode()).unwrap().snapshot, completed);
    let dispatch = ComputeDispatchV1 { schema_version: COMPUTE_SCHEMA_VERSION, workload_kind: WorkloadKind::CpuChecksum, state: ComputeState::Placed as u8, operation: OperationId::ComputeRequest as u32, task_id: 900, epoch: 4, work_units: 70, memory_bytes: 1024 * 1024, deadline: 500, scope: 77, workload_id: [21; 16], input_refs: [[22; 16], [23; 16]], target_node: peer_b, cpu_units: 10, priority: 128, result_contract: COMPUTE_RESULT_CONTRACT_V1, error: 0, flags: 0, used_ticks: 0 };
    assert_eq!(ComputeDispatchV1::decode(&dispatch.encode()).unwrap(), dispatch);
    let remote_slice = execute_remote_dispatch(dispatch, &mut execution_b, peer_b, 100).unwrap(); assert_eq!(remote_slice.state, ComputeState::Running as u8); assert_eq!(remote_slice.used_ticks, 64); assert_eq!(remote_slice.work_units, 6); assert_eq!(execution_b.count(), 0);

    let denied = request(WorkloadKind::CpuChecksum, ComputeDurability::Pinned, 999_999, 12);
    assert_eq!(compute.request(denied, caller, &capabilities, &mut directory, local, 20), Err(ComputeError::AccessDenied));
    let revoked_cap = grant_compute(&mut capabilities, caller, 13); let revoked_task = compute.request(request(WorkloadKind::CpuChecksum, ComputeDurability::Pinned, revoked_cap, 13), caller, &capabilities, &mut directory, local, 20).unwrap();
    capabilities.revoke(revoked_cap).unwrap();
    assert_eq!(compute.start(revoked_task, 1, caller, &capabilities, &mut execution_b, &mut directory, 21), Err(ComputeError::AccessDenied)); assert_eq!(compute.inspect(revoked_task).unwrap().state, ComputeState::Failed);

    let cancel_use = grant_compute(&mut capabilities, caller, 14); let mut long = request(WorkloadKind::CpuBoundedCounter, ComputeDurability::Pinned, cancel_use, 14); long.work_units = 10_000;
    let cancel_task = compute.request(long, caller, &capabilities, &mut directory, local, 30).unwrap(); compute.start(cancel_task, 1, caller, &capabilities, &mut execution_b, &mut directory, 31).unwrap();
    let cancel_cap = capabilities.grant(CapabilityType::ComputeCancel, cancel_task, 1, 0, caller, caller, Some(2_000), 0).unwrap();
    compute.cancel(cancel_task, caller, cancel_cap, &capabilities, &mut execution_b, &mut directory, 32).unwrap(); assert_eq!(compute.inspect(cancel_task).unwrap().state, ComputeState::Cancelled); assert_eq!(execution_b.count(), 0);

    let deadline_cap = grant_compute(&mut capabilities, caller, 15); let mut expiring = request(WorkloadKind::CpuChecksum, ComputeDurability::Pinned, deadline_cap, 15); expiring.deadline = 40;
    let deadline_task = compute.request(expiring, caller, &capabilities, &mut directory, local, 35).unwrap(); assert_eq!(compute.start(deadline_task, 1, caller, &capabilities, &mut execution_b, &mut directory, 40), Err(ComputeError::DeadlineExceeded));

    let restart_cap = grant_compute(&mut capabilities, caller, 16); let restart_task = compute.request(request(WorkloadKind::CpuBoundedCounter, ComputeDurability::Restartable, restart_cap, 16), caller, &capabilities, &mut directory, local, 50).unwrap(); compute.start(restart_task, 1, caller, &capabilities, &mut execution_b, &mut directory, 51).unwrap(); compute.run_slice(restart_task, 1, caller, &capabilities, &mut execution_b, 3, &mut directory, 52).unwrap();
    let pinned_cap = grant_compute(&mut capabilities, caller, 17); let pinned_task = compute.request(request(WorkloadKind::CpuChecksum, ComputeDurability::Pinned, pinned_cap, 17), caller, &capabilities, &mut directory, local, 50).unwrap(); compute.start(pinned_task, 1, caller, &capabilities, &mut execution_b, &mut directory, 51).unwrap();
    directory.mark_peer_offline(peer_b); compute.node_lost(peer_b, &mut directory, local, 53);
    let replaced = compute.inspect(restart_task).unwrap(); assert_eq!(replaced.state, ComputeState::Placed); assert_eq!(replaced.node, peer_c); assert_eq!(replaced.epoch, 2); assert_eq!(replaced.accounting.restart_count, 1);
    assert_eq!(compute.inspect(pinned_task).unwrap().state, ComputeState::Failed);
    assert_eq!(compute.run_slice(restart_task, 1, caller, &capabilities, &mut execution_b, 10, &mut directory, 54), Err(ComputeError::StaleResult));
    compute.start(restart_task, 2, caller, &capabilities, &mut execution_c, &mut directory, 55).unwrap(); let restarted_done = compute.run_slice(restart_task, 2, caller, &capabilities, &mut execution_c, 64, &mut directory, 56).unwrap(); assert_eq!(restarted_done.state, ComputeState::Completed);

    let task_manager = TaskManager::new(); assert_eq!(task_manager.distributed_task_count(&compute), compute.task_count()); assert_eq!(task_manager.distributed_inspect(&compute, restart_task).unwrap(), compute.inspect(restart_task).unwrap()); assert_eq!(task_manager.distributed_task_nth(&compute, 0), compute.task_nth(0));

    let completed_notice = (0..compute.notice_count()).filter_map(|index| compute.notice_nth(index)).find(|notice| notice.task_id == restart_task && notice.state == ComputeState::Completed).unwrap();
    let event_type = completed_notice.event_type(); let publish_cap = capabilities.grant(CapabilityType::EventPublish, event_type as u64, 1, restart_task, caller, caller, Some(2_000), 0).unwrap(); let subscribe_cap = capabilities.grant(CapabilityType::EventSubscribe, event_type as u64, 1, restart_task, caller, caller, Some(2_000), 0).unwrap();
    let mut events = EventFabric::new(); let lease = events.subscribe(caller, subscribe_cap, EventFilter { type_id: event_type, scope: Some(restart_task) }, 2_000, OverflowPolicy::DropOldest, 1, &capabilities, 100).unwrap();
    publish_notice(completed_notice, caller, publish_cap, &mut events, &capabilities, 101).unwrap(); events.receive(lease, 101).unwrap(); publish_notice(completed_notice, caller, publish_cap, &mut events, &capabilities, 102).unwrap(); publish_notice(completed_notice, caller, publish_cap, &mut events, &capabilities, 103).unwrap(); events.receive(lease, 103).unwrap(); assert!(events.sequence_gap(lease).unwrap()); assert_eq!(compute.inspect(restart_task).unwrap().state, ComputeState::Completed);

    let accelerator_cap = grant_compute(&mut capabilities, caller, 18); let accelerator_task = compute.request(request(WorkloadKind::AcceleratorInferenceFixture, ComputeDurability::Pinned, accelerator_cap, 18), caller, &capabilities, &mut directory, local, 110).unwrap(); assert_eq!(compute.inspect(accelerator_task).unwrap().node, peer_c); compute.start(accelerator_task, 1, caller, &capabilities, &mut execution_c, &mut directory, 111).unwrap(); let accelerator_result = compute.run_slice(accelerator_task, 1, caller, &capabilities, &mut execution_c, 64, &mut directory, 112).unwrap(); assert_eq!(accelerator_result.state, ComputeState::Completed); assert_ne!(accelerator_result.output_digest, [0; 16]);

    let mut ai = AiRuntime::new(); ai.initialize().unwrap(); ai.providers.register(ProviderDescriptor { id: 42, local: false, online: true, capabilities: CAP_VISION, privacy_floor: PrivacyPolicy::SystemMetadata, latency_class: 2, power_class: 2, quality_class: 2 }).unwrap();
    let ai_cap = grant_compute(&mut capabilities, caller, 24); let mut ai_request = ModelExecutionRequest { model: None, capability_class: CAP_VISION, input: b"provider-neutral fixture", input_refs: [[31; 16], [0; 16], [0; 16], [0; 16]], input_ref_count: 1, options: InferenceOptions { maximum_output_units: 8, deterministic: true, priority: 160 }, resource_policy: ResourcePolicy { workload: WorkloadClass::Interactive, memory_limit: 2 * 1024 * 1024, cpu_weight: 20, queue_limit: 2 }, privacy_policy: PrivacyPolicy::SystemMetadata, provider_policy: ProviderPolicy::RemoteAllowed, deadline: 1_000, correlation_id: 2_400, caller, caller_capability: 0 };
    let ai_task = ai.request_distributed_inference(&ai_request, [24; 16], ai_cap, &mut compute, &capabilities, &mut directory, local, 130).unwrap(); assert_eq!(compute.inspect(ai_task).unwrap().node, peer_c); assert_eq!(compute.inspect(ai_task).unwrap().workload_kind, WorkloadKind::AcceleratorInferenceFixture);
    ai_request.privacy_policy = PrivacyPolicy::Personal; assert_eq!(ai.request_distributed_inference(&ai_request, [24; 16], ai_cap, &mut compute, &capabilities, &mut directory, local, 130), Err(AiError::PrivacyDenied));

    assert!(compute.needs_persistence()); let persisted = compute.encode_state(); let mut restored = ComputeService::new(); restored.restore_state(&persisted).unwrap(); assert_eq!(restored.inspect(restart_task).unwrap(), compute.inspect(restart_task).unwrap()); assert_eq!(restored.inspect(pinned_task).unwrap().state, ComputeState::Failed); assert!(!restored.needs_persistence());

    let unsupported_cap = grant_compute(&mut capabilities, caller, 19); assert_eq!(compute.request(request(WorkloadKind::CpuChecksum, ComputeDurability::CheckpointableScaffold, unsupported_cap, 19), caller, &capabilities, &mut directory, local, 120), Err(ComputeError::UnsupportedDurability));
    println!("Milestone 11 native distributed compute behavior: PASS");
}
