//! Bounded interface, address, and route state with deterministic selection.

use super::types::*;

pub struct InterfaceManager {
    interfaces: [Option<NetworkInterface>; MAX_INTERFACES],
    addresses: [Option<NetworkAddress>; MAX_ADDRESSES],
    routes: [Option<Route>; MAX_ROUTES],
    next_address: AddressId,
    next_route: RouteId,
}

impl InterfaceManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates empty bounded interface, address, and route tables.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            interfaces: [None; MAX_INTERFACES],
            addresses: [None; MAX_ADDRESSES],
            routes: [None; MAX_ROUTES],
            next_address: 1,
            next_route: 1,
        }
    }

    // ------------------------=
    // FUNC: install_loopback
    // DESC: Installs the native host-local interface and its IPv4 and IPv6 addresses.
    // ------------------=
    pub fn install_loopback(&mut self) -> Result<(), NetworkError> {
        if self.interface(1).is_some() {
            return Ok(());
        }
        self.add_interface(NetworkInterface {
            id: 1,
            device: NetworkDevice {
                device_id: 1,
                driver_id: 0x4c4f4f50,
                link_type: LinkType::Loopback,
                hardware_address: None,
                link_state: LinkState::Up,
                maximum_frame_size: MAX_PACKET_BYTES as u16,
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
        })?;
        self.add_address(
            1,
            IpAddress::V4([127, 0, 0, 1]),
            8,
            AddressScope::Host,
            AddressSource::System,
            None,
            None,
        )?;
        let mut v6 = [0; 16];
        v6[15] = 1;
        self.add_address(
            1,
            IpAddress::V6(v6),
            128,
            AddressScope::Host,
            AddressSource::System,
            None,
            None,
        )?;
        self.add_route(
            IpAddress::V4([127, 0, 0, 0]),
            8,
            None,
            1,
            0,
            RouteSource::System,
            None,
        )?;
        self.add_route(
            IpAddress::V6([0; 16]),
            0,
            None,
            1,
            u32::MAX,
            RouteSource::System,
            Some(u64::MAX),
        )?;
        Ok(())
    }

    // ------------------------=
    // FUNC: add_interface
    // DESC: Adds a uniquely identified typed network interface within its resource bound.
    // ------------------=
    pub fn add_interface(&mut self, interface: NetworkInterface) -> Result<(), NetworkError> {
        if self.interface(interface.id).is_some() {
            return Err(NetworkError::AlreadyExists);
        }
        let slot = self
            .interfaces
            .iter()
            .position(Option::is_none)
            .ok_or(NetworkError::ResourceLimitExceeded)?;
        self.interfaces[slot] = Some(interface);
        Ok(())
    }

    // ------------------------=
    // FUNC: set_state
    // DESC: Enables or disables one interface without changing unrelated network state.
    // ------------------=
    pub fn set_state(&mut self, id: InterfaceId, enabled: bool) -> Result<(), NetworkError> {
        let interface = self
            .interfaces
            .iter_mut()
            .flatten()
            .find(|v| v.id == id)
            .ok_or(NetworkError::InterfaceNotFound)?;
        interface.enabled = enabled;
        interface.device.operational_state = if enabled {
            OperationalState::Ready
        } else {
            OperationalState::Offline
        };
        Ok(())
    }

    // ------------------------=
    // FUNC: interface
    // DESC: Resolves an interface by stable identifier.
    // ------------------=
    pub fn interface(&self, id: InterfaceId) -> Option<&NetworkInterface> {
        self.interfaces.iter().flatten().find(|v| v.id == id)
    }

    // ------------------------=
    // FUNC: bind_native_device
    // DESC: Replaces firmware metadata with an actual native NIC binding while retaining configured addresses.
    // ------------------=
    pub fn bind_native_device(&mut self, id: InterfaceId, device: NetworkDevice) -> Result<(), NetworkError> {
        let interface = self.interfaces.iter_mut().flatten().find(|v| v.id == id).ok_or(NetworkError::InterfaceNotFound)?;
        interface.device = device;
        Ok(())
    }

    // ------------------------=
    // FUNC: update_native_counters
    // DESC: Publishes observed descriptor completions and ingress drops into the authoritative interface state.
    // ------------------=
    pub fn update_native_counters(&mut self, id: InterfaceId, rx: u64, tx: u64, drops: u64) -> Result<(), NetworkError> {
        let interface = self.interfaces.iter_mut().flatten().find(|v| v.id == id).ok_or(NetworkError::InterfaceNotFound)?;
        interface.rx_packets = rx;
        interface.tx_packets = tx;
        interface.rx_drops = drops;
        Ok(())
    }

    // ------------------------=
    // FUNC: interface_count
    // DESC: Returns the number of discovered and system interfaces.
    // ------------------=
    pub fn interface_count(&self) -> usize {
        self.interfaces.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: interface_nth
    // DESC: Returns the indexed interface for typed inspection.
    // ------------------=
    pub fn interface_nth(&self, index: usize) -> Option<&NetworkInterface> {
        self.interfaces.iter().flatten().nth(index)
    }

    // ------------------------=
    // FUNC: setup_interface
    // DESC: Selects the first real interface compatible with a post-install wired or wireless setup choice.
    // ------------------=
    pub fn setup_interface(&self, mode: NetworkSetupMode) -> Option<&NetworkInterface> {
        self.interfaces
            .iter()
            .flatten()
            .find(|interface| match mode {
                NetworkSetupMode::Wired => matches!(
                    interface.device.link_type,
                    LinkType::Ethernet | LinkType::Virtual
                ),
                NetworkSetupMode::Wireless => interface.device.link_type == LinkType::Wireless,
                NetworkSetupMode::Automatic => interface.device.link_type != LinkType::Loopback,
                NetworkSetupMode::Offline => false,
            })
    }

    // ------------------------=
    // FUNC: add_address
    // DESC: Validates and adds an interface-scoped IPv4 or IPv6 address.
    // ------------------=
    pub fn add_address(
        &mut self,
        interface_id: InterfaceId,
        address: IpAddress,
        prefix_length: u8,
        scope: AddressScope,
        source: AddressSource,
        valid_until: Option<u64>,
        preferred_until: Option<u64>,
    ) -> Result<AddressId, NetworkError> {
        if self.interface(interface_id).is_none() {
            return Err(NetworkError::InterfaceNotFound);
        }
        let maximum = if address.family() == AddressFamily::Ipv4 {
            32
        } else {
            128
        };
        if prefix_length > maximum {
            return Err(NetworkError::AddressUnavailable);
        }
        if self
            .addresses
            .iter()
            .flatten()
            .any(|v| v.interface_id == interface_id && v.address == address)
        {
            return Err(NetworkError::AlreadyExists);
        }
        let slot = self
            .addresses
            .iter()
            .position(Option::is_none)
            .ok_or(NetworkError::ResourceLimitExceeded)?;
        let id = self.next_address;
        self.next_address = self.next_address.wrapping_add(1).max(1);
        self.addresses[slot] = Some(NetworkAddress {
            id,
            interface_id,
            address,
            prefix_length,
            scope,
            source,
            state: AddressState::Preferred,
            valid_until,
            preferred_until,
        });
        Ok(id)
    }

    // ------------------------=
    // FUNC: remove_address
    // DESC: Removes one address by identity without altering the interface.
    // ------------------=
    pub fn remove_address(&mut self, id: AddressId) -> Result<(), NetworkError> {
        let slot = self
            .addresses
            .iter()
            .position(|v| v.map(|a| a.id) == Some(id))
            .ok_or(NetworkError::AddressUnavailable)?;
        self.addresses[slot] = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: remove_static_addresses
    // DESC: Removes only user-configured static addresses from one interface before atomic replacement.
    // ------------------=
    pub fn remove_static_addresses(&mut self, interface_id: InterfaceId) {
        for address in self.addresses.iter().flatten().filter(|value| value.interface_id == interface_id && value.source == AddressSource::Static) {
            if let IpAddress::V4(ip) = address.address {
                let mask = u32::MAX.checked_shl(32 - address.prefix_length as u32).unwrap_or(0);
                let destination = IpAddress::V4((u32::from_be_bytes(ip) & mask).to_be_bytes());
                for route in &mut self.routes {
                    if route.map(|value| value.interface_id == interface_id && value.source == RouteSource::Interface && value.destination == destination && value.prefix_length == address.prefix_length && value.next_hop.is_none()).unwrap_or(false) {
                        *route = None;
                    }
                }
            }
        }
        for address in &mut self.addresses {
            if address
                .map(|value| {
                    value.interface_id == interface_id && value.source == AddressSource::Static
                })
                .unwrap_or(false)
            {
                *address = None;
            }
        }
    }

    // ------------------------=
    // FUNC: address_count
    // DESC: Returns the bounded address-table population.
    // ------------------=
    pub fn address_count(&self) -> usize {
        self.addresses.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: address_nth
    // DESC: Returns one typed address for inspection.
    // ------------------=
    pub fn address_nth(&self, index: usize) -> Option<&NetworkAddress> {
        self.addresses.iter().flatten().nth(index)
    }

    // ------------------------=
    // FUNC: add_route
    // DESC: Adds a validated route with deterministic source and metric data.
    // ------------------=
    pub fn add_route(
        &mut self,
        destination: IpAddress,
        prefix_length: u8,
        next_hop: Option<IpAddress>,
        interface_id: InterfaceId,
        metric: u32,
        source: RouteSource,
        policy_scope: Option<u64>,
    ) -> Result<RouteId, NetworkError> {
        if self.interface(interface_id).is_none() {
            return Err(NetworkError::InterfaceNotFound);
        }
        let max = if destination.family() == AddressFamily::Ipv4 {
            32
        } else {
            128
        };
        if prefix_length > max
            || next_hop.map(|v| v.family()) != None
                && next_hop.map(|v| v.family()) != Some(destination.family())
        {
            return Err(NetworkError::Conflict);
        }
        if self.routes.iter().flatten().any(|r| {
            r.destination == destination
                && r.prefix_length == prefix_length
                && r.interface_id == interface_id
                && r.metric == metric
        }) {
            return Err(NetworkError::AlreadyExists);
        }
        let slot = self
            .routes
            .iter()
            .position(Option::is_none)
            .ok_or(NetworkError::ResourceLimitExceeded)?;
        let id = self.next_route;
        self.next_route = self.next_route.wrapping_add(1).max(1);
        self.routes[slot] = Some(Route {
            id,
            destination,
            prefix_length,
            next_hop,
            interface_id,
            metric,
            source,
            state: RouteState::Active,
            policy_scope,
        });
        Ok(id)
    }

    // ------------------------=
    // FUNC: remove_route
    // DESC: Removes one route from the active table.
    // ------------------=
    pub fn remove_route(&mut self, id: RouteId) -> Result<(), NetworkError> {
        let slot = self
            .routes
            .iter()
            .position(|v| v.map(|r| r.id) == Some(id))
            .ok_or(NetworkError::NoRoute)?;
        self.routes[slot] = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: remove_static_default_routes
    // DESC: Removes user-configured default routes for one interface without touching discovered routes.
    // ------------------=
    pub fn remove_static_default_routes(&mut self, interface_id: InterfaceId) {
        for route in &mut self.routes {
            if route
                .map(|value| {
                    value.interface_id == interface_id
                        && value.prefix_length == 0
                        && value.source == RouteSource::Static
                })
                .unwrap_or(false)
            {
                *route = None;
            }
        }
    }

    // ------------------------=
    // FUNC: select_route
    // DESC: Selects the longest matching prefix, then lowest metric, then lowest stable route ID.
    // ------------------=
    pub fn select_route(
        &self,
        destination: IpAddress,
        policy_scope: Option<u64>,
    ) -> Result<Route, NetworkError> {
        self.routes
            .iter()
            .flatten()
            .filter(|r| {
                r.state == RouteState::Active
                    && r.destination.family() == destination.family()
                    && destination.matches_prefix(r.destination, r.prefix_length)
                    && (r.policy_scope.is_none() || r.policy_scope == policy_scope)
            })
            .filter(|r| {
                self.interface(r.interface_id)
                    .map(|i| i.enabled && i.device.link_state == LinkState::Up)
                    .unwrap_or(false)
            })
            .copied()
            .min_by_key(|r| (u8::MAX - r.prefix_length, r.metric, r.id))
            .ok_or(NetworkError::NoRoute)
    }

    // ------------------------=
    // FUNC: route_count
    // DESC: Returns the bounded route-table population.
    // ------------------=
    pub fn route_count(&self) -> usize {
        self.routes.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: route_nth
    // DESC: Returns one typed route for inspection.
    // ------------------=
    pub fn route_nth(&self, index: usize) -> Option<&Route> {
        self.routes.iter().flatten().nth(index)
    }

    // ------------------------=
    // FUNC: replace_static_ipv4
    // DESC: Atomically replaces one interface's static IPv4 address and default gateway configuration.
    // ------------------=
    pub fn replace_static_ipv4(
        &mut self,
        interface_id: InterfaceId,
        address: IpAddress,
        prefix_length: u8,
        gateway: Option<IpAddress>,
        metric: u32,
    ) -> Result<(AddressId, Option<RouteId>), NetworkError> {
        if !matches!(address, IpAddress::V4(_))
            || gateway
                .map(|value| !matches!(value, IpAddress::V4(_)))
                .unwrap_or(false)
        {
            return Err(NetworkError::InvalidEndpoint);
        }
        let addresses = self.addresses;
        let routes = self.routes;
        let counters = (self.next_address, self.next_route);
        self.remove_static_addresses(interface_id);
        self.remove_static_default_routes(interface_id);
        let configured = self.add_address(
            interface_id,
            address,
            prefix_length,
            AddressScope::Private,
            AddressSource::Static,
            None,
            None,
        );
        let address_id = match configured {
            Ok(id) => id,
            Err(error) => {
                self.addresses = addresses;
                self.routes = routes;
                (self.next_address, self.next_route) = counters;
                return Err(error);
            }
        };
        let IpAddress::V4(ip) = address else { unreachable!() };
        let mask = u32::MAX.checked_shl(32 - prefix_length as u32).unwrap_or(0);
        let destination = IpAddress::V4((u32::from_be_bytes(ip) & mask).to_be_bytes());
        if let Err(error) = self.add_route(destination, prefix_length, None, interface_id, 0, RouteSource::Interface, None) {
            self.addresses = addresses;
            self.routes = routes;
            (self.next_address, self.next_route) = counters;
            return Err(error);
        }
        let route_id = if let Some(next_hop) = gateway {
            match self.add_route(
                IpAddress::V4([0, 0, 0, 0]),
                0,
                Some(next_hop),
                interface_id,
                metric,
                RouteSource::Static,
                None,
            ) {
                Ok(id) => Some(id),
                Err(error) => {
                    self.addresses = addresses;
                    self.routes = routes;
                    (self.next_address, self.next_route) = counters;
                    return Err(error);
                }
            }
        } else {
            None
        };
        Ok((address_id, route_id))
    }

    // ------------------------=
    // FUNC: connectivity
    // DESC: Classifies connectivity from committed link, address, and routing state without claiming Internet reachability.
    // ------------------=
    pub fn connectivity(&self, forced_offline: bool) -> ConnectivityClass {
        if forced_offline {
            return ConnectivityClass::Offline;
        }
        let link = self.interfaces.iter().flatten().any(|i| {
            i.enabled
                && i.device.link_state == LinkState::Up
                && i.device.link_type != LinkType::Loopback
        });
        if !link {
            return ConnectivityClass::Offline;
        }
        let address = self
            .addresses
            .iter()
            .flatten()
            .any(|a| a.scope != AddressScope::Host && a.state == AddressState::Preferred);
        if !address {
            return ConnectivityClass::LinkOnly;
        }
        let routed = self.routes.iter().flatten().any(|r| {
            r.prefix_length == 0
                && r.state == RouteState::Active
                && r.policy_scope != Some(u64::MAX)
        });
        if routed {
            ConnectivityClass::Routed
        } else {
            ConnectivityClass::LocalNetwork
        }
    }
}
