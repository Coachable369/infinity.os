//! Restricted SSH persistence adapters. These never use the generic file APIs.
use super::{object::{ObjectError, ObjectType, Space, SSH_SERVICE_OWNER}, with_store};
use infinity_ssh::{hostkey::{HostKeyStorage, HOST_KEY_BYTES}, Configuration, Failure};

const KEY_PATH: &[u8] = b"/system/security/ssh/host-key";
const CONFIG_PATH: &[u8] = b"/system/security/ssh/config";

pub(crate) struct HostStore;
impl HostKeyStorage for HostStore {
    // ------------------------=
    // FUNC: read_protected
    // DESC: Loads only the private SSH host-key object and fails on malformed metadata or unavailable storage.
    // ------------------=
    fn read_protected(&mut self, out: &mut [u8; HOST_KEY_BYTES]) -> Result<bool, Failure> {
        with_store(|store| {
            let id = match store.resolve(KEY_PATH) {
                Ok(id) => id,
                Err(ObjectError::NamespaceNotFound) => return Ok(false),
                Err(error) => return Err(error),
            };
            let length = store.read_ssh_private(id, b"@ssh-host-key", out)?;
            if length != HOST_KEY_BYTES { return Err(ObjectError::CorruptContent); }
            Ok(true)
        }).map_err(|_| Failure::HostKeyUnavailable)
    }
    // ------------------------=
    // FUNC: create_protected
    // DESC: Atomically creates protected metadata plus its namespace; never overwrites a persisted host key.
    // ------------------=
    fn create_protected(&mut self, content: &[u8; HOST_KEY_BYTES]) -> Result<(), Failure> {
        with_store(|store| store.create_owned_attached(b"@ssh-host-key", ObjectType::Metadata, Space::System, content, KEY_PATH, SSH_SERVICE_OWNER).map(|_| ()))
            .map_err(|_| Failure::HostKeyUnavailable)
    }
}

// ------------------------=
// FUNC: load_configuration
// DESC: Restores only validated native listener configuration; new installations remain disabled.
// ------------------=
pub(crate) fn load_configuration() -> Result<Configuration, Failure> {
    with_store(|store| {
        let id = match store.resolve(CONFIG_PATH) {
            Ok(id) => id,
            Err(ObjectError::NamespaceNotFound) => return Ok(Configuration::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = [0; 12];
        if store.read_ssh_private(id, b"@ssh-config", &mut bytes)? != bytes.len()
            || &bytes[..8] != b"INFSSHC1" || bytes[8] > 1 {
            return Err(ObjectError::CorruptContent);
        }
        Configuration { enabled: bytes[8] == 1, port: u16::from_le_bytes([bytes[9], bytes[10]]), max_connections: bytes[11] }
            .validate().map_err(|_| ObjectError::CorruptContent)
    }).map_err(|_| Failure::Configuration)
}

// ------------------------=
// FUNC: save_configuration
// DESC: Persists a locally authorized listener change before activation; caller must validate administrator session.
// ------------------=
pub(crate) fn save_configuration(config: Configuration) -> Result<(), Failure> {
    let config = config.validate()?;
    let mut bytes = [0; 12];
    bytes[..8].copy_from_slice(b"INFSSHC1");
    bytes[8] = config.enabled as u8;
    bytes[9..11].copy_from_slice(&config.port.to_le_bytes());
    bytes[11] = config.max_connections;
    with_store(|store| match store.resolve(CONFIG_PATH) {
        Ok(id) => {
            let mut old = [0; 12];
            store.read_ssh_private(id, b"@ssh-config", &mut old)?;
            store.replace_state(id, &bytes).map(|_| ())
        }
        Err(ObjectError::NamespaceNotFound) => store.create_owned_attached(b"@ssh-config", ObjectType::Metadata, Space::System, &bytes, CONFIG_PATH, SSH_SERVICE_OWNER).map(|_| ()),
        Err(error) => Err(error),
    }).map_err(|_| Failure::Configuration)
}
