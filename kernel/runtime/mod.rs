pub mod ai;
pub mod capability;
pub mod console_language;
pub mod event;
pub mod execution;
pub mod font;
pub mod identity;
pub mod iop;
pub mod network;
pub mod object_navigation;
pub mod scheduler;
pub mod service;

use capability::{CapabilityManager, CapabilityType};
use event::{EventClass, EventFabric, RoutingDomain};
#[cfg(not(target_os = "none"))]
use event::{EventFilter, OverflowPolicy};
use execution::ExecutionManager;
#[cfg(not(target_os = "none"))]
use iop::IopMessage;
use iop::{IopRouter, OperationId};
use scheduler::Scheduler;
use service::*;

pub const EVENT_SERVICE_STATE_CHANGED: u32 = 0x10001;
pub const EVENT_INSTALLER_PLAN_CONFIRMED: u32 = 0x80001;
pub const EVENT_INSTALLER_COMPLETED: u32 = 0x80002;
pub const EVENT_AI_PROVIDER_CHANGED: u32 = 0x90001;
pub const EVENT_AI_MODEL_LOADED: u32 = 0x90002;
pub const EVENT_AI_MODEL_UNLOADED: u32 = 0x90003;
pub const EVENT_AI_REMOTE_PROCESSING_REQUESTED: u32 = 0x90004;
pub const EVENT_AI_INFERENCE_STARTED: u32 = 0x90003;
pub const EVENT_AI_INFERENCE_COMPLETED: u32 = 0x90004;
pub const EVENT_AI_INFERENCE_FAILED: u32 = 0x90005;
pub const EVENT_VOICE_LISTENING_STARTED: u32 = 0x94001;
pub const EVENT_VOICE_LISTENING_STOPPED: u32 = 0x94002;
pub const EVENT_VOICE_TRANSCRIPT_READY: u32 = 0x94003;
pub const EVENT_AGENT_STARTED: u32 = 0x95001;
pub const EVENT_AGENT_STOPPED: u32 = 0x95002;
pub const EVENT_AGENT_FAILED: u32 = 0x95003;
pub const EVENT_IDENTITY_STATE_CHANGED: u32 = 0x96001;
pub const EVENT_APPEARANCE_CHANGED: u32 = 0x97001;
pub const EVENT_WINDOW_CREATED: u32 = 0x98001;
pub const EVENT_WINDOW_DESTROYED: u32 = 0x98002;
pub const EVENT_WINDOW_FOCUSED: u32 = 0x98003;
pub const EVENT_WINDOW_STATE_CHANGED: u32 = 0x98004;
pub const EVENT_DISPLAY_CONFIGURATION_CHANGED: u32 = 0x98005;
pub const EVENT_WINDOW_MOVED: u32 = 0x98006;
pub const EVENT_WINDOW_RESIZED: u32 = 0x98007;
pub const EVENT_SURFACE_COMMITTED: u32 = 0x98008;
pub const EVENT_COMPOSITOR_DEGRADED: u32 = 0x98009;
pub const EVENT_COMPOSITOR_RECOVERED: u32 = 0x9800a;
pub const EVENT_SECURE_INPUT_STARTED: u32 = 0x9800b;
pub const EVENT_SECURE_INPUT_STOPPED: u32 = 0x9800c;
pub const EVENT_NETWORK_INTERFACE_STATE_CHANGED: u32 = 0x99001;
pub const EVENT_NETWORK_ADDRESS_CHANGED: u32 = 0x99002;
pub const EVENT_NETWORK_ROUTE_CHANGED: u32 = 0x99003;
pub const EVENT_NETWORK_CONNECTIVITY_CHANGED: u32 = 0x99004;
pub const EVENT_NETWORK_CONNECTION_OPENED: u32 = 0x99005;
pub const EVENT_NETWORK_CONNECTION_CLOSED: u32 = 0x99006;
pub const EVENT_NETWORK_CONNECTION_FAILED: u32 = 0x99007;
pub const EVENT_NETWORK_POLICY_CHANGED: u32 = 0x99008;
pub const EVENT_NETWORK_PROFILE_ACTIVATED: u32 = 0x99009;
pub const EVENT_NETWORK_PROFILE_CHANGED: u32 = 0x9900a;
pub const EVENT_NETWORK_RESOLVER_STATE_CHANGED: u32 = 0x9900b;
pub const EVENT_NETWORK_SERVICE_DISCOVERED: u32 = 0x9900c;
pub const EVENT_NETWORK_SERVICE_LOST: u32 = 0x9900d;
pub const EVENT_NETWORK_DEGRADED: u32 = 0x9900e;
pub const EVENT_NETWORK_RECOVERED: u32 = 0x9900f;
pub const EVENT_SHELL_PROFILE_CREATED: u32 = 0x9a001;
pub const EVENT_SHELL_PROFILE_UPDATED: u32 = 0x9a002;
pub const EVENT_SHELL_PROFILE_ENABLED: u32 = 0x9a003;
pub const EVENT_SHELL_PROFILE_DISABLED: u32 = 0x9a004;
pub const EVENT_SHELL_PROFILE_DELETED: u32 = 0x9a005;
pub const EVENT_SHELL_ALIAS_DELETED: u32 = 0x9a006;
pub const EVENT_SHELL_ACTIVE_SET_CHANGED: u32 = 0x9a007;
pub const EVENT_NAMESPACE_REFERENCE_CREATED: u32 = 0x9b001;
pub const EVENT_NAMESPACE_REFERENCE_REMOVED: u32 = 0x9b002;
pub const EVENT_NAMESPACE_REFERENCE_MOVED: u32 = 0x9b003;
pub const EVENT_OBJECT_CREATED: u32 = 0x9c001;
pub const EVENT_OBJECT_DESTROYED: u32 = 0x9c002;
pub const EVENT_TRASH_ITEM_ADDED: u32 = 0x9d001;
pub const EVENT_TRASH_ITEM_RESTORED: u32 = 0x9d002;
pub const EVENT_TRASH_ITEM_DESTROYED: u32 = 0x9d003;

const NETWORK_EVENT_TYPES: [u32; 15] = [
    EVENT_NETWORK_INTERFACE_STATE_CHANGED, EVENT_NETWORK_ADDRESS_CHANGED,
    EVENT_NETWORK_ROUTE_CHANGED, EVENT_NETWORK_CONNECTIVITY_CHANGED,
    EVENT_NETWORK_CONNECTION_OPENED, EVENT_NETWORK_CONNECTION_CLOSED,
    EVENT_NETWORK_CONNECTION_FAILED, EVENT_NETWORK_POLICY_CHANGED,
    EVENT_NETWORK_PROFILE_ACTIVATED, EVENT_NETWORK_PROFILE_CHANGED,
    EVENT_NETWORK_RESOLVER_STATE_CHANGED, EVENT_NETWORK_SERVICE_DISCOVERED,
    EVENT_NETWORK_SERVICE_LOST, EVENT_NETWORK_DEGRADED, EVENT_NETWORK_RECOVERED,
];

const UI_EVENT_TYPES: [u32; 12] = [
    EVENT_WINDOW_CREATED,
    EVENT_WINDOW_DESTROYED,
    EVENT_WINDOW_FOCUSED,
    EVENT_WINDOW_STATE_CHANGED,
    EVENT_DISPLAY_CONFIGURATION_CHANGED,
    EVENT_WINDOW_MOVED,
    EVENT_WINDOW_RESIZED,
    EVENT_SURFACE_COMMITTED,
    EVENT_COMPOSITOR_DEGRADED,
    EVENT_COMPOSITOR_RECOVERED,
    EVENT_SECURE_INPUT_STARTED,
    EVENT_SECURE_INPUT_STOPPED,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiOperationError {
    AccessDenied,
    Surface(crate::ui::surface::SurfaceError),
    Window(crate::ui::window::WindowError),
}
pub struct InfinityRuntime {
    pub execution: ExecutionManager,
    pub scheduler: Scheduler,
    pub capabilities: CapabilityManager,
    pub iop: IopRouter,
    pub events: EventFabric,
    pub services: ServiceManager,
    pub identity: identity::IdentitySystem,
    pub fonts: font::FontCatalog,
    pub ui: crate::ui::InfinityUiRuntime,
    pub network: network::NetworkRuntime,
    pub shell_profiles: Option<object_navigation::ShellProfileService>,
    pub file_navigator: Option<object_navigation::FileNavigatorState>,
    pub live_profile: bool,
    service_event_cap: Option<u64>,
    identity_event_cap: Option<u64>,
    installer_event_caps: [Option<u64>; 2],
    installer_authority: [Option<u64>; 4],
    ai_console_capability: Option<u64>,
    ai_event_capabilities: [Option<u64>; 3],
    ui_event_capabilities: [Option<u64>; UI_EVENT_TYPES.len()],
    network_event_capabilities: [Option<u64>; NETWORK_EVENT_TYPES.len()],
    settings_network_profile_capability: Option<u64>,
    onboarding_network_profile_capability: Option<u64>,
}
impl InfinityRuntime {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new(live_profile: bool) -> Self {
        Self {
            execution: ExecutionManager::new(),
            scheduler: Scheduler::new(),
            capabilities: CapabilityManager::new(),
            iop: IopRouter::new(),
            events: EventFabric::new(),
            services: ServiceManager::new(),
            identity: identity::IdentitySystem::new(),
            fonts: font::FontCatalog::new(),
            ui: crate::ui::InfinityUiRuntime::new(),
            network: network::NetworkRuntime::new(),
            shell_profiles: None,
            file_navigator: None,
            live_profile,
            service_event_cap: None,
            identity_event_cap: None,
            installer_event_caps: [None; 2],
            installer_authority: [None; 4],
            ai_console_capability: None,
            ai_event_capabilities: [None; 3],
            ui_event_capabilities: [None; UI_EVENT_TYPES.len()],
            network_event_capabilities: [None; NETWORK_EVENT_TYPES.len()],
            settings_network_profile_capability: None,
            onboarding_network_profile_capability: None,
        }
    }

    // ------------------------=
    // FUNC: create_surface
    // DESC: Creates an ordinary retained surface only after validating explicit caller authority.
    // ------------------=
    pub fn create_surface(
        &mut self,
        caller: execution::SecurityIdentity,
        context: crate::ui::window::ContextId,
        capability: capability::CapabilityId,
        size: crate::ui::geometry::Size,
        format: crate::ui::surface::PixelFormat,
        security_class: crate::ui::surface::SurfaceSecurityClass,
        now: u64,
    ) -> Result<crate::ui::window::SurfaceId, UiOperationError> {
        self.capabilities
            .validate(
                capability,
                caller,
                CapabilityType::SurfaceCreate,
                SERVICE_WINDOW_SERVER as u64,
                1,
                0,
                now,
            )
            .map_err(|_| UiOperationError::AccessDenied)?;
        self.ui
            .surfaces
            .create(context, size, format, security_class, false)
            .map_err(UiOperationError::Surface)
    }

    // ------------------------=
    // FUNC: publish_surface
    // DESC: Publishes a newer owned surface generation through an independently revocable capability.
    // ------------------=
    pub fn publish_surface(
        &mut self,
        caller: execution::SecurityIdentity,
        context: crate::ui::window::ContextId,
        capability: capability::CapabilityId,
        surface: crate::ui::window::SurfaceId,
        generation: u64,
        now: u64,
    ) -> Result<(), UiOperationError> {
        self.capabilities
            .validate(
                capability,
                caller,
                CapabilityType::SurfacePublish,
                surface.0 as u64,
                1,
                0,
                now,
            )
            .map_err(|_| UiOperationError::AccessDenied)?;
        self.ui
            .surfaces
            .publish(context, surface, generation)
            .map_err(UiOperationError::Surface)?;
        let mut payload = [0u8; 24];
        payload[0] = 1;
        payload[4..8].copy_from_slice(&surface.0.to_le_bytes());
        payload[8..12].copy_from_slice(&context.0.to_le_bytes());
        payload[16..24].copy_from_slice(&generation.to_le_bytes());
        let _ = self.publish_ui_event(
            EVENT_SURFACE_COMMITTED,
            surface.0 as u64,
            &payload,
            140,
            now,
            generation,
            generation,
        );
        Ok(())
    }

    // ------------------------=
    // FUNC: resize_surface
    // DESC: Reallocates owned retained-surface metadata only under a surface-scoped resize capability.
    // ------------------=
    pub fn resize_surface(
        &mut self,
        caller: execution::SecurityIdentity,
        context: crate::ui::window::ContextId,
        capability: capability::CapabilityId,
        surface: crate::ui::window::SurfaceId,
        size: crate::ui::geometry::Size,
        now: u64,
    ) -> Result<crate::ui::surface::SurfaceDescriptor, UiOperationError> {
        self.capabilities
            .validate(
                capability,
                caller,
                CapabilityType::SurfaceResize,
                surface.0 as u64,
                1,
                0,
                now,
            )
            .map_err(|_| UiOperationError::AccessDenied)?;
        self.ui
            .surfaces
            .resize(context, surface, size)
            .map_err(UiOperationError::Surface)
    }

    // ------------------------=
    // FUNC: destroy_surface
    // DESC: Reclaims an owned surface only after scoped authorization and after all windows release it.
    // ------------------=
    pub fn destroy_surface(
        &mut self,
        caller: execution::SecurityIdentity,
        context: crate::ui::window::ContextId,
        capability: capability::CapabilityId,
        surface: crate::ui::window::SurfaceId,
        now: u64,
    ) -> Result<(), UiOperationError> {
        self.capabilities
            .validate(
                capability,
                caller,
                CapabilityType::SurfaceDestroy,
                surface.0 as u64,
                1,
                0,
                now,
            )
            .map_err(|_| UiOperationError::AccessDenied)?;
        if self.ui.windows.uses_surface(surface) {
            return Err(UiOperationError::Surface(
                crate::ui::surface::SurfaceError::InUse,
            ));
        }
        self.ui
            .surfaces
            .destroy(context, surface)
            .map_err(UiOperationError::Surface)
    }

    // ------------------------=
    // FUNC: inspect_surface
    // DESC: Returns pixel-free surface metadata only under an explicitly scoped inspection capability.
    // ------------------=
    pub fn inspect_surface(
        &self,
        caller: execution::SecurityIdentity,
        capability: capability::CapabilityId,
        surface: crate::ui::window::SurfaceId,
        now: u64,
    ) -> Result<crate::ui::surface::SurfaceDescriptor, UiOperationError> {
        self.capabilities
            .validate(
                capability,
                caller,
                CapabilityType::SurfaceInspectMetadata,
                surface.0 as u64,
                1,
                0,
                now,
            )
            .map_err(|_| UiOperationError::AccessDenied)?;
        self.ui
            .surfaces
            .inspect(surface)
            .copied()
            .ok_or(UiOperationError::Surface(
                crate::ui::surface::SurfaceError::Unknown,
            ))
    }

    // ------------------------=
    // FUNC: create_window
    // DESC: Registers an owned window only when the surface owner and explicit window capability agree.
    // ------------------=
    pub fn create_window(
        &mut self,
        caller: execution::SecurityIdentity,
        context: crate::ui::window::ContextId,
        capability: capability::CapabilityId,
        surface: crate::ui::window::SurfaceId,
        bounds: crate::ui::geometry::Rect,
        z_class: crate::ui::window::ZOrderClass,
        now: u64,
    ) -> Result<crate::ui::window::WindowId, UiOperationError> {
        self.capabilities
            .validate(
                capability,
                caller,
                CapabilityType::WindowCreate,
                SERVICE_WINDOW_SERVER as u64,
                1,
                0,
                now,
            )
            .map_err(|_| UiOperationError::AccessDenied)?;
        if self.ui.surfaces.inspect(surface).map(|entry| entry.owner) != Some(context) {
            return Err(UiOperationError::AccessDenied);
        }
        let window = self
            .ui
            .windows
            .create(context, surface, bounds, z_class)
            .map_err(UiOperationError::Window)?;
        self.flush_window_events(now, window.0 as u64, window.0 as u64);
        Ok(window)
    }

    // ------------------------=
    // FUNC: move_window
    // DESC: Moves an owned window only while its scoped management capability remains valid.
    // ------------------=
    pub fn move_window(
        &mut self,
        caller: execution::SecurityIdentity,
        context: crate::ui::window::ContextId,
        capability: capability::CapabilityId,
        window: crate::ui::window::WindowId,
        point: crate::ui::geometry::Point,
        work_area: crate::ui::geometry::Rect,
        now: u64,
    ) -> Result<crate::ui::geometry::Rect, UiOperationError> {
        self.capabilities
            .validate(
                capability,
                caller,
                CapabilityType::WindowManageOwn,
                window.0 as u64,
                1,
                0,
                now,
            )
            .map_err(|_| UiOperationError::AccessDenied)?;
        let moved = self
            .ui
            .windows
            .move_window(context, window, point, work_area)
            .map_err(UiOperationError::Window)?;
        self.flush_window_events(now, window.0 as u64, window.0 as u64);
        Ok(moved)
    }

    // ------------------------=
    // FUNC: inspect_window
    // DESC: Returns semantic window geometry and state without granting access to content pixels.
    // ------------------=
    pub fn inspect_window(
        &self,
        caller: execution::SecurityIdentity,
        capability: capability::CapabilityId,
        window: crate::ui::window::WindowId,
        now: u64,
    ) -> Result<crate::ui::window::Window, UiOperationError> {
        self.capabilities
            .validate(
                capability,
                caller,
                CapabilityType::WindowInspectMetadata,
                window.0 as u64,
                1,
                0,
                now,
            )
            .map_err(|_| UiOperationError::AccessDenied)?;
        self.ui
            .windows
            .inspect(window)
            .copied()
            .ok_or(UiOperationError::Window(
                crate::ui::window::WindowError::Unknown,
            ))
    }

    // ------------------------=
    // FUNC: flush_window_events
    // DESC: Announces committed bounded Window Server transitions through capability-checked typed IEF events.
    // ------------------=
    pub fn flush_window_events(
        &mut self,
        now: u64,
        correlation_id: u64,
        causation_id: u64,
    ) -> usize {
        let mut published = 0;
        while let Some(event) = self.ui.windows.next_event() {
            let type_id = match event.kind {
                crate::ui::window::WindowEventKind::Created => EVENT_WINDOW_CREATED,
                crate::ui::window::WindowEventKind::Destroyed
                | crate::ui::window::WindowEventKind::ContextFailed => EVENT_WINDOW_DESTROYED,
                crate::ui::window::WindowEventKind::Focused => EVENT_WINDOW_FOCUSED,
                crate::ui::window::WindowEventKind::Moved => EVENT_WINDOW_MOVED,
                crate::ui::window::WindowEventKind::Resized => EVENT_WINDOW_RESIZED,
                crate::ui::window::WindowEventKind::StateChanged
                | crate::ui::window::WindowEventKind::CaptureChanged => EVENT_WINDOW_STATE_CHANGED,
            };
            let mut payload = [0u8; crate::ui::window::WindowEvent::ENCODED_BYTES];
            event.encode_v1(&mut payload);
            if self.publish_ui_event(
                type_id,
                event.window.0 as u64,
                &payload,
                180,
                now,
                correlation_id,
                causation_id,
            ) {
                published += 1;
            }
        }
        published
    }

    // ------------------------=
    // FUNC: publish_ui_event
    // DESC: Uses the Window Server's non-ambient event capability to publish one post-commit UI transition.
    // ------------------=
    fn publish_ui_event(
        &mut self,
        type_id: u32,
        scope: u64,
        payload: &[u8],
        priority: u8,
        now: u64,
        correlation_id: u64,
        causation_id: u64,
    ) -> bool {
        let Some(index) = UI_EVENT_TYPES
            .iter()
            .position(|candidate| *candidate == type_id)
        else {
            return false;
        };
        let Some(capability) = self.ui_event_capabilities[index] else {
            return false;
        };
        let Some(source) = self.service_identity(SERVICE_WINDOW_SERVER) else {
            return false;
        };
        self.events
            .publish(
                EventClass::StateChange,
                RoutingDomain::System,
                type_id,
                source,
                scope,
                correlation_id,
                causation_id,
                payload,
                priority,
                now,
                &self.capabilities,
                capability,
            )
            .is_ok()
    }
    // ------------------------=
    // FUNC: define_bootstrap
    // DESC: Implements the define bootstrap operation.
    // ------------------=
    pub fn define_bootstrap(&mut self) -> Result<(), ServiceError> {
        let none = [0; MAX_DEPENDENCIES];
        let noops = [0; MAX_OPERATIONS];
        self.services.define(manifest(
            SERVICE_RUNTIME,
            none,
            0,
            [
                OperationId::RuntimeContexts as u32,
                OperationId::RuntimeResources as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            2,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Critical,
        ))?;
        self.services.define(manifest(
            SERVICE_DEVICE,
            [SERVICE_RUNTIME, 0, 0, 0],
            1,
            noops,
            0,
            RestartPolicy::OnFailure,
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_STORAGE,
            [SERVICE_DEVICE, 0, 0, 0],
            1,
            [
                OperationId::StorageQuery as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            1,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Critical,
        ))?;
        self.services.define(manifest(
            SERVICE_OBJECT,
            [SERVICE_STORAGE, 0, 0, 0],
            1,
            [
                OperationId::ObjectCreate as u32,
                OperationId::ObjectRead as u32,
                OperationId::ObjectUpdate as u32,
                OperationId::ObjectQuery as u32,
                OperationId::ObjectHistory as u32,
                OperationId::ObjectFilter as u32,
                OperationId::ObjectDestroy as u32,
                0,
                0,
                0,
                0,
                0,
            ],
            7,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Critical,
        ))?;
        self.services.define(manifest(
            SERVICE_NAMESPACE,
            [SERVICE_OBJECT, 0, 0, 0],
            1,
            [
                OperationId::NamespaceResolve as u32,
                OperationId::NamespaceMove as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            2,
            RestartPolicy::OnFailure,
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_ORGANIZATION,
            [SERVICE_OBJECT, SERVICE_NAMESPACE, 0, 0],
            2,
            [
                OperationId::ProjectList as u32,
                OperationId::ProjectInspect as u32,
                OperationId::ProjectCreate as u32,
                OperationId::CollectionList as u32,
                OperationId::CollectionInspect as u32,
                OperationId::CollectionCreate as u32,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            6,
            RestartPolicy::OnFailure,
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_EVENT,
            [SERVICE_RUNTIME, 0, 0, 0],
            1,
            [
                OperationId::EventSubscribe as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            1,
            RestartPolicy::OnFailure,
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_CONSOLE,
            [SERVICE_NAMESPACE, SERVICE_ORGANIZATION, SERVICE_EVENT, 0],
            3,
            [
                OperationId::ServiceList as u32,
                OperationId::ServiceInspect as u32,
                OperationId::ServiceRestart as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            3,
            RestartPolicy::OnFailure,
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_LOCAL_ML,
            [SERVICE_RUNTIME, SERVICE_OBJECT, 0, 0],
            2,
            [
                OperationId::ModelList as u32,
                OperationId::ModelInspect as u32,
                OperationId::ModelLoad as u32,
                OperationId::ModelUnload as u32,
                OperationId::ModelCapabilities as u32,
                OperationId::ModelInfer as u32,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            6,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_AI,
            [SERVICE_LOCAL_ML, SERVICE_NAMESPACE, SERVICE_EVENT, 0],
            3,
            [
                OperationId::IntentResolve as u32,
                OperationId::ContextRequest as u32,
                OperationId::ToolInvoke as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            3,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_VOICE,
            [SERVICE_AI, SERVICE_DEVICE, SERVICE_EVENT, 0],
            3,
            [
                OperationId::VoiceSessionStart as u32,
                OperationId::VoiceSessionStop as u32,
                OperationId::SpeechRecognize as u32,
                OperationId::SpeechSynthesize as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            4,
            RestartPolicy::OnFailure,
            Criticality::NonCritical,
        ))?;
        self.services.define(manifest(
            SERVICE_AGENT,
            [SERVICE_AI, SERVICE_EVENT, 0, 0],
            2,
            [
                OperationId::AgentList as u32,
                OperationId::AgentInspect as u32,
                OperationId::AgentRequestTask as u32,
                OperationId::AgentTaskResult as u32,
                OperationId::AgentCancelTask as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            5,
            RestartPolicy::OnFailure,
            Criticality::NonCritical,
        ))?;
        self.services.define(manifest(
            SERVICE_IDENTITY,
            [SERVICE_OBJECT, SERVICE_NAMESPACE, SERVICE_EVENT, 0],
            3,
            [
                OperationId::IdentityCreate as u32,
                OperationId::IdentityRead as u32,
                OperationId::IdentityList as u32,
                OperationId::IdentityUpdate as u32,
                OperationId::IdentityDelete as u32,
                OperationId::MachineRead as u32,
                OperationId::MachineUpdate as u32,
                OperationId::ProfileRead as u32,
                OperationId::ProfileUpdate as u32,
                OperationId::PersonalSpaceRead as u32,
                0,
                0,
            ],
            10,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_AUTHENTICATION,
            [SERVICE_IDENTITY, SERVICE_EVENT, 0, 0],
            2,
            [
                OperationId::CredentialCreate as u32,
                OperationId::CredentialList as u32,
                OperationId::CredentialDelete as u32,
                OperationId::AuthenticationVerify as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            4,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_SESSION,
            [SERVICE_AUTHENTICATION, SERVICE_IDENTITY, SERVICE_EVENT, 0],
            3,
            [
                OperationId::SessionCreate as u32,
                OperationId::SessionRead as u32,
                OperationId::SessionList as u32,
                OperationId::SessionLock as u32,
                OperationId::SessionUnlock as u32,
                OperationId::SessionEnd as u32,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            6,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_SETTINGS,
            [SERVICE_IDENTITY, SERVICE_SESSION, SERVICE_AI, SERVICE_VOICE],
            4,
            [
                OperationId::SettingsRead as u32,
                OperationId::SettingsUpdate as u32,
                OperationId::AiProfileRead as u32,
                OperationId::AiProfileUpdate as u32,
                OperationId::VoiceProfileRead as u32,
                OperationId::VoiceProfileUpdate as u32,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            6,
            RestartPolicy::OnFailure,
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_ONBOARDING,
            [SERVICE_SETTINGS, SERVICE_SESSION, SERVICE_OBJECT, 0],
            3,
            [
                OperationId::OnboardingRead as u32,
                OperationId::OnboardingAdvance as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            2,
            RestartPolicy::OnFailure,
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_SHELL,
            [SERVICE_SESSION, SERVICE_SETTINGS, SERVICE_CONSOLE, 0],
            3,
            [
                OperationId::ShellOpen as u32,
                OperationId::SystemPowerOff as u32,
                OperationId::SystemRestart as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            3,
            RestartPolicy::OnFailure,
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_FONT,
            [SERVICE_OBJECT, SERVICE_SETTINGS, 0, 0],
            2,
            [
                OperationId::FontList as u32,
                OperationId::FontOpen as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            2,
            RestartPolicy::OnFailure,
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_SKIN_REGISTRY,
            [SERVICE_OBJECT, SERVICE_SETTINGS, 0, 0],
            2,
            [
                OperationId::SkinList as u32,
                OperationId::SkinInspect as u32,
                OperationId::SkinValidate as u32,
                OperationId::AppearanceRead as u32,
                OperationId::AppearanceSetSkin as u32,
                OperationId::AppearanceSetScale as u32,
                OperationId::AppearanceSetAccent as u32,
                OperationId::AppearanceSetWallpaper as u32,
                0,
                0,
                0,
                0,
            ],
            8,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_WINDOW_SERVER,
            [SERVICE_RUNTIME, SERVICE_DEVICE, 0, 0],
            2,
            [
                OperationId::WindowList as u32,
                OperationId::SurfaceCreate as u32,
                OperationId::SurfaceDestroy as u32,
                OperationId::SurfacePresent as u32,
                OperationId::WindowCreate as u32,
                OperationId::WindowClose as u32,
                OperationId::WindowMove as u32,
                OperationId::WindowResize as u32,
                OperationId::WindowSetState as u32,
                OperationId::WindowFocus as u32,
                OperationId::WindowCapturePointer as u32,
                OperationId::WindowReleasePointer as u32,
            ],
            12,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_INFINITY_UI,
            [
                SERVICE_WINDOW_SERVER,
                SERVICE_SKIN_REGISTRY,
                SERVICE_FONT,
                SERVICE_SESSION,
            ],
            4,
            [
                OperationId::UiInspectTree as u32,
                OperationId::UiInspectFocus as u32,
                OperationId::UiInspectDamage as u32,
                OperationId::WindowInspect as u32,
                OperationId::SurfaceList as u32,
                OperationId::SurfaceInspect as u32,
                OperationId::SurfaceResize as u32,
                OperationId::SurfaceCommit as u32,
                OperationId::CompositorStatus as u32,
                OperationId::CompositorDiagnostics as u32,
                OperationId::DisplayQuery as u32,
                OperationId::SecureInputStatus as u32,
            ],
            12,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_CLIPBOARD,
            [SERVICE_INFINITY_UI, SERVICE_SESSION, 0, 0],
            2,
            [
                OperationId::ClipboardRead as u32,
                OperationId::ClipboardWrite as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            2,
            RestartPolicy::OnFailure,
            Criticality::NonCritical,
        ))?;
        self.services.define(manifest(
            SERVICE_NETWORK,
            [SERVICE_DEVICE, SERVICE_EVENT, 0, 0],
            2,
            [
                OperationId::NetworkInterfaceList as u32,
                OperationId::NetworkInterfaceInspect as u32,
                OperationId::NetworkInterfaceSetState as u32,
                OperationId::NetworkAddressList as u32,
                OperationId::NetworkAddressConfigure as u32,
                OperationId::NetworkAddressRemove as u32,
                OperationId::NetworkRouteList as u32,
                OperationId::NetworkRouteInspect as u32,
                OperationId::NetworkRouteAdd as u32,
                OperationId::NetworkRouteRemove as u32,
                OperationId::NetworkStatus as u32,
                OperationId::NetworkDiagnostics as u32,
            ],
            12,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_NETWORK_POLICY,
            [SERVICE_NETWORK, SERVICE_EVENT, 0, 0],
            2,
            [
                OperationId::NetworkPolicyList as u32,
                OperationId::NetworkPolicyInspect as u32,
                OperationId::NetworkPolicyCreate as u32,
                OperationId::NetworkPolicyUpdate as u32,
                OperationId::NetworkPolicyDelete as u32,
                OperationId::NetworkProfileList as u32,
                OperationId::NetworkProfileInspect as u32,
                OperationId::NetworkProfileActivate as u32,
                OperationId::NetworkProfileCreate as u32,
                OperationId::NetworkProfileUpdate as u32,
                OperationId::NetworkProfileDelete as u32,
                0,
            ],
            11,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_NETWORK_TRANSPORT,
            [SERVICE_NETWORK, SERVICE_NETWORK_POLICY, SERVICE_EVENT, 0],
            3,
            [
                OperationId::NetworkResolve as u32,
                OperationId::NetworkConnect as u32,
                OperationId::NetworkListen as u32,
                OperationId::NetworkAccept as u32,
                OperationId::NetworkSend as u32,
                OperationId::NetworkReceive as u32,
                OperationId::NetworkClose as u32,
                OperationId::NetworkConnectionInspect as u32,
                OperationId::NetworkConnectionList as u32,
                0,
                0,
                0,
            ],
            9,
            RestartPolicy::BoundedRetry { maximum: 3 },
            Criticality::Important,
        ))?;
        self.services.define(manifest(
            SERVICE_NETWORK_DISCOVERY,
            [SERVICE_NETWORK_TRANSPORT, SERVICE_EVENT, 0, 0],
            2,
            [
                OperationId::ServiceDiscoverLocal as u32,
                OperationId::ServiceAdvertiseLocal as u32,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            2,
            RestartPolicy::OnFailure,
            Criticality::NonCritical,
        ))?;
        if self.live_profile {
            self.services.define(manifest(
                SERVICE_INSTALLER,
                [SERVICE_STORAGE, SERVICE_OBJECT, SERVICE_EVENT, 0],
                3,
                noops,
                0,
                RestartPolicy::Never,
                Criticality::Important,
            ))?;
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: start_all
    // DESC: Initializes start all state.
    // ------------------=
    pub fn start_all(&mut self, now: u64) {
        for _ in 0..MAX_SERVICES {
            self.services.start_ready(&mut self.execution, now);
            for id in 1..=SERVICE_NETWORK_DISCOVERY {
                if self
                    .services
                    .inspect(id)
                    .map(|s| s.state == ServiceState::Starting)
                    .unwrap_or(false)
                {
                    let _ = self.services.announce_ready(id);
                }
            }
        }
        self.ensure_runtime_capabilities(now);
    }
    // ------------------------=
    // FUNC: service_identity
    // DESC: Implements the service identity operation.
    // ------------------=
    fn service_identity(&self, id: u32) -> Option<execution::SecurityIdentity> {
        self.services
            .inspect(id)
            .and_then(|s| s.context)
            .and_then(|h| self.execution.get(h))
            .map(|c| c.security_identity)
    }
    // ------------------------=
    // FUNC: ensure_runtime_capabilities
    // DESC: Implements the ensure runtime capabilities operation.
    // ------------------=
    fn ensure_runtime_capabilities(&mut self, now: u64) {
        let Some(runtime) = self.service_identity(SERVICE_RUNTIME) else {
            return;
        };
        if self.service_event_cap.is_none() {
            self.service_event_cap = self
                .capabilities
                .grant(
                    CapabilityType::EventPublish,
                    EVENT_SERVICE_STATE_CHANGED as u64,
                    1,
                    0,
                    runtime,
                    runtime,
                    None,
                    0,
                )
                .ok();
        }
        if self.identity_event_cap.is_none() {
            if let Some(identity) = self.service_identity(SERVICE_IDENTITY) {
                self.identity_event_cap = self
                    .capabilities
                    .grant(
                        CapabilityType::EventPublish,
                        EVENT_IDENTITY_STATE_CHANGED as u64,
                        1,
                        0,
                        runtime,
                        identity,
                        None,
                        0,
                    )
                    .ok();
            }
        }
        if self.ui_event_capabilities[0].is_none() {
            if let Some(window_server) = self.service_identity(SERVICE_WINDOW_SERVER) {
                for (index, event_type) in UI_EVENT_TYPES.iter().copied().enumerate() {
                    self.ui_event_capabilities[index] = self
                        .capabilities
                        .grant(
                            CapabilityType::EventPublish,
                            event_type as u64,
                            1,
                            0,
                            runtime,
                            window_server,
                            None,
                            0,
                        )
                        .ok();
                }
            }
        }
        if self.network_event_capabilities[0].is_none() {
            if let Some(network) = self.service_identity(SERVICE_NETWORK) {
                for (index, event_type) in NETWORK_EVENT_TYPES.iter().copied().enumerate() {
                    self.network_event_capabilities[index] = self
                        .capabilities
                        .grant(
                            CapabilityType::EventPublish,
                            event_type as u64,
                            1,
                            0,
                            runtime,
                            network,
                            None,
                            0,
                        )
                        .ok();
                }
            }
        }
        if self.settings_network_profile_capability.is_none() {
            if let Some(settings) = self.service_identity(SERVICE_SETTINGS) {
                self.settings_network_profile_capability = self.capabilities.grant(
                    CapabilityType::NetworkProfileActivate,
                    0,
                    1,
                    0,
                    runtime,
                    settings,
                    None,
                    0,
                ).ok();
            }
        }
        if self.onboarding_network_profile_capability.is_none() {
            if let Some(onboarding) = self.service_identity(SERVICE_ONBOARDING) {
                self.onboarding_network_profile_capability = self.capabilities.grant(
                    CapabilityType::NetworkProfileActivate,
                    0,
                    1,
                    0,
                    runtime,
                    onboarding,
                    None,
                    0,
                ).ok();
            }
        }
        if self.live_profile && self.installer_authority[0].is_none() {
            let Some(installer) = self.service_identity(SERVICE_INSTALLER) else {
                return;
            };
            let expiry = Some(now.saturating_add(300_000));
            self.installer_authority[0] = self
                .capabilities
                .grant(
                    CapabilityType::StorageDiscover,
                    0,
                    1,
                    0,
                    runtime,
                    installer,
                    expiry,
                    0,
                )
                .ok();
            self.installer_authority[1] = self
                .capabilities
                .grant(
                    CapabilityType::StorageProvision,
                    0,
                    1,
                    0,
                    runtime,
                    installer,
                    expiry,
                    0,
                )
                .ok();
            self.installer_authority[2] = self
                .capabilities
                .grant(
                    CapabilityType::BootInstall,
                    0,
                    1,
                    0,
                    runtime,
                    installer,
                    expiry,
                    0,
                )
                .ok();
            self.installer_authority[3] = self
                .capabilities
                .grant(
                    CapabilityType::SystemInstall,
                    0,
                    1,
                    0,
                    runtime,
                    installer,
                    expiry,
                    0,
                )
                .ok();
            self.installer_event_caps[0] = self
                .capabilities
                .grant(
                    CapabilityType::EventPublish,
                    EVENT_INSTALLER_PLAN_CONFIRMED as u64,
                    1,
                    0,
                    runtime,
                    installer,
                    expiry,
                    0,
                )
                .ok();
            self.installer_event_caps[1] = self
                .capabilities
                .grant(
                    CapabilityType::EventPublish,
                    EVENT_INSTALLER_COMPLETED as u64,
                    1,
                    0,
                    runtime,
                    installer,
                    expiry,
                    0,
                )
                .ok();
        }
        if self.ai_console_capability.is_none() {
            let Some(ai_service) = self.service_identity(SERVICE_AI) else {
                return;
            };
            let Some(console) = self.service_identity(SERVICE_CONSOLE) else {
                return;
            };
            self.ai_console_capability = self
                .capabilities
                .grant(
                    CapabilityType::AiInfer,
                    ai::model::LOCAL_INTENT_MODEL_ID as u64,
                    1,
                    0,
                    ai_service,
                    console,
                    None,
                    0,
                )
                .ok();
        }
        if self.ai_event_capabilities[0].is_none() {
            let Some(ai_service) = self.service_identity(SERVICE_AI) else {
                return;
            };
            let Some(runtime) = self.service_identity(SERVICE_RUNTIME) else {
                return;
            };
            for (index, event) in [
                EVENT_AI_INFERENCE_STARTED,
                EVENT_AI_INFERENCE_COMPLETED,
                EVENT_AI_INFERENCE_FAILED,
            ]
            .iter()
            .copied()
            .enumerate()
            {
                self.ai_event_capabilities[index] = self
                    .capabilities
                    .grant(
                        CapabilityType::EventPublish,
                        event as u64,
                        1,
                        0,
                        runtime,
                        ai_service,
                        None,
                        0,
                    )
                    .ok();
            }
        }
    }

    // ------------------------=
    // FUNC: resolve_console_intent
    // DESC: Validates console AI authority and returns a typed plan without executing it.
    // ------------------=
    pub fn resolve_console_intent(
        &mut self,
        input: &[u8],
        now: u64,
        correlation_id: u64,
    ) -> Result<ai::intent::IntentPlan, ai::types::AiError> {
        let caller = self
            .service_identity(SERVICE_CONSOLE)
            .ok_or(ai::types::AiError::AccessDenied)?;
        let capability = self
            .ai_console_capability
            .ok_or(ai::types::AiError::AccessDenied)?;
        self.capabilities
            .validate(
                capability,
                caller,
                CapabilityType::AiInfer,
                ai::model::LOCAL_INTENT_MODEL_ID as u64,
                1,
                0,
                now,
            )
            .map_err(|_| ai::types::AiError::AccessDenied)?;
        let source = self
            .service_identity(SERVICE_AI)
            .ok_or(ai::types::AiError::AccessDenied)?;
        if let Some(event_capability) = self.ai_event_capabilities[0] {
            let _ = self.events.publish(
                EventClass::StateChange,
                RoutingDomain::System,
                EVENT_AI_INFERENCE_STARTED,
                source,
                0,
                correlation_id,
                correlation_id,
                &ai::model::LOCAL_INTENT_MODEL_ID.to_le_bytes(),
                160,
                now,
                &self.capabilities,
                event_capability,
            );
        }
        let result = ai::with_ai_runtime(|runtime| {
            runtime.resolve_intent(input, caller, capability, now, correlation_id)
        });
        let (event_type, event_capability, payload) = match result {
            Ok(_) => (
                EVENT_AI_INFERENCE_COMPLETED,
                self.ai_event_capabilities[1],
                b"local-success".as_slice(),
            ),
            Err(_) => (
                EVENT_AI_INFERENCE_FAILED,
                self.ai_event_capabilities[2],
                b"local-failure".as_slice(),
            ),
        };
        if let Some(event_capability) = event_capability {
            let _ = self.events.publish(
                EventClass::StateChange,
                RoutingDomain::System,
                event_type,
                source,
                0,
                correlation_id,
                correlation_id,
                payload,
                160,
                now,
                &self.capabilities,
                event_capability,
            );
        }
        result
    }
    // ------------------------=
    // FUNC: installer_authorized
    // DESC: Handles installer authorized input or state transitions.
    // ------------------=
    pub fn installer_authorized(&self, now: u64) -> bool {
        let Some(holder) = self.service_identity(SERVICE_INSTALLER) else {
            return false;
        };
        let kinds = [
            CapabilityType::StorageDiscover,
            CapabilityType::StorageProvision,
            CapabilityType::BootInstall,
            CapabilityType::SystemInstall,
        ];
        self.installer_authority
            .iter()
            .zip(kinds)
            .all(|(id, kind)| {
                id.map(|id| {
                    self.capabilities
                        .validate(id, holder, kind, 0, 1, 0, now)
                        .is_ok()
                })
                .unwrap_or(false)
            })
    }
    // ------------------------=
    // FUNC: installer_record
    // DESC: Handles installer record input or state transitions.
    // ------------------=
    pub fn installer_record(&mut self, event_type: u32, now: u64) -> bool {
        let Some(holder) = self.service_identity(SERVICE_INSTALLER) else {
            return false;
        };
        let cap = if event_type == EVENT_INSTALLER_PLAN_CONFIRMED {
            self.installer_event_caps[0]
        } else {
            self.installer_event_caps[1]
        };
        cap.and_then(|cap| {
            self.events
                .publish(
                    EventClass::Record,
                    RoutingDomain::System,
                    event_type,
                    holder,
                    0,
                    event_type as u64,
                    event_type as u64,
                    b"installer security record",
                    255,
                    now,
                    &self.capabilities,
                    cap,
                )
                .ok()
        })
        .is_some()
    }
    // ------------------------=
    // FUNC: revoke_installer_authority
    // DESC: Removes or invalidates revoke installer authority state.
    // ------------------=
    pub fn revoke_installer_authority(&mut self) {
        for id in self
            .installer_authority
            .iter()
            .chain(self.installer_event_caps.iter())
            .flatten()
        {
            let _ = self.capabilities.revoke(*id);
        }
    }
    // ------------------------=
    // FUNC: revoke_ai_authority
    // DESC: Revokes capabilities tied to a failed AI service identity before restart.
    // ------------------=
    fn revoke_ai_authority(&mut self) {
        if let Some(capability) = self.ai_console_capability.take() {
            let _ = self.capabilities.revoke(capability);
        }
        for capability in &mut self.ai_event_capabilities {
            if let Some(id) = capability.take() {
                let _ = self.capabilities.revoke(id);
            }
        }
    }
    // ------------------------=
    // FUNC: fail_service
    // DESC: Implements the fail service operation.
    // ------------------=
    pub fn fail_service(&mut self, id: u32, now: u64) -> Result<(), ServiceError> {
        if id == SERVICE_AI {
            self.revoke_ai_authority();
        }
        let result = self.services.fail(id, &mut self.execution, now);
        let Some(source) = self.service_identity(SERVICE_RUNTIME) else {
            return result;
        };
        if let Some(cap) = self.service_event_cap {
            let payload = id.to_le_bytes();
            let _ = self.events.publish(
                EventClass::StateChange,
                RoutingDomain::System,
                EVENT_SERVICE_STATE_CHANGED,
                source,
                0,
                id as u64,
                id as u64,
                &payload,
                220,
                now,
                &self.capabilities,
                cap,
            );
        }
        result
    }
}

static mut RUNTIME: InfinityRuntime = InfinityRuntime::new(cfg!(feature = "installer"));
// ------------------------=
// FUNC: runtime_mut
// DESC: Implements the runtime mut operation.
// ------------------=
fn runtime_mut() -> &'static mut InfinityRuntime {
    unsafe { &mut *(&raw mut RUNTIME) }
}
// ------------------------=
// FUNC: runtime_ref
// DESC: Implements the runtime ref operation.
// ------------------=
fn runtime_ref() -> &'static InfinityRuntime {
    unsafe { &*(&raw const RUNTIME) }
}
#[inline(never)]
// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub fn initialize() {
    let runtime = runtime_mut();
    runtime.shell_profiles = Some(object_navigation::ShellProfileService::new());
    runtime.file_navigator = object_navigation::FileNavigatorState::new(b"/home/default").ok();
    let _ = runtime.define_bootstrap();
    // Bootstrap only the dependency roots. Storage/object/namespace readiness
    // is completed after the storage subsystem has initialized.
    runtime.services.start_ready(&mut runtime.execution, 0);
    let _ = runtime.services.announce_ready(SERVICE_RUNTIME);
    runtime.services.start_ready(&mut runtime.execution, 0);
    let _ = runtime.services.announce_ready(SERVICE_DEVICE);
    let _ = runtime.services.announce_ready(SERVICE_EVENT);
    runtime.services.start_ready(&mut runtime.execution, 0);
    if runtime.network.initialize().is_err() {
        crate::output_text(b"[network] degraded: native network bootstrap failed\n");
    }
    crate::output_text(b"[runtime] execution manager online\n[runtime] capability manager online\n[iop] router online\n[event] fabric online\n");
}

// ------------------------=
// FUNC: register_firmware_network_device
// DESC: Adds one firmware-discovered network adapter to the authoritative runtime before user setup begins.
// ------------------=
pub fn register_firmware_network_device(
    device: network::types::FirmwareNetworkDevice,
) -> bool {
    runtime_mut().network.register_firmware_device(device).is_ok()
}
#[inline(never)]
// ------------------------=
// FUNC: storage_initialized
// DESC: Implements the storage initialized operation.
// ------------------=
pub fn storage_initialized() {
    with_runtime(|runtime| {
        let _ = runtime.services.announce_ready(SERVICE_STORAGE);
        for _ in 0..MAX_SERVICES {
            runtime.services.start_ready(&mut runtime.execution, 0);
            for id in [
                SERVICE_OBJECT,
                SERVICE_NAMESPACE,
                SERVICE_CONSOLE,
                SERVICE_INSTALLER,
                SERVICE_LOCAL_ML,
                SERVICE_AI,
                SERVICE_VOICE,
                SERVICE_AGENT,
                SERVICE_ORGANIZATION,
                SERVICE_IDENTITY,
                SERVICE_AUTHENTICATION,
                SERVICE_SESSION,
                SERVICE_SETTINGS,
                SERVICE_ONBOARDING,
                SERVICE_SHELL,
                SERVICE_FONT,
                SERVICE_SKIN_REGISTRY,
                SERVICE_WINDOW_SERVER,
                SERVICE_INFINITY_UI,
                SERVICE_CLIPBOARD,
                SERVICE_NETWORK,
                SERVICE_NETWORK_POLICY,
                SERVICE_NETWORK_TRANSPORT,
                SERVICE_NETWORK_DISCOVERY,
            ] {
                if runtime
                    .services
                    .inspect(id)
                    .map(|s| s.state == ServiceState::Starting)
                    .unwrap_or(false)
                {
                    let _ = runtime.services.announce_ready(id);
                }
            }
        }
        runtime.ensure_runtime_capabilities(0);
        #[cfg(target_os = "none")]
        {
            let mut persisted = [0u8; network::NETWORK_STATE_BYTES];
            if crate::storage::network_state_load(&mut persisted)
                .ok()
                .filter(|length| *length == network::NETWORK_STATE_BYTES)
                .is_some()
            {
                let _ = runtime.network.restore_state(&persisted);
            }
        }
        if ai::initialize_global() {
            crate::output_text(b"[ai] local CPU inference online\n[ai] model registry verified\n");
        } else {
            crate::output_text(b"[ai] degraded: no verified local model\n");
        }
        #[cfg(target_os = "none")]
        {
            let mut persisted = [0u8; object_navigation::PROFILE_STATE_BYTES];
            if crate::storage::shell_profile_state_load(&mut persisted)
                .ok()
                .filter(|length| *length == object_navigation::PROFILE_STATE_BYTES)
                .is_some()
            {
                if let Ok(state) = object_navigation::ShellProfileService::decode(&persisted) {
                    runtime.shell_profiles = Some(state);
                }
            }
        }
        #[cfg(target_os = "none")]
        {
            let mut persisted = [0u8; identity::IDENTITY_STATE_BYTES];
            if let Some(length) = crate::storage::identity_state_load(&mut persisted).ok().filter(
                |length| {
                    matches!(
                        *length,
                        identity::LEGACY_IDENTITY_STATE_BYTES | identity::IDENTITY_STATE_BYTES
                    )
                },
            ) {
                match identity::IdentitySystem::decode(&persisted[..length]) {
                    Ok(state) => runtime.identity = state,
                    Err(_) => crate::output_text(
                        b"[identity] durable state invalid; onboarding recovery required\n",
                    ),
                }
            }
        }
    });
}

// ------------------------=
// FUNC: persist_shell_profile_state
// DESC: Commits declarative Shell Profile objects before any observable profile event.
// ------------------=
pub fn persist_shell_profile_state() -> bool {
    #[cfg(target_os = "none")]
    {
        let Some(service) = runtime_ref().shell_profiles.as_ref() else {
            return false;
        };
        crate::storage::shell_profile_state_commit(&service.encode()).is_ok()
    }
    #[cfg(not(target_os = "none"))]
    {
        true
    }
}

// ------------------------=
// FUNC: persist_identity_state
// DESC: Commits authoritative identity state as a versioned native object.
// ------------------=
pub fn persist_identity_state() -> bool {
    #[cfg(target_os = "none")]
    {
        let encoded = runtime_ref().identity.encode();
        if crate::storage::identity_state_commit(&encoded).is_err() {
            return false;
        }
        // State is authoritative and commits before its notification. A missed
        // event is therefore recoverable through Identity.Read.
        let runtime = runtime_mut();
        if let (Some(source), Some(capability)) = (
            runtime.service_identity(SERVICE_IDENTITY),
            runtime.identity_event_cap,
        ) {
            let generation = runtime.identity.generation();
            let _ = runtime.events.publish(
                EventClass::Record,
                RoutingDomain::Session,
                EVENT_IDENTITY_STATE_CHANGED,
                source,
                0,
                generation,
                generation,
                &generation.to_le_bytes(),
                220,
                generation,
                &runtime.capabilities,
                capability,
            );
        }
        return true;
    }
    #[cfg(not(target_os = "none"))]
    {
        true
    }
}

// ------------------------=
// FUNC: activate_network_profile_from_settings
// DESC: Performs trusted Settings profile activation, durable commit, and post-commit event publication.
// ------------------=
pub fn activate_network_profile_from_settings(profile_id: u32, now: u64, correlation_id: u64) -> bool {
    let runtime = runtime_mut();
    let Some(settings) = runtime.service_identity(SERVICE_SETTINGS) else { return false; };
    let Some(authority) = runtime.settings_network_profile_capability else { return false; };
    let _previous = runtime.network.profiles.active_id();
    let Ok(generation) = runtime.network.activate_profile_authorized(profile_id, settings, authority, now, &runtime.capabilities) else { return false; };
    #[cfg(target_os = "none")]
    if crate::storage::network_state_commit(&runtime.network.encode_state()).is_err() {
        let _ = runtime.network.activate_profile(_previous);
        return false;
    }
    if let Some(index) = NETWORK_EVENT_TYPES.iter().position(|event| *event == EVENT_NETWORK_PROFILE_ACTIVATED) {
        if let (Some(capability), Some(source)) = (runtime.network_event_capabilities[index], runtime.service_identity(SERVICE_NETWORK)) {
            let mut payload = [0u8; 16]; payload[..4].copy_from_slice(&profile_id.to_le_bytes()); payload[8..16].copy_from_slice(&generation.to_le_bytes());
            let _ = runtime.events.publish(EventClass::Record, RoutingDomain::Network, EVENT_NETWORK_PROFILE_ACTIVATED, source, profile_id as u64, correlation_id, correlation_id, &payload, 220, now, &runtime.capabilities, capability);
        }
    }
    true
}

// ------------------------=
// FUNC: reconfigure_network_from_settings
// DESC: Applies and durably commits an explicit post-install connection mode from trusted System Settings.
// ------------------=
pub fn reconfigure_network_from_settings(
    mode: network::types::NetworkSetupMode,
    now: u64,
    correlation_id: u64,
) -> bool {
    let runtime = runtime_mut();
    let Some(settings) = runtime.service_identity(SERVICE_SETTINGS) else {
        return false;
    };
    let Some(authority) = runtime.settings_network_profile_capability else {
        return false;
    };
    let _previous = runtime.network.encode_state();
    if runtime
        .network
        .reconfigure_authorized(mode, settings, authority, now, &runtime.capabilities)
        .is_err()
    {
        return false;
    }
    #[cfg(target_os = "none")]
    if crate::storage::network_state_commit(&runtime.network.encode_state()).is_err() {
        let _ = runtime.network.restore_state(&_previous);
        return false;
    }
    if let Some(index) = NETWORK_EVENT_TYPES
        .iter()
        .position(|event| *event == EVENT_NETWORK_PROFILE_ACTIVATED)
    {
        if let (Some(capability), Some(source)) = (
            runtime.network_event_capabilities[index],
            runtime.service_identity(SERVICE_NETWORK),
        ) {
            let state = runtime.network.encode_state();
            let _ = runtime.events.publish(
                EventClass::Record,
                RoutingDomain::Network,
                EVENT_NETWORK_PROFILE_ACTIVATED,
                source,
                runtime.network.profiles.active_id() as u64,
                correlation_id,
                correlation_id,
                &state[..24],
                220,
                now,
                &runtime.capabilities,
                capability,
            );
        }
    }
    true
}

// ------------------------=
// FUNC: select_network_mode_from_onboarding
// DESC: Stages one explicit first-boot connectivity choice for shared GUI and keyboard interaction.
// ------------------=
pub fn select_network_mode_from_onboarding(mode: network::types::NetworkSetupMode) -> bool {
    let runtime = runtime_mut();
    if runtime.service_identity(SERVICE_ONBOARDING).is_none() { return false; }
    runtime.network.select_setup_mode(mode);
    true
}

// ------------------------=
// FUNC: apply_network_mode_from_onboarding
// DESC: Applies and durably commits first-boot connectivity through the onboarding service's scoped authority.
// ------------------=
pub fn apply_network_mode_from_onboarding(now: u64, correlation_id: u64) -> bool {
    let runtime = runtime_mut();
    let Some(onboarding) = runtime.service_identity(SERVICE_ONBOARDING) else { return false; };
    let Some(authority) = runtime.onboarding_network_profile_capability else { return false; };
    if runtime.capabilities.validate(authority, onboarding, CapabilityType::NetworkProfileActivate, 0, 1, 0, now).is_err() { return false; }
    let _previous = runtime.network.encode_state();
    let profile_id: u32 = if runtime.network.setup_snapshot().selected == network::types::NetworkSetupMode::Offline { 3 } else { 1 };
    if runtime.network.apply_setup_mode().is_err() { return false; }
    #[cfg(target_os = "none")]
    if crate::storage::network_state_commit(&runtime.network.encode_state()).is_err() {
        let _ = runtime.network.restore_state(&_previous);
        return false;
    }
    if let Some(index) = NETWORK_EVENT_TYPES.iter().position(|event| *event == EVENT_NETWORK_PROFILE_ACTIVATED) {
        if let (Some(capability), Some(source)) = (runtime.network_event_capabilities[index], runtime.service_identity(SERVICE_NETWORK)) {
            let generation = runtime.network.profiles.generation();
            let mut payload = [0u8; 16];
            payload[..4].copy_from_slice(&profile_id.to_le_bytes());
            payload[8..16].copy_from_slice(&generation.to_le_bytes());
            let _ = runtime.events.publish(EventClass::Record, RoutingDomain::Network, EVENT_NETWORK_PROFILE_ACTIVATED, source, profile_id as u64, correlation_id, correlation_id, &payload, 220, now, &runtime.capabilities, capability);
        }
    }
    true
}
// ------------------------=
// FUNC: announce_services
// DESC: Implements the announce services operation.
// ------------------=
pub fn announce_services() {
    let runtime = runtime_ref();
    for (id, name) in [
        (SERVICE_STORAGE, b"storage".as_slice()),
        (SERVICE_OBJECT, b"object".as_slice()),
        (SERVICE_NAMESPACE, b"namespace".as_slice()),
        (SERVICE_CONSOLE, b"console".as_slice()),
        (SERVICE_LOCAL_ML, b"local-ml".as_slice()),
        (SERVICE_AI, b"infinity-ai".as_slice()),
        (SERVICE_VOICE, b"voice".as_slice()),
        (SERVICE_AGENT, b"agent".as_slice()),
        (SERVICE_ORGANIZATION, b"organization".as_slice()),
        (SERVICE_IDENTITY, b"identity".as_slice()),
        (SERVICE_AUTHENTICATION, b"authentication".as_slice()),
        (SERVICE_SESSION, b"session".as_slice()),
        (SERVICE_SETTINGS, b"settings".as_slice()),
        (SERVICE_ONBOARDING, b"onboarding".as_slice()),
        (SERVICE_SHELL, b"shell".as_slice()),
        (SERVICE_FONT, b"font".as_slice()),
        (SERVICE_SKIN_REGISTRY, b"skin-registry".as_slice()),
        (SERVICE_WINDOW_SERVER, b"window-server".as_slice()),
        (SERVICE_INFINITY_UI, b"infinity-ui".as_slice()),
        (SERVICE_CLIPBOARD, b"clipboard".as_slice()),
        (SERVICE_NETWORK, b"network".as_slice()),
        (SERVICE_NETWORK_POLICY, b"network-policy".as_slice()),
        (SERVICE_NETWORK_TRANSPORT, b"network-transport".as_slice()),
        (SERVICE_NETWORK_DISCOVERY, b"network-discovery".as_slice()),
    ] {
        if runtime
            .services
            .inspect(id)
            .map(|s| s.state == ServiceState::Ready)
            .unwrap_or(false)
        {
            crate::output_text(b"[service] ");
            crate::output_text(name);
            crate::output_text(b" ready\n")
        }
    }
    crate::output_text(b"Infinity Runtime online.\n")
}
// ------------------------=
// FUNC: with_runtime
// DESC: Implements the with runtime operation.
// ------------------=
pub fn with_runtime<T>(f: impl FnOnce(&mut InfinityRuntime) -> T) -> Option<T> {
    Some(f(runtime_mut()))
}

#[cfg(not(target_os = "none"))]
// ------------------------=
// FUNC: acceptance_self_test
// DESC: Verifies acceptance self test behavior.
// ------------------=
pub fn acceptance_self_test() -> bool {
    let mut r = InfinityRuntime::new(true);
    if r.define_bootstrap().is_err() {
        return false;
    }
    r.start_all(0);
    let Some(a) = r.execution.nth(0).map(|c| c.security_identity) else {
        return false;
    };
    let Some(b) = r.execution.nth(1).map(|c| c.security_identity) else {
        return false;
    };
    if r.iop.register_endpoint(41, a).is_err() || r.iop.register_endpoint(42, b).is_err() {
        return false;
    }
    let call = match r.capabilities.grant(
        CapabilityType::ServiceCall,
        OperationId::TestEcho as u64,
        1,
        0,
        b,
        a,
        Some(100),
        0,
    ) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let message =
        match IopMessage::request(OperationId::TestEcho, 0xa81f, a, call, 50, 0xa81f, b"hello") {
            Ok(v) => v,
            Err(_) => return false,
        };
    if r.iop.send(42, message, &r.capabilities, 1).is_err() {
        return false;
    }
    let Ok(received) = r.iop.receive(42, 1) else {
        return false;
    };
    if received.bytes() != b"hello"
        || r.iop.respond(41, &received, b, b"hello", 1).is_err()
        || r.iop
            .receive(41, 1)
            .map(|m| m.header.message_type != iop::MessageType::Response || m.bytes() != b"hello")
            .unwrap_or(true)
    {
        return false;
    }
    if r.capabilities.revoke(call).is_err() {
        return false;
    }
    let denied = IopMessage::request(OperationId::TestEcho, 2, a, call, 50, 2, b"hello")
        .ok()
        .and_then(|m| r.iop.send(42, m, &r.capabilities, 2).err())
        .is_some();
    let pubcap = r
        .capabilities
        .grant(CapabilityType::EventPublish, 77, 1, 9, b, b, Some(100), 0)
        .ok();
    let subcap = r
        .capabilities
        .grant(CapabilityType::EventSubscribe, 77, 1, 9, b, a, Some(100), 0)
        .ok();
    let (Some(pubcap), Some(subcap)) = (pubcap, subcap) else {
        return false;
    };
    let lease = match r.events.subscribe(
        a,
        subcap,
        EventFilter {
            type_id: 77,
            scope: Some(9),
        },
        100,
        OverflowPolicy::DropOldest,
        2,
        &r.capabilities,
        1,
    ) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if r.events
        .publish(
            EventClass::StateChange,
            RoutingDomain::System,
            77,
            b,
            9,
            2,
            2,
            b"changed",
            10,
            2,
            &r.capabilities,
            pubcap,
        )
        .is_err()
        || r.events.receive(lease, 2).is_err()
    {
        return false;
    }
    if r.capabilities.revoke(subcap).is_err() {
        return false;
    }
    let _ = r.events.publish(
        EventClass::StateChange,
        RoutingDomain::System,
        77,
        b,
        9,
        3,
        3,
        b"again",
        10,
        3,
        &r.capabilities,
        pubcap,
    );
    let suppressed = r.events.receive(lease, 3).is_err();
    denied && suppressed
}
