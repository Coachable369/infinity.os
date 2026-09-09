//! Deterministic Infinity Console language. Commands become typed operation
//! graphs; rendered text is never used as input to another operation.

use super::iop::OperationId;

pub const MAX_STAGES: usize = 4;
pub const MAX_ARGUMENTS: usize = 8;
pub const MAX_VARIABLES: usize = 8;
pub const MAX_SESSION_REFS: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValueType {
    Unit,
    SystemStatus,
    DeviceSet,
    StorageStatus,
    Object,
    ObjectSet,
    NamespaceResult,
    Project,
    ProjectSet,
    Collection,
    CollectionSet,
    ServiceSet,
    RuntimeContextSet,
    CapabilitySet,
    EventSet,
    ModelSet,
    AgentSet,
    VoiceStatus,
    OperationPlan,
    User,
    UserSet,
    Machine,
    CredentialSet,
    Session,
    SessionSet,
    Profile,
    PersonalSpace,
    Settings,
    SkinSet,
    UiTree,
    WindowSet,
    ClipboardData,
    NetworkStatus,
    NetworkInterfaceSet,
    NetworkAddressSet,
    NetworkRouteSet,
    NetworkConnectionSet,
    NetworkPolicySet,
    NetworkProfileSet,
    NetworkDiagnostics,
    ServiceDiscoverySet,
    NodeSet,
    NodeSessionSet,
    NodePolicy,
    NodeAuditSet,
    MeshDomainSet,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SideEffectClass {
    Query,
    ReversibleChange,
    DestructiveChange,
    SecurityChange,
    ExternalEffect,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArgumentType {
    Text,
    Boolean,
    ObjectType,
    ObjectRef,
    ProjectRef,
    CollectionRef,
    DeviceRef,
    ServiceRef,
    NamespacePath,
    Temporal,
    UserRef,
    SessionRef,
    MachineRef,
    NetworkRef,
    NodeRef,
}

#[derive(Clone, Copy)]
pub struct ArgumentSchema {
    pub name: &'static [u8],
    pub value_type: ArgumentType,
    pub required: bool,
}

#[derive(Clone, Copy)]
pub struct OperationSchema {
    pub domain: &'static [u8],
    pub action: &'static [u8],
    pub description: &'static [u8],
    pub operation: OperationId,
    pub input: ValueType,
    pub output: ValueType,
    pub target: Option<ArgumentType>,
    pub arguments: &'static [ArgumentSchema],
    pub capability: u64,
    pub side_effect: SideEffectClass,
    pub example: &'static [u8],
}

#[derive(Clone, Copy)]
pub struct DomainSchema {
    pub name: &'static [u8],
    pub description: &'static [u8],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReferenceKind {
    Object,
    Project,
    Collection,
    Device,
    Service,
    Session,
    Namespace,
    User,
    Machine,
    Network,
    Node,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HumanReference<'a> {
    pub kind: ReferenceKind,
    pub value: &'a [u8],
}

#[derive(Clone, Copy)]
pub struct ParsedArgument<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
    pub value_type: ArgumentType,
}

#[derive(Clone, Copy)]
pub struct OperationNode<'a> {
    pub schema: &'static OperationSchema,
    pub target: Option<HumanReference<'a>>,
    pub arguments: [Option<ParsedArgument<'a>>; MAX_ARGUMENTS],
    pub argument_count: u8,
}

#[derive(Clone, Copy)]
pub struct OperationGraph<'a> {
    pub nodes: [Option<OperationNode<'a>>; MAX_STAGES],
    pub node_count: u8,
    pub plan_only: bool,
    pub assignment: Option<&'a [u8]>,
    pub result_type: ValueType,
    pub maximum_effect: SideEffectClass,
}

impl OperationGraph<'_> {
    // ------------------------=
    // FUNC: executable_pool_mutation
    // DESC: Admits only implemented single-stage Pool mutations; execution still requires the authenticated IOP broker.
    // ------------------=
    pub fn executable_pool_mutation(&self) -> bool {
        !self.plan_only && self.node_count == 1 && self.nodes[0].map(|node| {
            node.schema.domain == b"pool" && matches!(node.schema.operation,
                OperationId::ObjectCreate | OperationId::ObjectUpdate |
                OperationId::ObjectCopy | OperationId::ObjectSetPolicy | OperationId::ResourceAdvertise | OperationId::PoolHeal |
                OperationId::ObjectDelete | OperationId::PoolUploadBegin | OperationId::PoolUploadAppend | OperationId::PoolUploadCommit | OperationId::PoolUploadAbort)
        }).unwrap_or(false)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TypedVariable {
    pub name: [u8; 24],
    pub name_length: u8,
    pub value_type: ValueType,
}

pub struct ConsoleSession {
    variables: [Option<TypedVariable>; MAX_VARIABLES],
    references: [Option<[u8; 16]>; MAX_SESSION_REFS],
    reference_count: u8,
}

impl ConsoleSession {
    // ------------------------=
    // FUNC: new
    // DESC: Creates isolated typed variables and contextual references for one Console session.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            variables: [None; MAX_VARIABLES],
            references: [None; MAX_SESSION_REFS],
            reference_count: 0,
        }
    }

    // ------------------------=
    // FUNC: assign
    // DESC: Binds a name to a result type without performing string substitution.
    // ------------------=
    pub fn assign(
        &mut self,
        name: &[u8],
        value_type: ValueType,
    ) -> Result<(), ConsoleLanguageError> {
        if name.is_empty() || name.len() > 24 {
            return Err(ConsoleLanguageError::InvalidAssignment);
        }
        let mut stored = [0; 24];
        stored[..name.len()].copy_from_slice(name);
        let variable = TypedVariable {
            name: stored,
            name_length: name.len() as u8,
            value_type,
        };
        if let Some(slot) = self.variables.iter_mut().find(|item| {
            item.as_ref()
                .map(|value| &value.name[..value.name_length as usize] == name)
                .unwrap_or(false)
        }) {
            *slot = Some(variable);
            return Ok(());
        }
        let slot = self
            .variables
            .iter_mut()
            .find(|item| item.is_none())
            .ok_or(ConsoleLanguageError::TooManyArguments)?;
        *slot = Some(variable);
        Ok(())
    }

    // ------------------------=
    // FUNC: variable
    // DESC: Resolves a typed variable descriptor without rendered-text evaluation.
    // ------------------=
    pub fn variable(&self, name: &[u8]) -> Option<TypedVariable> {
        self.variables
            .iter()
            .flatten()
            .find(|value| &value.name[..value.name_length as usize] == name)
            .copied()
    }

    // ------------------------=
    // FUNC: bind_references
    // DESC: Replaces ephemeral numbered references with the latest typed ObjectSet.
    // ------------------=
    pub fn bind_references(&mut self, refs: &[[u8; 16]]) {
        self.references = [None; MAX_SESSION_REFS];
        self.reference_count = refs.len().min(MAX_SESSION_REFS) as u8;
        for (index, reference) in refs.iter().take(MAX_SESSION_REFS).enumerate() {
            self.references[index] = Some(*reference);
        }
    }

    // ------------------------=
    // FUNC: resolve_contextual
    // DESC: Resolves @1-style references only inside the current Console session.
    // ------------------=
    pub fn resolve_contextual(&self, token: &[u8]) -> Result<[u8; 16], ConsoleLanguageError> {
        let digits = token
            .strip_prefix(b"@")
            .ok_or(ConsoleLanguageError::InvalidArgumentType)?;
        let mut number = 0usize;
        if digits.is_empty() || !digits.iter().all(|byte| byte.is_ascii_digit()) {
            return Err(ConsoleLanguageError::InvalidArgumentType);
        }
        for digit in digits {
            number = number
                .saturating_mul(10)
                .saturating_add((digit - b'0') as usize);
        }
        if number == 0 || number > self.reference_count as usize {
            return Err(ConsoleLanguageError::ReferenceNotFound);
        }
        self.references[number - 1].ok_or(ConsoleLanguageError::ReferenceNotFound)
    }
}

#[derive(Clone, Copy)]
pub enum ParseOutcome<'a> {
    Graph(OperationGraph<'a>),
    DomainDiscovery(&'static DomainSchema),
    OperationDiscovery(&'static OperationSchema),
    Help(
        Option<&'static DomainSchema>,
        Option<&'static OperationSchema>,
    ),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConsoleLanguageError {
    Empty,
    UnknownDomain,
    UnknownOperation,
    MissingArgument,
    InvalidArgument,
    InvalidArgumentType,
    DuplicateArgument,
    UnterminatedQuote,
    TooManyStages,
    TooManyArguments,
    TypeMismatch,
    InvalidAssignment,
    ReferenceNotFound,
}

const NO_ARGS: &[ArgumentSchema] = &[];
const POOL_PEER: ArgumentSchema = ArgumentSchema { name: b"peer", value_type: ArgumentType::NodeRef, required: false };
const POOL_GRANT: ArgumentSchema = ArgumentSchema { name: b"grant", value_type: ArgumentType::Text, required: false };
const POOL_RESULT_ARGS: &[ArgumentSchema] = &[ArgumentSchema { name: b"request", value_type: ArgumentType::Text, required: true }];
const POOL_FIXTURE_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema {name:b"length",value_type:ArgumentType::Text,required:true},
    ArgumentSchema {name:b"seed",value_type:ArgumentType::Text,required:true},
    ArgumentSchema {name:b"policy",value_type:ArgumentType::Text,required:true},
    ArgumentSchema {name:b"confirm",value_type:ArgumentType::Text,required:true},
];
const POOL_RETIRE_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema{name:b"peer",value_type:ArgumentType::Text,required:true},
    ArgumentSchema{name:b"grant",value_type:ArgumentType::Text,required:true},
    ArgumentSchema{name:b"lease",value_type:ArgumentType::Text,required:true},
    ArgumentSchema{name:b"confirm",value_type:ArgumentType::Text,required:true},
];
const POOL_UPLOAD_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema { name:b"length",value_type:ArgumentType::Text,required:true },
    ArgumentSchema { name:b"hash",value_type:ArgumentType::Text,required:true },
    ArgumentSchema { name:b"policy",value_type:ArgumentType::Text,required:true },
    ArgumentSchema { name:b"nonce",value_type:ArgumentType::Text,required:true },
    ArgumentSchema { name:b"generation",value_type:ArgumentType::Text,required:false },
];
const POOL_APPEND_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema { name:b"offset",value_type:ArgumentType::Text,required:true },
    ArgumentSchema { name:b"hex",value_type:ArgumentType::Text,required:true },
];
const POOL_DELETE_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema { name:b"generation",value_type:ArgumentType::Text,required:true },
    ArgumentSchema { name:b"version",value_type:ArgumentType::Text,required:true },
    ArgumentSchema { name:b"confirm",value_type:ArgumentType::Text,required:true },
];
const POOL_PARTICIPATE_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema { name: b"peer", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"begin", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"chunk", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"commit", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"inspect", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"read", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"lease", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"confirm", value_type: ArgumentType::Text, required: true },
];
const POOL_ADVERTISE_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema { name: b"peer", value_type: ArgumentType::NodeRef, required: true },
    ArgumentSchema { name: b"grant", value_type: ArgumentType::Text, required: true },
];
const POOL_CREATE_ARGS: &[ArgumentSchema] = &[
    POOL_PEER, POOL_GRANT,
    ArgumentSchema { name: b"nonce", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"policy", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"content", value_type: ArgumentType::Text, required: false },
];
const POOL_INSPECT_ARGS: &[ArgumentSchema] = &[
    POOL_PEER, POOL_GRANT,
    ArgumentSchema { name: b"offset", value_type: ArgumentType::Text, required: false },
    ArgumentSchema { name: b"generation", value_type: ArgumentType::Text, required: false },
];
const POOL_POLICY_ARGS: &[ArgumentSchema] = &[
    POOL_PEER, POOL_GRANT,
    ArgumentSchema { name: b"generation", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"version", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"policy", value_type: ArgumentType::Text, required: true },
];
const POOL_READ_ARGS: &[ArgumentSchema] = &[
    POOL_PEER, POOL_GRANT,
    ArgumentSchema { name: b"generation", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"version", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"offset", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"length", value_type: ArgumentType::Text, required: true },
];
const POOL_WRITE_ARGS: &[ArgumentSchema] = &[
    POOL_PEER, POOL_GRANT,
    ArgumentSchema { name: b"generation", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"version", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"content", value_type: ArgumentType::Text, required: false },
];
const POOL_COPY_ARGS: &[ArgumentSchema] = &[
    POOL_PEER, POOL_GRANT,
    ArgumentSchema { name: b"generation", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"version", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"nonce", value_type: ArgumentType::Text, required: true },
];
const NODE_LINK_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema { name: b"local", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"remote", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"local-port", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"remote-port", value_type: ArgumentType::Text, required: true },
];
const NODE_GRANT_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema { name: b"name", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"seconds", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"confirm", value_type: ArgumentType::Boolean, required: false },
];
const NODE_REMOTE_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema { name: b"grant", value_type: ArgumentType::Text, required: true },
];
const NODE_REMOTE_POLICY_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema { name: b"grant", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"name", value_type: ArgumentType::Text, required: true },
    ArgumentSchema { name: b"value", value_type: ArgumentType::Text, required: true },
];
const FIND_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema {
        name: b"type",
        value_type: ArgumentType::ObjectType,
        required: false,
    },
    ArgumentSchema {
        name: b"name",
        value_type: ArgumentType::Text,
        required: false,
    },
    ArgumentSchema {
        name: b"tag",
        value_type: ArgumentType::Text,
        required: false,
    },
    ArgumentSchema {
        name: b"project",
        value_type: ArgumentType::ProjectRef,
        required: false,
    },
    ArgumentSchema {
        name: b"collection",
        value_type: ArgumentType::CollectionRef,
        required: false,
    },
    ArgumentSchema {
        name: b"modified",
        value_type: ArgumentType::Temporal,
        required: false,
    },
    ArgumentSchema {
        name: b"owner",
        value_type: ArgumentType::ObjectRef,
        required: false,
    },
    ArgumentSchema {
        name: b"space",
        value_type: ArgumentType::Text,
        required: false,
    },
];
const DESTINATION: &[ArgumentSchema] = &[ArgumentSchema {
    name: b"destination",
    value_type: ArgumentType::NamespacePath,
    required: true,
}];
const NAME_ARG: &[ArgumentSchema] = &[ArgumentSchema {
    name: b"name",
    value_type: ArgumentType::Text,
    required: true,
}];
const USER_CREATE_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema {
        name: b"handle",
        value_type: ArgumentType::Text,
        required: true,
    },
    ArgumentSchema {
        name: b"display-name",
        value_type: ArgumentType::Text,
        required: true,
    },
];
const DISPLAY_NAME_ARG: &[ArgumentSchema] = &[ArgumentSchema {
    name: b"display-name",
    value_type: ArgumentType::Text,
    required: true,
}];
const MACHINE_NAME_ARG: &[ArgumentSchema] = &[ArgumentSchema {
    name: b"name",
    value_type: ArgumentType::Text,
    required: true,
}];
const CREDENTIAL_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema {
        name: b"user",
        value_type: ArgumentType::UserRef,
        required: true,
    },
    ArgumentSchema {
        name: b"type",
        value_type: ArgumentType::Text,
        required: true,
    },
];
const AI_PROFILE_ARGS: &[ArgumentSchema] = &[ArgumentSchema {
    name: b"provider-policy",
    value_type: ArgumentType::Text,
    required: true,
}];
const VOICE_PROFILE_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema {
        name: b"enabled",
        value_type: ArgumentType::Boolean,
        required: true,
    },
    ArgumentSchema {
        name: b"activation",
        value_type: ArgumentType::Text,
        required: true,
    },
];
const SETTING_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema {
        name: b"name",
        value_type: ArgumentType::Text,
        required: true,
    },
    ArgumentSchema {
        name: b"value",
        value_type: ArgumentType::Text,
        required: true,
    },
];
const PAIR_CONFIRM_ARGS: &[ArgumentSchema] = &[ArgumentSchema {
    name: b"code",
    value_type: ArgumentType::Text,
    required: true,
}];
const NETWORK_CONFIG_ARGS: &[ArgumentSchema] = &[
    ArgumentSchema {
        name: b"address",
        value_type: ArgumentType::Text,
        required: false,
    },
    ArgumentSchema {
        name: b"prefix",
        value_type: ArgumentType::Text,
        required: false,
    },
    ArgumentSchema {
        name: b"gateway",
        value_type: ArgumentType::Text,
        required: false,
    },
    ArgumentSchema {
        name: b"interface",
        value_type: ArgumentType::NetworkRef,
        required: false,
    },
    ArgumentSchema {
        name: b"metric",
        value_type: ArgumentType::Text,
        required: false,
    },
    ArgumentSchema {
        name: b"subject",
        value_type: ArgumentType::Text,
        required: false,
    },
    ArgumentSchema {
        name: b"destination",
        value_type: ArgumentType::Text,
        required: false,
    },
    ArgumentSchema {
        name: b"duration",
        value_type: ArgumentType::Text,
        required: false,
    },
];

pub static DOMAINS: &[DomainSchema] = &[
    DomainSchema { name: b"pool", description: b"Authoritative native Pool objects and protection policy" },
    DomainSchema {
        name: b"system",
        description: b"System health, identity, and generations",
    },
    DomainSchema {
        name: b"device",
        description: b"Typed hardware capabilities",
    },
    DomainSchema {
        name: b"storage",
        description: b"Infinity Pool state and usage",
    },
    DomainSchema {
        name: b"object",
        description: b"Persistent objects and queries",
    },
    DomainSchema {
        name: b"namespace",
        description: b"Human namespace references",
    },
    DomainSchema {
        name: b"project",
        description: b"First-class project objects",
    },
    DomainSchema {
        name: b"collection",
        description: b"Static and dynamic object collections",
    },
    DomainSchema {
        name: b"service",
        description: b"System service lifecycle",
    },
    DomainSchema {
        name: b"runtime",
        description: b"Execution contexts and resources",
    },
    DomainSchema {
        name: b"capability",
        description: b"Explicit authority and leases",
    },
    DomainSchema {
        name: b"event",
        description: b"Typed event subscriptions and traces",
    },
    DomainSchema {
        name: b"ai",
        description: b"AI provider and inference state",
    },
    DomainSchema {
        name: b"model",
        description: b"Installed model objects",
    },
    DomainSchema {
        name: b"agent",
        description: b"Constrained agent contexts",
    },
    DomainSchema {
        name: b"voice",
        description: b"Voice session state",
    },
    DomainSchema {
        name: b"user",
        description: b"Native user identities and Personal Space ownership",
    },
    DomainSchema {
        name: b"identity",
        description: b"Stable user identity inspection",
    },
    DomainSchema {
        name: b"credential",
        description: b"Private authentication methods",
    },
    DomainSchema {
        name: b"session",
        description: b"Authenticated session lifecycle",
    },
    DomainSchema {
        name: b"machine",
        description: b"Stable machine identity and display name",
    },
    DomainSchema {
        name: b"profile",
        description: b"User profile metadata",
    },
    DomainSchema {
        name: b"personal-space",
        description: b"User-scoped native object ownership",
    },
    DomainSchema {
        name: b"ai-profile",
        description: b"User-scoped AI privacy policy",
    },
    DomainSchema {
        name: b"voice-profile",
        description: b"Explicit voice and microphone preferences",
    },
    DomainSchema {
        name: b"settings",
        description: b"Typed scoped system preferences",
    },
    DomainSchema {
        name: b"appearance",
        description: b"Active skin, scale, accent, and wallpaper preferences",
    },
    DomainSchema {
        name: b"skin",
        description: b"Installed versioned InfinityUI skin packages",
    },
    DomainSchema {
        name: b"ui",
        description: b"InfinityUI semantic tree, focus, and damage diagnostics",
    },
    DomainSchema {
        name: b"window",
        description: b"Owned desktop windows and compositor surfaces",
    },
    DomainSchema {
        name: b"clipboard",
        description: b"Capability-gated typed clipboard content",
    },
    DomainSchema {
        name: b"network",
        description: b"Native connectivity, interfaces, routes, policy, profiles, and diagnostics",
    },
    DomainSchema {
        name: b"node",
        description: b"Cryptographic node identity, trust, pairing, sessions, policy, and audit",
    },
    DomainSchema {
        name: b"mesh",
        description: b"Explicit resource-domain membership and node health",
    },
];

pub static OPERATIONS: &[OperationSchema] = &[
    op(b"pool",b"retire-authority",b"Approve exact recipient retirement authority for a bounded lease",OperationId::PoolHeal,ValueType::Unit,ValueType::Unit,None,POOL_RETIRE_ARGS,1,SideEffectClass::ReversibleChange,b"pool retire-authority peer=node:<id> grant=1 lease=3600 confirm=true"),
    op(b"pool",b"fixture",b"Generate explicitly synthetic bounded QA content through ordinary upload operations",OperationId::PoolUploadBegin,ValueType::Unit,ValueType::Object,None,POOL_FIXTURE_ARGS,1,SideEffectClass::ReversibleChange,b"pool fixture length=32768 seed=17 policy=critical confirm=true"),
    op(b"pool",b"upload",b"Begin a bounded durable object upload",OperationId::PoolUploadBegin,ValueType::Unit,ValueType::Object,None,POOL_UPLOAD_ARGS,1,SideEffectClass::ReversibleChange,b"pool upload length=0 hash=<sha256> policy=critical nonce=1"),
    op(b"pool",b"append",b"Append at most sixty bytes to an owned durable upload",OperationId::PoolUploadAppend,ValueType::Unit,ValueType::Object,Some(ArgumentType::ObjectRef),POOL_APPEND_ARGS,1,SideEffectClass::ReversibleChange,b"pool append obj:<upload> offset=0 hex=0102"),
    op(b"pool",b"finish",b"Advance bounded upload verification and commit only complete content",OperationId::PoolUploadCommit,ValueType::Unit,ValueType::Object,Some(ArgumentType::ObjectRef),&[],1,SideEffectClass::ReversibleChange,b"pool finish obj:<upload>"),
    op(b"pool",b"abort",b"Abort an owned incomplete upload",OperationId::PoolUploadAbort,ValueType::Unit,ValueType::Unit,Some(ArgumentType::ObjectRef),&[],1,SideEffectClass::ReversibleChange,b"pool abort obj:<upload>"),
    op(b"pool",b"delete",b"Delete an explicitly confirmed object while preserving other content references",OperationId::ObjectDelete,ValueType::Unit,ValueType::Unit,Some(ArgumentType::ObjectRef),POOL_DELETE_ARGS,1,SideEffectClass::ReversibleChange,b"pool delete obj:<id> generation=1 version=1 confirm=true"),
    op(b"pool", b"participate", b"Approve bounded persistent automatic replication to an explicitly granted peer", OperationId::PoolHeal,
        ValueType::Unit, ValueType::Unit, None, POOL_PARTICIPATE_ARGS, 1, SideEffectClass::ReversibleChange, b"pool participate peer=node:<id> begin=1 chunk=2 commit=3 inspect=4 read=5 lease=3600 confirm=true"),
    op(b"pool", b"advertise", b"Persist bounded measured-storage publication under an explicit peer grant", OperationId::ResourceAdvertise,
        ValueType::Unit, ValueType::Unit, None, POOL_ADVERTISE_ARGS, 1, SideEffectClass::ReversibleChange, b"pool advertise peer=node:<id> grant=1"),
    op(b"pool", b"result", b"Collect an authenticated operator's asynchronous storage result", OperationId::PoolInspect,
        ValueType::Unit, ValueType::Object, None, POOL_RESULT_ARGS, 1, SideEffectClass::Query, b"pool result request=1"),
    op(b"pool", b"copy", b"Create an independent object sharing immutable content", OperationId::ObjectCopy,
        ValueType::Unit, ValueType::Object, Some(ArgumentType::ObjectRef), POOL_COPY_ARGS, 1, SideEffectClass::ReversibleChange, b"pool copy obj:<id> generation=1 version=1 nonce=2"),
    op(b"pool", b"list", b"Inspect one owner-scoped committed Pool object", OperationId::PoolInspect,
        ValueType::Unit, ValueType::ObjectSet, None, POOL_INSPECT_ARGS, 1, SideEffectClass::Query, b"pool list offset=0"),
    op(b"pool", b"create", b"Create a native Pool object with an explicit protection contract", OperationId::ObjectCreate,
        ValueType::Unit, ValueType::Object, None, POOL_CREATE_ARGS, 1, SideEffectClass::ReversibleChange, b"pool create nonce=1 policy=critical content=Example"),
    op(b"pool", b"inspect", b"Read a bounded canonical manifest page", OperationId::ObjectInspect,
        ValueType::Unit, ValueType::Object, Some(ArgumentType::ObjectRef), POOL_INSPECT_ARGS, 1, SideEffectClass::Query, b"pool inspect obj:<id> offset=0"),
    op(b"pool", b"policy", b"Commit a generation-fenced protection policy", OperationId::ObjectSetPolicy,
        ValueType::Unit, ValueType::Object, Some(ArgumentType::ObjectRef), POOL_POLICY_ARGS, 1, SideEffectClass::ReversibleChange, b"pool policy obj:<id> generation=1 version=1 policy=critical"),
    op(b"pool", b"read", b"Read a verified immutable Pool range", OperationId::ObjectRead,
        ValueType::Unit, ValueType::Object, Some(ArgumentType::ObjectRef), POOL_READ_ARGS, 1, SideEffectClass::Query, b"pool read obj:<id> generation=1 version=1 offset=0 length=8"),
    op(b"pool", b"write", b"Atomically replace bounded content and its manifest", OperationId::ObjectUpdate,
        ValueType::Unit, ValueType::Object, Some(ArgumentType::ObjectRef), POOL_WRITE_ARGS, 1, SideEffectClass::ReversibleChange, b"pool write obj:<id> generation=1 version=1 content=Updated"),
    op(
        b"system",
        b"status",
        b"Show system readiness",
        OperationId::SystemStatus,
        ValueType::Unit,
        ValueType::SystemStatus,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"system status",
    ),
    op(
        b"system",
        b"info",
        b"Inspect system identity",
        OperationId::SystemInfo,
        ValueType::Unit,
        ValueType::SystemStatus,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"system info",
    ),
    op(
        b"system",
        b"generation",
        b"Inspect the active System Generation",
        OperationId::SystemGenerationInspect,
        ValueType::Unit,
        ValueType::SystemStatus,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"system generation",
    ),
    op(
        b"system",
        b"boot",
        b"Inspect the verified boot source",
        OperationId::SystemBootStatus,
        ValueType::Unit,
        ValueType::SystemStatus,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"system boot",
    ),
    op(
        b"device",
        b"list",
        b"List discovered devices",
        OperationId::DeviceList,
        ValueType::Unit,
        ValueType::DeviceSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"device list",
    ),
    op(
        b"device",
        b"inspect",
        b"Inspect one device",
        OperationId::DeviceInspect,
        ValueType::Unit,
        ValueType::Object,
        Some(ArgumentType::DeviceRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"device inspect device:storage0",
    ),
    op(
        b"storage",
        b"status",
        b"Show Infinity Pool state",
        OperationId::StorageQuery,
        ValueType::Unit,
        ValueType::StorageStatus,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"storage status",
    ),
    op(
        b"storage",
        b"usage",
        b"Show object allocation",
        OperationId::StorageUsage,
        ValueType::Unit,
        ValueType::StorageStatus,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"storage usage",
    ),
    op(
        b"storage",
        b"inspect",
        b"Inspect a storage capability",
        OperationId::StorageQuery,
        ValueType::Unit,
        ValueType::StorageStatus,
        Some(ArgumentType::DeviceRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"storage inspect device:storage0",
    ),
    op(
        b"object",
        b"find",
        b"Query object metadata and relationships",
        OperationId::ObjectQuery,
        ValueType::Unit,
        ValueType::ObjectSet,
        None,
        FIND_ARGS,
        1,
        SideEffectClass::Query,
        b"object find type=document project=InfinityOS",
    ),
    op(
        b"object",
        b"filter",
        b"Filter an ObjectSet",
        OperationId::ObjectFilter,
        ValueType::ObjectSet,
        ValueType::ObjectSet,
        None,
        FIND_ARGS,
        1,
        SideEffectClass::Query,
        b"object filter tag=architecture",
    ),
    op(
        b"object",
        b"inspect",
        b"Inspect an object",
        OperationId::ObjectRead,
        ValueType::Unit,
        ValueType::Object,
        Some(ArgumentType::ObjectRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"object inspect obj:7f84",
    ),
    op(
        b"object",
        b"destroy",
        b"Destroy unreferenced objects",
        OperationId::ObjectDestroy,
        ValueType::ObjectSet,
        ValueType::ObjectSet,
        Some(ArgumentType::ObjectRef),
        FIND_ARGS,
        2,
        SideEffectClass::DestructiveChange,
        b"plan object destroy project=OldPrototype",
    ),
    op(
        b"namespace",
        b"move",
        b"Move namespace references without changing Object IDs",
        OperationId::NamespaceMove,
        ValueType::ObjectSet,
        ValueType::NamespaceResult,
        Some(ArgumentType::ObjectRef),
        DESTINATION,
        2,
        SideEffectClass::ReversibleChange,
        b"namespace move obj:7f84 destination=/archive/spec",
    ),
    op(
        b"project",
        b"list",
        b"List project objects",
        OperationId::ProjectList,
        ValueType::Unit,
        ValueType::ProjectSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"project list",
    ),
    op(
        b"project",
        b"inspect",
        b"Inspect a project object",
        OperationId::ProjectInspect,
        ValueType::Unit,
        ValueType::Project,
        Some(ArgumentType::ProjectRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"project inspect project:InfinityOS",
    ),
    op(
        b"project",
        b"create",
        b"Create a project object",
        OperationId::ProjectCreate,
        ValueType::Unit,
        ValueType::Project,
        None,
        NAME_ARG,
        2,
        SideEffectClass::ReversibleChange,
        b"project create name=InfinityOS",
    ),
    op(
        b"collection",
        b"list",
        b"List collection objects",
        OperationId::CollectionList,
        ValueType::Unit,
        ValueType::CollectionSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"collection list",
    ),
    op(
        b"collection",
        b"inspect",
        b"Inspect a collection",
        OperationId::CollectionInspect,
        ValueType::Unit,
        ValueType::Collection,
        Some(ArgumentType::CollectionRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"collection inspect collection:Installer-Artwork",
    ),
    op(
        b"collection",
        b"create",
        b"Create a collection object",
        OperationId::CollectionCreate,
        ValueType::Unit,
        ValueType::Collection,
        None,
        NAME_ARG,
        2,
        SideEffectClass::ReversibleChange,
        b"collection create name=Installer-Artwork",
    ),
    op(
        b"service",
        b"list",
        b"List registered services",
        OperationId::ServiceList,
        ValueType::Unit,
        ValueType::ServiceSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"service list",
    ),
    op(
        b"runtime",
        b"contexts",
        b"List execution contexts",
        OperationId::RuntimeContexts,
        ValueType::Unit,
        ValueType::RuntimeContextSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"runtime contexts",
    ),
    op(
        b"capability",
        b"list",
        b"List visible capabilities",
        OperationId::CapabilityList,
        ValueType::Unit,
        ValueType::CapabilitySet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"capability list",
    ),
    op(
        b"event",
        b"subscriptions",
        b"List event subscriptions",
        OperationId::EventSubscriptions,
        ValueType::Unit,
        ValueType::EventSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"event subscriptions",
    ),
    op(
        b"ai",
        b"status",
        b"Show AI runtime status",
        OperationId::IntentResolve,
        ValueType::Unit,
        ValueType::SystemStatus,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"ai status",
    ),
    op(
        b"model",
        b"list",
        b"List model objects",
        OperationId::ModelList,
        ValueType::Unit,
        ValueType::ModelSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"model list",
    ),
    op(
        b"agent",
        b"list",
        b"List constrained agents",
        OperationId::AgentList,
        ValueType::Unit,
        ValueType::AgentSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"agent list",
    ),
    op(
        b"voice",
        b"status",
        b"Show voice-session status",
        OperationId::VoiceStatus,
        ValueType::Unit,
        ValueType::VoiceStatus,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"voice status",
    ),
    op(
        b"user",
        b"create",
        b"Create a native user identity",
        OperationId::IdentityCreate,
        ValueType::Unit,
        ValueType::User,
        None,
        USER_CREATE_ARGS,
        2,
        SideEffectClass::SecurityChange,
        b"user create handle=aurelius display-name=\"Aurelius Prime\"",
    ),
    op(
        b"user",
        b"list",
        b"List user identities",
        OperationId::IdentityList,
        ValueType::Unit,
        ValueType::UserSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"user list",
    ),
    op(
        b"user",
        b"read",
        b"Read one safe user projection",
        OperationId::IdentityRead,
        ValueType::Unit,
        ValueType::User,
        Some(ArgumentType::UserRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"user read user:1",
    ),
    op(
        b"user",
        b"update",
        b"Update allowed user profile fields",
        OperationId::IdentityUpdate,
        ValueType::Unit,
        ValueType::User,
        Some(ArgumentType::UserRef),
        DISPLAY_NAME_ARG,
        2,
        SideEffectClass::SecurityChange,
        b"user update user:1 display-name=\"Aurelius Prime\"",
    ),
    op(
        b"user",
        b"delete",
        b"Deactivate a user while retaining audit identity",
        OperationId::IdentityDelete,
        ValueType::Unit,
        ValueType::User,
        Some(ArgumentType::UserRef),
        NO_ARGS,
        4,
        SideEffectClass::SecurityChange,
        b"user delete user:2",
    ),
    op(
        b"identity",
        b"read",
        b"Read the current stable identity",
        OperationId::IdentityRead,
        ValueType::Unit,
        ValueType::User,
        Some(ArgumentType::UserRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"identity read user:1",
    ),
    op(
        b"machine",
        b"read",
        b"Read stable machine identity",
        OperationId::MachineRead,
        ValueType::Unit,
        ValueType::Machine,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"machine read",
    ),
    op(
        b"machine",
        b"update",
        b"Rename this machine without changing identity",
        OperationId::MachineUpdate,
        ValueType::Unit,
        ValueType::Machine,
        None,
        MACHINE_NAME_ARG,
        2,
        SideEffectClass::ReversibleChange,
        b"machine update name=InfinityNode",
    ),
    op(
        b"credential",
        b"create",
        b"Create a credential through private secret entry",
        OperationId::CredentialCreate,
        ValueType::Unit,
        ValueType::CredentialSet,
        None,
        CREDENTIAL_ARGS,
        2,
        SideEffectClass::SecurityChange,
        b"credential create user=user:1 type=password",
    ),
    op(
        b"credential",
        b"list",
        b"List safe credential metadata",
        OperationId::CredentialList,
        ValueType::Unit,
        ValueType::CredentialSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"credential list",
    ),
    op(
        b"credential",
        b"delete",
        b"Revoke a credential",
        OperationId::CredentialDelete,
        ValueType::Unit,
        ValueType::CredentialSet,
        Some(ArgumentType::ObjectRef),
        NO_ARGS,
        2,
        SideEffectClass::SecurityChange,
        b"credential delete obj:2",
    ),
    op(
        b"session",
        b"list",
        b"List active and historical sessions",
        OperationId::SessionList,
        ValueType::Unit,
        ValueType::SessionSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"session list",
    ),
    op(
        b"session",
        b"read",
        b"Read one session",
        OperationId::SessionRead,
        ValueType::Unit,
        ValueType::Session,
        Some(ArgumentType::SessionRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"session read session:1",
    ),
    op(
        b"session",
        b"lock",
        b"Lock the current authenticated session",
        OperationId::SessionLock,
        ValueType::Unit,
        ValueType::Session,
        None,
        NO_ARGS,
        2,
        SideEffectClass::SecurityChange,
        b"session lock",
    ),
    op(
        b"session",
        b"delete",
        b"End a session without deleting audit state",
        OperationId::SessionEnd,
        ValueType::Unit,
        ValueType::Session,
        Some(ArgumentType::SessionRef),
        NO_ARGS,
        2,
        SideEffectClass::SecurityChange,
        b"session delete session:1",
    ),
    op(
        b"personal-space",
        b"read",
        b"Read Personal Space ownership",
        OperationId::PersonalSpaceRead,
        ValueType::Unit,
        ValueType::PersonalSpace,
        Some(ArgumentType::UserRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"personal-space read user:1",
    ),
    op(
        b"ai-profile",
        b"read",
        b"Read user AI privacy policy",
        OperationId::AiProfileRead,
        ValueType::Unit,
        ValueType::Profile,
        Some(ArgumentType::UserRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"ai-profile read user:1",
    ),
    op(
        b"ai-profile",
        b"update",
        b"Update user AI privacy policy",
        OperationId::AiProfileUpdate,
        ValueType::Unit,
        ValueType::Profile,
        Some(ArgumentType::UserRef),
        AI_PROFILE_ARGS,
        2,
        SideEffectClass::SecurityChange,
        b"ai-profile update user:1 provider-policy=local-only",
    ),
    op(
        b"voice-profile",
        b"read",
        b"Read explicit voice preference",
        OperationId::VoiceProfileRead,
        ValueType::Unit,
        ValueType::Profile,
        Some(ArgumentType::UserRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"voice-profile read user:1",
    ),
    op(
        b"voice-profile",
        b"update",
        b"Update voice without implicit microphone authority",
        OperationId::VoiceProfileUpdate,
        ValueType::Unit,
        ValueType::Profile,
        Some(ArgumentType::UserRef),
        VOICE_PROFILE_ARGS,
        2,
        SideEffectClass::SecurityChange,
        b"voice-profile update user:1 enabled=true activation=push-to-talk",
    ),
    op(
        b"settings",
        b"read",
        b"Read typed scoped preferences",
        OperationId::SettingsRead,
        ValueType::Unit,
        ValueType::Settings,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"settings read",
    ),
    op(
        b"settings",
        b"update",
        b"Update an allowed typed preference",
        OperationId::SettingsUpdate,
        ValueType::Unit,
        ValueType::Settings,
        None,
        SETTING_ARGS,
        2,
        SideEffectClass::ReversibleChange,
        b"settings update name=appearance.theme value=cosmic-dark",
    ),
    op(
        b"appearance",
        b"read",
        b"Read effective machine, user, and session appearance",
        OperationId::AppearanceRead,
        ValueType::Unit,
        ValueType::Settings,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"appearance read",
    ),
    op(
        b"appearance",
        b"set-skin",
        b"Transactionally activate a validated skin package",
        OperationId::AppearanceSetSkin,
        ValueType::Unit,
        ValueType::Settings,
        None,
        SETTING_ARGS,
        2,
        SideEffectClass::ReversibleChange,
        b"appearance set-skin name=skin value=infinity.default.dark",
    ),
    op(
        b"appearance",
        b"set-scale",
        b"Set a supported logical display scale",
        OperationId::AppearanceSetScale,
        ValueType::Unit,
        ValueType::Settings,
        None,
        SETTING_ARGS,
        2,
        SideEffectClass::ReversibleChange,
        b"appearance set-scale name=scale value=1.25",
    ),
    op(
        b"skin",
        b"list",
        b"List installed native InfinityUI skin packages",
        OperationId::SkinList,
        ValueType::Unit,
        ValueType::SkinSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"skin list",
    ),
    op(
        b"skin",
        b"inspect",
        b"Inspect one skin package and inheritance metadata",
        OperationId::SkinInspect,
        ValueType::Unit,
        ValueType::SkinSet,
        None,
        NAME_ARG,
        1,
        SideEffectClass::Query,
        b"skin inspect name=infinity.default.dark",
    ),
    op(
        b"ui",
        b"tree",
        b"Inspect the retained semantic element tree",
        OperationId::UiInspectTree,
        ValueType::Unit,
        ValueType::UiTree,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"ui tree",
    ),
    op(
        b"ui",
        b"focus",
        b"Inspect keyboard focus and active modal boundaries",
        OperationId::UiInspectFocus,
        ValueType::Unit,
        ValueType::UiTree,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"ui focus",
    ),
    op(
        b"window",
        b"list",
        b"List owned windows and surface classes",
        OperationId::WindowList,
        ValueType::Unit,
        ValueType::WindowSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"window list",
    ),
    op(
        b"clipboard",
        b"read",
        b"Read authorized typed clipboard content",
        OperationId::ClipboardRead,
        ValueType::Unit,
        ValueType::ClipboardData,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"clipboard read",
    ),
    op(
        b"clipboard",
        b"write",
        b"Write bounded typed clipboard content",
        OperationId::ClipboardWrite,
        ValueType::Unit,
        ValueType::ClipboardData,
        None,
        SETTING_ARGS,
        2,
        SideEffectClass::ReversibleChange,
        b"clipboard write name=text value=hello",
    ),
    op(
        b"network",
        b"status",
        b"Inspect authoritative connectivity state",
        OperationId::NetworkStatus,
        ValueType::Unit,
        ValueType::NetworkStatus,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network status",
    ),
    op(
        b"network",
        b"interface-list",
        b"List typed network interfaces",
        OperationId::NetworkInterfaceList,
        ValueType::Unit,
        ValueType::NetworkInterfaceSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network interface-list",
    ),
    op(
        b"network",
        b"interface-read",
        b"Inspect one network interface",
        OperationId::NetworkInterfaceInspect,
        ValueType::Unit,
        ValueType::NetworkInterfaceSet,
        Some(ArgumentType::NetworkRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network interface-read interface:1",
    ),
    op(
        b"network",
        b"address-list",
        b"List interface-scoped addresses",
        OperationId::NetworkAddressList,
        ValueType::Unit,
        ValueType::NetworkAddressSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network address-list",
    ),
    op(
        b"network",
        b"address-create",
        b"Configure a typed network address",
        OperationId::NetworkAddressConfigure,
        ValueType::Unit,
        ValueType::NetworkAddressSet,
        None,
        NETWORK_CONFIG_ARGS,
        3,
        SideEffectClass::SecurityChange,
        b"network address-create address=10.42.0.2 prefix=16 interface=interface:2",
    ),
    op(
        b"network",
        b"address-delete",
        b"Remove one configured address",
        OperationId::NetworkAddressRemove,
        ValueType::Unit,
        ValueType::NetworkAddressSet,
        Some(ArgumentType::NetworkRef),
        NO_ARGS,
        3,
        SideEffectClass::SecurityChange,
        b"network address-delete address:4",
    ),
    op(
        b"network",
        b"route-list",
        b"List deterministic route state",
        OperationId::NetworkRouteList,
        ValueType::Unit,
        ValueType::NetworkRouteSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network route-list",
    ),
    op(
        b"network",
        b"route-create",
        b"Create a typed route",
        OperationId::NetworkRouteAdd,
        ValueType::Unit,
        ValueType::NetworkRouteSet,
        None,
        NETWORK_CONFIG_ARGS,
        3,
        SideEffectClass::SecurityChange,
        b"network route-create destination=10.42.0.0 prefix=16 interface=interface:2 metric=100",
    ),
    op(
        b"network",
        b"route-delete",
        b"Remove one route",
        OperationId::NetworkRouteRemove,
        ValueType::Unit,
        ValueType::NetworkRouteSet,
        Some(ArgumentType::NetworkRef),
        NO_ARGS,
        3,
        SideEffectClass::SecurityChange,
        b"network route-delete route:2",
    ),
    op(
        b"network",
        b"resolve",
        b"Resolve a name under deadline and policy",
        OperationId::NetworkResolve,
        ValueType::Unit,
        ValueType::NetworkAddressSet,
        Some(ArgumentType::NetworkRef),
        NO_ARGS,
        1,
        SideEffectClass::ExternalEffect,
        b"network resolve name:infinity.local",
    ),
    op(
        b"network",
        b"connection-list",
        b"List authorized typed connections",
        OperationId::NetworkConnectionList,
        ValueType::Unit,
        ValueType::NetworkConnectionSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network connection-list",
    ),
    op(
        b"network",
        b"connection-read",
        b"Inspect an authorized connection",
        OperationId::NetworkConnectionInspect,
        ValueType::Unit,
        ValueType::NetworkConnectionSet,
        Some(ArgumentType::NetworkRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network connection-read connection:1",
    ),
    op(
        b"network",
        b"policy-list",
        b"List authorized network policy",
        OperationId::NetworkPolicyList,
        ValueType::Unit,
        ValueType::NetworkPolicySet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network policy-list",
    ),
    op(
        b"network",
        b"policy-read",
        b"Inspect one policy rule",
        OperationId::NetworkPolicyInspect,
        ValueType::Unit,
        ValueType::NetworkPolicySet,
        Some(ArgumentType::NetworkRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network policy-read policy:1",
    ),
    op(
        b"network",
        b"policy-create",
        b"Create identity-scoped network authority",
        OperationId::NetworkPolicyCreate,
        ValueType::Unit,
        ValueType::NetworkPolicySet,
        None,
        NETWORK_CONFIG_ARGS,
        4,
        SideEffectClass::SecurityChange,
        b"network policy-create subject=application:browser destination=public duration=persistent",
    ),
    op(
        b"network",
        b"policy-delete",
        b"Delete one network policy rule",
        OperationId::NetworkPolicyDelete,
        ValueType::Unit,
        ValueType::NetworkPolicySet,
        Some(ArgumentType::NetworkRef),
        NO_ARGS,
        4,
        SideEffectClass::SecurityChange,
        b"network policy-delete policy:1",
    ),
    op(
        b"network",
        b"profile-list",
        b"List operational connectivity profiles",
        OperationId::NetworkProfileList,
        ValueType::Unit,
        ValueType::NetworkProfileSet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network profile-list",
    ),
    op(
        b"network",
        b"profile-read",
        b"Inspect one connectivity profile",
        OperationId::NetworkProfileInspect,
        ValueType::Unit,
        ValueType::NetworkProfileSet,
        Some(ArgumentType::NetworkRef),
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network profile-read network-profile:1",
    ),
    op(
        b"network",
        b"profile-activate",
        b"Transactionally activate a connectivity posture",
        OperationId::NetworkProfileActivate,
        ValueType::Unit,
        ValueType::NetworkStatus,
        Some(ArgumentType::NetworkRef),
        NO_ARGS,
        4,
        SideEffectClass::SecurityChange,
        b"network profile-activate network-profile:3",
    ),
    op(
        b"network",
        b"diagnostics",
        b"Inspect observed network counters and pressure",
        OperationId::NetworkDiagnostics,
        ValueType::Unit,
        ValueType::NetworkDiagnostics,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network diagnostics",
    ),
    op(
        b"network",
        b"service-discover",
        b"Discover untrusted local service advertisements",
        OperationId::ServiceDiscoverLocal,
        ValueType::Unit,
        ValueType::ServiceDiscoverySet,
        None,
        NO_ARGS,
        1,
        SideEffectClass::Query,
        b"network service-discover",
    ),
    op(b"node", b"list", b"List signed discovered nodes without granting trust", OperationId::NodeList, ValueType::Unit, ValueType::NodeSet, None, NO_ARGS, 1, SideEffectClass::Query, b"node list"),
    op(b"node", b"capability-grant", b"Review and approve one leased peer operation in scope zero", OperationId::NodeCapabilityGrant, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), NODE_GRANT_ARGS, 9, SideEffectClass::SecurityChange, b"node capability-grant node:<complete-id> name=inspect seconds=600 confirm=true"),
    op(b"node", b"capability-revoke", b"Revoke one peer grant immediately", OperationId::NodeCapabilityRevoke, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"node capability-revoke 1"),
    op(b"node", b"capability-list", b"Inspect bounded peer grants", OperationId::NodeCapabilityList, ValueType::Unit, ValueType::NodePolicy, None, NO_ARGS, 1, SideEffectClass::Query, b"node capability-list"),
    op(b"node", b"remote-read", b"Queue authenticated inspection of this node at a peer", OperationId::NodeInspect, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NODE_REMOTE_ARGS, 1, SideEffectClass::Query, b"node remote-read node:<complete-id> grant=1"),
    op(b"node", b"remote-domain", b"Queue authenticated shared-domain inspection at a peer", OperationId::NodeDomainInspect, ValueType::Unit, ValueType::MeshDomainSet, Some(ArgumentType::NodeRef), NODE_REMOTE_ARGS, 1, SideEffectClass::Query, b"node remote-domain node:<complete-id> grant=1"),
    op(b"node", b"remote-result", b"Collect only this operator session's pending remote result", OperationId::NodeDiagnostics, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"node remote-result 1"),
    op(b"node", b"remote-policy-update", b"Queue a separately authorized mutation of this node's peer policy", OperationId::NodePolicyUpdate, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), NODE_REMOTE_POLICY_ARGS, 9, SideEffectClass::SecurityChange, b"node remote-policy-update node:<complete-id> grant=1 name=object value=deny"),
    op(b"node", b"link-configure", b"Approve exact discovery endpoints without granting peer trust", OperationId::NodeLinkConfigure, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NODE_LINK_ARGS, 9, SideEffectClass::SecurityChange, b"node link-configure 1 local=10.42.0.1 remote=10.42.0.2 local-port=49152 remote-port=49152"),
    op(b"node", b"link-list", b"Inspect persistent approved discovery endpoints", OperationId::NodeLinkList, ValueType::Unit, ValueType::NodeSet, None, NO_ARGS, 1, SideEffectClass::Query, b"node link-list"),
    op(b"node", b"link-remove", b"Withdraw discovery endpoint authority", OperationId::NodeLinkRemove, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"node link-remove 1"),
    op(b"node", b"session-open", b"Establish fresh traffic keys with a previously confirmed peer", OperationId::NodeSessionOpen, ValueType::Unit, ValueType::NodeSessionSet, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"node session-open node:<complete-id>"),
    op(b"node", b"read", b"Inspect one stable cryptographic node identity", OperationId::NodeInspect, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"node read node:7f84"),
    op(b"node", b"discover-status", b"Inspect bounded signed discovery state", OperationId::NodeDiscoverStatus, ValueType::Unit, ValueType::NodeSet, None, NO_ARGS, 1, SideEffectClass::Query, b"node discover-status"),
    op(b"node", b"pair", b"Begin an explicit verified node pairing", OperationId::NodePairBegin, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"node pair node:7f84"),
    op(b"node", b"pair-confirm", b"Confirm the displayed fingerprint and manually entered short code", OperationId::NodePairConfirm, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), PAIR_CONFIRM_ARGS, 9, SideEffectClass::SecurityChange, b"node pair-confirm pairing:1 code=847291"),
    op(b"node", b"pair-cancel", b"Cancel pairing without granting authority", OperationId::NodePairCancel, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"node pair-cancel pairing:1"),
    op(b"node", b"trust-read", b"Read the scoped trust relationship", OperationId::NodeTrustRead, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"node trust-read node:7f84"),
    op(b"node", b"trust-update", b"Update explicit node trust without ambient authority", OperationId::NodeTrustUpdate, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), SETTING_ARGS, 9, SideEffectClass::SecurityChange, b"node trust-update node:7f84 name=state value=restricted"),
    op(b"node", b"trust-revoke", b"Revoke trust, sessions, and remote grants immediately", OperationId::NodeRevokeTrust, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"node trust-revoke node:7f84"),
    op(b"node", b"block", b"Block one node and close its authority", OperationId::NodeBlock, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"node block node:7f84"),
    op(b"node", b"unblock", b"Return one blocked node to untrusted state", OperationId::NodeUnblock, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"node unblock node:7f84"),
    op(b"node", b"session-list", b"List mutually authenticated node sessions", OperationId::NodeSessionList, ValueType::Unit, ValueType::NodeSessionSet, None, NO_ARGS, 1, SideEffectClass::Query, b"node session-list"),
    op(b"node", b"session-read", b"Inspect one secure node session", OperationId::NodeSessionInspect, ValueType::Unit, ValueType::NodeSessionSet, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"node session-read session:1"),
    op(b"node", b"session-close", b"Close and zeroize one node session", OperationId::NodeSessionClose, ValueType::Unit, ValueType::NodeSessionSet, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"node session-close session:1"),
    op(b"node", b"policy-read", b"Read per-node access policy", OperationId::NodePolicyRead, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"node policy-read node:7f84"),
    op(b"node", b"policy-update", b"Commit a scoped per-node access policy", OperationId::NodePolicyUpdate, ValueType::Unit, ValueType::NodePolicy, Some(ArgumentType::NodeRef), SETTING_ARGS, 9, SideEffectClass::SecurityChange, b"node policy-update node:7f84 name=object-access value=session"),
    op(b"node", b"health", b"Inspect authenticated liveness and compatibility", OperationId::NodeHealth, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"node health node:7f84"),
    op(b"node", b"diagnostics", b"Inspect bounded node protocol diagnostics", OperationId::NodeDiagnostics, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"node diagnostics node:7f84"),
    op(b"node", b"audit-list", b"List structured node security records", OperationId::NodeAuditList, ValueType::Unit, ValueType::NodeAuditSet, None, NO_ARGS, 1, SideEffectClass::Query, b"node audit-list"),
    op(b"node", b"audit-read", b"Inspect one structured node security record", OperationId::NodeAuditInspect, ValueType::Unit, ValueType::NodeAuditSet, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"node audit-read record:1"),
    op(b"mesh", b"list", b"List explicit mesh resource domains", OperationId::NodeDomainList, ValueType::Unit, ValueType::MeshDomainSet, None, NO_ARGS, 1, SideEffectClass::Query, b"mesh list"),
    op(b"mesh", b"read", b"Inspect one mesh domain", OperationId::NodeDomainInspect, ValueType::Unit, ValueType::MeshDomainSet, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"mesh read mesh:1"),
    op(b"mesh", b"members", b"List explicit members of a mesh domain", OperationId::MeshMemberList, ValueType::Unit, ValueType::NodeSet, Some(ArgumentType::NodeRef), NO_ARGS, 1, SideEffectClass::Query, b"mesh members mesh:1"),
    op(b"mesh", b"join", b"Join an authorized mesh domain without changing trust", OperationId::NodeJoin, ValueType::Unit, ValueType::MeshDomainSet, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"mesh join mesh:1"),
    op(b"mesh", b"leave", b"Leave a mesh domain while preserving independent trust", OperationId::NodeLeave, ValueType::Unit, ValueType::MeshDomainSet, Some(ArgumentType::NodeRef), NO_ARGS, 9, SideEffectClass::SecurityChange, b"mesh leave mesh:1"),
];

// ------------------------=
// FUNC: op
// DESC: Defines a static typed Console operation schema.
// ------------------=
const fn op(
    domain: &'static [u8],
    action: &'static [u8],
    description: &'static [u8],
    operation: OperationId,
    input: ValueType,
    output: ValueType,
    target: Option<ArgumentType>,
    arguments: &'static [ArgumentSchema],
    capability: u64,
    side_effect: SideEffectClass,
    example: &'static [u8],
) -> OperationSchema {
    OperationSchema {
        domain,
        action,
        description,
        operation,
        input,
        output,
        target,
        arguments,
        capability,
        side_effect,
        example,
    }
}

// ------------------------=
// FUNC: domain
// DESC: Finds a registered Console domain by canonical name.
// ------------------=
pub fn domain(name: &[u8]) -> Option<&'static DomainSchema> {
    DOMAINS.iter().find(|item| eq_ascii(item.name, name))
}

// ------------------------=
// FUNC: operation
// DESC: Finds a registered operation schema by domain and action.
// ------------------=
pub fn operation(domain_name: &[u8], action: &[u8]) -> Option<&'static OperationSchema> {
    OPERATIONS
        .iter()
        .find(|item| eq_ascii(item.domain, domain_name) && eq_ascii(item.action, action))
}

// ------------------------=
// FUNC: parse
// DESC: Parses deterministic Console input into discovery, help, or a typed operation graph.
// ------------------=
pub fn parse(input: &[u8]) -> Result<ParseOutcome<'_>, ConsoleLanguageError> {
    let mut source = trim(input);
    if source.is_empty() {
        return Err(ConsoleLanguageError::Empty);
    }
    let mut assignment = None;
    if let Some((name, remainder)) = assignment_prefix(source)? {
        assignment = Some(name);
        source = remainder;
    }
    let mut plan_only = false;
    if let Some(rest) = word_prefix(source, b"plan") {
        plan_only = true;
        source = trim(rest);
    }
    if eq_ascii(source, b"help") {
        return Ok(ParseOutcome::Help(None, None));
    }
    if let Some(rest) = word_prefix(source, b"help") {
        let (first, remaining) = next_token(trim(rest))?;
        let selected_domain = domain(first).ok_or(ConsoleLanguageError::UnknownDomain)?;
        let remaining = trim(remaining);
        if remaining.is_empty() {
            return Ok(ParseOutcome::Help(Some(selected_domain), None));
        }
        let (action, tail) = next_token(remaining)?;
        if !trim(tail).is_empty() {
            return Err(ConsoleLanguageError::InvalidArgument);
        }
        let selected_operation =
            operation(first, action).ok_or(ConsoleLanguageError::UnknownOperation)?;
        return Ok(ParseOutcome::Help(
            Some(selected_domain),
            Some(selected_operation),
        ));
    }
    if !source.windows(2).any(|window| window == b"|>") {
        let (first, remainder) = next_token(source)?;
        if let Some(selected_domain) = domain(first) {
            let remainder = trim(remainder);
            if remainder.is_empty() {
                return Ok(ParseOutcome::DomainDiscovery(selected_domain));
            }
            let (action, tail) = next_token(remainder)?;
            if let Some(selected_operation) = operation(first, action) {
                if trim(tail).is_empty()
                    && (selected_operation.target.is_some()
                        || selected_operation
                            .arguments
                            .iter()
                            .any(|item| item.required))
                {
                    return Ok(ParseOutcome::OperationDiscovery(selected_operation));
                }
            }
        }
    }
    let mut graph = OperationGraph {
        nodes: [None; MAX_STAGES],
        node_count: 0,
        plan_only,
        assignment,
        result_type: ValueType::Unit,
        maximum_effect: SideEffectClass::Query,
    };
    let mut remainder = source;
    loop {
        if graph.node_count as usize == MAX_STAGES {
            return Err(ConsoleLanguageError::TooManyStages);
        }
        let (stage, tail) = split_stage(remainder)?;
        let node = parse_stage(stage)?;
        if graph.node_count > 0 && !accepts_input(node.schema, graph.result_type, false) {
            return Err(ConsoleLanguageError::TypeMismatch);
        }
        if graph.node_count == 0 && !accepts_input(node.schema, ValueType::Unit, true) {
            return Err(ConsoleLanguageError::TypeMismatch);
        }
        graph.result_type = if plan_only {
            ValueType::OperationPlan
        } else {
            node.schema.output
        };
        graph.maximum_effect = stronger(graph.maximum_effect, node.schema.side_effect);
        graph.nodes[graph.node_count as usize] = Some(node);
        graph.node_count += 1;
        let Some(next) = tail else {
            break;
        };
        remainder = next;
        if plan_only {
            graph.result_type = node.schema.output;
        }
    }
    if plan_only {
        graph.result_type = ValueType::OperationPlan;
    }
    Ok(ParseOutcome::Graph(graph))
}

// ------------------------=
// FUNC: parse_stage
// DESC: Validates one command stage against its registered operation schema.
// ------------------=
fn parse_stage(source: &[u8]) -> Result<OperationNode<'_>, ConsoleLanguageError> {
    let (domain_name, rest) = next_token(trim(source))?;
    let selected_domain = domain(domain_name).ok_or(ConsoleLanguageError::UnknownDomain)?;
    let rest = trim(rest);
    if rest.is_empty() {
        return Err(ConsoleLanguageError::MissingArgument);
    }
    let (action, mut rest) = next_token(rest)?;
    let schema =
        operation(selected_domain.name, action).ok_or(ConsoleLanguageError::UnknownOperation)?;
    let mut node = OperationNode {
        schema,
        target: None,
        arguments: [None; MAX_ARGUMENTS],
        argument_count: 0,
    };
    while !trim(rest).is_empty() {
        let (token, tail) = next_token(trim(rest))?;
        rest = tail;
        if let Some(at) = token.iter().position(|byte| *byte == b'=') {
            let name = &token[..at];
            let value = &token[at + 1..];
            if name.is_empty() || value.is_empty() {
                return Err(ConsoleLanguageError::InvalidArgument);
            }
            let argument_schema = schema
                .arguments
                .iter()
                .find(|item| eq_ascii(item.name, name))
                .ok_or(ConsoleLanguageError::InvalidArgument)?;
            if node
                .arguments
                .iter()
                .flatten()
                .any(|item| eq_ascii(item.name, name))
            {
                return Err(ConsoleLanguageError::DuplicateArgument);
            }
            validate_value(argument_schema.value_type, value)?;
            if node.argument_count as usize == MAX_ARGUMENTS {
                return Err(ConsoleLanguageError::TooManyArguments);
            }
            node.arguments[node.argument_count as usize] = Some(ParsedArgument {
                name: argument_schema.name,
                value: unquote(value),
                value_type: argument_schema.value_type,
            });
            node.argument_count += 1;
        } else if node.target.is_none() {
            let target_type = schema.target.ok_or(ConsoleLanguageError::InvalidArgument)?;
            validate_value(target_type, token)?;
            node.target = Some(parse_reference(unquote(token), target_type)?);
        } else {
            return Err(ConsoleLanguageError::InvalidArgument);
        }
    }
    if schema.target.is_some() && node.target.is_none() && schema.input == ValueType::Unit {
        return Err(ConsoleLanguageError::MissingArgument);
    }
    if schema.arguments.iter().any(|required| {
        required.required
            && !node
                .arguments
                .iter()
                .flatten()
                .any(|arg| eq_ascii(arg.name, required.name))
    }) {
        return Err(ConsoleLanguageError::MissingArgument);
    }
    Ok(node)
}

// ------------------------=
// FUNC: parse_reference
// DESC: Converts a human reference token into a typed non-authoritative reference.
// ------------------=
pub fn parse_reference(
    value: &[u8],
    expected: ArgumentType,
) -> Result<HumanReference<'_>, ConsoleLanguageError> {
    let (kind, raw) = if value.starts_with(b"obj:") {
        (ReferenceKind::Object, &value[4..])
    } else if value.starts_with(b"project:") {
        (ReferenceKind::Project, &value[8..])
    } else if value.starts_with(b"collection:") {
        (ReferenceKind::Collection, &value[11..])
    } else if value.starts_with(b"device:") {
        (ReferenceKind::Device, &value[7..])
    } else if value.starts_with(b"service:") {
        (ReferenceKind::Service, &value[8..])
    } else if value.starts_with(b"user:") {
        (ReferenceKind::User, &value[5..])
    } else if value.starts_with(b"session:") {
        (ReferenceKind::Session, &value[8..])
    } else if value.starts_with(b"machine:") {
        (ReferenceKind::Machine, &value[8..])
    } else if value.starts_with(b"interface:") {
        (ReferenceKind::Network, &value[10..])
    } else if value.starts_with(b"address:") {
        (ReferenceKind::Network, &value[8..])
    } else if value.starts_with(b"route:") {
        (ReferenceKind::Network, &value[6..])
    } else if value.starts_with(b"connection:") {
        (ReferenceKind::Network, &value[11..])
    } else if value.starts_with(b"policy:") {
        (ReferenceKind::Network, &value[7..])
    } else if value.starts_with(b"network-profile:") {
        (ReferenceKind::Network, &value[16..])
    } else if value.starts_with(b"name:") {
        (ReferenceKind::Network, &value[5..])
    } else if value.starts_with(b"node:") {
        (ReferenceKind::Node, &value[5..])
    } else if value.starts_with(b"pairing:") {
        (ReferenceKind::Node, &value[8..])
    } else if value.starts_with(b"mesh:") {
        (ReferenceKind::Node, &value[5..])
    } else if value.starts_with(b"record:") {
        (ReferenceKind::Node, &value[7..])
    } else if value.starts_with(b"@") {
        (ReferenceKind::Session, &value[1..])
    } else if value.starts_with(b"/") {
        (ReferenceKind::Namespace, value)
    } else if matches!(expected, ArgumentType::ProjectRef) {
        (ReferenceKind::Project, value)
    } else if matches!(expected, ArgumentType::CollectionRef) {
        (ReferenceKind::Collection, value)
    } else if matches!(expected, ArgumentType::UserRef) {
        (ReferenceKind::User, value)
    } else if matches!(expected, ArgumentType::SessionRef) {
        (ReferenceKind::Session, value)
    } else if matches!(expected, ArgumentType::MachineRef) {
        (ReferenceKind::Machine, value)
    } else if matches!(expected, ArgumentType::NetworkRef) {
        (ReferenceKind::Network, value)
    } else if matches!(expected, ArgumentType::NodeRef) {
        (ReferenceKind::Node, value)
    } else {
        return Err(ConsoleLanguageError::InvalidArgumentType);
    };
    if raw.is_empty() {
        return Err(ConsoleLanguageError::InvalidArgumentType);
    }
    let valid = match expected {
        ArgumentType::ObjectRef => matches!(
            kind,
            ReferenceKind::Object | ReferenceKind::Session | ReferenceKind::Namespace
        ),
        ArgumentType::ProjectRef => matches!(
            kind,
            ReferenceKind::Project | ReferenceKind::Object | ReferenceKind::Session
        ),
        ArgumentType::CollectionRef => matches!(
            kind,
            ReferenceKind::Collection | ReferenceKind::Object | ReferenceKind::Session
        ),
        ArgumentType::DeviceRef => matches!(kind, ReferenceKind::Device | ReferenceKind::Session),
        ArgumentType::ServiceRef => matches!(kind, ReferenceKind::Service | ReferenceKind::Session),
        ArgumentType::NamespacePath => kind == ReferenceKind::Namespace,
        ArgumentType::UserRef => kind == ReferenceKind::User,
        ArgumentType::SessionRef => kind == ReferenceKind::Session,
        ArgumentType::MachineRef => kind == ReferenceKind::Machine,
        ArgumentType::NetworkRef => kind == ReferenceKind::Network,
        ArgumentType::NodeRef => kind == ReferenceKind::Node,
        _ => true,
    };
    if !valid {
        return Err(ConsoleLanguageError::InvalidArgumentType);
    }
    Ok(HumanReference { kind, value: raw })
}

// ------------------------=
// FUNC: validate_value
// DESC: Checks a command value against its schema without converting it to rendered text.
// ------------------=
fn validate_value(kind: ArgumentType, value: &[u8]) -> Result<(), ConsoleLanguageError> {
    let value = unquote(value);
    if value.is_empty() {
        return Err(ConsoleLanguageError::InvalidArgumentType);
    }
    match kind {
        ArgumentType::Boolean if value != b"true" && value != b"false" => {
            Err(ConsoleLanguageError::InvalidArgumentType)
        }
        ArgumentType::Temporal if !temporal_valid(value) => {
            Err(ConsoleLanguageError::InvalidArgumentType)
        }
        ArgumentType::ObjectRef
        | ArgumentType::ProjectRef
        | ArgumentType::CollectionRef
        | ArgumentType::DeviceRef
        | ArgumentType::ServiceRef
        | ArgumentType::UserRef
        | ArgumentType::SessionRef
        | ArgumentType::MachineRef
        | ArgumentType::NetworkRef
        | ArgumentType::NodeRef
        | ArgumentType::NamespacePath => parse_reference(value, kind).map(|_| ()),
        _ => Ok(()),
    }
}

// ------------------------=
// FUNC: temporal_valid
// DESC: Restricts deterministic temporal expressions to the documented bounded grammar.
// ------------------=
fn temporal_valid(value: &[u8]) -> bool {
    matches!(
        value,
        b"today" | b"yesterday" | b"this-week" | b"last-week" | b"last-30-days"
    ) || date_expression(value, b"before=")
        || date_expression(value, b"after=")
}

// ------------------------=
// FUNC: date_expression
// DESC: Validates a bounded ISO calendar-date expression.
// ------------------=
fn date_expression(value: &[u8], prefix: &[u8]) -> bool {
    if !value.starts_with(prefix) {
        return false;
    }
    let date = &value[prefix.len()..];
    date.len() == 10
        && date[4] == b'-'
        && date[7] == b'-'
        && date
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
}

// ------------------------=
// FUNC: assignment_prefix
// DESC: Extracts an optional typed-variable assignment before a command graph.
// ------------------=
fn assignment_prefix(source: &[u8]) -> Result<Option<(&[u8], &[u8])>, ConsoleLanguageError> {
    let Some(equal) = source.iter().position(|byte| *byte == b'=') else {
        return Ok(None);
    };
    let before = trim(&source[..equal]);
    if before.contains(&b' ') || before.is_empty() {
        return Ok(None);
    }
    let after = trim(&source[equal + 1..]);
    if !before
        .iter()
        .enumerate()
        .all(|(i, b)| b.is_ascii_alphabetic() || *b == b'_' || (i > 0 && b.is_ascii_digit()))
        || after.is_empty()
    {
        return Err(ConsoleLanguageError::InvalidAssignment);
    }
    Ok(Some((before, after)))
}

// ------------------------=
// FUNC: split_stage
// DESC: Splits one top-level typed-composition stage while honoring quoted values.
// ------------------=
fn split_stage(source: &[u8]) -> Result<(&[u8], Option<&[u8]>), ConsoleLanguageError> {
    let mut quote = false;
    let mut escaped = false;
    for index in 0..source.len() {
        let byte = source[index];
        if escaped {
            escaped = false;
            continue;
        }
        if quote && byte == b'\\' {
            escaped = true;
            continue;
        }
        if byte == b'"' {
            quote = !quote;
            continue;
        }
        if !quote && byte == b'|' && source.get(index + 1) == Some(&b'>') {
            let left = trim(&source[..index]);
            let right = trim(&source[index + 2..]);
            if left.is_empty() || right.is_empty() {
                return Err(ConsoleLanguageError::InvalidArgument);
            }
            return Ok((left, Some(right)));
        }
    }
    if quote {
        Err(ConsoleLanguageError::UnterminatedQuote)
    } else {
        Ok((trim(source), None))
    }
}

// ------------------------=
// FUNC: next_token
// DESC: Reads one whitespace-delimited token with deliberate quoted-value escaping.
// ------------------=
fn next_token(source: &[u8]) -> Result<(&[u8], &[u8]), ConsoleLanguageError> {
    if source.is_empty() {
        return Err(ConsoleLanguageError::MissingArgument);
    }
    let mut quote = false;
    let mut escaped = false;
    for index in 0..source.len() {
        let byte = source[index];
        if escaped {
            escaped = false;
            continue;
        }
        if quote && byte == b'\\' {
            escaped = true;
            continue;
        }
        if byte == b'"' {
            quote = !quote;
            continue;
        }
        if !quote && byte.is_ascii_whitespace() {
            return Ok((&source[..index], &source[index..]));
        }
    }
    if quote {
        Err(ConsoleLanguageError::UnterminatedQuote)
    } else {
        Ok((source, &[]))
    }
}

// ------------------------=
// FUNC: word_prefix
// DESC: Matches a whole leading grammar word case-insensitively.
// ------------------=
fn word_prefix<'a>(source: &'a [u8], word: &[u8]) -> Option<&'a [u8]> {
    if source.len() > word.len()
        && eq_ascii(&source[..word.len()], word)
        && source[word.len()].is_ascii_whitespace()
    {
        Some(&source[word.len()..])
    } else {
        None
    }
}

// ------------------------=
// FUNC: unquote
// DESC: Removes surrounding quotes while leaving bytes otherwise unchanged for typed decoding.
// ------------------=
fn unquote(value: &[u8]) -> &[u8] {
    if value.len() >= 2 && value[0] == b'"' && value[value.len() - 1] == b'"' {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

// ------------------------=
// FUNC: trim
// DESC: Removes ASCII grammar whitespace from both ends of a byte slice.
// ------------------=
fn trim(mut value: &[u8]) -> &[u8] {
    while value
        .first()
        .map(|b| b.is_ascii_whitespace())
        .unwrap_or(false)
    {
        value = &value[1..];
    }
    while value
        .last()
        .map(|b| b.is_ascii_whitespace())
        .unwrap_or(false)
    {
        value = &value[..value.len() - 1];
    }
    value
}

// ------------------------=
// FUNC: eq_ascii
// DESC: Compares canonical grammar names without locale-dependent rules.
// ------------------=
fn eq_ascii(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

// ------------------------=
// FUNC: stronger
// DESC: Returns the higher-policy side-effect classification for a graph.
// ------------------=
fn stronger(left: SideEffectClass, right: SideEffectClass) -> SideEffectClass {
    if effect_rank(right) > effect_rank(left) {
        right
    } else {
        left
    }
}

// ------------------------=
// FUNC: accepts_input
// DESC: Validates graph input types, including operations that accept either an ObjectSet or an explicit query/target.
// ------------------=
fn accepts_input(schema: &OperationSchema, produced: ValueType, first: bool) -> bool {
    schema.input == produced
        || (first
            && matches!(
                schema.operation,
                OperationId::ObjectDestroy | OperationId::NamespaceMove
            ))
}

// ------------------------=
// FUNC: effect_rank
// DESC: Maps side effects to conservative policy ordering.
// ------------------=
const fn effect_rank(effect: SideEffectClass) -> u8 {
    match effect {
        SideEffectClass::Query => 0,
        SideEffectClass::ReversibleChange => 1,
        SideEffectClass::DestructiveChange => 2,
        SideEffectClass::SecurityChange => 3,
        SideEffectClass::ExternalEffect => 4,
    }
}
