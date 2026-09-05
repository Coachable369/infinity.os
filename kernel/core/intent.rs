#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConsoleMode {
    Startup,
    Console,
    Repair,
    Installer,
    Onboarding,
    Desktop,
    AppLauncher,
    SystemMenu,
    Settings,
    Authentication,
    Locked,
}

pub struct IntentContext {
    pub console_mode: ConsoleMode,
    pub architecture: &'static str,
    pub known_devices: usize,
    pub environment: &'static str,
}

#[derive(Clone, Copy)]
pub enum SystemOperation {
    Help,
    ClearConsole,
    DeviceList,
    SystemStatus,
    SystemInfo,
    SystemGenerationInspect,
    SystemBootStatus,
    MemoryStatus,
    ExitConsole,
    Greeting,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResolutionSource {
    ExactCommand,
    BuiltInIntent,
}

pub struct ResolvedIntent {
    pub operation: SystemOperation,
    pub source: ResolutionSource,
    pub canonical_command: &'static [u8],
}

#[derive(Clone, Copy)]
pub enum IntentError {
    NotUnderstood,
}

pub trait IntentResolver {
    // ------------------------=
    // FUNC: resolve
    // DESC: Implements the resolve operation.
    // ------------------=
    fn resolve(&self, input: &str, context: &IntentContext) -> Result<ResolvedIntent, IntentError>;
}

pub struct ExactCommandResolver;

impl IntentResolver for ExactCommandResolver {
    // ------------------------=
    // FUNC: resolve
    // DESC: Implements the resolve operation.
    // ------------------=
    fn resolve(
        &self,
        input: &str,
        _context: &IntentContext,
    ) -> Result<ResolvedIntent, IntentError> {
        let command = input.trim();
        let (operation, canonical_command) = if command.eq_ignore_ascii_case("help") {
            (SystemOperation::Help, &b"help"[..])
        } else if command.eq_ignore_ascii_case("clear") {
            (SystemOperation::ClearConsole, &b"clear"[..])
        } else if command.eq_ignore_ascii_case("device list") {
            (SystemOperation::DeviceList, &b"device list"[..])
        } else if command.eq_ignore_ascii_case("system status") {
            (SystemOperation::SystemStatus, &b"system status"[..])
        } else if command.eq_ignore_ascii_case("system info") {
            (SystemOperation::SystemInfo, &b"system info"[..])
        } else if command.eq_ignore_ascii_case("system generation") {
            (
                SystemOperation::SystemGenerationInspect,
                &b"system generation"[..],
            )
        } else if command.eq_ignore_ascii_case("system boot") {
            (SystemOperation::SystemBootStatus, &b"system boot"[..])
        } else if command.eq_ignore_ascii_case("memory status") {
            (SystemOperation::MemoryStatus, &b"memory status"[..])
        } else if command.eq_ignore_ascii_case("exit") {
            (SystemOperation::ExitConsole, &b"exit"[..])
        } else if command.eq_ignore_ascii_case("hello") {
            (SystemOperation::Greeting, &b"hello"[..])
        } else {
            return Err(IntentError::NotUnderstood);
        };
        Ok(ResolvedIntent {
            operation,
            source: ResolutionSource::ExactCommand,
            canonical_command,
        })
    }
}

pub struct BuiltInIntentResolver;

impl IntentResolver for BuiltInIntentResolver {
    // ------------------------=
    // FUNC: resolve
    // DESC: Implements the resolve operation.
    // ------------------=
    fn resolve(
        &self,
        input: &str,
        _context: &IntentContext,
    ) -> Result<ResolvedIntent, IntentError> {
        let phrase = input.trim();
        let (operation, canonical_command) = if matches_phrase(
            phrase,
            &[
                "show connected devices",
                "what hardware is connected",
                "list my devices",
                "show devices",
                "what devices do you see",
            ],
        ) {
            (SystemOperation::DeviceList, &b"device list"[..])
        } else if matches_phrase(
            phrase,
            &[
                "how is the system doing",
                "show system status",
                "what is the system status",
            ],
        ) {
            (SystemOperation::SystemStatus, &b"system status"[..])
        } else if matches_phrase(
            phrase,
            &[
                "tell me about this machine",
                "show system information",
                "what system is this",
            ],
        ) {
            (SystemOperation::SystemInfo, &b"system info"[..])
        } else if matches_phrase(
            phrase,
            &[
                "what system version am i booted into",
                "show active system generation",
            ],
        ) {
            (
                SystemOperation::SystemGenerationInspect,
                &b"system generation"[..],
            )
        } else if matches_phrase(phrase, &["how did this system boot", "show boot status"]) {
            (SystemOperation::SystemBootStatus, &b"system boot"[..])
        } else if matches_phrase(
            phrase,
            &[
                "show memory status",
                "how much memory information is available",
            ],
        ) {
            (SystemOperation::MemoryStatus, &b"memory status"[..])
        } else {
            return Err(IntentError::NotUnderstood);
        };
        Ok(ResolvedIntent {
            operation,
            source: ResolutionSource::BuiltInIntent,
            canonical_command,
        })
    }
}

// ------------------------=
// FUNC: matches_phrase
// DESC: Implements the matches phrase operation.
// ------------------=
fn matches_phrase(input: &str, phrases: &[&str]) -> bool {
    phrases
        .iter()
        .any(|phrase| input.eq_ignore_ascii_case(phrase))
}

pub struct IntentRuntime {
    exact: ExactCommandResolver,
    built_in: BuiltInIntentResolver,
}

impl IntentRuntime {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            exact: ExactCommandResolver,
            built_in: BuiltInIntentResolver,
        }
    }

    // ------------------------=
    // FUNC: resolve
    // DESC: Implements the resolve operation.
    // ------------------=
    pub fn resolve(
        &self,
        input: &str,
        context: &IntentContext,
    ) -> Result<ResolvedIntent, IntentError> {
        self.exact
            .resolve(input, context)
            .or_else(|_| self.built_in.resolve(input, context))
    }
}

pub struct IntentProviderDescriptor {
    pub name: &'static str,
    pub capabilities: &'static [&'static str],
    pub is_remote: bool,
}

/// Provider-neutral seam for future local or remote model plugins. Providers
/// may only propose a typed operation; they cannot invoke kernel services.
pub trait IntentProvider {
    // ------------------------=
    // FUNC: descriptor
    // DESC: Implements the descriptor operation.
    // ------------------=
    fn descriptor(&self) -> IntentProviderDescriptor;
    // ------------------------=
    // FUNC: resolve
    // DESC: Implements the resolve operation.
    // ------------------=
    fn resolve(&self, input: &str, context: &IntentContext) -> Result<ResolvedIntent, IntentError>;
}

pub trait OperationPolicy {
    // ------------------------=
    // FUNC: authorize
    // DESC: Implements the authorize operation.
    // ------------------=
    fn authorize(&self, operation: SystemOperation, context: &IntentContext) -> bool;
}

/// Milestone 2 permits only the closed SystemOperation enum. This explicit
/// pass-through is the insertion point for the future capability policy.
pub struct KnownOperationPolicy;

impl OperationPolicy for KnownOperationPolicy {
    // ------------------------=
    // FUNC: authorize
    // DESC: Implements the authorize operation.
    // ------------------=
    fn authorize(&self, _operation: SystemOperation, context: &IntentContext) -> bool {
        let _ = (
            context.console_mode,
            context.architecture,
            context.known_devices,
            context.environment,
        );
        true
    }
}
