//! Bounded application handlers, per-profile defaults, and typed open requests.

pub const MAX_APPLICATION_HANDLERS: usize = 12;
pub const MAX_DEFAULT_ASSOCIATIONS: usize = 7;
pub const MAX_HANDLER_EXTENSIONS: usize = 6;
pub const MAX_EXTENSION_BYTES: usize = 12;
pub const MAX_APPLICATION_NAME_BYTES: usize = 32;
pub const MAX_APPLICATION_ID_BYTES: usize = 48;
pub const ASSOCIATION_STATE_BYTES: usize = 512;
const ASSOCIATION_STATE_MAGIC: &[u8; 8] = b"INFASSO1";
const ASSOCIATION_STATE_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ContentKind { Binary=1, Utf8Text=2, Namespace=3, System=4, Application=5, Metadata=6 }

impl ContentKind {
    // ------------------------=
    // FUNC: from_u8
    // DESC: Converts one stable ObjectStore content discriminator into an association content kind.
    // ------------------=
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value { 1=>Some(Self::Binary),2=>Some(Self::Utf8Text),3=>Some(Self::Namespace),
            4=>Some(Self::System),5=>Some(Self::Application),6=>Some(Self::Metadata),_=>None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchAdapter {
    TextEditor,
    FileNavigator,
    InstalledApplication(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationError {
    UnknownApplication,
    IncompatibleApplication,
    ApplicationUnavailable,
    Capacity,
    InvalidManifest,
    CorruptState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundedText<const N: usize> {
    bytes: [u8; N],
    length: u8,
}

impl<const N: usize> BoundedText<N> {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates an empty bounded application identity, label, or extension.
    // ------------------=
    pub const fn empty() -> Self {
        Self { bytes: [0; N], length: 0 }
    }

    // ------------------------=
    // FUNC: literal
    // DESC: Copies one compile-time trusted built-in identifier into bounded storage.
    // ------------------=
    pub const fn literal(value: &[u8]) -> Self {
        let mut result = Self::empty();
        let mut index = 0;
        while index < value.len() && index < N {
            result.bytes[index] = value[index];
            index += 1;
        }
        result.length = index as u8;
        result
    }

    // ------------------------=
    // FUNC: new
    // DESC: Copies one nonempty ASCII application identity, label, or extension into bounded storage.
    // ------------------=
    pub fn new(value: &[u8]) -> Result<Self, AssociationError> {
        if value.is_empty() || value.len() > N || !value.iter().all(|byte| byte.is_ascii_graphic() || *byte == b' ') {
            return Err(AssociationError::InvalidManifest);
        }
        let mut result = Self::empty();
        result.bytes[..value.len()].copy_from_slice(value);
        result.length = value.len() as u8;
        Ok(result)
    }

    // ------------------------=
    // FUNC: bytes
    // DESC: Returns only initialized bounded text bytes.
    // ------------------=
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.length as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApplicationHandler {
    pub identity: BoundedText<MAX_APPLICATION_ID_BYTES>,
    pub display_name: BoundedText<MAX_APPLICATION_NAME_BYTES>,
    pub launch: LaunchAdapter,
    content_mask: u8,
    extensions: [BoundedText<MAX_EXTENSION_BYTES>; MAX_HANDLER_EXTENSIONS],
}

impl ApplicationHandler {
    // ------------------------=
    // FUNC: new
    // DESC: Creates one validated installed-application handler with no implicit file authority.
    // ------------------=
    pub fn new(identity: &[u8], display_name: &[u8], launch: LaunchAdapter) -> Result<Self, AssociationError> {
        if !identity.starts_with(b"app.") || identity.iter().any(|byte| !byte.is_ascii_alphanumeric() && !matches!(*byte, b'.' | b'-' | b'_')) {
            return Err(AssociationError::InvalidManifest);
        }
        Ok(Self {
            identity: BoundedText::new(identity)?,
            display_name: BoundedText::new(display_name)?,
            launch,
            content_mask: 0,
            extensions: [BoundedText::empty(); MAX_HANDLER_EXTENSIONS],
        })
    }

    // ------------------------=
    // FUNC: with_content_type
    // DESC: Adds one typed content capability to an application handler.
    // ------------------=
    pub const fn with_content_type(mut self, content_type: ContentKind) -> Self {
        self.content_mask |= 1u8 << ((content_type as u8).saturating_sub(1));
        self
    }

    // ------------------------=
    // FUNC: add_extension
    // DESC: Adds one lowercase filename extension capability without granting unrelated content types.
    // ------------------=
    pub fn add_extension(&mut self, extension: &[u8]) -> Result<(), AssociationError> {
        let extension = normalized_extension(extension).ok_or(AssociationError::InvalidManifest)?;
        if self.extensions.iter().any(|value| value.bytes() == extension) {
            return Ok(());
        }
        let slot = self.extensions.iter_mut().find(|value| value.length == 0).ok_or(AssociationError::Capacity)?;
        *slot = BoundedText::new(extension)?;
        Ok(())
    }

    // ------------------------=
    // FUNC: supports
    // DESC: Determines compatibility from the authoritative object content type and optional filename extension.
    // ------------------=
    pub fn supports(&self, content_type: ContentKind, extension: &[u8]) -> bool {
        let type_match = self.content_mask & (1u8 << ((content_type as u8).saturating_sub(1))) != 0;
        let extension = normalized_extension(extension).unwrap_or(&[]);
        let extension_match = !extension.is_empty()
            && self
                .extensions
                .iter()
                .any(|candidate| candidate.bytes().eq_ignore_ascii_case(extension));
        // Imported documents can have Binary metadata. Restrict the fallback
        // to document extensions; the editor still validates the actual bytes.
        let editable_document = self.launch == LaunchAdapter::TextEditor
            && matches!(content_type, ContentKind::Binary | ContentKind::Metadata)
            && [b"json".as_slice(), b"jsonl", b"txt", b"md", b"csv", b"xml",
                b"yaml", b"yml", b"toml", b"ini", b"log", b"c", b"h", b"rs",
                b"js", b"css", b"html", b"py", b"sh"]
                .iter().any(|suffix| extension.eq_ignore_ascii_case(suffix));
        type_match || extension_match || editable_document
    }

    // ------------------------=
    // FUNC: from_manifest
    // DESC: Builds a custom installed-app handler from bounded declarative identity and open-capability fields.
    // ------------------=
    pub fn from_manifest(manifest: &[u8]) -> Result<Self, AssociationError> {
        if manifest.len() > 4096 { return Err(AssociationError::InvalidManifest); }
        let mut identity = None;
        let mut display_name = None;
        let mut image = None;
        let mut content_types = None;
        let mut extensions = None;
        for line in manifest.split(|byte| *byte == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            let Some(separator) = line.iter().position(|byte| *byte == b'=') else { continue; };
            let (key, value) = (&line[..separator], &line[separator + 1..]);
            match key {
                b"identity" => identity = Some(value),
                b"display-name" => display_name = Some(value),
                b"image-id" => image = parse_image_id(value),
                b"open-content-types" => content_types = Some(value),
                b"open-extensions" => extensions = Some(value),
                _ => {}
            }
        }
        let mut handler = Self::new(
            identity.ok_or(AssociationError::InvalidManifest)?,
            display_name.ok_or(AssociationError::InvalidManifest)?,
            LaunchAdapter::InstalledApplication(image.ok_or(AssociationError::InvalidManifest)?),
        )?;
        if let Some(values) = content_types {
            for value in values.split(|byte| *byte == b',') {
                let content_type = match value {
                    b"binary" => ContentKind::Binary, b"utf8-text" => ContentKind::Utf8Text,
                    b"namespace" => ContentKind::Namespace, b"system" => ContentKind::System,
                    b"application" => ContentKind::Application, b"metadata" => ContentKind::Metadata,
                    b"" => continue, _ => return Err(AssociationError::InvalidManifest),
                };
                handler = handler.with_content_type(content_type);
            }
        }
        if let Some(values) = extensions {
            for value in values.split(|byte| *byte == b',') {
                if !value.is_empty() { handler.add_extension(value)?; }
            }
        }
        if handler.content_mask == 0 && handler.extensions.iter().all(|extension| extension.length == 0) {
            return Err(AssociationError::InvalidManifest);
        }
        Ok(handler)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DefaultAssociation {
    used: bool,
    content_type: u8,
    extension: BoundedText<MAX_EXTENSION_BYTES>,
    application: BoundedText<MAX_APPLICATION_ID_BYTES>,
}

impl DefaultAssociation {
    const EMPTY: Self = Self {
        used: false,
        content_type: 0,
        extension: BoundedText::empty(),
        application: BoundedText::empty(),
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenRequest {
    pub application: BoundedText<MAX_APPLICATION_ID_BYTES>,
    pub path: BoundedText<96>,
    pub content_type: ContentKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registry {
    handlers: [Option<ApplicationHandler>; MAX_APPLICATION_HANDLERS],
    defaults: [DefaultAssociation; MAX_DEFAULT_ASSOCIATIONS],
    pending: Option<OpenRequest>,
}

impl Registry {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the shared handler registry with built-in OS applications registered through the same capability contract as custom apps.
    // ------------------=
    pub const fn new() -> Self {
        let text = ApplicationHandler {
            identity: BoundedText::literal(b"app.infinity.text-editor"),
            display_name: BoundedText::literal(b"Text Editor"),
            launch: LaunchAdapter::TextEditor,
            content_mask: 1u8 << ((ContentKind::Utf8Text as u8) - 1),
            extensions: [BoundedText::empty(); MAX_HANDLER_EXTENSIONS],
        };
        let files = ApplicationHandler {
            identity: BoundedText::literal(b"app.infinity.file-navigator"),
            display_name: BoundedText::literal(b"File Navigator"),
            launch: LaunchAdapter::FileNavigator,
            content_mask: 1u8 << ((ContentKind::Namespace as u8) - 1),
            extensions: [BoundedText::empty(); MAX_HANDLER_EXTENSIONS],
        };
        Self {
            handlers: [Some(text), Some(files), None, None, None, None, None, None, None, None, None, None],
            defaults: [DefaultAssociation::EMPTY; MAX_DEFAULT_ASSOCIATIONS],
            pending: None,
        }
    }

    // ------------------------=
    // FUNC: install
    // DESC: Registers or replaces one validated OS or custom installed-application handler.
    // ------------------=
    pub fn install(&mut self, handler: ApplicationHandler) -> Result<(), AssociationError> {
        if let Some(slot) = self.handlers.iter_mut().find(|slot| slot.is_some_and(|current| current.identity == handler.identity)) {
            if slot.is_some_and(|current| current.launch != handler.launch) {
                return Err(AssociationError::InvalidManifest);
            }
            *slot = Some(handler);
            return Ok(());
        }
        let slot = self.handlers.iter_mut().find(|slot| slot.is_none()).ok_or(AssociationError::Capacity)?;
        *slot = Some(handler);
        Ok(())
    }

    // ------------------------=
    // FUNC: compatible_count
    // DESC: Counts launchable installed applications which declare compatibility with one selected object.
    // ------------------=
    pub fn compatible_count(&self, content_type: ContentKind, extension: &[u8]) -> usize {
        self.handlers.iter().flatten().filter(|handler| handler.supports(content_type, extension)).count()
    }

    // ------------------------=
    // FUNC: compatible_nth
    // DESC: Returns one stable compatible application in installed registry order.
    // ------------------=
    pub fn compatible_nth(&self, content_type: ContentKind, extension: &[u8], index: usize) -> Option<ApplicationHandler> {
        self.handlers.iter().flatten().filter(|handler| handler.supports(content_type, extension)).nth(index).copied()
    }

    // ------------------------=
    // FUNC: resolve
    // DESC: Resolves the persisted compatible default, falling back deterministically to the first installed compatible handler.
    // ------------------=
    pub fn resolve(&self, content_type: ContentKind, extension: &[u8]) -> Result<ApplicationHandler, AssociationError> {
        if let Some(default) = self.default_identity(content_type, extension) {
            if let Some(handler) = self.handlers.iter().flatten().find(|handler| handler.identity == default && handler.supports(content_type, extension)) {
                return Ok(*handler);
            }
        }
        self.compatible_nth(content_type, extension, 0).ok_or(AssociationError::IncompatibleApplication)
    }

    // ------------------------=
    // FUNC: set_default
    // DESC: Persists one compatible application as the extension-specific or content-type default.
    // ------------------=
    pub fn set_default(&mut self, content_type: ContentKind, extension: &[u8], application: &[u8]) -> Result<(), AssociationError> {
        let handler = self.handlers.iter().flatten().find(|handler| handler.identity.bytes() == application)
            .copied().ok_or(AssociationError::UnknownApplication)?;
        if !handler.supports(content_type, extension) {
            return Err(AssociationError::IncompatibleApplication);
        }
        let extension = normalized_extension(extension).unwrap_or(&[]);
        let existing = self.defaults.iter().position(|value| value.used
            && value.content_type == content_type as u8 && value.extension.bytes().eq_ignore_ascii_case(extension));
        let slot = existing.or_else(|| self.defaults.iter().position(|value| !value.used)).ok_or(AssociationError::Capacity)?;
        self.defaults[slot] = DefaultAssociation {
            used: true,
            content_type: content_type as u8,
            extension: if extension.is_empty() { BoundedText::empty() } else { BoundedText::new(extension)? },
            application: handler.identity,
        };
        Ok(())
    }

    // ------------------------=
    // FUNC: is_default
    // DESC: Reports whether one compatible application is the persisted default for a selected object.
    // ------------------=
    pub fn is_default(&self, content_type: ContentKind, extension: &[u8], application: &[u8]) -> bool {
        self.default_identity(content_type, extension).is_some_and(|identity| identity.bytes() == application)
    }

    // ------------------------=
    // FUNC: stage_open
    // DESC: Validates compatibility and creates one bounded launch request carrying the exact selected NamespaceRef.
    // ------------------=
    pub fn stage_open(&mut self, handler: ApplicationHandler, path: &[u8], content_type: ContentKind, extension: &[u8]) -> Result<OpenRequest, AssociationError> {
        if !handler.supports(content_type, extension) {
            return Err(AssociationError::IncompatibleApplication);
        }
        let request = OpenRequest {
            application: handler.identity,
            path: BoundedText::new(path).map_err(|_| AssociationError::ApplicationUnavailable)?,
            content_type,
        };
        self.pending = Some(request);
        Ok(request)
    }

    // ------------------------=
    // FUNC: take_open_request
    // DESC: Delivers a startup object exactly once to the launched application runtime.
    // ------------------=
    pub fn take_open_request(&mut self, application: &[u8]) -> Option<OpenRequest> {
        if self.pending.is_some_and(|request| request.application.bytes() == application) {
            self.pending.take()
        } else {
            None
        }
    }

    // ------------------------=
    // FUNC: encode_defaults
    // DESC: Serializes only per-user default choices; installed handler capabilities remain authoritative manifests.
    // ------------------=
    pub fn encode_defaults(&self) -> [u8; ASSOCIATION_STATE_BYTES] {
        let mut bytes = [0u8; ASSOCIATION_STATE_BYTES];
        bytes[..8].copy_from_slice(ASSOCIATION_STATE_MAGIC);
        bytes[8..10].copy_from_slice(&ASSOCIATION_STATE_VERSION.to_le_bytes());
        for (index, value) in self.defaults.iter().enumerate() {
            let offset = 16 + index * 64;
            bytes[offset] = value.used as u8;
            bytes[offset + 1] = value.content_type;
            bytes[offset + 2] = value.extension.length;
            bytes[offset + 3] = value.application.length;
            bytes[offset + 4..offset + 16].copy_from_slice(&value.extension.bytes);
            bytes[offset + 16..offset + 64].copy_from_slice(&value.application.bytes);
        }
        let checksum = checksum(&bytes[..ASSOCIATION_STATE_BYTES - 4]);
        bytes[ASSOCIATION_STATE_BYTES - 4..].copy_from_slice(&checksum.to_le_bytes());
        bytes
    }

    // ------------------------=
    // FUNC: restore_defaults
    // DESC: Atomically validates and restores per-user default choices without altering installed handlers.
    // ------------------=
    pub fn restore_defaults(&mut self, bytes: &[u8]) -> Result<(), AssociationError> {
        if bytes.len() != ASSOCIATION_STATE_BYTES || &bytes[..8] != ASSOCIATION_STATE_MAGIC
            || u16::from_le_bytes([bytes[8], bytes[9]]) != ASSOCIATION_STATE_VERSION
            || u32::from_le_bytes(bytes[ASSOCIATION_STATE_BYTES - 4..].try_into().unwrap()) != checksum(&bytes[..ASSOCIATION_STATE_BYTES - 4]) {
            return Err(AssociationError::CorruptState);
        }
        let mut restored = [DefaultAssociation::EMPTY; MAX_DEFAULT_ASSOCIATIONS];
        for (index, value) in restored.iter_mut().enumerate() {
            let offset = 16 + index * 64;
            if bytes[offset] == 0 { continue; }
            let extension_length = bytes[offset + 2] as usize;
            let application_length = bytes[offset + 3] as usize;
            if extension_length > MAX_EXTENSION_BYTES || application_length == 0 || application_length > MAX_APPLICATION_ID_BYTES
                || !(1..=6).contains(&bytes[offset + 1]) {
                return Err(AssociationError::CorruptState);
            }
            *value = DefaultAssociation {
                used: true,
                content_type: bytes[offset + 1],
                extension: if extension_length == 0 { BoundedText::empty() } else {
                    BoundedText::new(&bytes[offset + 4..offset + 4 + extension_length]).map_err(|_| AssociationError::CorruptState)?
                },
                application: BoundedText::new(&bytes[offset + 16..offset + 16 + application_length]).map_err(|_| AssociationError::CorruptState)?,
            };
        }
        self.defaults = restored;
        Ok(())
    }

    // ------------------------=
    // FUNC: default_identity
    // DESC: Resolves an extension-specific default before the broader content-type fallback.
    // ------------------=
    fn default_identity(&self, content_type: ContentKind, extension: &[u8]) -> Option<BoundedText<MAX_APPLICATION_ID_BYTES>> {
        let extension = normalized_extension(extension).unwrap_or(&[]);
        self.defaults.iter().find(|value| value.used && value.content_type == content_type as u8
            && !extension.is_empty() && value.extension.bytes().eq_ignore_ascii_case(extension))
            .or_else(|| self.defaults.iter().find(|value| value.used && value.content_type == content_type as u8 && value.extension.length == 0))
            .map(|value| value.application)
    }
}

// ------------------------=
// FUNC: path_extension
// DESC: Extracts a bounded final filename extension without treating parent dots or hidden-file prefixes as types.
// ------------------=
pub fn path_extension(path: &[u8]) -> &[u8] {
    let name = path.rsplit(|byte| *byte == b'/').next().unwrap_or(path);
    let Some(dot) = name.iter().rposition(|byte| *byte == b'.') else { return &[]; };
    if dot == 0 || dot + 1 == name.len() { &[] } else { &name[dot + 1..] }
}

// ------------------------=
// FUNC: normalized_extension
// DESC: Accepts one bounded ASCII extension with or without a leading dot.
// ------------------=
fn normalized_extension(extension: &[u8]) -> Option<&[u8]> {
    let extension = extension.strip_prefix(b".").unwrap_or(extension);
    (!extension.is_empty() && extension.len() <= MAX_EXTENSION_BYTES
        && extension.iter().all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'+' | b'-' | b'_'))).then_some(extension)
}

// ------------------------=
// FUNC: parse_image_id
// DESC: Parses a nonzero decimal or 0x-prefixed hexadecimal installed application image identity.
// ------------------=
fn parse_image_id(value: &[u8]) -> Option<u32> {
    let (digits, radix) = value.strip_prefix(b"0x").map(|digits| (digits, 16)).unwrap_or((value, 10));
    if digits.is_empty() { return None; }
    let mut parsed = 0u32;
    for byte in digits {
        let digit = match byte {
            b'0'..=b'9' => (byte - b'0') as u32,
            b'a'..=b'f' if radix == 16 => (byte - b'a' + 10) as u32,
            b'A'..=b'F' if radix == 16 => (byte - b'A' + 10) as u32,
            _ => return None,
        };
        if digit >= radix { return None; }
        parsed = parsed.checked_mul(radix)?.checked_add(digit)?;
    }
    (parsed != 0).then_some(parsed)
}

// ------------------------=
// FUNC: checksum
// DESC: Detects torn or corrupt private association preference records.
// ------------------=
fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c9dc5u32, |hash, byte| (hash ^ *byte as u32).wrapping_mul(0x01000193))
}
