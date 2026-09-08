#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"] mod ui;
#[path = "../kernel/runtime/mod.rs"] mod runtime;

// ------------------------=
// FUNC: output_text
// DESC: Provides the host-test kernel output boundary.
// ------------------=
fn output_text(_: &[u8]) {}

use runtime::capability::{CapabilityManager, CapabilityType};
use runtime::console_language::{parse, ParseOutcome, SideEffectClass, ValueType};
use runtime::event::{EventClass, EventFabric, EventFilter, OverflowPolicy, RoutingDomain};
use runtime::execution::SecurityIdentity;
use runtime::iop::{NetworkConnectV1, OperationId, NETWORK_CONNECT_V1_BYTES};
use runtime::network::discovery::DiscoveredService;
use runtime::network::policy::{EndpointSelector, NetworkPolicyRule};
use runtime::network::profile::{NetworkProfile, ProfileKind};
use runtime::network::types::*;
use runtime::network::NetworkRuntime;
use runtime::service::*;
use ui::system_layout::{
    NetworkSettingsTarget, OnboardingTarget, SettingsWindowState, SystemLayout,
};

// ------------------------=
// FUNC: identity
// DESC: Creates a stable test security identity.
// ------------------=
fn identity(value: u8) -> SecurityIdentity { SecurityIdentity([value; 16]) }

// ------------------------=
// FUNC: endpoint
// DESC: Creates a typed IPv4 endpoint for behavioral tests.
// ------------------=
fn endpoint(address: [u8;4], port: u16) -> Endpoint { Endpoint { address: IpAddress::V4(address), port } }

// ------------------------=
// FUNC: route_behavior
// DESC: Verifies IPv4 and IPv6-safe deterministic route selection and mutation.
// ------------------=
fn route_behavior() {
    let mut network = NetworkRuntime::new(); network.initialize().unwrap();
    network.interfaces.add_interface(NetworkInterface { id: 2, device: NetworkDevice { device_id: 22, driver_id: 7, link_type: LinkType::Virtual, hardware_address: None, link_state: LinkState::Up, maximum_frame_size: 1500, can_receive: true, can_transmit: true, offload_capabilities: 0, operational_state: OperationalState::Ready, error_code: 0 }, enabled: true, rx_packets: 0, tx_packets: 0, rx_drops: 0, tx_drops: 0 }).unwrap();
    network.interfaces.add_address(2, IpAddress::V4([10,42,0,2]), 16, AddressScope::Private, AddressSource::TestFixture, None, None).unwrap();
    let default = network.interfaces.add_route(IpAddress::V4([0,0,0,0]), 0, Some(IpAddress::V4([10,42,0,1])), 2, 100, RouteSource::TestFixture, None).unwrap();
    let specific = network.interfaces.add_route(IpAddress::V4([10,42,8,0]), 24, None, 2, 500, RouteSource::TestFixture, None).unwrap();
    assert_eq!(network.interfaces.select_route(IpAddress::V4([10,42,8,9]), None).unwrap().id, specific);
    assert_eq!(network.interfaces.select_route(IpAddress::V4([8,8,8,8]), None).unwrap().id, default);
    network.interfaces.remove_route(default).unwrap();
    assert_eq!(network.interfaces.select_route(IpAddress::V4([8,8,8,8]), None), Err(NetworkError::NoRoute));
    assert_eq!(IpAddress::V6([0;16]).family(), AddressFamily::Ipv6);
}

// ------------------------=
// FUNC: profile_behavior
// DESC: Verifies transactional activation, offline local operation, rollback, and binary persistence.
// ------------------=
fn profile_behavior() {
    let mut network = NetworkRuntime::new(); network.initialize().unwrap();
    let initial = network.status(); assert_eq!(initial.active_profile, 1);
    network.activate_profile(3).unwrap(); assert_eq!(network.status().connectivity, ConnectivityClass::Offline);
    let bytes = network.encode_state();
    let mut restored = NetworkRuntime::new(); restored.initialize().unwrap(); restored.restore_state(&bytes).unwrap(); assert_eq!(restored.status().active_profile, 3);
    let invalid = NetworkProfile { id: 0, kind: ProfileKind::Custom, name_code: 99, interfaces_enabled: false, dynamic_addressing: false, resolver_enabled: true, default_route_enabled: false, local_discovery_enabled: false, internet_allowed: false, inbound_listeners_allowed: false, audit_decisions: true, protected: false };
    assert_eq!(network.profiles.create(invalid), Err(NetworkError::InvalidProfile));
    assert_eq!(network.status().active_profile, 3);
    assert!(network.activate_profile(1).is_ok());
}

// ------------------------=
// FUNC: configurable_state_behavior
// DESC: Verifies static addressing, routes, resolver configuration, interface state, and policy survive binary persistence.
// ------------------=
fn configurable_state_behavior() {
    let adapter = NetworkInterface {
        id: 2,
        device: NetworkDevice {
            device_id: 22,
            driver_id: 7,
            link_type: LinkType::Virtual,
            hardware_address: None,
            link_state: LinkState::Up,
            maximum_frame_size: 1500,
            can_receive: true,
            can_transmit: true,
            offload_capabilities: 0,
            operational_state: OperationalState::Ready,
            error_code: 0,
        },
        enabled: true,
        rx_packets: 0,
        tx_packets: 0,
        rx_drops: 0,
        tx_drops: 0,
    };
    let mut configured = NetworkRuntime::new();
    configured.initialize().unwrap();
    configured.interfaces.add_interface(adapter).unwrap();
    configured.interfaces.replace_static_ipv4(
        2,
        IpAddress::V4([10, 20, 30, 40]),
        24,
        Some(IpAddress::V4([10, 20, 30, 1])),
        75,
    ).unwrap();
    configured.resolver.set_enabled(false);
    configured.resolver.set_server(0, Some(IpAddress::V4([9, 9, 9, 9]))).unwrap();
    configured.resolver.set_server(1, Some(IpAddress::V4([1, 1, 1, 1]))).unwrap();
    configured.policy.set_default_action(PolicyAction::Allow);
    configured.interfaces.set_state(2, false).unwrap();

    let encoded = configured.encode_state();
    let mut restored = NetworkRuntime::new();
    restored.initialize().unwrap();
    restored.interfaces.add_interface(adapter).unwrap();
    restored.restore_state(&encoded).unwrap();

    let address = (0..restored.interfaces.address_count())
        .filter_map(|index| restored.interfaces.address_nth(index))
        .find(|value| value.interface_id == 2 && value.source == AddressSource::Static)
        .unwrap();
    let route = (0..restored.interfaces.route_count())
        .filter_map(|index| restored.interfaces.route_nth(index))
        .find(|value| value.interface_id == 2 && value.source == RouteSource::Static)
        .unwrap();
    assert_eq!(address.address, IpAddress::V4([10, 20, 30, 40]));
    assert_eq!(address.prefix_length, 24);
    assert_eq!(route.next_hop, Some(IpAddress::V4([10, 20, 30, 1])));
    assert_eq!(route.metric, 75);
    assert_eq!(restored.resolver.server(0), Some(IpAddress::V4([9, 9, 9, 9])));
    assert_eq!(restored.resolver.server(1), Some(IpAddress::V4([1, 1, 1, 1])));
    assert!(!restored.resolver.enabled());
    assert_eq!(restored.policy.default_action(), PolicyAction::Allow);
    assert!(!restored.interfaces.interface(2).unwrap().enabled);

    assert!(restored.interfaces.replace_static_ipv4(
        2,
        IpAddress::V4([10, 20, 30, 50]),
        33,
        Some(IpAddress::V4([10, 20, 30, 1])),
        75,
    ).is_err());
    let unchanged = (0..restored.interfaces.address_count())
        .filter_map(|index| restored.interfaces.address_nth(index))
        .find(|value| value.interface_id == 2 && value.source == AddressSource::Static)
        .unwrap();
    assert_eq!(unchanged.address, IpAddress::V4([10, 20, 30, 40]));
    restored.interfaces.set_state(2, true).unwrap();
    let connected = restored.interfaces.select_route(IpAddress::V4([10, 20, 30, 99]), None).unwrap();
    assert_eq!((connected.destination, connected.prefix_length, connected.next_hop, connected.source),
        (IpAddress::V4([10, 20, 30, 0]), 24, None, RouteSource::Interface));
    restored.interfaces.replace_static_ipv4(2, IpAddress::V4([10, 42, 0, 1]), 24, None, 100).unwrap();
    assert_eq!(restored.interfaces.select_route(IpAddress::V4([10, 20, 30, 99]), None), Err(NetworkError::NoRoute));
    assert_eq!(restored.interfaces.select_route(IpAddress::V4([10, 42, 0, 2]), None).unwrap().next_hop, None);
    let saved = restored.encode_state();
    let mut rebooted = NetworkRuntime::new();
    rebooted.initialize().unwrap();
    rebooted.interfaces.add_interface(adapter).unwrap();
    rebooted.restore_state(&saved).unwrap();
    assert_eq!(rebooted.interfaces.select_route(IpAddress::V4([10, 42, 0, 2]), None).unwrap().interface_id, 2);
    let original = rebooted.interfaces.select_route(IpAddress::V4([10, 42, 0, 2]), None).unwrap();
    for octet in 1..MAX_ROUTES as u8 {
        if rebooted.interfaces.add_route(IpAddress::V4([172, octet, 0, 0]), 16, None, 2, 10, RouteSource::Static, None).is_err() { break; }
    }
    let count = rebooted.interfaces.route_count();
    assert_eq!(count, MAX_ROUTES);
    assert_eq!(rebooted.interfaces.replace_static_ipv4(2, IpAddress::V4([10, 43, 0, 1]), 24, Some(IpAddress::V4([10, 43, 0, 254])), 100), Err(NetworkError::ResourceLimitExceeded));
    assert_eq!(rebooted.interfaces.route_count(), count);
    assert_eq!(rebooted.interfaces.select_route(IpAddress::V4([10, 42, 0, 2]), None).unwrap(), original);
    assert_eq!(rebooted.interfaces.select_route(IpAddress::V4([10, 43, 0, 2]), None), Err(NetworkError::NoRoute));
    rebooted.interfaces.remove_static_addresses(2);
    assert_eq!(rebooted.interfaces.select_route(IpAddress::V4([10, 42, 0, 2]), None), Err(NetworkError::NoRoute));
}

// ------------------------=
// FUNC: onboarding_network_behavior
// DESC: Verifies real interface availability, wired and wireless activation, offline persistence, and network-step hit geometry.
// ------------------=
fn onboarding_network_behavior() {
    let mut offline = NetworkRuntime::new();
    offline.initialize().unwrap();
    assert!(!offline.setup_snapshot().wired_available);
    assert!(!offline.setup_snapshot().wireless_available);
    offline.select_setup_mode(NetworkSetupMode::Offline);
    assert_eq!(offline.apply_setup_mode().unwrap(), ConnectivityClass::Offline);
    let state = offline.encode_state();
    let mut restored = NetworkRuntime::new();
    restored.initialize().unwrap();
    restored.restore_state(&state).unwrap();
    assert_eq!(restored.setup_snapshot().selected, NetworkSetupMode::Offline);

    let mut wired = NetworkRuntime::new();
    wired.initialize().unwrap();
    wired.interfaces.add_interface(NetworkInterface { id: 2, device: NetworkDevice { device_id: 22, driver_id: 7, link_type: LinkType::Virtual, hardware_address: None, link_state: LinkState::Up, maximum_frame_size: 1500, can_receive: true, can_transmit: true, offload_capabilities: 0, operational_state: OperationalState::Ready, error_code: 0 }, enabled: false, rx_packets: 0, tx_packets: 0, rx_drops: 0, tx_drops: 0 }).unwrap();
    wired.select_setup_mode(NetworkSetupMode::Wired);
    assert_eq!(wired.apply_setup_mode().unwrap(), ConnectivityClass::LinkOnly);
    assert!(wired.interfaces.interface(2).unwrap().enabled);

    let mut wireless = NetworkRuntime::new();
    wireless.initialize().unwrap();
    wireless.interfaces.add_interface(NetworkInterface { id: 3, device: NetworkDevice { device_id: 33, driver_id: 8, link_type: LinkType::Wireless, hardware_address: None, link_state: LinkState::Up, maximum_frame_size: 1500, can_receive: true, can_transmit: true, offload_capabilities: 0, operational_state: OperationalState::Ready, error_code: 0 }, enabled: false, rx_packets: 0, tx_packets: 0, rx_drops: 0, tx_drops: 0 }).unwrap();
    wireless.select_setup_mode(NetworkSetupMode::Wireless);
    assert_eq!(wireless.apply_setup_mode().unwrap(), ConnectivityClass::LinkOnly);
    assert!(wireless.interfaces.interface(3).unwrap().enabled);

    let layout = SystemLayout::new(1920, 1080);
    let mut discovered = [None; 3];
    for y in 0..1000 {
        if let Some(OnboardingTarget::NetworkChoice(index)) = layout.onboarding_target(6, 200, y) {
            if index < discovered.len() && discovered[index].is_none() {
                discovered[index] = Some(y);
            }
        }
    }
    assert!(discovered.iter().all(Option::is_some));
    for y in discovered.into_iter().flatten() {
        assert!(!matches!(
            layout.onboarding_target(5, 200, y),
            Some(OnboardingTarget::NetworkChoice(_))
        ));
    }
}

// ------------------------=
// FUNC: firmware_network_discovery_behavior
// DESC: Verifies that a firmware-discovered NAT adapter becomes an available wired interface with observed properties.
// ------------------=
fn firmware_network_discovery_behavior() {
    let mut network = NetworkRuntime::new();
    network.initialize().unwrap();
    let mac = [0x08, 0x00, 0x27, 0x12, 0x34, 0x56];
    let interface_id = network.register_firmware_device(FirmwareNetworkDevice {
        firmware_handle: 0x1000,
        device_id: 0x5634_1227_0008,
        hardware_address: Some(mac),
        link_state: LinkState::Up,
        maximum_frame_size: 1500,
        can_receive: true,
        can_transmit: true,
    }).unwrap();
    let interface = network.interfaces.interface(interface_id).unwrap();
    assert_eq!(interface.device.hardware_address, Some(mac));
    assert_eq!(interface.device.maximum_frame_size, 1500);
    assert_eq!(interface.device.link_state, LinkState::Up);
    assert!(interface.device.can_receive && interface.device.can_transmit);
    let setup = network.setup_snapshot();
    assert!(setup.wired_available);
    assert_eq!(setup.wired_link, LinkState::Up);
    network.register_firmware_device(FirmwareNetworkDevice {
        firmware_handle: 0, device_id: 1, hardware_address: Some(mac),
        link_state: LinkState::Down, maximum_frame_size: 1500,
        can_receive: true, can_transmit: true,
    }).unwrap();
    assert_eq!(network.setup_snapshot().wired_link, LinkState::Down);
    assert_eq!(network.status().connectivity, ConnectivityClass::Offline);
    network.register_firmware_device(FirmwareNetworkDevice {
        firmware_handle: 0, device_id: 1, hardware_address: Some(mac),
        link_state: LinkState::Up, maximum_frame_size: 1500,
        can_receive: true, can_transmit: true,
    }).unwrap();
    assert_eq!(network.status().connectivity, ConnectivityClass::LinkOnly);
    network.resolver.set_enabled(false);
    assert!(!network.status().resolver_enabled);

    let mut pci_only = NetworkRuntime::new();
    pci_only.initialize().unwrap();
    pci_only.register_firmware_device(FirmwareNetworkDevice {
        firmware_handle: 0x8000_0000_8086_100e,
        device_id: 0x8000_0000_8086_100e,
        hardware_address: None,
        link_state: LinkState::Unknown,
        maximum_frame_size: 0,
        can_receive: false,
        can_transmit: false,
    }).unwrap();
    let pci_setup = pci_only.setup_snapshot();
    assert!(pci_setup.wired_available);
    assert_eq!(pci_setup.wired_link, LinkState::Unknown);
    pci_only.select_setup_mode(NetworkSetupMode::Wired);
    assert!(pci_only.apply_setup_mode().is_ok());
}

// ------------------------=
// FUNC: settings_dashboard_behavior
// DESC: Verifies that every responsive Network settings page and control remains bounded, non-overlapping, and directly interactive.
// ------------------=
fn settings_dashboard_behavior() {
    for (width, height) in [(1280usize, 800usize), (1920, 1080), (2560, 1440)] {
        let layout = SystemLayout::new(width, height);
        let state = SettingsWindowState {
            x: 140,
            y: 150,
            width: 720,
            height: 720,
            maximized: false,
            expanded_row: None,
            scroll_offset: 0,
            control_focus: 0,
            row_count: 8,
        };
        let window = layout.settings_window_geometry(state);
        let dashboard = layout.network_settings_geometry(state);
        for panel in [dashboard.summary, dashboard.main, dashboard.sidebar] {
            assert!(window.content.contains(ui::geometry::Point { x: panel.x, y: panel.y }));
            assert!(panel.right() <= window.content.right());
            assert!(panel.bottom() <= window.content.bottom());
        }
        assert!(dashboard.summary.bottom() <= dashboard.main.y);
        assert!(dashboard.main.right() <= dashboard.sidebar.x);
        for (index, card) in dashboard.tabs.iter().enumerate() {
            assert!(card.width > 0 && card.height > 0);
            let normalized_x = (card.x + card.width as i32 / 2) * 1000 / width as i32;
            let normalized_y = (card.y + card.height as i32 / 2) * 1000 / height as i32;
            assert_eq!(
                layout.network_settings_target(normalized_x, normalized_y, state),
                Some(NetworkSettingsTarget::Page(index))
            );
        }
        for (index, card) in dashboard.controls.iter().enumerate() {
            assert!(card.width > 0 && card.height > 0);
            assert!(dashboard.main.contains(ui::geometry::Point { x: card.x, y: card.y }));
            assert!(card.right() <= dashboard.main.right());
            assert!(card.bottom() <= dashboard.main.bottom());
            let normalized_x = (card.x + card.width as i32 / 2) * 1000 / width as i32;
            let normalized_y = (card.y + card.height as i32 / 2) * 1000 / height as i32;
            assert_eq!(
                layout.network_settings_target(normalized_x, normalized_y, state),
                Some(NetworkSettingsTarget::Control(index))
            );
        }
    }
}

// ------------------------=
// FUNC: policy_and_transport_behavior
// DESC: Verifies per-identity allow and deny, revocation, lease expiry, and bounded connection queues.
// ------------------=
fn policy_and_transport_behavior() {
    let mut network = NetworkRuntime::new(); network.initialize().unwrap();
    let issuer = identity(1); let app_a = identity(2); let app_b = identity(3);
    let subject_a = Subject::Application(100); let subject_b = Subject::Application(200);
    let local = endpoint([127,0,0,1], 4000); let approved = endpoint([127,0,0,1], 8443); let denied = endpoint([127,0,0,1], 9443);
    network.policy.create(NetworkPolicyRule { id: 0, subject: subject_a, direction: Direction::Outbound, interface_id: Some(1), local: EndpointSelector::any(), remote: EndpointSelector { network: Some(approved.address), prefix_length: 32, port: Some(8443), protocol: Some(TransportProtocol::Stream), local_only: true }, action: PolicyAction::Allow, priority: 1, logging: LoggingMode::Decisions, enabled: true, expires_at: None, policy_source: 10 }).unwrap();
    network.policy.create(NetworkPolicyRule { id: 0, subject: subject_b, direction: Direction::Outbound, interface_id: Some(1), local: EndpointSelector::any(), remote: EndpointSelector::any(), action: PolicyAction::Deny, priority: 1, logging: LoggingMode::Denials, enabled: true, expires_at: None, policy_source: 11 }).unwrap();
    let mut caps = CapabilityManager::new();
    let connect = caps.grant(CapabilityType::NetworkConnect, 0, 1, 0, issuer, app_a, Some(100), 0).unwrap();
    let id = network.connections.connect(app_a, subject_a, connect, local, approved, TransportProtocol::Stream, 2, 1, 50, &caps, &mut network.policy).unwrap();
    assert_eq!(network.connections.connect(app_a, subject_a, connect, local, denied, TransportProtocol::Stream, 2, 1, 50, &caps, &mut network.policy), Err(NetworkError::PolicyDenied));
    let bcap = caps.grant(CapabilityType::NetworkConnect, 0, 1, 0, issuer, app_b, None, 0).unwrap();
    assert_eq!(network.connections.connect(app_b, subject_b, bcap, local, approved, TransportProtocol::Stream, 2, 1, 50, &caps, &mut network.policy), Err(NetworkError::PolicyDenied));
    let send = caps.grant(CapabilityType::NetworkSend, id as u64, 1, 0, issuer, app_a, Some(20), 0).unwrap();
    let receive = caps.grant(CapabilityType::NetworkReceive, id as u64, 1, 0, issuer, app_a, None, 0).unwrap();
    network.connections.send(app_a, send, id, b"one", 2, &caps).unwrap(); network.connections.send(app_a, send, id, b"two", 2, &caps).unwrap();
    assert_eq!(network.connections.send(app_a, send, id, b"three", 2, &caps), Err(NetworkError::QueueFull));
    assert_eq!(&network.connections.receive(app_a, receive, id, 2, &caps).unwrap().bytes[..3], b"one");
    caps.revoke(send).unwrap(); assert_eq!(network.connections.send(app_a, send, id, b"x", 3, &caps), Err(NetworkError::CapabilityRevoked));
    let leased = caps.grant(CapabilityType::NetworkSend, id as u64, 1, 0, issuer, app_a, Some(5), 0).unwrap();
    assert_eq!(network.connections.send(app_a, leased, id, b"x", 5, &caps), Err(NetworkError::CapabilityRevoked));
    assert_eq!(network.inspect_connection(id, app_b, false), Err(NetworkError::AccessDenied));
    assert_eq!(network.inspect_connection(id, app_a, false).unwrap().owner, app_a);
}

// ------------------------=
// FUNC: resolver_and_discovery_behavior
// DESC: Verifies deadlines, cache expiry, negative results, leases, and discovery flood bounds.
// ------------------=
fn resolver_and_discovery_behavior() {
    let mut network = NetworkRuntime::new(); network.initialize().unwrap();
    network.resolver.install_fixture(99, &[IpAddress::V4([10,0,0,99])], 100).unwrap();
    network.resolver.set_server(0, Some(IpAddress::V4([1,1,1,1]))).unwrap();
    assert_eq!(network.resolver.resolve(99, 1, 10), Err(NetworkError::ResolverUnavailable));
    assert_eq!(network.resolver.server(0), Some(IpAddress::V4([1,1,1,1])));
    network.resolver.install_fixture(42, &[IpAddress::V4([10,0,0,4])], 10).unwrap();
    assert_eq!(network.resolver.resolve(42, 1, 9).unwrap().addresses[0], Some(IpAddress::V4([10,0,0,4])));
    assert_eq!(network.resolver.resolve(42, 10, 20), Err(NetworkError::ResolverUnavailable));
    assert_eq!(network.resolver.resolve(1, 5, 5), Err(NetworkError::ResolutionTimeout));
    for index in 0..MAX_DISCOVERED_SERVICES { network.discovery.advertise(DiscoveredService { service_identity: index as u32 + 1, service_type: 7, endpoint: endpoint([127,0,0,1], 1000 + index as u16), metadata_schema: 1, metadata_hash: index as u64, expires_at: 10, healthy: true, future_node_identity: None }, 1).unwrap(); }
    assert_eq!(network.discovery.advertise(DiscoveredService { service_identity: 99, service_type: 7, endpoint: endpoint([127,0,0,1], 9999), metadata_schema: 1, metadata_hash: 0, expires_at: 10, healthy: true, future_node_identity: None }, 1), Err(NetworkError::ResourceLimitExceeded));
    network.tick(10); assert_eq!(network.discovery.count(), 0);
}

// ------------------------=
// FUNC: management_capability_behavior
// DESC: Verifies address, route, resolver, and profile management reject missing, wrong, revoked, or expired authority.
// ------------------=
fn management_capability_behavior() {
    let mut network = NetworkRuntime::new(); network.initialize().unwrap();
    network.interfaces.add_interface(NetworkInterface { id: 2, device: NetworkDevice { device_id: 22, driver_id: 7, link_type: LinkType::Virtual, hardware_address: None, link_state: LinkState::Up, maximum_frame_size: 1500, can_receive: true, can_transmit: true, offload_capabilities: 0, operational_state: OperationalState::Ready, error_code: 0 }, enabled: true, rx_packets: 0, tx_packets: 0, rx_drops: 0, tx_drops: 0 }).unwrap();
    let issuer = identity(30); let caller = identity(31); let other = identity(32); let mut capabilities = CapabilityManager::new();
    let address_cap = capabilities.grant(CapabilityType::NetworkAddressConfigure, 2, 1, 0, issuer, caller, None, 0).unwrap();
    assert!(network.configure_address_authorized(2, IpAddress::V4([10, 1, 2, 3]), 24, AddressScope::Private, AddressSource::Static, caller, address_cap, 1, &capabilities).is_ok());
    assert_eq!(network.configure_address_authorized(2, IpAddress::V4([10, 1, 2, 4]), 24, AddressScope::Private, AddressSource::Static, other, address_cap, 1, &capabilities), Err(NetworkError::AccessDenied));
    let route_cap = capabilities.grant(CapabilityType::NetworkRouteModify, 2, 1, 0, issuer, caller, Some(4), 0).unwrap();
    assert!(network.add_route_authorized(IpAddress::V4([0,0,0,0]), 0, Some(IpAddress::V4([10,1,2,1])), 2, 10, RouteSource::Static, None, caller, route_cap, 2, &capabilities).is_ok());
    assert_eq!(network.add_route_authorized(IpAddress::V4([10,2,0,0]), 16, None, 2, 10, RouteSource::Static, None, caller, route_cap, 4, &capabilities), Err(NetworkError::CapabilityRevoked));
    network.resolver.install_fixture(77, &[IpAddress::V4([127,0,0,1])], 20).unwrap();
    let resolve_cap = capabilities.grant(CapabilityType::NetworkResolve, 0, 1, 0, issuer, caller, None, 0).unwrap();
    assert!(network.resolve_authorized(77, caller, resolve_cap, 1, 5, &capabilities).is_ok());
    capabilities.revoke(resolve_cap).unwrap();
    assert_eq!(network.resolve_authorized(77, caller, resolve_cap, 2, 5, &capabilities), Err(NetworkError::CapabilityRevoked));
    let profile_cap = capabilities.grant(CapabilityType::NetworkProfileActivate, 0, 1, 0, issuer, caller, None, 0).unwrap();
    assert!(network.activate_profile_authorized(3, caller, profile_cap, 2, &capabilities).is_ok());
    assert_eq!(
        network.reconfigure_authorized(
            NetworkSetupMode::Wired,
            caller,
            profile_cap,
            2,
            &capabilities,
        ),
        Ok(ConnectivityClass::Routed)
    );
    assert_eq!(network.setup_snapshot().selected, NetworkSetupMode::Wired);
    assert_eq!(network.status().active_profile, 1);
    assert_eq!(
        network.reconfigure_authorized(
            NetworkSetupMode::Wireless,
            caller,
            profile_cap,
            2,
            &capabilities,
        ),
        Err(NetworkError::InterfaceNotFound)
    );
    assert_eq!(network.setup_snapshot().selected, NetworkSetupMode::Wired);
    assert_eq!(network.status().active_profile, 1);
}

// ------------------------=
// FUNC: iop_and_console_behavior
// DESC: Verifies the versioned network wire schema and deterministic Console mapping to typed operations.
// ------------------=
fn iop_and_console_behavior() {
    let request = NetworkConnectV1 { address_family: 6, remote_address: [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1], remote_port: 443, protocol: 1, secure: true, queue_limit: 4, expected_identity: 99 };
    let mut encoded = [0u8; NETWORK_CONNECT_V1_BYTES]; request.encode(&mut encoded);
    assert_eq!(NetworkConnectV1::decode(&encoded).unwrap(), request);
    encoded[0] = 9; assert!(NetworkConnectV1::decode(&encoded).is_err());

    match parse(b"network status").unwrap() {
        ParseOutcome::Graph(graph) => {
            assert_eq!(graph.node_count, 1);
            assert_eq!(graph.nodes[0].unwrap().schema.operation, OperationId::NetworkStatus);
            assert_eq!(graph.result_type, ValueType::NetworkStatus);
            assert_eq!(graph.maximum_effect, SideEffectClass::Query);
        }
        _ => panic!("network status must produce an operation graph"),
    }
    match parse(b"network profile-activate network-profile:3").unwrap() {
        ParseOutcome::Graph(graph) => {
            assert_eq!(graph.nodes[0].unwrap().schema.operation, OperationId::NetworkProfileActivate);
            assert_eq!(graph.maximum_effect, SideEffectClass::SecurityChange);
        }
        _ => panic!("profile activation must produce an operation graph"),
    }
}

// ------------------------=
// FUNC: network_event_behavior
// DESC: Verifies Network-domain delivery, correlation, Record durability, and delivery-time revocation.
// ------------------=
fn network_event_behavior() {
    let source = identity(20); let subscriber = identity(21); let issuer = identity(22);
    let mut capabilities = CapabilityManager::new(); let mut events = EventFabric::new();
    let event_type = runtime::EVENT_NETWORK_PROFILE_ACTIVATED;
    let publish = capabilities.grant(CapabilityType::EventPublish, event_type as u64, 1, 7, issuer, source, None, 0).unwrap();
    let subscribe = capabilities.grant(CapabilityType::EventSubscribe, event_type as u64, 1, 7, issuer, subscriber, None, 0).unwrap();
    let lease = events.subscribe(subscriber, subscribe, EventFilter { type_id: event_type, scope: Some(7) }, 100, OverflowPolicy::Durable, 2, &capabilities, 1).unwrap();
    events.publish(EventClass::Record, RoutingDomain::Network, event_type, source, 7, 400, 399, &[3], 220, 2, &capabilities, publish).unwrap();
    let delivered = events.receive(lease, 3).unwrap();
    assert_eq!(delivered.domain, RoutingDomain::Network); assert_eq!(delivered.correlation_id, 400); assert_eq!(delivered.causation_id, 399); assert_eq!(events.record_count(), 1);
    capabilities.revoke(subscribe).unwrap();
    events.publish(EventClass::Record, RoutingDomain::Network, event_type, source, 7, 401, 400, &[1], 220, 4, &capabilities, publish).unwrap();
    assert_eq!(events.queue_depth(lease), Some(0));
}

// ------------------------=
// FUNC: service_recovery_behavior
// DESC: Verifies a network service failure leaves the runtime alive and honors bounded restart policy.
// ------------------=
fn service_recovery_behavior() {
    let mut runtime = runtime::InfinityRuntime::new(false); runtime.define_bootstrap().unwrap(); runtime.start_all(0);
    assert_eq!(runtime.services.inspect(SERVICE_NETWORK).unwrap().state, ServiceState::Ready);
    runtime.fail_service(SERVICE_NETWORK, 0).unwrap(); assert_eq!(runtime.services.inspect(SERVICE_NETWORK).unwrap().state, ServiceState::Restarting);
    runtime.start_all(1000); assert_eq!(runtime.services.inspect(SERVICE_NETWORK).unwrap().state, ServiceState::Ready);
    assert!(runtime.services.inspect(SERVICE_OBJECT).is_some());
}

// ------------------------=
// FUNC: main
// DESC: Runs Milestone 8 behavior-only host acceptance tests.
// ------------------=
fn main() { route_behavior(); profile_behavior(); configurable_state_behavior(); onboarding_network_behavior(); firmware_network_discovery_behavior(); settings_dashboard_behavior(); policy_and_transport_behavior(); resolver_and_discovery_behavior(); management_capability_behavior(); iop_and_console_behavior(); network_event_behavior(); service_recovery_behavior(); }
