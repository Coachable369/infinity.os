//! Test-only stopped-VM export of generated fixture keys; never mutates guest storage.
use super::*;
use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt, path::Path};
use storage::object::ObjectId;

// ------------------------=
// FUNC: run
// DESC: Reads only native identity and owner key objects from a stopped disposable installation, exporting a mode-0600 client fixture.
// ------------------=
pub(super) fn run(raw: &str, handle: &str, destination: &str) -> Result<String, String> {
    let output = Path::new(destination);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build").canonicalize().map_err(|_| "build_missing")?;
    let parent = output.parent().ok_or("output_parent")?.canonicalize().map_err(|_| "output_parent")?;
    if !parent.starts_with(root) { return Err("output_not_in_build".into()); }
    let file = File::open(raw).map_err(|_| "open_failed")?;
    let sectors = file.metadata().map_err(|_| "metadata_failed")?.len() / 512;
    let mut disk = FileDisk { file, sectors };
    let start = container(&mut disk)?;
    let mut store = ObjectStore::mount(disk, start).map_err(|_| "native_mount_failed")?;
    let state = store.resolve(b"/system/identity/state").map_err(|_| "identity_missing")?;
    let mut encoded = zeroize::Zeroizing::new([0; runtime::identity::IDENTITY_STATE_BYTES]);
    let n = store.read_identity_state(state, &mut *encoded).map_err(|_| "identity_read_failed")?;
    let identity = runtime::identity::IdentitySystem::decode(&encoded[..n]).map_err(|_| "identity_invalid")?;
    let user = identity.user_by_handle(handle.as_bytes()).ok_or("user_missing")?;
    let owner = ObjectId(user.id.0);
    let mut pair = [None; 2];
    for index in 0..store.persistent_namespace_count() {
        let (path, id) = store.namespace_entry(index).ok_or("namespace_missing")?;
        if store.metadata(id).map_err(|_| "metadata_invalid")?.owner != owner { continue; }
        if path.starts_with(b"/home/") && path.ends_with(b"/.ssh/id_ed25519") { pair[0] = Some(id); }
        if path.starts_with(b"/home/") && path.ends_with(b"/.ssh/id_ed25519.pub") { pair[1] = Some(id); }
    }
    let mut private = zeroize::Zeroizing::new([0; 512]);
    let n = store.read_user_ssh_key(pair[0].ok_or("private_missing")?, owner, &mut *private).map_err(|_| "private_denied")?;
    let mut public = [0; 96];
    let p = store.read_user_ssh_key(pair[1].ok_or("public_missing")?, owner, &mut public).map_err(|_| "public_denied")?;
    let mut command = b"ssh key add fixture ".to_vec(); command.extend_from_slice(&public[..p]);
    let infinity_ssh::management::Command::Key { key, .. } = infinity_ssh::management::parse(&command).map_err(|_| "public_invalid")?
        else { return Err("public_invalid".into()); };
    if !identity.ssh_key_active(user.id, &key) { return Err("key_not_registered".into()); }
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(output).map_err(|_| "output_exists")?;
    file.write_all(&private[..n]).map_err(|_| "export_failed")?;
    let mut public_file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(output.with_extension("pub")).map_err(|_| "public_output_exists")?;
    public_file.write_all(&public[..p]).map_err(|_| "export_failed")?;
    let fingerprint = infinity_ssh::management::fingerprint(&key);
    Ok(format!("{{\"registered\":true,\"fingerprint\":\"{}\"}}", core::str::from_utf8(&fingerprint).map_err(|_| "fingerprint")?))
}
