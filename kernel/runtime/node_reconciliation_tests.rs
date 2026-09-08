use super::*;
use execution::SecurityIdentity;
use iop::OperationId;
use node::reconciliation::request;

// ------------------------=
// FUNC: event_loss_rebuilds_settings_projection_without_capability_leaks
// DESC: Uses the real bootstrapped services, capability manager, IOP broker and IEF to recover dropped and missing checkpoint notifications.
// ------------------=
#[test]
fn event_loss_rebuilds_settings_projection_without_capability_leaks() {
    let mut runtime = InfinityRuntime::new(false);
    runtime.define_bootstrap().unwrap(); runtime.start_all(0);
    runtime.nodes.initialize(&[51; 32], true).unwrap();
    let mut peer = node::NodeRuntime::new(); let id = peer.initialize(&[52; 32], true).unwrap();
    runtime.nodes.discover(peer.advertise(1, 1, 1).unwrap(), 1).unwrap();
    refresh_node_projection(&mut runtime, 1);
    assert!(!runtime.node_projection.stale); assert_eq!(runtime.node_projection.node_count, 1);
    let capability_count = runtime.capabilities.count();
    for tick in 2..20 {
        let operation = if tick % 2 == 0 { OperationId::NodeBlock } else { OperationId::NodeUnblock };
        let (_, notice) = runtime.nodes.commit_control(operation, request(id.0, operation), tick, tick + 100, tick + 200, |_| true).unwrap();
        publish_node_checkpoint(&mut runtime, notice.version, notice.correlation, notice.causation, tick);
    }
    // A one-entry LatestOnly queue cannot retain these intermediate checkpoints.
    refresh_node_projection(&mut runtime, 20);
    assert!(!runtime.node_projection.stale); assert!(runtime.node_projection.gaps > 0);
    assert_eq!(runtime.node_projection.version, runtime.nodes.control_version());
    assert_eq!(runtime.node_projection.nodes[0][85], 1);
    runtime.nodes.commit_control(OperationId::NodeBlock, request(id.0, OperationId::NodeBlock), 21, 1, 2, |_| true).unwrap();
    // No notification at all: periodic typed IOP still discovers the new checkpoint.
    refresh_node_projection(&mut runtime, 21);
    assert_eq!(runtime.node_projection.nodes[0][85], 6);
    let refreshes = runtime.node_projection.refreshes;
    for _ in 0..1_000 { refresh_node_projection(&mut runtime, 21); }
    assert_eq!(runtime.node_projection.refreshes, refreshes);
    for tick in 22..300 { refresh_node_projection(&mut runtime, tick); }
    assert_eq!(runtime.capabilities.count(), capability_count);
    assert_eq!(runtime.node_projection.refreshes, refreshes);
}

// ------------------------=
// FUNC: missing_read_authority_keeps_last_valid_view_stale
// DESC: Exhausts capability slots to verify read backpressure cannot install partial data, then recovers after capacity is released.
// ------------------=
#[test]
fn missing_read_authority_keeps_last_valid_view_stale() {
    let mut runtime = InfinityRuntime::new(false); runtime.define_bootstrap().unwrap(); runtime.start_all(0);
    runtime.nodes.initialize(&[61; 32], true).unwrap();
    refresh_node_projection(&mut runtime, 0); assert!(!runtime.node_projection.stale);
    let issuer = SecurityIdentity([3; 16]);
    let mut last = None;
    while let Ok(cap) = runtime.capabilities.grant(CapabilityType::ServiceCall, 77, 1, 0, issuer, issuer, None, 0) { last = Some(cap); }
    let before = runtime.node_projection.version;
    refresh_node_projection(&mut runtime, 1); assert!(runtime.node_projection.stale); assert_eq!(runtime.node_projection.version, before);
    runtime.capabilities.retire_leaf(last.unwrap(), issuer).unwrap();
    refresh_node_projection(&mut runtime, 2); assert!(!runtime.node_projection.stale);
}
