//! Capability-checked Python HTTPS bridge. The existing BSP actor owns all TLS,
//! DNS, root certificates, trusted time and NIC state; Python never receives it.
use crate::drivers::https;
use crate::runtime::{execution::SecurityIdentity, capability::{CapabilityId, CapabilityType},
    native_memory::{PageTable, copy_from_user, copy_to_user, check_user_range}};

pub struct Network {
    owner: SecurityIdentity,
    enabled: bool,
    ticket: Option<https::Ticket>,
    response: Option<https::Response>,
    generation: u64,
    capabilities: Option<[CapabilityId; 4]>,
    profile_generation: u64,
    channel: bool,
}

impl Network {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a closed bridge without granting network authority or allocating sockets.
    // ------------------=
    pub const fn new() -> Self {
        Self { owner: SecurityIdentity([0; 16]), enabled: false, ticket: None, response: None,
            generation: 0, capabilities: None, profile_generation: 0, channel: false }
    }

    // ------------------------=
    // FUNC: close
    // DESC: Cancels only this exact transaction and erases its leased response before releasing the actor.
    // ------------------=
    pub fn close(&mut self) {
        if let Some(ticket) = self.ticket.take() {
            let _ = https::cancel_browser(self.owner, ticket);
            let _ = https::take_browser(self.owner, ticket);
        }
        self.response = None;
        self.capabilities = None;
        self.channel = false;
    }

    // ------------------------=
    // FUNC: reset
    // DESC: Fences each new invocation to its authenticated owner and explicitly admitted online operation.
    // ------------------=
    pub fn reset(&mut self, owner: SecurityIdentity, enabled: bool) {
        self.close(); self.owner = owner; self.enabled = enabled;
    }

    // ------------------------=
    // FUNC: failure
    // DESC: Preserves certificate, permission, timeout and capacity failures as distinct native ABI errors.
    // ------------------=
    fn failure(error: https::Failure) -> i64 {
        match error {
            https::Failure::Busy => -11,
            https::Failure::Denied => -13,
            https::Failure::Certificate => -129,
            https::Failure::Timeout => -110,
            https::Failure::Cancelled => -125,
            https::Failure::Capacity => -27,
            https::Failure::Invalid => -22,
            https::Failure::Unavailable => -100,
            https::Failure::Resolution => -113,
            _ => -5,
        }
    }

    // ------------------------=
    // FUNC: authorized
    // DESC: Rechecks original leases, the authenticated owner and current network policy even after TLS completion before copying private response bytes.
    // ------------------=
    fn authorized(&self) -> bool {
        let Some(caps) = self.capabilities else { return false; };
        let Some(now) = crate::runtime::node_client::clock() else { return false; };
        crate::runtime::with_runtime(|runtime| {
            let active = (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|i| runtime.identity.session_nth(i))
                .any(|s| s.id.0 == self.owner.0 && s.state == crate::runtime::identity::SessionState::Active);
            active && runtime.network.profiles.generation() == self.profile_generation
                && runtime.network.profiles.active().is_some_and(|profile|
                profile.interfaces_enabled && profile.internet_allowed && profile.resolver_enabled)
                && [CapabilityType::NetworkConnect, CapabilityType::NetworkSend,
                    CapabilityType::NetworkReceive, CapabilityType::NetworkResolve]
                    .into_iter().enumerate().all(|(i, kind)|
                        runtime.capabilities.validate(caps[i], self.owner, kind, 0, 1, 0, now).is_ok())
        }).unwrap_or(false)
    }

    // ------------------------=
    // FUNC: acquire
    // DESC: Obtains only the active initiating session's current native network leases after explicit invocation admission.
    // ------------------=
    fn acquire(&mut self) -> Result<[CapabilityId; 4], i64> {
        if !self.enabled { return Err(-13); }
        let Some(now) = crate::runtime::node_client::clock() else { return Err(-100); };
        let owner = self.owner;
        let admitted = crate::runtime::with_runtime(|runtime| {
            let active = (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|i| runtime.identity.session_nth(i))
                .any(|s| s.id.0 == owner.0 && s.state == crate::runtime::identity::SessionState::Active);
            runtime.network.browser_authority(&mut runtime.capabilities, owner, active, now)
                .map(|caps| (caps, runtime.network.profiles.generation()))
        }).flatten();
        let Some((caps, profile)) = admitted else { return Err(-13); };
        self.capabilities = Some(caps); self.profile_generation = profile; Ok(caps)
    }

    // ------------------------=
    // FUNC: syscall
    // DESC: Marshals bounded HTTPS operations after the caller's per-slice session check; memory and permissions are revalidated before effects.
    // ------------------=
    pub fn syscall(&mut self, tables: &mut impl PageTable, operation: u64, a: [u64; 6]) -> Option<i64> {
        if (0x1270..=0x1274).contains(&operation) { return Some(self.socket(tables, operation, a)); }
        if !(0x1240..=0x1243).contains(&operation) { return None; }
        Some(self.dispatch(tables, operation, a))
    }

    // ------------------------=
    // FUNC: socket
    // DESC: Marshals real DNS, TCP and strictly verified TLS operations through the shared actor with checked memory, revocation and generation fencing.
    // ------------------=
    fn socket(&mut self, tables: &mut impl PageTable, operation: u64, a: [u64; 6]) -> i64 {
        use crate::http_transport::channel::Command;
        if operation == 0x1274 {
            if !self.channel || a[0] != self.generation { return -9; }
            self.close(); return 0;
        }
        if !self.enabled { return -13; }
        if operation == 0x1270 {
            if self.ticket.is_some() || self.response.is_some() { return -16; }
            if a[0] > 1 || !(1..=253).contains(&a[2]) || !(1..=65535).contains(&a[3]) { return -22; }
            let mut host = [0; 253];
            if copy_from_user(tables, a[1], &mut host[..a[2] as usize]).is_err() { return -14; }
            let caps = match self.acquire() { Ok(caps) => caps, Err(error) => return error };
            let Some(generation) = self.generation.checked_add(1).filter(|v| *v <= i64::MAX as u64) else { return -75; };
            match https::socket_begin(self.owner, caps, &host[..a[2] as usize], None, a[3] as u16, a[0] == 1) {
                Ok(ticket) => { self.ticket = Some(ticket); self.channel = true; self.generation = generation; generation as i64 }
                Err(error) => Self::failure(error),
            }
        } else {
            if !self.channel || a[0] != self.generation { return -9; }
            if !self.authorized() { self.close(); return -13; }
            let Some(ticket) = self.ticket else { return -9; };
            match operation {
                0x1271 => {
                    if a[3] > 16384 { return -22; }
                    let mut bytes = [0; 16384];
                    let command = match a[1] {
                        0 => Command::Read(a[3] as usize),
                        1 => {
                            if copy_from_user(tables, a[2], &mut bytes[..a[3] as usize]).is_err() { return -14; }
                            Command::Write(a[3] as usize)
                        }
                        2 | 3 if (1..=253).contains(&a[3]) => {
                            let mut hostname = [0; 253];
                            if copy_from_user(tables, a[2], &mut hostname[..a[3] as usize]).is_err() { return -14; }
                            if hostname[..a[3] as usize].iter().any(|b| !b.is_ascii_alphanumeric() && !b".-".contains(b)) { return -22; }
                            Command::StartTls { hostname, length: a[3] as usize,
                                trust: if a[1] == 2 { crate::http_transport::trust::Policy::System }
                                    else { crate::http_transport::trust::Policy::Certifi20260617 } }
                        }
                        _ => return -22,
                    };
                    https::socket_command(self.owner, ticket, command,
                        if a[1] == 1 { &bytes[..a[3] as usize] } else { &[] }).map_or_else(Self::failure, |_| 0)
                }
                0x1272 => {
                    let result = match https::socket_result(self.owner, ticket) { Ok(r) => r, Err(e) => return Self::failure(e) };
                    if let Some(error) = result.error { return Self::failure(error); }
                    let mut metadata = [0; 24];
                    metadata[..8].copy_from_slice(&result.state.to_le_bytes());
                    metadata[8..16].copy_from_slice(&(result.count as u64).to_le_bytes());
                    metadata[16..20].copy_from_slice(&result.address);
                    copy_to_user(tables, a[1], &metadata).map_or(-14, |_| 0)
                }
                0x1273 => {
                    if a[2] > 16384 || check_user_range(tables, a[1], a[2] as usize, true).is_err() { return -14; }
                    let mut bytes = [0; 16384];
                    match https::socket_read(self.owner, ticket, &mut bytes[..a[2] as usize]) {
                        Ok(count) => copy_to_user(tables, a[1], &bytes[..count]).map_or(-14, |_| count as i64),
                        Err(error) => Self::failure(error),
                    }
                }
                _ => -38,
            }
        }
    }

    // ------------------------=
    // FUNC: dispatch
    // DESC: Exposes only authenticated HTTPS GET, generation-fenced response reads and cancellation, never raw sockets or insecure TLS modes.
    // ------------------=
    fn dispatch(&mut self, tables: &mut impl PageTable, operation: u64, a: [u64; 6]) -> i64 {
        if operation == 0x1243 {
            if a[0] != self.generation { return -9; }
            self.close(); return 0;
        }
        if !self.enabled { return -13; }
        if self.channel { return -16; }
        if operation == 0x1240 {
            if self.ticket.is_some() || self.response.is_some() { return -16; }
            if !(1..=8192).contains(&a[1]) || a[3] > 8192 { return -22; }
            let mut url = [0; 8192]; let mut headers = [0; 8192];
            if copy_from_user(tables, a[0], &mut url[..a[1] as usize]).is_err()
                || copy_from_user(tables, a[2], &mut headers[..a[3] as usize]).is_err() { return -14; }
            let Ok(url) = core::str::from_utf8(&url[..a[1] as usize]) else { return -22; };
            let Ok(crate::http_transport::geturl::Command::Get(request)) =
                crate::http_transport::geturl::parse(core::iter::once(url)) else { return -22; };
            let Some(now) = crate::runtime::node_client::clock() else { return -100; };
            let owner = self.owner;
            let capabilities = crate::runtime::with_runtime(|runtime| {
                let active = (0..crate::runtime::identity::MAX_SESSIONS)
                    .filter_map(|i| runtime.identity.session_nth(i))
                    .any(|s| s.id.0 == owner.0 && s.state == crate::runtime::identity::SessionState::Active);
                runtime.network.browser_authority(&mut runtime.capabilities, owner, active, now)
                    .map(|caps| (caps, runtime.network.profiles.generation()))
            }).flatten();
            let Some((caps, profile_generation)) = capabilities else { return -13; };
            let Some(generation) = self.generation.checked_add(1).filter(|v| *v <= i64::MAX as u64) else { return -75; };
            match https::get_package(owner, caps[0], caps[1], caps[2], caps[3],
                request.host, 443, request.target, &headers[..a[3] as usize]) {
                Ok(ticket) => { self.ticket = Some(ticket); self.capabilities = Some(caps);
                    self.profile_generation = profile_generation;
                    self.generation = generation; generation as i64 }
                Err(error) => Self::failure(error),
            }
        } else {
            if a[0] != self.generation { return -9; }
            if !self.authorized() { self.close(); return -13; }
            if operation == 0x1241 {
                if check_user_range(tables, a[1], 24, true).is_err() { return -14; }
                if let Some(ticket) = self.ticket {
                    match https::take_browser(self.owner, ticket) {
                        Ok(None) | Err(https::Failure::Busy) => return -11,
                        Ok(Some(Ok(response))) => { self.ticket = None; self.response = Some(response); }
                        Ok(Some(Err(error))) | Err(error) => { self.close(); return Self::failure(error); }
                    }
                }
                let Some(response) = self.response.as_ref() else { return -9; };
                let mut metadata = [0; 24];
                metadata[..8].copy_from_slice(&(response.status as u64).to_le_bytes());
                metadata[8..16].copy_from_slice(&(response.header_length as u64).to_le_bytes());
                metadata[16..].copy_from_slice(&(response.length as u64).to_le_bytes());
                copy_to_user(tables, a[1], &metadata).map_or(-14, |_| 0)
            } else {
                let Some(response) = self.response.as_ref() else { return -9; };
                let bytes = match a[1] { 0 => &response.headers[..response.header_length],
                    1 => &response.bytes[..response.length], _ => return -22 };
                if a[2] > bytes.len() as u64 || a[4] > 65536 { return -22; }
                let count = (bytes.len() - a[2] as usize).min(a[4] as usize);
                copy_to_user(tables, a[3], &bytes[a[2] as usize..a[2] as usize + count]).map_or(-14, |_| count as i64)
            }
        }
    }
}
