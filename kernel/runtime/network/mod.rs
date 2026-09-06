//! InfinityOS-native networking service architecture. Public contracts remain
//! independent of POSIX sockets and host network stacks.

pub mod discovery;
pub mod interface;
pub mod policy;
pub mod profile;
pub mod resolver;
pub mod transport;
pub mod types;

use discovery::DiscoveryManager;
use interface::InterfaceManager;
use policy::{EndpointSelector, NetworkPolicyRule, PolicyEngine};
use profile::{NetworkProfile, ProfileManager};
use resolver::Resolver;
use transport::ConnectionManager;
use types::*;
use crate::runtime::capability::{CapabilityError, CapabilityId, CapabilityManager, CapabilityType};
use crate::runtime::execution::SecurityIdentity;

pub const NETWORK_STATE_BYTES: usize = 32;
pub const NETWORK_STATE_MAGIC: [u8; 8] = *b"INFNET01";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NetworkStatus {
    pub connectivity: ConnectivityClass, pub active_profile: ProfileId,
    pub profile_generation: u64, pub interfaces: u8, pub addresses: u8,
    pub routes: u8, pub resolver_enabled: bool, pub policies: u8,
    pub active_connections: u8, pub discovered_services: u8, pub degraded_reason: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TopologySnapshot {
    pub local_machine: bool, pub interface_count: u8, pub route_count: u8,
    pub discovered_service_count: u8, pub remote_identity_count: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NetworkSetupSnapshot {
    pub selected: NetworkSetupMode,
    pub wired_available: bool,
    pub wired_link: LinkState,
    pub wireless_available: bool,
    pub wireless_link: LinkState,
    pub connectivity: ConnectivityClass,
}

pub struct NetworkRuntime {
    pub interfaces: InterfaceManager,
    pub policy: PolicyEngine,
    pub profiles: ProfileManager,
    pub resolver: Resolver,
    pub connections: ConnectionManager,
    pub discovery: DiscoveryManager,
    initialized: bool,
    degraded_reason: u32,
    setup_mode: NetworkSetupMode,
}

impl NetworkRuntime {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the separable native networking managers without ambient authority.
    // ------------------=
    pub const fn new() -> Self { Self { interfaces: InterfaceManager::new(), policy: PolicyEngine::new(), profiles: ProfileManager::new(), resolver: Resolver::new(), connections: ConnectionManager::new(), discovery: DiscoveryManager::new(), initialized: false, degraded_reason: 0, setup_mode: NetworkSetupMode::Automatic } }

    // ------------------------=
    // FUNC: initialize
    // DESC: Starts host-local networking and built-in operational profiles without requiring a physical link.
    // ------------------=
    pub fn initialize(&mut self) -> Result<(), NetworkError> {
        self.interfaces.install_loopback()?;
        self.profiles.install_builtins();
        self.install_bootstrap_policy()?;
        self.apply_active_profile()?;
        self.initialized = true;
        Ok(())
    }

    // ------------------------=
    // FUNC: register_firmware_device
    // DESC: Registers one UEFI-discovered adapter as an explicit typed interface without inventing address or reachability state.
    // ------------------=
    pub fn register_firmware_device(
        &mut self,
        device: FirmwareNetworkDevice,
    ) -> Result<InterfaceId, NetworkError> {
        let interface_id = 2;
        if self.interfaces.interface(interface_id).is_some() {
            return Ok(interface_id);
        }
        self.interfaces.add_interface(NetworkInterface {
            id: interface_id,
            device: NetworkDevice {
                device_id: device.device_id,
                driver_id: 0x534e_5030,
                link_type: LinkType::Ethernet,
                hardware_address: device.hardware_address,
                link_state: device.link_state,
                maximum_frame_size: device.maximum_frame_size,
                can_receive: device.can_receive,
                can_transmit: device.can_transmit,
                offload_capabilities: 0,
                operational_state: if device.link_state == LinkState::Up {
                    OperationalState::Ready
                } else {
                    OperationalState::Offline
                },
                error_code: 0,
            },
            enabled: true,
            rx_packets: 0,
            tx_packets: 0,
            rx_drops: 0,
            tx_drops: 0,
        })?;
        Ok(interface_id)
    }

    // ------------------------=
    // FUNC: install_bootstrap_policy
    // DESC: Installs explicit system and reference-service loopback policy rather than ambient network authority.
    // ------------------=
    fn install_bootstrap_policy(&mut self) -> Result<(), NetworkError> {
        if self.policy.count() != 0 { return Ok(()); }
        let local = EndpointSelector { local_only: true, ..EndpointSelector::any() };
        self.policy.create(NetworkPolicyRule { id: 0, subject: Subject::System, direction: Direction::Outbound, interface_id: Some(1), local: EndpointSelector::any(), remote: local, action: PolicyAction::Allow, priority: 10, logging: LoggingMode::Decisions, enabled: true, expires_at: None, policy_source: 1 })?;
        Ok(())
    }

    // ------------------------=
    // FUNC: activate_profile
    // DESC: Validates, stages, applies, verifies, and atomically commits a connectivity profile or rolls back.
    // ------------------=
    pub fn activate_profile(&mut self, id: ProfileId) -> Result<u64, NetworkError> {
        let staged = self.profiles.stage(id)?;
        if let Err(error) = self.apply_profile(staged) { self.profiles.rollback(); let _ = self.apply_active_profile(); return Err(error); }
        if staged.kind != profile::ProfileKind::Offline && self.interfaces.interface(1).is_none() { self.profiles.rollback(); let _ = self.apply_active_profile(); return Err(NetworkError::InterfaceNotFound); }
        self.profiles.commit(staged)
    }

    // ------------------------=
    // FUNC: activate_profile_authorized
    // DESC: Activates a profile only after validating the caller's live profile authority.
    // ------------------=
    pub fn activate_profile_authorized(&mut self, id: ProfileId, caller: SecurityIdentity, capability: CapabilityId, now: u64, capabilities: &CapabilityManager) -> Result<u64, NetworkError> {
        validate_authority(capabilities, capability, caller, CapabilityType::NetworkProfileActivate, 0, now)?;
        self.activate_profile(id)
    }

    // ------------------------=
    // FUNC: configure_address_authorized
    // DESC: Adds one typed interface address after validating interface-scoped authority.
    // ------------------=
    pub fn configure_address_authorized(&mut self, interface_id: InterfaceId, address: IpAddress, prefix_length: u8, scope: AddressScope, source: AddressSource, caller: SecurityIdentity, capability: CapabilityId, now: u64, capabilities: &CapabilityManager) -> Result<AddressId, NetworkError> {
        validate_authority(capabilities, capability, caller, CapabilityType::NetworkAddressConfigure, interface_id as u64, now)?;
        self.interfaces.add_address(interface_id, address, prefix_length, scope, source, None, None)
    }

    // ------------------------=
    // FUNC: add_route_authorized
    // DESC: Adds one typed route after validating interface-scoped route authority.
    // ------------------=
    pub fn add_route_authorized(&mut self, destination: IpAddress, prefix_length: u8, next_hop: Option<IpAddress>, interface_id: InterfaceId, metric: u32, source: RouteSource, policy_scope: Option<u64>, caller: SecurityIdentity, capability: CapabilityId, now: u64, capabilities: &CapabilityManager) -> Result<RouteId, NetworkError> {
        validate_authority(capabilities, capability, caller, CapabilityType::NetworkRouteModify, interface_id as u64, now)?;
        self.interfaces.add_route(destination, prefix_length, next_hop, interface_id, metric, source, policy_scope)
    }

    // ------------------------=
    // FUNC: resolve_authorized
    // DESC: Resolves through the bounded native resolver only for an authorized caller and deadline.
    // ------------------=
    pub fn resolve_authorized(&mut self, name_hash: u64, caller: SecurityIdentity, capability: CapabilityId, now: u64, deadline: u64, capabilities: &CapabilityManager) -> Result<resolver::ResolveResult, NetworkError> {
        validate_authority(capabilities, capability, caller, CapabilityType::NetworkResolve, 0, now)?;
        self.resolver.resolve(name_hash, now, deadline)
    }

    // ------------------------=
    // FUNC: apply_active_profile
    // DESC: Applies the currently committed profile to resolver, discovery, and policy managers.
    // ------------------=
    fn apply_active_profile(&mut self) -> Result<(), NetworkError> { let profile = *self.profiles.active().ok_or(NetworkError::InvalidProfile)?; self.apply_profile(profile) }

    // ------------------------=
    // FUNC: apply_profile
    // DESC: Applies staged profile controls while preserving host-local operation in Offline mode.
    // ------------------=
    fn apply_profile(&mut self, profile: NetworkProfile) -> Result<(), NetworkError> {
        self.profiles.validate(profile)?;
        self.resolver.set_enabled(profile.resolver_enabled);
        self.discovery.set_enabled(profile.local_discovery_enabled);
        self.policy.set_default_action(if profile.internet_allowed { PolicyAction::Ask } else { PolicyAction::Deny });
        Ok(())
    }

    // ------------------------=
    // FUNC: tick
    // DESC: Expires resolver and discovery leases without busy waiting.
    // ------------------=
    pub fn tick(&mut self, now: u64) { self.resolver.expire(now); self.discovery.expire(now); }

    // ------------------------=
    // FUNC: status
    // DESC: Projects authoritative typed manager state for Console and GUI parity.
    // ------------------=
    pub fn status(&self) -> NetworkStatus {
        let offline = self.profiles.active().map(|p| p.kind == profile::ProfileKind::Offline).unwrap_or(true);
        NetworkStatus { connectivity: self.interfaces.connectivity(offline), active_profile: self.profiles.active_id(), profile_generation: self.profiles.generation(), interfaces: self.interfaces.interface_count() as u8, addresses: self.interfaces.address_count() as u8, routes: self.interfaces.route_count() as u8, resolver_enabled: self.profiles.active().map(|p| p.resolver_enabled).unwrap_or(false), policies: self.policy.count() as u8, active_connections: self.connections.count() as u8, discovered_services: self.discovery.count() as u8, degraded_reason: self.degraded_reason }
    }

    // ------------------------=
    // FUNC: diagnostics
    // DESC: Aggregates only observed counters from native managers.
    // ------------------=
    pub fn diagnostics(&self) -> NetworkDiagnostics {
        let mut value = NetworkDiagnostics { connection_failures: self.connections.failures(), resolver_failures: self.resolver.failures(), queue_pressure: self.connections.queue_pressure(), policy_denials: self.policy.denials(), active_connections: self.connections.count() as u16, resolver_entries: self.resolver.count() as u16, discovered_services: self.discovery.count() as u16, ..NetworkDiagnostics::default() };
        for index in 0..self.interfaces.interface_count() { if let Some(interface) = self.interfaces.interface_nth(index) { value.rx_packets = value.rx_packets.saturating_add(interface.rx_packets); value.tx_packets = value.tx_packets.saturating_add(interface.tx_packets); value.drops = value.drops.saturating_add(interface.rx_drops).saturating_add(interface.tx_drops); } }
        value
    }

    // ------------------------=
    // FUNC: topology
    // DESC: Builds an authorization-neutral topology summary without exposing another subject's connection metadata.
    // ------------------=
    pub fn topology(&self) -> TopologySnapshot { TopologySnapshot { local_machine: true, interface_count: self.interfaces.interface_count() as u8, route_count: self.interfaces.route_count() as u8, discovered_service_count: self.discovery.count() as u8, remote_identity_count: 0 } }

    // ------------------------=
    // FUNC: setup_snapshot
    // DESC: Returns honest post-install connectivity choices from discovered interface state.
    // ------------------=
    pub fn setup_snapshot(&self) -> NetworkSetupSnapshot {
        let wired = self.interfaces.setup_interface(NetworkSetupMode::Wired);
        let wireless = self.interfaces.setup_interface(NetworkSetupMode::Wireless);
        NetworkSetupSnapshot {
            selected: self.setup_mode,
            wired_available: wired.is_some(),
            wired_link: wired.map(|interface| interface.device.link_state).unwrap_or(LinkState::Unknown),
            wireless_available: wireless.is_some(),
            wireless_link: wireless.map(|interface| interface.device.link_state).unwrap_or(LinkState::Unknown),
            connectivity: self.status().connectivity,
        }
    }

    // ------------------------=
    // FUNC: select_setup_mode
    // DESC: Stages the user's explicit first-boot connectivity choice without claiming a connection exists.
    // ------------------=
    pub fn select_setup_mode(&mut self, mode: NetworkSetupMode) { self.setup_mode = mode; }

    // ------------------------=
    // FUNC: apply_setup_mode
    // DESC: Applies the staged offline, wired, or already-associated wireless configuration through native profile state.
    // ------------------=
    pub fn apply_setup_mode(&mut self) -> Result<ConnectivityClass, NetworkError> {
        if self.setup_mode == NetworkSetupMode::Offline {
            self.activate_profile(3)?;
            return Ok(ConnectivityClass::Offline);
        }
        let selected = if self.setup_mode == NetworkSetupMode::Automatic {
            self.interfaces.setup_interface(NetworkSetupMode::Automatic)
        } else {
            self.interfaces.setup_interface(self.setup_mode)
        }.ok_or(NetworkError::InterfaceNotFound)?;
        if selected.device.link_state == LinkState::Down { return Err(NetworkError::LinkDown); }
        let id = selected.id;
        self.interfaces.set_state(id, true)?;
        self.activate_profile(1)?;
        Ok(self.status().connectivity)
    }

    // ------------------------=
    // FUNC: inspect_connection
    // DESC: Returns full ownership metadata only to the owner or an explicitly authorized inspector.
    // ------------------=
    pub fn inspect_connection(&self, id: ConnectionId, requester: crate::runtime::execution::SecurityIdentity, privileged: bool) -> Result<Connection, NetworkError> {
        let connection = self.connections.nth((0..self.connections.count()).find(|index| self.connections.nth(*index).map(|c| c.id) == Some(id)).ok_or(NetworkError::ConnectionClosed)?).copied().ok_or(NetworkError::ConnectionClosed)?;
        if connection.owner != requester && !privileged { return Err(NetworkError::AccessDenied); } Ok(connection)
    }

    // ------------------------=
    // FUNC: encode_state
    // DESC: Encodes the persistent profile selection as a typed versioned native object payload.
    // ------------------=
    pub fn encode_state(&self) -> [u8; NETWORK_STATE_BYTES] { let mut out = [0; NETWORK_STATE_BYTES]; out[..8].copy_from_slice(&NETWORK_STATE_MAGIC); out[8..10].copy_from_slice(&1u16.to_le_bytes()); out[10] = match self.setup_mode { NetworkSetupMode::Automatic => 0, NetworkSetupMode::Wired => 1, NetworkSetupMode::Wireless => 2, NetworkSetupMode::Offline => 3 }; out[12..16].copy_from_slice(&self.profiles.active_id().to_le_bytes()); out[16..24].copy_from_slice(&self.profiles.generation().to_le_bytes()); out }

    // ------------------------=
    // FUNC: restore_state
    // DESC: Validates and restores native persistent network state without ad-hoc text configuration.
    // ------------------=
    pub fn restore_state(&mut self, input: &[u8]) -> Result<(), NetworkError> { if input.len() < NETWORK_STATE_BYTES || input[..8] != NETWORK_STATE_MAGIC || u16::from_le_bytes([input[8],input[9]]) != 1 { return Err(NetworkError::InvalidProfile); } self.setup_mode = match input[10] { 0 => NetworkSetupMode::Automatic, 1 => NetworkSetupMode::Wired, 2 => NetworkSetupMode::Wireless, 3 => NetworkSetupMode::Offline, _ => return Err(NetworkError::InvalidProfile) }; let id = u32::from_le_bytes([input[12],input[13],input[14],input[15]]); let generation = u64::from_le_bytes([input[16],input[17],input[18],input[19],input[20],input[21],input[22],input[23]]); self.profiles.restore_active(id, generation)?; self.apply_active_profile() }

    // ------------------------=
    // FUNC: initialized
    // DESC: Reports whether native network bootstrap completed.
    // ------------------=
    pub const fn initialized(&self) -> bool { self.initialized }
}

// ------------------------=
// FUNC: validate_authority
// DESC: Maps live capability validation into the native network error contract without exposing capability internals.
// ------------------=
fn validate_authority(capabilities: &CapabilityManager, capability: CapabilityId, caller: SecurityIdentity, kind: CapabilityType, target: u64, now: u64) -> Result<(), NetworkError> {
    capabilities.validate(capability, caller, kind, target, 1, 0, now).map_err(|error| match error { CapabilityError::Revoked | CapabilityError::Expired => NetworkError::CapabilityRevoked, _ => NetworkError::AccessDenied })
}
