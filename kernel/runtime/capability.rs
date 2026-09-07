use super::execution::SecurityIdentity;

pub const MAX_CAPABILITIES: usize = 64;
pub type CapabilityId = u64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CapabilityType {
    ServiceCall,
    EventPublish,
    EventSubscribe,
    StorageRead,
    StorageWrite,
    StorageDiscover,
    StorageProvision,
    ObjectRead,
    ObjectWrite,
    BootInstall,
    SystemInstall,
    AiInfer,
    ModelUse,
    ContextRead,
    ToolInvoke,
    AudioInput,
    AudioOutput,
    AudioDevice,
    AgentControl,
    RemoteAi,
    SystemInspect,
    DeviceInspect,
    NamespaceRead,
    ServiceInspect,
    IdentityReadSelf,
    IdentityUpdateSelf,
    IdentityManage,
    CredentialManageSelf,
    AuthenticationUse,
    SessionManageSelf,
    PersonalSpaceRead,
    PersonalSpaceWrite,
    SettingsRead,
    SettingsUpdateAllowed,
    ShellUse,
    SurfaceCreate,
    SurfacePublish,
    SurfaceResize,
    SurfaceDestroy,
    SurfaceInspectMetadata,
    WindowCreate,
    WindowManageOwn,
    WindowFocus,
    WindowCaptureInput,
    WindowInspectMetadata,
    DisplayCapture,
    TrustedUiPresent,
    NetworkInterfaceInspect,
    NetworkAddressConfigure,
    NetworkRouteInspect,
    NetworkRouteModify,
    NetworkResolve,
    NetworkConnect,
    NetworkListen,
    NetworkAccept,
    NetworkSend,
    NetworkReceive,
    NetworkPolicyInspect,
    NetworkPolicyModify,
    NetworkProfileActivate,
    NetworkServiceDiscover,
    NetworkRawFrame,
    NodeInspect,
    NodePair,
    NodeTrustModify,
    NodeSessionOpen,
    NodeRemoteCall,
    NodeCapabilityGrant,
    MeshInspect,
    MeshModify,
    NodeAuditInspect,
    ResourceUse,
    ResourcePolicyInspect,
    ResourcePolicyModify,
    ResourceExpand,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Capability {
    pub id: CapabilityId,
    pub kind: CapabilityType,
    pub target: u64,
    pub rights: u32,
    pub constraints: u64,
    pub issuer: SecurityIdentity,
    pub holder: SecurityIdentity,
    pub expires_at: Option<u64>,
    pub delegation_rights: u32,
    pub parent: Option<CapabilityId>,
    pub revoked: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CapabilityError {
    Full,
    Unknown,
    Denied,
    Expired,
    Revoked,
    InvalidDelegation,
}

pub struct CapabilityManager {
    entries: [Option<Capability>; MAX_CAPABILITIES],
    next_id: CapabilityId,
}

impl CapabilityManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_CAPABILITIES],
            next_id: 1,
        }
    }
    // ------------------------=
    // FUNC: grant
    // DESC: Implements the grant operation.
    // ------------------=
    pub fn grant(
        &mut self,
        kind: CapabilityType,
        target: u64,
        rights: u32,
        constraints: u64,
        issuer: SecurityIdentity,
        holder: SecurityIdentity,
        expires_at: Option<u64>,
        delegation_rights: u32,
    ) -> Result<CapabilityId, CapabilityError> {
        let slot = self
            .entries
            .iter()
            .position(Option::is_none)
            .ok_or(CapabilityError::Full)?;
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.entries[slot] = Some(Capability {
            id,
            kind,
            target,
            rights,
            constraints,
            issuer,
            holder,
            expires_at,
            delegation_rights: delegation_rights & rights,
            parent: None,
            revoked: false,
        });
        Ok(id)
    }
    // ------------------------=
    // FUNC: get
    // DESC: Implements the get operation.
    // ------------------=
    pub fn get(&self, id: CapabilityId) -> Option<&Capability> {
        self.entries.iter().flatten().find(|c| c.id == id)
    }
    // ------------------------=
    // FUNC: actively_revoked
    // DESC: Implements the actively revoked operation.
    // ------------------=
    fn actively_revoked(&self, capability: &Capability) -> bool {
        if capability.revoked {
            return true;
        }
        let mut parent = capability.parent;
        for _ in 0..MAX_CAPABILITIES {
            let Some(id) = parent else {
                return false;
            };
            let Some(found) = self.get(id) else {
                return true;
            };
            if found.revoked {
                return true;
            }
            parent = found.parent;
        }
        true
    }
    // ------------------------=
    // FUNC: validate
    // DESC: Implements the validate operation.
    // ------------------=
    pub fn validate(
        &self,
        id: CapabilityId,
        holder: SecurityIdentity,
        kind: CapabilityType,
        target: u64,
        rights: u32,
        constraints: u64,
        now: u64,
    ) -> Result<(), CapabilityError> {
        let capability = self.get(id).ok_or(CapabilityError::Unknown)?;
        if self.actively_revoked(capability) {
            return Err(CapabilityError::Revoked);
        }
        if capability
            .expires_at
            .map(|expiry| now >= expiry)
            .unwrap_or(false)
        {
            return Err(CapabilityError::Expired);
        }
        if capability.holder != holder
            || capability.kind != kind
            || capability.target != target
            || rights & !capability.rights != 0
            || (capability.constraints != 0 && constraints != capability.constraints)
        {
            return Err(CapabilityError::Denied);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: revoke
    // DESC: Implements the revoke operation.
    // ------------------=
    pub fn revoke(&mut self, id: CapabilityId) -> Result<(), CapabilityError> {
        let entry = self
            .entries
            .iter_mut()
            .flatten()
            .find(|c| c.id == id)
            .ok_or(CapabilityError::Unknown)?;
        entry.revoked = true;
        Ok(())
    }
    // ------------------------=
    // FUNC: delegate
    // DESC: Implements the delegate operation.
    // ------------------=
    pub fn delegate(
        &mut self,
        parent_id: CapabilityId,
        holder: SecurityIdentity,
        rights: u32,
        constraints: u64,
        expires_at: Option<u64>,
        now: u64,
    ) -> Result<CapabilityId, CapabilityError> {
        let parent = *self.get(parent_id).ok_or(CapabilityError::Unknown)?;
        self.validate(
            parent_id,
            parent.holder,
            parent.kind,
            parent.target,
            parent.rights,
            parent.constraints,
            now,
        )?;
        if rights == 0
            || rights & !parent.delegation_rights != 0
            || (parent.constraints != 0 && constraints != parent.constraints)
            || matches!((parent.expires_at, expires_at), (Some(p), Some(c)) if c > p)
            || matches!((parent.expires_at, expires_at), (Some(_), None))
        {
            return Err(CapabilityError::InvalidDelegation);
        }
        let id = self.grant(
            parent.kind,
            parent.target,
            rights,
            constraints,
            parent.holder,
            holder,
            expires_at,
            parent.delegation_rights & rights,
        )?;
        self.entries
            .iter_mut()
            .flatten()
            .find(|c| c.id == id)
            .unwrap()
            .parent = Some(parent_id);
        Ok(id)
    }
    // ------------------------=
    // FUNC: count
    // DESC: Implements the count operation.
    // ------------------=
    pub fn count(&self) -> usize {
        self.entries.iter().flatten().count()
    }
    // ------------------------=
    // FUNC: nth
    // DESC: Calculates and returns nth.
    // ------------------=
    pub fn nth(&self, index: usize) -> Option<&Capability> {
        self.entries.iter().flatten().nth(index)
    }
}
