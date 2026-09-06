//! InfinityOS-native human navigation and declarative Shell Profiles.
//! Paths are Namespace references; stable object identity is always explicit.

pub const MAX_NAMESPACE_PATH: usize = 96;
pub const MAX_COMMAND: usize = 160;
pub const MAX_PROFILES: usize = 6;
pub const MAX_ALIASES_PER_PROFILE: usize = 12;
pub const PROFILE_NAME_CAPACITY: usize = 24;
pub const ALIAS_NAME_CAPACITY: usize = 20;
pub const PROFILE_STATE_BYTES: usize = 16_384;
pub const PROFILE_STATE_MAGIC: &[u8; 8] = b"INFSHL01";
const PROFILE_STATE_VERSION: u16 = 1;
pub const FILE_NAVIGATOR_APPLICATION_ID: &[u8] = b"app.infinity.file-navigator";
pub const FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT: usize = 2;
pub const FILE_NAVIGATOR_DEFAULT_SIZE: (u32, u32) = (780, 560);
pub const FILE_NAVIGATOR_MINIMUM_SIZE: (u32, u32) = (640, 420);
pub const FILE_NAVIGATOR_INTENTS: &[&[u8]] = &[b"BrowseNamespace", b"RevealObject", b"OpenObject"];
pub const FILE_NAVIGATOR_HISTORY_CAPACITY: usize = 12;
pub const FILE_NAVIGATOR_NO_SELECTION: u16 = u16::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationError {
    InvalidPath,
    MissingNamespace,
    AccessDenied,
    NoPreviousNamespace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileError {
    InvalidName,
    InvalidTemplate,
    Duplicate,
    NotFound,
    Immutable,
    Capacity,
    AccessDenied,
    CorruptState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileKind {
    Native,
    Compatibility,
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltInProfile {
    InfinityNative,
    Linux,
    Unix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Grid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteText<const N: usize> {
    bytes: [u8; N],
    length: u8,
}

impl<const N: usize> ByteText<N> {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates an empty bounded UTF-8 text value.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            bytes: [0; N],
            length: 0,
        }
    }

    // ------------------------=
    // FUNC: new
    // DESC: Validates and stores a bounded UTF-8 text value.
    // ------------------=
    pub fn new(value: &[u8]) -> Result<Self, ()> {
        if value.len() > N || core::str::from_utf8(value).is_err() {
            return Err(());
        }
        let mut output = Self::empty();
        output.bytes[..value.len()].copy_from_slice(value);
        output.length = value.len() as u8;
        Ok(output)
    }

    // ------------------------=
    // FUNC: as_bytes
    // DESC: Returns the initialized portion of a bounded text value.
    // ------------------=
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.length as usize]
    }

    // ------------------------=
    // FUNC: replace
    // DESC: Replaces a bounded UTF-8 value without exposing its backing storage.
    // ------------------=
    pub fn replace(&mut self, value: &[u8]) -> Result<(), ()> {
        *self = Self::new(value)?;
        Ok(())
    }

    // ------------------------=
    // FUNC: push_ascii
    // DESC: Appends one printable ASCII byte when bounded capacity permits it.
    // ------------------=
    pub fn push_ascii(&mut self, value: u8) -> bool {
        let length = self.length as usize;
        if !(0x20..=0x7e).contains(&value) || length >= N {
            return false;
        }
        self.bytes[length] = value;
        self.length += 1;
        true
    }

    // ------------------------=
    // FUNC: pop
    // DESC: Removes the final byte from an ASCII-backed bounded editor value.
    // ------------------=
    pub fn pop(&mut self) -> bool {
        if self.length == 0 {
            return false;
        }
        self.length -= 1;
        self.bytes[self.length as usize] = 0;
        true
    }

    // ------------------------=
    // FUNC: edit
    // DESC: Applies insertion, deletion, and movement at an explicit bounded caret.
    // ------------------=
    pub fn edit(&mut self, caret: &mut usize, key: crate::console::ConsoleKey) -> bool {
        let mut length = self.length as usize;
        let changed = match key {
            crate::console::ConsoleKey::Character(value) => {
                crate::ui::text_input::insert_ascii(&mut self.bytes, &mut length, caret, value)
            }
            crate::console::ConsoleKey::Backspace => {
                crate::ui::text_input::backspace(&mut self.bytes, &mut length, caret)
            }
            crate::console::ConsoleKey::Delete => {
                crate::ui::text_input::delete(&mut self.bytes, &mut length, caret)
            }
            crate::console::ConsoleKey::Left => {
                crate::ui::text_input::move_caret(caret, length, -1)
            }
            crate::console::ConsoleKey::Right => {
                crate::ui::text_input::move_caret(caret, length, 1)
            }
            crate::console::ConsoleKey::Home => {
                crate::ui::text_input::move_caret(caret, length, -2)
            }
            crate::console::ConsoleKey::End => crate::ui::text_input::move_caret(caret, length, 2),
            _ => false,
        };
        self.length = length as u8;
        changed
    }
}

impl<const N: usize> Default for ByteText<N> {
    // ------------------------=
    // FUNC: default
    // DESC: Returns an empty bounded text value.
    // ------------------=
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsoleNavigationContext {
    pub session_id: u64,
    pub current_namespace_ref: ByteText<MAX_NAMESPACE_PATH>,
    pub previous_namespace_ref: ByteText<MAX_NAMESPACE_PATH>,
    pub home_namespace_ref: ByteText<MAX_NAMESPACE_PATH>,
    pub user_identity: u64,
    pub capability_context: u64,
}

impl ConsoleNavigationContext {
    // ------------------------=
    // FUNC: new
    // DESC: Creates isolated per-session Namespace navigation state without a process CWD.
    // ------------------=
    pub fn new(session_id: u64, user_identity: u64, home: &[u8]) -> Result<Self, NavigationError> {
        let home = normalize_absolute_path(home)?;
        Ok(Self {
            session_id,
            current_namespace_ref: home,
            previous_namespace_ref: ByteText::empty(),
            home_namespace_ref: home,
            user_identity,
            capability_context: 0,
        })
    }

    // ------------------------=
    // FUNC: path
    // DESC: Returns the active human NamespaceRef projection for this Console session.
    // ------------------=
    pub fn path(&self) -> &[u8] {
        self.current_namespace_ref.as_bytes()
    }

    // ------------------------=
    // FUNC: navigate
    // DESC: Resolves a native idir or cd target and updates only this session context.
    // ------------------=
    pub fn navigate(
        &mut self,
        target: &[u8],
        can_traverse: impl Fn(&[u8]) -> bool,
    ) -> Result<&[u8], NavigationError> {
        let destination = if matches!(target, b"home" | b"~") {
            self.home_namespace_ref
        } else if matches!(target, b"previous" | b"-") {
            if self.previous_namespace_ref.as_bytes().is_empty() {
                return Err(NavigationError::NoPreviousNamespace);
            }
            self.previous_namespace_ref
        } else if matches!(target, b"parent" | b"..") {
            parent_path(self.current_namespace_ref.as_bytes())?
        } else if target == b"." || target.is_empty() {
            self.current_namespace_ref
        } else if target.first() == Some(&b'/') {
            normalize_absolute_path(target)?
        } else {
            join_path(self.current_namespace_ref.as_bytes(), target)?
        };
        if !can_traverse(destination.as_bytes()) {
            return Err(NavigationError::AccessDenied);
        }
        if destination != self.current_namespace_ref {
            self.previous_namespace_ref = self.current_namespace_ref;
            self.current_namespace_ref = destination;
        }
        Ok(self.current_namespace_ref.as_bytes())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AliasMapping {
    pub name: ByteText<ALIAS_NAME_CAPACITY>,
    pub template: ByteText<MAX_COMMAND>,
    pub enabled: bool,
}

impl AliasMapping {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates an unused declarative alias slot.
    // ------------------=
    const fn empty() -> Self {
        Self {
            name: ByteText::empty(),
            template: ByteText::empty(),
            enabled: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShellProfile {
    pub profile_id: u32,
    pub name: ByteText<PROFILE_NAME_CAPACITY>,
    pub owner_identity: u64,
    pub description: ByteText<64>,
    pub kind: ProfileKind,
    pub built_in: bool,
    pub enabled: bool,
    pub priority: i16,
    pub aliases: [AliasMapping; MAX_ALIASES_PER_PROFILE],
    pub alias_count: u8,
    pub created: u64,
    pub modified: u64,
}

impl ShellProfile {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates an unused profile slot.
    // ------------------=
    const fn empty() -> Self {
        Self {
            profile_id: 0,
            name: ByteText::empty(),
            owner_identity: 0,
            description: ByteText::empty(),
            kind: ProfileKind::User,
            built_in: false,
            enabled: false,
            priority: 0,
            aliases: [AliasMapping::empty(); MAX_ALIASES_PER_PROFILE],
            alias_count: 0,
            created: 0,
            modified: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AliasResolution {
    pub canonical: ByteText<MAX_COMMAND>,
    pub resolved_profile: u32,
    pub shadowed_profiles: [u32; MAX_PROFILES],
    pub shadowed_count: u8,
    pub native: bool,
}

pub struct ShellProfileService {
    profiles: [ShellProfile; MAX_PROFILES],
    profile_count: u8,
    next_profile_id: u32,
    default_profile: u32,
    revision: u64,
}

impl ShellProfileService {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the immutable native profile and disabled Linux and Unix compatibility layers.
    // ------------------=
    pub fn new() -> Self {
        let mut service = Self {
            profiles: [ShellProfile::empty(); MAX_PROFILES],
            profile_count: 0,
            next_profile_id: 4,
            default_profile: 1,
            revision: 1,
        };
        service.install_builtin(
            1,
            b"infinity.native",
            b"Canonical Infinity Native commands",
            ProfileKind::Native,
            true,
            30_000,
        );
        service.install_builtin(
            2,
            b"compat.linux",
            b"Linux vocabulary mapped to Infinity Native semantics",
            ProfileKind::Compatibility,
            false,
            100,
        );
        service.install_builtin(
            3,
            b"compat.unix",
            b"Conservative Unix vocabulary mapped to Infinity Native semantics",
            ProfileKind::Compatibility,
            false,
            50,
        );
        for (alias, template) in LINUX_ALIASES {
            let _ = service.add_builtin_alias(2, alias, template);
        }
        for (alias, template) in UNIX_ALIASES {
            let _ = service.add_builtin_alias(3, alias, template);
        }
        service
    }

    // ------------------------=
    // FUNC: install_builtin
    // DESC: Installs one signed immutable profile into a deterministic fixed slot.
    // ------------------=
    fn install_builtin(
        &mut self,
        id: u32,
        name: &[u8],
        description: &[u8],
        kind: ProfileKind,
        enabled: bool,
        priority: i16,
    ) {
        let index = self.profile_count as usize;
        self.profiles[index] = ShellProfile {
            profile_id: id,
            name: ByteText::new(name).unwrap_or_default(),
            owner_identity: 0,
            description: ByteText::new(description).unwrap_or_default(),
            kind,
            built_in: true,
            enabled,
            priority,
            aliases: [AliasMapping::empty(); MAX_ALIASES_PER_PROFILE],
            alias_count: 0,
            created: 1,
            modified: 1,
        };
        self.profile_count += 1;
    }

    // ------------------------=
    // FUNC: add_builtin_alias
    // DESC: Adds a validated immutable compatibility mapping during bootstrap.
    // ------------------=
    fn add_builtin_alias(
        &mut self,
        profile_id: u32,
        alias: &[u8],
        template: &[u8],
    ) -> Result<(), ProfileError> {
        let index = self
            .profile_index_by_id(profile_id)
            .ok_or(ProfileError::NotFound)?;
        Self::insert_alias(&mut self.profiles[index], alias, template)
    }

    // ------------------------=
    // FUNC: profile_count
    // DESC: Returns the number of installed built-in and user profiles.
    // ------------------=
    pub fn profile_count(&self) -> usize {
        self.profile_count as usize
    }

    // ------------------------=
    // FUNC: profile_nth
    // DESC: Returns one profile by stable deterministic enumeration order.
    // ------------------=
    pub fn profile_nth(&self, index: usize) -> Option<ShellProfile> {
        self.profiles
            .get(index)
            .copied()
            .filter(|profile| profile.profile_id != 0)
    }

    // ------------------------=
    // FUNC: profile
    // DESC: Resolves a profile by stable ID or canonical name.
    // ------------------=
    pub fn profile(&self, name: &[u8]) -> Option<ShellProfile> {
        self.profile_index(name).map(|index| self.profiles[index])
    }

    // ------------------------=
    // FUNC: create
    // DESC: Creates a user-owned declarative Shell Profile object.
    // ------------------=
    pub fn create(&mut self, owner: u64, name: &[u8], now: u64) -> Result<u32, ProfileError> {
        validate_profile_name(name)?;
        if self.profile_index(name).is_some() {
            return Err(ProfileError::Duplicate);
        }
        let index = self.profile_count as usize;
        if index >= MAX_PROFILES {
            return Err(ProfileError::Capacity);
        }
        let id = self.next_profile_id;
        self.next_profile_id = self.next_profile_id.saturating_add(1);
        self.profiles[index] = ShellProfile {
            profile_id: id,
            name: ByteText::new(name).map_err(|_| ProfileError::InvalidName)?,
            owner_identity: owner,
            description: ByteText::new(b"User declarative command mappings").unwrap_or_default(),
            kind: ProfileKind::User,
            built_in: false,
            enabled: false,
            priority: 1_000,
            aliases: [AliasMapping::empty(); MAX_ALIASES_PER_PROFILE],
            alias_count: 0,
            created: now,
            modified: now,
        };
        self.profile_count += 1;
        self.revision = self.revision.saturating_add(1);
        Ok(id)
    }

    // ------------------------=
    // FUNC: clone_profile
    // DESC: Clones declarative mappings into a new user-owned profile.
    // ------------------=
    pub fn clone_profile(
        &mut self,
        owner: u64,
        source: &[u8],
        destination: &[u8],
        now: u64,
    ) -> Result<u32, ProfileError> {
        let source = self.profile(source).ok_or(ProfileError::NotFound)?;
        let id = self.create(owner, destination, now)?;
        let index = self.profile_index_by_id(id).ok_or(ProfileError::NotFound)?;
        self.profiles[index].aliases = source.aliases;
        self.profiles[index].alias_count = source.alias_count;
        Ok(id)
    }

    // ------------------------=
    // FUNC: enable
    // DESC: Enables an authorized profile without changing native command meaning.
    // ------------------=
    pub fn enable(&mut self, actor: u64, name: &[u8], now: u64) -> Result<(), ProfileError> {
        self.set_enabled(actor, name, true, now)
    }

    // ------------------------=
    // FUNC: disable
    // DESC: Disables an authorized compatibility or user profile while retaining its object.
    // ------------------=
    pub fn disable(&mut self, actor: u64, name: &[u8], now: u64) -> Result<(), ProfileError> {
        self.set_enabled(actor, name, false, now)
    }

    // ------------------------=
    // FUNC: set_enabled
    // DESC: Applies common ownership and immutable-native rules for profile activation.
    // ------------------=
    fn set_enabled(
        &mut self,
        actor: u64,
        name: &[u8],
        enabled: bool,
        now: u64,
    ) -> Result<(), ProfileError> {
        let index = self.profile_index(name).ok_or(ProfileError::NotFound)?;
        let profile = &mut self.profiles[index];
        if profile.kind == ProfileKind::Native && !enabled {
            return Err(ProfileError::Immutable);
        }
        if profile.kind == ProfileKind::User && profile.owner_identity != actor {
            return Err(ProfileError::AccessDenied);
        }
        profile.enabled = enabled;
        profile.modified = now;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: set_default
    // DESC: Selects one enabled authorized profile as the user's default mapping layer.
    // ------------------=
    pub fn set_default(&mut self, actor: u64, name: &[u8]) -> Result<(), ProfileError> {
        let profile = self.profile(name).ok_or(ProfileError::NotFound)?;
        if !profile.enabled {
            return Err(ProfileError::InvalidName);
        }
        if profile.kind == ProfileKind::User && profile.owner_identity != actor {
            return Err(ProfileError::AccessDenied);
        }
        self.default_profile = profile.profile_id;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: delete
    // DESC: Disables and removes one user-owned profile while rejecting immutable built-ins.
    // ------------------=
    pub fn delete(&mut self, actor: u64, name: &[u8]) -> Result<u8, ProfileError> {
        let index = self.profile_index(name).ok_or(ProfileError::NotFound)?;
        let profile = self.profiles[index];
        if profile.built_in {
            return Err(ProfileError::Immutable);
        }
        if profile.owner_identity != actor {
            return Err(ProfileError::AccessDenied);
        }
        if self.default_profile == profile.profile_id {
            self.default_profile = 1;
        }
        let aliases_removed = profile.alias_count;
        for item in index..self.profile_count as usize - 1 {
            self.profiles[item] = self.profiles[item + 1];
        }
        self.profile_count -= 1;
        self.profiles[self.profile_count as usize] = ShellProfile::empty();
        self.revision = self.revision.saturating_add(1);
        Ok(aliases_removed)
    }

    // ------------------------=
    // FUNC: alias_add
    // DESC: Adds a validated declarative mapping to a user-owned profile.
    // ------------------=
    pub fn alias_add(
        &mut self,
        actor: u64,
        profile: &[u8],
        alias: &[u8],
        template: &[u8],
        now: u64,
    ) -> Result<(), ProfileError> {
        let index = self.profile_index(profile).ok_or(ProfileError::NotFound)?;
        if self.profiles[index].built_in {
            return Err(ProfileError::Immutable);
        }
        if self.profiles[index].owner_identity != actor {
            return Err(ProfileError::AccessDenied);
        }
        Self::insert_alias(&mut self.profiles[index], alias, template)?;
        self.profiles[index].modified = now;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: insert_alias
    // DESC: Validates command spoofing and canonical-template rules before committing an alias.
    // ------------------=
    fn insert_alias(
        profile: &mut ShellProfile,
        alias: &[u8],
        template: &[u8],
    ) -> Result<(), ProfileError> {
        validate_alias_name(alias)?;
        if is_native_command(alias) {
            return Err(ProfileError::Immutable);
        }
        validate_template(template)?;
        if profile.aliases[..profile.alias_count as usize]
            .iter()
            .any(|mapping| mapping.name.as_bytes() == alias)
        {
            return Err(ProfileError::Duplicate);
        }
        let index = profile.alias_count as usize;
        if index >= MAX_ALIASES_PER_PROFILE {
            return Err(ProfileError::Capacity);
        }
        profile.aliases[index] = AliasMapping {
            name: ByteText::new(alias).map_err(|_| ProfileError::InvalidName)?,
            template: ByteText::new(template).map_err(|_| ProfileError::InvalidTemplate)?,
            enabled: true,
        };
        profile.alias_count += 1;
        Ok(())
    }

    // ------------------------=
    // FUNC: alias_delete
    // DESC: Removes exactly one authorized alias and invalidates it immediately.
    // ------------------=
    pub fn alias_delete(
        &mut self,
        actor: u64,
        profile: &[u8],
        alias: &[u8],
        now: u64,
    ) -> Result<(), ProfileError> {
        let index = self.profile_index(profile).ok_or(ProfileError::NotFound)?;
        let profile = &mut self.profiles[index];
        if profile.built_in {
            return Err(ProfileError::Immutable);
        }
        if profile.owner_identity != actor {
            return Err(ProfileError::AccessDenied);
        }
        let at = profile.aliases[..profile.alias_count as usize]
            .iter()
            .position(|mapping| mapping.name.as_bytes() == alias)
            .ok_or(ProfileError::NotFound)?;
        for item in at..profile.alias_count as usize - 1 {
            profile.aliases[item] = profile.aliases[item + 1];
        }
        profile.alias_count -= 1;
        profile.aliases[profile.alias_count as usize] = AliasMapping::empty();
        profile.modified = now;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: resolve
    // DESC: Resolves native commands first and aliases by deterministic priority with shadow reporting.
    // ------------------=
    pub fn resolve(&self, input: &[u8]) -> Result<AliasResolution, ProfileError> {
        if input.is_empty() || input.len() > MAX_COMMAND {
            return Err(ProfileError::InvalidTemplate);
        }
        let split = input
            .iter()
            .position(|byte| byte.is_ascii_whitespace())
            .unwrap_or(input.len());
        let command = &input[..split];
        if is_native_command(command) {
            return Ok(AliasResolution {
                canonical: ByteText::new(input).map_err(|_| ProfileError::InvalidTemplate)?,
                resolved_profile: 1,
                shadowed_profiles: [0; MAX_PROFILES],
                shadowed_count: 0,
                native: true,
            });
        }
        let mut selected: Option<(i16, u32, &[u8])> = None;
        let mut shadowed = [0u32; MAX_PROFILES];
        let mut shadowed_count = 0usize;
        for profile in self.profiles[..self.profile_count as usize].iter() {
            if !profile.enabled {
                continue;
            }
            let Some(mapping) = profile.aliases[..profile.alias_count as usize]
                .iter()
                .find(|mapping| mapping.enabled && mapping.name.as_bytes() == command)
            else {
                continue;
            };
            if selected
                .map(|current| {
                    (profile.priority, core::cmp::Reverse(profile.profile_id))
                        > (current.0, core::cmp::Reverse(current.1))
                })
                .unwrap_or(true)
            {
                if let Some(current) = selected {
                    shadowed[shadowed_count] = current.1;
                    shadowed_count += 1;
                }
                selected = Some((
                    profile.priority,
                    profile.profile_id,
                    mapping.template.as_bytes(),
                ));
            } else if shadowed_count < MAX_PROFILES {
                shadowed[shadowed_count] = profile.profile_id;
                shadowed_count += 1;
            }
        }
        let (_, profile_id, template) = selected.ok_or(ProfileError::NotFound)?;
        let mut expanded = [0u8; MAX_COMMAND];
        let mut length = template.len();
        expanded[..length].copy_from_slice(template);
        if split < input.len() {
            let suffix = &input[split..];
            if length + suffix.len() > expanded.len() {
                return Err(ProfileError::InvalidTemplate);
            }
            expanded[length..length + suffix.len()].copy_from_slice(suffix);
            length += suffix.len();
        }
        Ok(AliasResolution {
            canonical: ByteText::new(&expanded[..length])
                .map_err(|_| ProfileError::InvalidTemplate)?,
            resolved_profile: profile_id,
            shadowed_profiles: shadowed,
            shadowed_count: shadowed_count as u8,
            native: false,
        })
    }

    // ------------------------=
    // FUNC: encode
    // DESC: Encodes persistent declarative profile state with fixed bounds and a checksum.
    // ------------------=
    pub fn encode(&self) -> [u8; PROFILE_STATE_BYTES] {
        let mut output = [0u8; PROFILE_STATE_BYTES];
        output[..8].copy_from_slice(PROFILE_STATE_MAGIC);
        put_u16(&mut output, 8, PROFILE_STATE_VERSION);
        put_u16(&mut output, 10, self.profile_count as u16);
        put_u32(&mut output, 12, self.next_profile_id);
        put_u32(&mut output, 16, self.default_profile);
        put_u64(&mut output, 20, self.revision);
        let mut at = 32usize;
        for profile in self.profiles[..self.profile_count as usize].iter() {
            put_u32(&mut output, at, profile.profile_id);
            put_u64(&mut output, at + 4, profile.owner_identity);
            put_u16(&mut output, at + 12, profile.priority as u16);
            output[at + 14] = profile.kind as u8;
            output[at + 15] = profile.built_in as u8;
            output[at + 16] = profile.enabled as u8;
            output[at + 17] = profile.alias_count;
            put_u64(&mut output, at + 18, profile.created);
            put_u64(&mut output, at + 26, profile.modified);
            output[at + 34] = profile.name.as_bytes().len() as u8;
            output[at + 35..at + 35 + profile.name.as_bytes().len()]
                .copy_from_slice(profile.name.as_bytes());
            output[at + 59] = profile.description.as_bytes().len() as u8;
            output[at + 60..at + 60 + profile.description.as_bytes().len()]
                .copy_from_slice(profile.description.as_bytes());
            at += 124;
            for alias in profile.aliases[..profile.alias_count as usize].iter() {
                output[at] = alias.name.as_bytes().len() as u8;
                output[at + 1] = alias.template.as_bytes().len() as u8;
                output[at + 2] = alias.enabled as u8;
                output[at + 4..at + 4 + alias.name.as_bytes().len()]
                    .copy_from_slice(alias.name.as_bytes());
                output[at + 24..at + 24 + alias.template.as_bytes().len()]
                    .copy_from_slice(alias.template.as_bytes());
                at += 184;
            }
        }
        let state_checksum = checksum(&output[..PROFILE_STATE_BYTES - 4]);
        put_u32(&mut output, PROFILE_STATE_BYTES - 4, state_checksum);
        output
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Restores persistent profiles only after validating framing, checksum, built-ins, and aliases.
    // ------------------=
    pub fn decode(input: &[u8]) -> Result<Self, ProfileError> {
        if input.len() != PROFILE_STATE_BYTES
            || &input[..8] != PROFILE_STATE_MAGIC
            || get_u16(input, 8) != PROFILE_STATE_VERSION
            || get_u32(input, PROFILE_STATE_BYTES - 4)
                != checksum(&input[..PROFILE_STATE_BYTES - 4])
        {
            return Err(ProfileError::CorruptState);
        }
        let count = get_u16(input, 10) as usize;
        if !(3..=MAX_PROFILES).contains(&count) {
            return Err(ProfileError::CorruptState);
        }
        let mut service = Self {
            profiles: [ShellProfile::empty(); MAX_PROFILES],
            profile_count: count as u8,
            next_profile_id: get_u32(input, 12),
            default_profile: get_u32(input, 16),
            revision: get_u64(input, 20),
        };
        let mut at = 32usize;
        for index in 0..count {
            if at + 124 > PROFILE_STATE_BYTES - 4 {
                return Err(ProfileError::CorruptState);
            }
            let name_length = input[at + 34] as usize;
            let description_length = input[at + 59] as usize;
            let alias_count = input[at + 17] as usize;
            if name_length > PROFILE_NAME_CAPACITY
                || description_length > 64
                || alias_count > MAX_ALIASES_PER_PROFILE
            {
                return Err(ProfileError::CorruptState);
            }
            let kind = match input[at + 14] {
                0 => ProfileKind::Native,
                1 => ProfileKind::Compatibility,
                2 => ProfileKind::User,
                _ => return Err(ProfileError::CorruptState),
            };
            let mut profile = ShellProfile {
                profile_id: get_u32(input, at),
                owner_identity: get_u64(input, at + 4),
                priority: get_u16(input, at + 12) as i16,
                kind,
                built_in: input[at + 15] != 0,
                enabled: input[at + 16] != 0,
                alias_count: alias_count as u8,
                created: get_u64(input, at + 18),
                modified: get_u64(input, at + 26),
                name: ByteText::new(&input[at + 35..at + 35 + name_length])
                    .map_err(|_| ProfileError::CorruptState)?,
                description: ByteText::new(&input[at + 60..at + 60 + description_length])
                    .map_err(|_| ProfileError::CorruptState)?,
                aliases: [AliasMapping::empty(); MAX_ALIASES_PER_PROFILE],
            };
            at += 124;
            for alias_index in 0..alias_count {
                if at + 184 > PROFILE_STATE_BYTES - 4 {
                    return Err(ProfileError::CorruptState);
                }
                let name_length = input[at] as usize;
                let template_length = input[at + 1] as usize;
                if name_length > ALIAS_NAME_CAPACITY || template_length > MAX_COMMAND {
                    return Err(ProfileError::CorruptState);
                }
                let name = &input[at + 4..at + 4 + name_length];
                let template = &input[at + 24..at + 24 + template_length];
                validate_alias_name(name).map_err(|_| ProfileError::CorruptState)?;
                validate_template(template).map_err(|_| ProfileError::CorruptState)?;
                profile.aliases[alias_index] = AliasMapping {
                    name: ByteText::new(name).map_err(|_| ProfileError::CorruptState)?,
                    template: ByteText::new(template).map_err(|_| ProfileError::CorruptState)?,
                    enabled: input[at + 2] != 0,
                };
                at += 184;
            }
            service.profiles[index] = profile;
        }
        if service.profiles[0].profile_id != 1
            || service.profiles[0].name.as_bytes() != b"infinity.native"
            || !service.profiles[0].enabled
            || !service.profiles[0].built_in
        {
            return Err(ProfileError::CorruptState);
        }
        Ok(service)
    }

    // ------------------------=
    // FUNC: profile_index
    // DESC: Resolves a canonical profile name or stable built-in short name.
    // ------------------=
    fn profile_index(&self, name: &[u8]) -> Option<usize> {
        let canonical = match name {
            b"native" => b"infinity.native".as_slice(),
            b"linux" => b"compat.linux".as_slice(),
            b"unix" => b"compat.unix".as_slice(),
            _ => name,
        };
        self.profiles[..self.profile_count as usize]
            .iter()
            .position(|profile| profile.name.as_bytes() == canonical)
    }

    // ------------------------=
    // FUNC: profile_index_by_id
    // DESC: Resolves one profile slot by stable profile identity.
    // ------------------=
    fn profile_index_by_id(&self, id: u32) -> Option<usize> {
        self.profiles[..self.profile_count as usize]
            .iter()
            .position(|profile| profile.profile_id == id)
    }
}

impl Default for ShellProfileService {
    // ------------------------=
    // FUNC: default
    // DESC: Returns the built-in native Shell Profile set.
    // ------------------=
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileNavigatorState {
    pub active_namespace_ref: ByteText<MAX_NAMESPACE_PATH>,
    pub back_namespace_ref: ByteText<MAX_NAMESPACE_PATH>,
    pub forward_namespace_ref: ByteText<MAX_NAMESPACE_PATH>,
    pub view_mode: ViewMode,
    pub inspector_open: bool,
    pub selected_reference_id: u64,
    pub scroll_offset: usize,
    pub sort_key: u8,
    pub sort_descending: bool,
    pub selected_index: u16,
    pub location_editing: bool,
    pub rename_editing: bool,
    pub editor_text: ByteText<MAX_NAMESPACE_PATH>,
    pub editor_cursor: usize,
    pub context_menu_open: bool,
    pub context_x: i32,
    pub context_y: i32,
    pub context_item: u16,
    history: [ByteText<MAX_NAMESPACE_PATH>; FILE_NAVIGATOR_HISTORY_CAPACITY],
    history_length: u8,
    history_cursor: u8,
}

impl FileNavigatorState {
    // ------------------------=
    // FUNC: new
    // DESC: Creates native File Navigator UI state around an explicit NamespaceRef.
    // ------------------=
    pub fn new(path: &[u8]) -> Result<Self, NavigationError> {
        let path = normalize_absolute_path(path)?;
        let mut history = [ByteText::empty(); FILE_NAVIGATOR_HISTORY_CAPACITY];
        history[0] = path;
        Ok(Self {
            active_namespace_ref: path,
            back_namespace_ref: ByteText::empty(),
            forward_namespace_ref: ByteText::empty(),
            view_mode: ViewMode::List,
            inspector_open: true,
            selected_reference_id: 0,
            scroll_offset: 0,
            sort_key: 0,
            sort_descending: false,
            selected_index: FILE_NAVIGATOR_NO_SELECTION,
            location_editing: false,
            rename_editing: false,
            editor_text: ByteText::empty(),
            editor_cursor: 0,
            context_menu_open: false,
            context_x: 0,
            context_y: 0,
            context_item: FILE_NAVIGATOR_NO_SELECTION,
            history,
            history_length: 1,
            history_cursor: 0,
        })
    }

    // ------------------------=
    // FUNC: navigate
    // DESC: Pushes bounded back/forward history while changing the active explicit NamespaceRef.
    // ------------------=
    pub fn navigate(&mut self, path: &[u8]) -> Result<(), NavigationError> {
        let path = normalize_absolute_path(path)?;
        if path != self.active_namespace_ref {
            self.back_namespace_ref = self.active_namespace_ref;
            let next = self.history_cursor as usize + 1;
            self.history_length = next.min(FILE_NAVIGATOR_HISTORY_CAPACITY - 1) as u8;
            if next < FILE_NAVIGATOR_HISTORY_CAPACITY {
                self.history[next] = path;
                self.history_cursor = next as u8;
                self.history_length = (next + 1) as u8;
            } else {
                self.history
                    .copy_within(1..FILE_NAVIGATOR_HISTORY_CAPACITY, 0);
                self.history[FILE_NAVIGATOR_HISTORY_CAPACITY - 1] = path;
                self.history_cursor = (FILE_NAVIGATOR_HISTORY_CAPACITY - 1) as u8;
                self.history_length = FILE_NAVIGATOR_HISTORY_CAPACITY as u8;
            }
            self.active_namespace_ref = path;
            self.forward_namespace_ref = ByteText::empty();
            self.selected_reference_id = 0;
            self.selected_index = FILE_NAVIGATOR_NO_SELECTION;
            self.scroll_offset = 0;
            self.context_menu_open = false;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: navigate_navigation_entry
    // DESC: Applies traditional current and parent directory behavior for the virtual dot rows.
    // ------------------=
    pub fn navigate_navigation_entry(
        &mut self,
        index: usize,
    ) -> Result<bool, NavigationError> {
        let destination = match index {
            0 => self.active_namespace_ref,
            1 => parent_path(self.active_namespace_ref.as_bytes())?,
            _ => return Ok(false),
        };
        self.navigate(destination.as_bytes())?;
        Ok(true)
    }

    // ------------------------=
    // FUNC: back
    // DESC: Swaps the active and previous explicit Namespace references.
    // ------------------=
    pub fn back(&mut self) -> Result<(), NavigationError> {
        if self.history_cursor == 0 {
            return Err(NavigationError::NoPreviousNamespace);
        }
        self.forward_namespace_ref = self.active_namespace_ref;
        self.history_cursor -= 1;
        self.active_namespace_ref = if self.history_cursor == 0 {
            self.back_namespace_ref
        } else {
            self.history[self.history_cursor as usize]
        };
        self.back_namespace_ref = if self.history_cursor == 0 {
            ByteText::empty()
        } else if self.history_cursor == 1 {
            self.history[0]
        } else {
            self.history[self.history_cursor as usize - 1]
        };
        self.selected_index = FILE_NAVIGATOR_NO_SELECTION;
        Ok(())
    }

    // ------------------------=
    // FUNC: forward
    // DESC: Restores the explicit Namespace reference most recently left by Back.
    // ------------------=
    pub fn forward(&mut self) -> Result<(), NavigationError> {
        if self.history_cursor as usize + 1 >= self.history_length as usize {
            return Err(NavigationError::NoPreviousNamespace);
        }
        self.back_namespace_ref = self.active_namespace_ref;
        self.history_cursor += 1;
        self.active_namespace_ref = self.history[self.history_cursor as usize];
        self.forward_namespace_ref =
            if self.history_cursor as usize + 1 < self.history_length as usize {
                self.history[self.history_cursor as usize + 1]
            } else {
                ByteText::empty()
            };
        self.selected_index = FILE_NAVIGATOR_NO_SELECTION;
        Ok(())
    }

    // ------------------------=
    // FUNC: begin_location_edit
    // DESC: Focuses the location editor with the active NamespaceRef selected for editing.
    // ------------------=
    pub fn begin_location_edit(&mut self) {
        self.editor_text = self.active_namespace_ref;
        self.editor_cursor = self.editor_text.as_bytes().len();
        self.location_editing = true;
        self.rename_editing = false;
        self.context_menu_open = false;
    }

    // ------------------------=
    // FUNC: begin_rename
    // DESC: Focuses the bounded inline rename editor for one selected namespace entry.
    // ------------------=
    pub fn begin_rename(&mut self, name: &[u8]) -> bool {
        let Ok(text) = ByteText::new(name) else {
            return false;
        };
        self.editor_text = text;
        self.editor_cursor = self.editor_text.as_bytes().len();
        self.rename_editing = true;
        self.location_editing = false;
        self.context_menu_open = false;
        true
    }

    // ------------------------=
    // FUNC: cancel_edit
    // DESC: Cancels any location or rename edit without changing namespace state.
    // ------------------=
    pub fn cancel_edit(&mut self) {
        self.location_editing = false;
        self.rename_editing = false;
        self.editor_text = ByteText::empty();
        self.editor_cursor = 0;
    }

    // ------------------------=
    // FUNC: open_context_menu
    // DESC: Opens the native context menu at a compositor-relative pointer location.
    // ------------------=
    pub fn open_context_menu(&mut self, x: i32, y: i32, item: Option<usize>) {
        self.context_menu_open = true;
        self.context_x = x;
        self.context_y = y;
        self.context_item = item
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or(FILE_NAVIGATOR_NO_SELECTION);
        self.selected_index = self.context_item;
        self.location_editing = false;
        self.rename_editing = false;
    }

    // ------------------------=
    // FUNC: move_selection
    // DESC: Moves list selection without skipping the first item when no row is selected.
    // ------------------=
    pub fn move_selection(&mut self, count: usize, previous: bool) {
        if count == 0 {
            self.selected_index = FILE_NAVIGATOR_NO_SELECTION;
            return;
        }
        let next = if self.selected_index == FILE_NAVIGATOR_NO_SELECTION {
            if previous { count - 1 } else { 0 }
        } else if previous {
            (self.selected_index as usize).saturating_sub(1)
        } else {
            (self.selected_index as usize + 1).min(count - 1)
        };
        self.selected_index = next as u16;
    }

    // ------------------------=
    // FUNC: scroll_by
    // DESC: Applies bounded File Navigator scrolling so wheel input cannot move beyond the final visible page.
    // ------------------=
    pub fn scroll_by(
        &mut self,
        delta: isize,
        total: usize,
        viewport: usize,
        item_extent: usize,
    ) {
        let content_height = total.saturating_mul(item_extent.max(1));
        let maximum = content_height.saturating_sub(viewport);
        self.scroll_offset = if delta < 0 {
            self.scroll_offset.saturating_sub(delta.unsigned_abs())
        } else {
            self.scroll_offset.saturating_add(delta as usize).min(maximum)
        };
    }

    // ------------------------=
    // FUNC: visible_range
    // DESC: Computes a bounded virtualized result window for arbitrarily large namespaces.
    // ------------------=
    pub fn visible_range(
        total: usize,
        scroll: usize,
        viewport: usize,
        item_extent: usize,
    ) -> (usize, usize) {
        let extent = item_extent.max(1);
        let first = (scroll / extent).min(total);
        let visible = viewport.saturating_add(extent - 1) / extent + 2;
        (first, first.saturating_add(visible).min(total))
    }
}

// ------------------------=
// FUNC: is_immediate_namespace_child
// DESC: Reports whether a candidate NamespaceRef is one direct child of a parent.
// ------------------=
pub fn is_immediate_namespace_child(parent: &[u8], candidate: &[u8]) -> bool {
    if candidate == parent || !candidate.starts_with(parent) {
        return false;
    }
    let remainder = if parent == b"/" {
        &candidate[1..]
    } else {
        if candidate.get(parent.len()) != Some(&b'/') {
            return false;
        }
        &candidate[parent.len() + 1..]
    };
    !remainder.is_empty() && !remainder.contains(&b'/')
}

// ------------------------=
// FUNC: namespace_basename
// DESC: Returns the final human-readable component of an absolute NamespaceRef.
// ------------------=
pub fn namespace_basename(path: &[u8]) -> &[u8] {
    path.iter()
        .rposition(|value| *value == b'/')
        .map(|index| &path[index + 1..])
        .filter(|value| !value.is_empty())
        .unwrap_or(b"/")
}

// ------------------------=
// FUNC: namespace_child_path
// DESC: Builds and validates an absolute child NamespaceRef from a parent and leaf name.
// ------------------=
pub fn namespace_child_path(
    parent: &[u8],
    name: &[u8],
) -> Result<ByteText<MAX_NAMESPACE_PATH>, NavigationError> {
    if name.is_empty() || name.contains(&b'/') || matches!(name, b"." | b"..") {
        return Err(NavigationError::InvalidPath);
    }
    join_path(parent, name)
}

pub const NATIVE_COMMANDS: &[&[u8]] = &[
    b"path",
    b"idir",
    b"cd",
    b"list",
    b"examine",
    b"resolve",
    b"references",
    b"versions",
    b"relationships",
    b"find",
    b"open",
    b"namespace",
    b"object",
    b"reference",
    b"trash",
    b"space",
    b"shell",
    b"navigator",
    b"commands",
    b"help",
    b"console",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeCommandRegistration {
    pub command: &'static [u8],
    pub operation: super::iop::OperationId,
}

pub const NATIVE_OPERATION_REGISTRY: &[NativeCommandRegistration] = &[
    NativeCommandRegistration {
        command: b"path",
        operation: super::iop::OperationId::NamespaceResolve,
    },
    NativeCommandRegistration {
        command: b"idir",
        operation: super::iop::OperationId::NamespaceResolve,
    },
    NativeCommandRegistration {
        command: b"cd",
        operation: super::iop::OperationId::NamespaceResolve,
    },
    NativeCommandRegistration {
        command: b"list",
        operation: super::iop::OperationId::NamespaceList,
    },
    NativeCommandRegistration {
        command: b"examine",
        operation: super::iop::OperationId::ObjectInspect,
    },
    NativeCommandRegistration {
        command: b"resolve",
        operation: super::iop::OperationId::NamespaceResolve,
    },
    NativeCommandRegistration {
        command: b"references",
        operation: super::iop::OperationId::NamespaceList,
    },
    NativeCommandRegistration {
        command: b"versions",
        operation: super::iop::OperationId::ObjectHistory,
    },
    NativeCommandRegistration {
        command: b"relationships",
        operation: super::iop::OperationId::ObjectRelationships,
    },
    NativeCommandRegistration {
        command: b"find",
        operation: super::iop::OperationId::ObjectSearch,
    },
    NativeCommandRegistration {
        command: b"open",
        operation: super::iop::OperationId::ApplicationAssociationResolve,
    },
    NativeCommandRegistration {
        command: b"namespace create",
        operation: super::iop::OperationId::NamespaceCreate,
    },
    NativeCommandRegistration {
        command: b"namespace delete",
        operation: super::iop::OperationId::NamespaceDelete,
    },
    NativeCommandRegistration {
        command: b"namespace move",
        operation: super::iop::OperationId::NamespaceMove,
    },
    NativeCommandRegistration {
        command: b"namespace list",
        operation: super::iop::OperationId::NamespaceList,
    },
    NativeCommandRegistration {
        command: b"object create",
        operation: super::iop::OperationId::ObjectCreate,
    },
    NativeCommandRegistration {
        command: b"object copy",
        operation: super::iop::OperationId::ObjectCopy,
    },
    NativeCommandRegistration {
        command: b"object delete",
        operation: super::iop::OperationId::ObjectDelete,
    },
    NativeCommandRegistration {
        command: b"object inspect",
        operation: super::iop::OperationId::ObjectInspect,
    },
    NativeCommandRegistration {
        command: b"object history",
        operation: super::iop::OperationId::ObjectHistory,
    },
    NativeCommandRegistration {
        command: b"object relationships",
        operation: super::iop::OperationId::ObjectRelationships,
    },
    NativeCommandRegistration {
        command: b"object destroy",
        operation: super::iop::OperationId::ObjectDestroy,
    },
    NativeCommandRegistration {
        command: b"reference create",
        operation: super::iop::OperationId::NamespaceAttach,
    },
    NativeCommandRegistration {
        command: b"reference delete",
        operation: super::iop::OperationId::NamespaceDetach,
    },
    NativeCommandRegistration {
        command: b"reference list",
        operation: super::iop::OperationId::NamespaceList,
    },
    NativeCommandRegistration {
        command: b"trash add",
        operation: super::iop::OperationId::TrashMove,
    },
    NativeCommandRegistration {
        command: b"trash list",
        operation: super::iop::OperationId::TrashList,
    },
    NativeCommandRegistration {
        command: b"trash restore",
        operation: super::iop::OperationId::TrashRestore,
    },
    NativeCommandRegistration {
        command: b"trash delete",
        operation: super::iop::OperationId::TrashDelete,
    },
    NativeCommandRegistration {
        command: b"trash empty",
        operation: super::iop::OperationId::TrashEmpty,
    },
    NativeCommandRegistration {
        command: b"shell profile list",
        operation: super::iop::OperationId::ShellProfileList,
    },
    NativeCommandRegistration {
        command: b"shell profile inspect",
        operation: super::iop::OperationId::ShellProfileInspect,
    },
    NativeCommandRegistration {
        command: b"shell profile create",
        operation: super::iop::OperationId::ShellProfileCreate,
    },
    NativeCommandRegistration {
        command: b"shell profile clone",
        operation: super::iop::OperationId::ShellProfileClone,
    },
    NativeCommandRegistration {
        command: b"shell profile enable",
        operation: super::iop::OperationId::ShellProfileEnable,
    },
    NativeCommandRegistration {
        command: b"shell profile disable",
        operation: super::iop::OperationId::ShellProfileDisable,
    },
    NativeCommandRegistration {
        command: b"shell profile set-default",
        operation: super::iop::OperationId::ShellProfileSetDefault,
    },
    NativeCommandRegistration {
        command: b"shell profile delete",
        operation: super::iop::OperationId::ShellProfileDelete,
    },
    NativeCommandRegistration {
        command: b"shell alias list",
        operation: super::iop::OperationId::ShellAliasList,
    },
    NativeCommandRegistration {
        command: b"shell alias add",
        operation: super::iop::OperationId::ShellAliasAdd,
    },
    NativeCommandRegistration {
        command: b"shell alias delete",
        operation: super::iop::OperationId::ShellAliasDelete,
    },
    NativeCommandRegistration {
        command: b"shell alias resolve",
        operation: super::iop::OperationId::ShellAliasResolve,
    },
    NativeCommandRegistration {
        command: b"navigator",
        operation: super::iop::OperationId::ApplicationLaunch,
    },
];

// ------------------------=
// FUNC: native_operation
// DESC: Resolves an exact protected command signature to its typed IOP operation.
// ------------------=
pub fn native_operation(command: &[u8]) -> Option<super::iop::OperationId> {
    NATIVE_OPERATION_REGISTRY
        .iter()
        .find(|registration| registration.command == command)
        .map(|registration| registration.operation)
}

const LINUX_ALIASES: &[(&[u8], &[u8])] = &[
    (b"pwd", b"path"),
    (b"ls", b"list"),
    (b"tree", b"list tree"),
    (b"stat", b"examine"),
    (b"cp", b"object copy"),
    (b"mv", b"namespace move"),
    (b"ln", b"reference create"),
    (b"unlink", b"reference delete"),
    (b"rm", b"object delete"),
    (b"mkdir", b"namespace create"),
];

const UNIX_ALIASES: &[(&[u8], &[u8])] = &[
    (b"pwd", b"path"),
    (b"ls", b"list"),
    (b"cp", b"object copy"),
    (b"mv", b"namespace move"),
    (b"rm", b"object delete"),
    (b"mkdir", b"namespace create"),
    (b"cat", b"object read-text"),
    (b"find", b"find"),
];

// ------------------------=
// FUNC: is_native_command
// DESC: Determines whether the first token is protected canonical Infinity Native vocabulary.
// ------------------=
pub fn is_native_command(command: &[u8]) -> bool {
    NATIVE_COMMANDS.iter().any(|native| *native == command)
}

// ------------------------=
// FUNC: validate_profile_name
// DESC: Rejects deceptive, empty, overlong, or reserved Shell Profile names.
// ------------------=
fn validate_profile_name(name: &[u8]) -> Result<(), ProfileError> {
    if name.is_empty()
        || name.len() > PROFILE_NAME_CAPACITY
        || !name.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(*byte, b'-' | b'.')
        })
        || matches!(name, b"infinity.native" | b"compat.linux" | b"compat.unix")
    {
        return Err(ProfileError::InvalidName);
    }
    Ok(())
}

// ------------------------=
// FUNC: validate_alias_name
// DESC: Rejects Unicode spoofing, controls, separators, whitespace, and invalid alias names.
// ------------------=
fn validate_alias_name(name: &[u8]) -> Result<(), ProfileError> {
    if name.is_empty()
        || name.len() > ALIAS_NAME_CAPACITY
        || !name
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        return Err(ProfileError::InvalidName);
    }
    Ok(())
}

// ------------------------=
// FUNC: validate_template
// DESC: Restricts aliases to bounded canonical command templates without control operators or recursion.
// ------------------=
fn validate_template(template: &[u8]) -> Result<(), ProfileError> {
    if template.is_empty()
        || template.len() > MAX_COMMAND
        || template.iter().any(|byte| {
            !byte.is_ascii()
                || byte.is_ascii_control()
                || matches!(*byte, b';' | b'|' | b'&' | b'`' | b'$')
        })
    {
        return Err(ProfileError::InvalidTemplate);
    }
    let command = template
        .split(|byte| byte.is_ascii_whitespace())
        .next()
        .unwrap_or(&[]);
    if !is_native_command(command) {
        return Err(ProfileError::InvalidTemplate);
    }
    Ok(())
}

// ------------------------=
// FUNC: normalize_absolute_path
// DESC: Normalizes an absolute human Namespace path without creating filesystem semantics.
// ------------------=
pub fn normalize_absolute_path(
    path: &[u8],
) -> Result<ByteText<MAX_NAMESPACE_PATH>, NavigationError> {
    if path.is_empty()
        || path.first() != Some(&b'/')
        || path.len() > MAX_NAMESPACE_PATH
        || core::str::from_utf8(path).is_err()
        || path
            .iter()
            .any(|byte| byte.is_ascii_control() || !byte.is_ascii())
    {
        return Err(NavigationError::InvalidPath);
    }
    let mut output = [0u8; MAX_NAMESPACE_PATH];
    let mut length = 1usize;
    output[0] = b'/';
    for component in path
        .split(|byte| *byte == b'/')
        .filter(|part| !part.is_empty())
    {
        if component == b"." {
            continue;
        }
        if component == b".." {
            while length > 1 && output[length - 1] != b'/' {
                length -= 1;
            }
            if length > 1 {
                length -= 1;
            }
            continue;
        }
        if component.iter().any(|byte| matches!(*byte, b'\\' | b':')) {
            return Err(NavigationError::InvalidPath);
        }
        if length > 1 {
            if length == output.len() {
                return Err(NavigationError::InvalidPath);
            }
            output[length] = b'/';
            length += 1;
        }
        if length + component.len() > output.len() {
            return Err(NavigationError::InvalidPath);
        }
        output[length..length + component.len()].copy_from_slice(component);
        length += component.len();
    }
    ByteText::new(&output[..length]).map_err(|_| NavigationError::InvalidPath)
}

// ------------------------=
// FUNC: parent_path
// DESC: Resolves the parent of an explicit NamespaceRef while keeping root bounded.
// ------------------=
pub fn parent_path(path: &[u8]) -> Result<ByteText<MAX_NAMESPACE_PATH>, NavigationError> {
    let normalized = normalize_absolute_path(path)?;
    let path = normalized.as_bytes();
    if path == b"/" {
        return Ok(normalized);
    }
    let split = path.iter().rposition(|byte| *byte == b'/').unwrap_or(0);
    normalize_absolute_path(if split == 0 { b"/" } else { &path[..split] })
}

// ------------------------=
// FUNC: join_path
// DESC: Joins a relative human name to an explicit current NamespaceRef and normalizes it.
// ------------------=
pub fn join_path(
    base: &[u8],
    relative: &[u8],
) -> Result<ByteText<MAX_NAMESPACE_PATH>, NavigationError> {
    if relative.is_empty() || relative.first() == Some(&b'/') {
        return Err(NavigationError::InvalidPath);
    }
    let mut output = [0u8; MAX_NAMESPACE_PATH];
    if base.len() + relative.len() + 1 > output.len() {
        return Err(NavigationError::InvalidPath);
    }
    output[..base.len()].copy_from_slice(base);
    let mut length = base.len();
    if base != b"/" {
        output[length] = b'/';
        length += 1;
    }
    output[length..length + relative.len()].copy_from_slice(relative);
    length += relative.len();
    normalize_absolute_path(&output[..length])
}

// ------------------------=
// FUNC: checksum
// DESC: Computes the bounded FNV-1a integrity checksum for Shell Profile state.
// ------------------=
fn checksum(input: &[u8]) -> u32 {
    let mut value = 0x811c_9dc5u32;
    for byte in input {
        value ^= *byte as u32;
        value = value.wrapping_mul(0x0100_0193);
    }
    value
}

// ------------------------=
// FUNC: put_u16
// DESC: Writes one little-endian profile-state integer.
// ------------------=
fn put_u16(output: &mut [u8], at: usize, value: u16) {
    output[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: put_u32
// DESC: Writes one little-endian profile-state integer.
// ------------------=
fn put_u32(output: &mut [u8], at: usize, value: u32) {
    output[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: put_u64
// DESC: Writes one little-endian profile-state integer.
// ------------------=
fn put_u64(output: &mut [u8], at: usize, value: u64) {
    output[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: get_u16
// DESC: Reads one little-endian profile-state integer.
// ------------------=
fn get_u16(input: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([input[at], input[at + 1]])
}

// ------------------------=
// FUNC: get_u32
// DESC: Reads one little-endian profile-state integer.
// ------------------=
fn get_u32(input: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([input[at], input[at + 1], input[at + 2], input[at + 3]])
}

// ------------------------=
// FUNC: get_u64
// DESC: Reads one little-endian profile-state integer.
// ------------------=
fn get_u64(input: &[u8], at: usize) -> u64 {
    u64::from_le_bytes([
        input[at],
        input[at + 1],
        input[at + 2],
        input[at + 3],
        input[at + 4],
        input[at + 5],
        input[at + 6],
        input[at + 7],
    ])
}
