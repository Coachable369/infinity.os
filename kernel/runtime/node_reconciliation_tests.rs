use super::*;
use execution::SecurityIdentity;
use iop::OperationId;
use node::reconciliation::request;

// ------------------------=
// FUNC: authenticated_transcript_invalidates_node_presentation
// DESC: Exchanges real signed pairing frames and checks that asynchronous verification arrival changes the retained view without a control action.
// ------------------=
#[test]
fn authenticated_transcript_invalidates_node_presentation() {
    use network::types::{Endpoint, IpAddress};
    use node::transport::LinkSnapshot;
    let mut left = InfinityRuntime::new(false);
    let mut right = InfinityRuntime::new(false);
    let lid = left.nodes.initialize(&[81; 32], true).unwrap();
    let rid = right.nodes.initialize(&[82; 32], true).unwrap();
    left.nodes.discover(right.nodes.advertise(1, 1, 1).unwrap(), 1).unwrap();
    right.nodes.discover(left.nodes.advertise(1, 1, 1).unwrap(), 1).unwrap();
    left.node_transport.trust.initialize(&[83; 32]);
    right.node_transport.trust.initialize(&[84; 32]);
    left.node_selection = Some(rid);
    right.node_selection = Some(lid);
    let local = Endpoint { address: IpAddress::V4([10, 42, 0, 1]), port: 49152 };
    let remote = Endpoint { address: IpAddress::V4([10, 42, 0, 2]), port: 49152 };
    let ll = LinkSnapshot { connection: 1, peer: Some(rid), local, remote };
    let rl = LinkSnapshot { connection: 2, peer: Some(lid), local: remote, remote: local };
    left.node_transport.trust.begin(&mut left.nodes, ll, 0, false, 2).unwrap();
    let waiting = node_client::presentation(&left);
    assert!(waiting.verification.is_none());
    for now in 3..12 {
        left.node_clock = Some(now);
        right.node_clock = Some(now);
        if let Some(packet) = left.node_transport.trust.outgoing(1, now) {
            right.node_transport.trust.ingest(&mut right.nodes, rl, &packet.bytes[..packet.length as usize], now).unwrap();
            left.node_transport.trust.sent(1);
        }
        if let Some(packet) = right.node_transport.trust.outgoing(2, now) {
            left.node_transport.trust.ingest(&mut left.nodes, ll, &packet.bytes[..packet.length as usize], now).unwrap();
            right.node_transport.trust.sent(2);
        }
        if node_client::presentation(&left).verification.is_some() && node_client::presentation(&right).verification.is_some() { break; }
    }
    let ready = node_client::presentation(&left);
    assert_ne!(ready, waiting);
    let a = ready.verification.unwrap();
    let b = node_client::presentation(&right).verification.unwrap();
    assert_eq!(a.fingerprint, b.fingerprint);
    assert_eq!(a.code, b.code);
    assert_eq!(a.peer, b.local);
    assert!(ready.remaining > 0);
    let mut approval = request(rid.0, OperationId::NodePairConfirm);
    approval.handle = a.pairing;
    approval.value = a.code;
    approval.flags = iop::NODE_OPERATION_HUMAN_APPROVED;
    left.nodes.commit_wire_control(&mut left.node_transport.trust, Some(ll), OperationId::NodePairConfirm, approval, 12, 13, 14, |_| true).unwrap();
    assert_ne!(node_client::presentation(&left), ready);
    assert_eq!(left.nodes.discovered_nodes()[0].unwrap().trust, node::types::TrustState::PairingPending);
    left.node_clock = Some(a.expires);
    left.node_transport.trust.tick(&mut left.nodes, a.expires);
    assert!(node_client::presentation(&left).verification.is_none());
}

// ------------------------=
// FUNC: node_presentation_tracks_visible_state_not_idle_clock
// DESC: Exercises retained-view invalidation for selection, committed changes and projection recovery while idle ticks remain equal.
// ------------------=
#[test]
fn node_presentation_tracks_visible_state_not_idle_clock() {
    let mut runtime = InfinityRuntime::new(false);
    runtime.nodes.initialize(&[51; 32], true).unwrap();
    let mut peer = node::NodeRuntime::new();
    let id = peer.initialize(&[52; 32], true).unwrap();
    runtime.nodes.discover(peer.advertise(1, 1, 1).unwrap(), 1).unwrap();
    let initial = node_client::presentation(&runtime);
    runtime.node_clock = Some(123);
    assert_eq!(node_client::presentation(&runtime), initial);
    runtime.node_selection = Some(id);
    let selected = node_client::presentation(&runtime);
    assert_ne!(selected, initial);
    runtime.nodes.commit_control(OperationId::NodeBlock, request(id.0, OperationId::NodeBlock), 2, 3, 4, |_| true).unwrap();
    assert_ne!(node_client::presentation(&runtime), selected);
    let committed = node_client::presentation(&runtime);
    runtime.node_projection.stale = !runtime.node_projection.stale;
    assert_ne!(node_client::presentation(&runtime), committed);
    assert!(!crate::ui::redraw::clock_change_requires_structural_redraw(4, true));
}

// ------------------------=
// FUNC: pairing_input_is_exclusive_scoped_and_expires
// DESC: Exercises trusted input ownership, surface isolation, expiry and stale-lease rejection without a textual oracle.
// ------------------=
#[test]
fn pairing_input_is_exclusive_scoped_and_expires() {
    use crate::ui::trusted::{TrustedUiManager, TrustedSurface, TrustedUiError};
    let mut manager = TrustedUiManager::new();
    assert_eq!(manager.acquire_secure_input(false, 7, TrustedSurface::NodePairing, 60), Err(TrustedUiError::AccessDenied));
    let lease = manager.acquire_secure_input(true, 7, TrustedSurface::NodePairing, 60).unwrap();
    assert!(manager.input_is_for(7, TrustedSurface::NodePairing, 59));
    assert!(!manager.input_is_for(8, TrustedSurface::NodePairing, 59));
    assert!(!manager.input_is_for(7, TrustedSurface::Authentication, 59));
    assert!(!manager.input_is_for(7, TrustedSurface::NodePairing, 60));
    assert_eq!(manager.acquire_secure_input(true, 8, TrustedSurface::NodePairing, 80), Err(TrustedUiError::Busy));
    assert!(manager.expire(60));
    let replacement = manager.acquire_secure_input(true, 7, TrustedSurface::NodePairing, 120).unwrap();
    assert_eq!(manager.release_secure_input(lease), Err(TrustedUiError::AccessDenied));
    assert!(manager.authorize_trusted_window(lease).is_err());
    manager.release_secure_input(replacement).unwrap();
    assert_eq!(manager.secure_input_owner(61), None);
}

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

// ------------------------=
// FUNC: durable_link_configuration_provisions_and_reclaims_exact_authority
// DESC: Applies real endpoint intent via local IOP, restores it from the binary object, and verifies deferred NIC binding, policy narrowing and resource reclamation.
// ------------------=
#[test]
fn durable_link_configuration_provisions_and_reclaims_exact_authority() {
    use network::types::*;
    let mut runtime = InfinityRuntime::new(false); runtime.define_bootstrap().unwrap(); runtime.start_all(0);
    runtime.nodes.initialize(&[71; 32], true).unwrap(); runtime.node_transport.initialize(&[72; 32], true).unwrap();
    let caller = SecurityIdentity([81; 16]); let service = runtime.service_identity(SERVICE_NODE_TRUST).unwrap();
    let owner = runtime.service_identity(SERVICE_NODE_DISCOVERY).unwrap();
    runtime.iop.ensure_owned_endpoint(1, service).unwrap(); runtime.iop.ensure_owned_endpoint(2, caller).unwrap();
    let op = OperationId::NodeLinkConfigure;
    let mut input = request([0; 32], op); input.handle = 1; input.node_id[..4].copy_from_slice(&[10, 42, 0, 1]); input.node_id[4..8].copy_from_slice(&[10, 42, 0, 2]); input.rights = 49152; input.value = 49152;
    let cap = runtime.capabilities.grant(CapabilityType::ServiceCall, op.machine_id() as u64, 1, 0, service, caller, Some(100), 0).unwrap();
    let mut disk = [0; node::types::NODE_STATE_BYTES];
    runtime.iop.send(1, iop::IopMessage::request(op, 1, caller, cap, 99, 2, &input.encode()).unwrap(), &runtime.capabilities, 1).unwrap();
    runtime.iop.dispatch_node_transaction(&runtime.capabilities, &mut runtime.nodes, op, 1, 2, service, 1, |nodes, input, correlation, causation| nodes.commit_link_configuration(op, input, 1, correlation, causation, |bytes| { disk.copy_from_slice(bytes); true })).unwrap();
    runtime.iop.receive(2, 1).unwrap(); runtime.capabilities.retire_leaf(cap, service).unwrap();
    let mut restored = node::NodeRuntime::new(); restored.restore_state(&disk).unwrap(); assert_eq!(restored.configured_links(), runtime.nodes.configured_links());
    let caps_before = runtime.capabilities.count(); let rules_before = runtime.network.policy.count();
    for tick in 2..50 { runtime.node_links.reconcile(&mut runtime.nodes, &mut runtime.node_transport, &mut runtime.network, &mut runtime.capabilities, owner, tick); }
    assert!(runtime.node_links.errors[0].is_some()); assert_eq!(runtime.capabilities.count(), caps_before); assert_eq!(runtime.network.policy.count(), rules_before);
    runtime.network.register_firmware_device(FirmwareNetworkDevice { firmware_handle: 0, device_id: 0x100e8086, hardware_address: Some([2, 0, 0, 0, 0, 1]), link_state: LinkState::Up, maximum_frame_size: 1500, can_receive: true, can_transmit: true }).unwrap();
    runtime.network.interfaces.replace_static_ipv4(2, IpAddress::V4([10, 42, 0, 1]), 24, None, 100).unwrap();
    runtime.network.connections.bind_native_address(Some([10, 42, 0, 1]));
    runtime.node_links.reconcile(&mut runtime.nodes, &mut runtime.node_transport, &mut runtime.network, &mut runtime.capabilities, owner, 50);
    assert_eq!(runtime.node_links.errors[0], None); assert_eq!(runtime.network.connections.count(), 1); assert_eq!(runtime.capabilities.count(), caps_before + 3); assert_eq!(runtime.network.policy.count(), rules_before + 2);
    assert!(runtime.nodes.discovered_nodes().iter().all(Option::is_none)); // Carriage never fabricates discovery or trust.
    let remove = iop::NodeOperationV1 { handle: 1, ..request([0; 32], OperationId::NodeLinkRemove) };
    runtime.nodes.commit_link_configuration(OperationId::NodeLinkRemove, remove, 51, 3, 4, |_| true).unwrap();
    runtime.node_links.reconcile(&mut runtime.nodes, &mut runtime.node_transport, &mut runtime.network, &mut runtime.capabilities, owner, 51);
    assert_eq!(runtime.network.connections.count(), 0); assert_eq!(runtime.capabilities.count(), caps_before); assert_eq!(runtime.network.policy.count(), rules_before);
    let before = runtime.nodes.encode_state().unwrap();
    assert!(runtime.nodes.commit_link_configuration(op, input, 52, 3, 4, |_| false).is_err()); assert_eq!(runtime.nodes.encode_state().unwrap(), before);
    input.node_id[4] = 127;
    assert!(runtime.nodes.commit_link_configuration(op, input, 53, 3, 4, |_| panic!("invalid endpoint reached persistence")).is_err());
}
