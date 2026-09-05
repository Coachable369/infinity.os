//! Deterministic, identity-based network policy. Subjects are stable runtime,
//! service, application, session, or system identities rather than code paths.

use super::types::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EndpointSelector {
    pub network: Option<IpAddress>, pub prefix_length: u8, pub port: Option<u16>,
    pub protocol: Option<TransportProtocol>, pub local_only: bool,
}

impl EndpointSelector {
    // ------------------------=
    // FUNC: any
    // DESC: Creates a selector that matches every valid endpoint.
    // ------------------=
    pub const fn any() -> Self { Self { network: None, prefix_length: 0, port: None, protocol: None, local_only: false } }

    // ------------------------=
    // FUNC: matches
    // DESC: Evaluates an endpoint against address, port, protocol, and locality constraints.
    // ------------------=
    pub fn matches(self, endpoint: Endpoint, protocol: TransportProtocol) -> bool {
        if self.local_only && !endpoint.address.is_host_local() { return false; }
        if self.port.map(|v| v != endpoint.port).unwrap_or(false) { return false; }
        if self.protocol.map(|v| v != protocol).unwrap_or(false) { return false; }
        self.network.map(|v| endpoint.address.matches_prefix(v, self.prefix_length)).unwrap_or(true)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NetworkPolicyRule {
    pub id: PolicyId, pub subject: Subject, pub direction: Direction,
    pub interface_id: Option<InterfaceId>, pub local: EndpointSelector,
    pub remote: EndpointSelector, pub action: PolicyAction, pub priority: u16,
    pub logging: LoggingMode, pub enabled: bool, pub expires_at: Option<u64>,
    pub policy_source: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PolicyDecision { pub action: PolicyAction, pub rule_id: Option<PolicyId>, pub policy_source: u64 }

pub struct PolicyEngine {
    rules: [Option<NetworkPolicyRule>; MAX_POLICIES],
    next_id: PolicyId,
    default_action: PolicyAction,
    denials: u64,
}

impl PolicyEngine {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a deny-by-default bounded policy engine.
    // ------------------=
    pub const fn new() -> Self { Self { rules: [None; MAX_POLICIES], next_id: 1, default_action: PolicyAction::Deny, denials: 0 } }

    // ------------------------=
    // FUNC: create
    // DESC: Persists a typed rule and assigns a stable policy identifier.
    // ------------------=
    pub fn create(&mut self, mut rule: NetworkPolicyRule) -> Result<PolicyId, NetworkError> {
        let slot = self.rules.iter().position(Option::is_none).ok_or(NetworkError::ResourceLimitExceeded)?;
        let id = self.next_id; self.next_id = self.next_id.wrapping_add(1).max(1); rule.id = id;
        self.rules[slot] = Some(rule); Ok(id)
    }

    // ------------------------=
    // FUNC: update
    // DESC: Replaces a rule while preserving its stable identity.
    // ------------------=
    pub fn update(&mut self, id: PolicyId, mut rule: NetworkPolicyRule) -> Result<(), NetworkError> {
        let slot = self.rules.iter().position(|v| v.map(|r| r.id) == Some(id)).ok_or(NetworkError::Conflict)?;
        rule.id = id; self.rules[slot] = Some(rule); Ok(())
    }

    // ------------------------=
    // FUNC: delete
    // DESC: Removes one policy rule by identity.
    // ------------------=
    pub fn delete(&mut self, id: PolicyId) -> Result<(), NetworkError> {
        let slot = self.rules.iter().position(|v| v.map(|r| r.id) == Some(id)).ok_or(NetworkError::Conflict)?;
        self.rules[slot] = None; Ok(())
    }

    // ------------------------=
    // FUNC: evaluate
    // DESC: Resolves matching rules by priority then stable ID with explicit deny by default.
    // ------------------=
    pub fn evaluate(&mut self, subject: Subject, direction: Direction, interface_id: Option<InterfaceId>, local: Endpoint, remote: Endpoint, protocol: TransportProtocol, now: u64) -> PolicyDecision {
        let selected = self.rules.iter().flatten().filter(|r| r.enabled && r.subject == subject && r.direction == direction && r.expires_at.map(|v| now < v).unwrap_or(true) && r.interface_id.map(|v| Some(v) == interface_id).unwrap_or(true) && r.local.matches(local, protocol) && r.remote.matches(remote, protocol)).copied().min_by_key(|r| (r.priority, r.id));
        let decision = selected.map(|r| PolicyDecision { action: r.action, rule_id: Some(r.id), policy_source: r.policy_source }).unwrap_or(PolicyDecision { action: self.default_action, rule_id: None, policy_source: 0 });
        if matches!(decision.action, PolicyAction::Deny | PolicyAction::Ask) { self.denials = self.denials.saturating_add(1); }
        decision
    }

    // ------------------------=
    // FUNC: count
    // DESC: Returns the number of installed policy rules.
    // ------------------=
    pub fn count(&self) -> usize { self.rules.iter().flatten().count() }

    // ------------------------=
    // FUNC: nth
    // DESC: Returns one typed rule for authorized inspection.
    // ------------------=
    pub fn nth(&self, index: usize) -> Option<&NetworkPolicyRule> { self.rules.iter().flatten().nth(index) }

    // ------------------------=
    // FUNC: denials
    // DESC: Returns the observed policy-denial count.
    // ------------------=
    pub const fn denials(&self) -> u64 { self.denials }

    // ------------------------=
    // FUNC: set_default_action
    // DESC: Changes the explicit fallback action used when no rule matches.
    // ------------------=
    pub fn set_default_action(&mut self, action: PolicyAction) { self.default_action = action; }

    // ------------------------=
    // FUNC: default_action
    // DESC: Returns the current explicit fallback policy action.
    // ------------------=
    pub const fn default_action(&self) -> PolicyAction { self.default_action }
}
