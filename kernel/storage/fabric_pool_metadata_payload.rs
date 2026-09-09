//! Actual immutable manifest/namespace/policy/delegation payloads paired with
//! durable signed Replica state. No receipt or freshness claim is synthesized.
use super::*;
use crate::runtime::fabric::metadata_bundle::{Bundle, BUNDLE_BYTES};
const PAYLOAD_BYTES: usize = 64 + BUNDLE_BYTES * 2;
// ------------------------=
// FUNC: path
// DESC: Produces one bounded native metadata object name per stable logical ObjectId.
// ------------------=
fn path(object: [u8; 16]) -> ([u8; 95], usize) {
    let prefix = b"/system/storage/pool-quorum-payload/";
    let mut b = [0; 95];
    b[..prefix.len()].copy_from_slice(prefix);
    for (i, v) in object.iter().enumerate() {
        b[prefix.len() + i * 2] = b"0123456789abcdef"[(v >> 4) as usize];
        b[prefix.len() + i * 2 + 1] = b"0123456789abcdef"[(v & 15) as usize];
    }
    (b, prefix.len() + 32)
}
// ------------------------=
// FUNC: read_payload
// DESC: Loads both immutable payload slots and rejects malformed or partially recorded state.
// ------------------=
fn read_payload<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    object: [u8; 16],
) -> Result<[u8; PAYLOAD_BYTES], Error> {
    let (p, n) = path(object);
    let mut b = [0; PAYLOAD_BYTES];
    b[..8].copy_from_slice(b"INFQPY02");
    let id = match store.resolve(&p[..n]) {
        Ok(id) => id,
        Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => return Ok(b),
        Err(_) => return Err(Error::Persistence),
    };
    let size = store
        .read(id, None, &mut b)
        .map_err(|_| Error::Persistence)?;
    if size == PAYLOAD_BYTES - 32 && &b[..8] == b"INFQPY01" && b[10..32] == [0; 22] {
        b.copy_within(32..size, 64);
        b[16..64].fill(0);
        b[..8].copy_from_slice(b"INFQPY02");
    } else if size != PAYLOAD_BYTES {
        return Err(Error::Invalid);
    }
    if &b[..8] != b"INFQPY02" || b[8] > 1 || b[9] > 1 || b[10..16] != [0; 6] {
        return Err(Error::Invalid);
    }
    for i in 0..2 {
        let bytes = &b[64 + i * BUNDLE_BYTES..64 + (i + 1) * BUNDLE_BYTES];
        if b[8 + i] == 1 {
            let bundle = Bundle::decode(bytes)?;
            if bundle.value.record.object != object || (i == 1 && bundle.certificate.is_none()) {
                return Err(Error::Conflict);
            }
        } else if bytes.iter().any(|v| *v != 0) {
            return Err(Error::Invalid);
        }
    }
    Ok(b)
}
// ------------------------=
// FUNC: candidate
// DESC: Extracts a present canonical slot without inventing a default manifest.
// ------------------=
fn candidate(b: &[u8; PAYLOAD_BYTES], slot: usize) -> Result<Option<Bundle>, Error> {
    if b[8 + slot] == 0 {
        Ok(None)
    } else {
        Ok(Some(Bundle::decode(
            &b[64 + slot * BUNDLE_BYTES..64 + (slot + 1) * BUNDLE_BYTES],
        )?))
    }
}
// ------------------------=
// FUNC: same_payload
// DESC: Compares immutable payloads independently of the publication receipts added after durable staging.
// ------------------=
fn same_payload(mut a: Bundle, mut b: Bundle) -> Result<bool, Error> {
    a.certificate = None;
    b.certificate = None;
    Ok(a.encode()? == b.encode()?)
}
// ------------------------=
// FUNC: load_bundle
// DESC: Enumerates only durable committed payloads whose exact signed value matches the recovered quorum catalog.
// ------------------=
pub(crate) fn load_bundle<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    index: usize,
) -> Result<Option<Bundle>, Error> {
    if index >= CAPACITY {
        return Ok(None);
    }
    let (bytes, _) = read_catalog(store)?;
    let at = 32 + index * ENTRY;
    if bytes[at..at + ENTRY].iter().all(|v| *v == 0) {
        return Ok(None);
    }
    let g = decode_group(&bytes[at..at + 240])?;
    let replica = decode_replica(&bytes[at + 240..at + ENTRY], &g)?;
    let Some(c) = replica.committed else {
        return Ok(None);
    };
    let payload = read_payload(store, c.value.record.object)?;
    let bundle = candidate(&payload, 1)?.ok_or(Error::Persistence)?;
    if bundle.value != c.value
        || bundle.certificate != Some(c)
        || group_bytes(&bundle.group) != group_bytes(&g)
    {
        return Err(Error::Conflict);
    }
    Ok(Some(bundle))
}
// ------------------------=
// FUNC: read_bundle
// DESC: Resolves one exact committed metadata identity without relying on the original owner node.
// ------------------=
pub(crate) fn read_bundle<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    object: [u8; 16],
) -> Result<Bundle, Error> {
    for i in 0..CAPACITY {
        if let Some(b) = load_bundle(store, i)? {
            if b.value.record.object == object {
                return Ok(b);
            }
        }
    }
    Err(Error::Invalid)
}
// ------------------------=
// FUNC: stage_bundle
// DESC: Validates the actual signed payload and commits staging plus its native quorum record atomically before any prepare receipt is issued.
// ------------------=
pub(crate) fn stage_bundle<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    bundle: Bundle,
) -> Result<(), Error> {
    bundle.validate()?;
    if bundle.certificate.is_some() {
        return Err(Error::Invalid);
    }
    commit_bundle(store, bundle, false)
}
// ------------------------=
// FUNC: publish_bundle
// DESC: Publishes the exact staged bytes with a valid certificate or performs certified read write-back; no unsigned state becomes visible.
// ------------------=
pub(crate) fn publish_bundle<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    object: [u8; 16],
    certificate: Certificate,
) -> Result<(), Error> {
    let payload = read_payload(store, object)?;
    let mut bundle = candidate(&payload, 0)?
        .filter(|b| b.value == certificate.value)
        .or(candidate(&payload, 1)?.filter(|b| b.value == certificate.value))
        .ok_or(Error::Persistence)?;
    if bundle.value != certificate.value {
        return Err(Error::Conflict);
    }
    bundle.certificate = Some(certificate);
    commit_bundle(store, bundle, true)
}
// ------------------------=
// FUNC: publish_received_bundle
// DESC: Persists complete certified bytes during authenticated read write-back, including a node that missed initial staging.
// ------------------=
pub(crate) fn publish_received_bundle<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    bundle: Bundle,
) -> Result<(), Error> {
    commit_bundle(store, bundle, true)
}
// ------------------------=
// FUNC: commit_bundle
// DESC: Atomically replaces bounded actual payload slots and signed replica state; acknowledged commits cannot reference missing immutable metadata.
// ------------------=
fn commit_bundle<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    bundle: Bundle,
    publish: bool,
) -> Result<(), Error> {
    bundle.validate()?;
    let object = bundle.value.record.object;
    let (mut catalog, _) = read_catalog(store)?;
    let slot = locate(&catalog, &bundle.group, object)?;
    for i in 0..CAPACITY {
        let at = 32 + i * ENTRY;
        if catalog[at..at + ENTRY].iter().all(|v| *v == 0) {
            continue;
        }
        let g = decode_group(&catalog[at..at + 240])?;
        if g.owner != bundle.group.owner {
            continue;
        }
        let r = decode_replica(&catalog[at + 240..at + ENTRY], &g)?;
        let other = r
            .staged
            .or(r.committed.map(|c| c.value))
            .ok_or(Error::Invalid)?
            .record
            .object;
        if other == object {
            continue;
        }
        let bytes = read_payload(store, other)?;
        for s in 0..2 {
            if let Some(b) = candidate(&bytes, s)? {
                if b.path() == bundle.path() {
                    return Err(Error::Conflict);
                }
            }
        }
    }
    let current = match slot {
        Some(i) => decode_replica(
            &catalog[32 + i * ENTRY + 240..32 + (i + 1) * ENTRY],
            &bundle.group,
        )?,
        None => Replica::default(),
    };
    let mut next = current;
    if publish {
        next.publish(
            &bundle.group,
            bundle.certificate.ok_or(Error::Quorum)?,
            |_| Ok(()),
        )?;
    } else {
        next.prepare(&bundle.group, bundle.value, |_| Ok(()))?;
    }
    let mut payload = read_payload(store, object)?;
    if replica_bytes(next) == replica_bytes(current) {
        let prior = if publish {
            candidate(&payload, 1)?
        } else {
            candidate(&payload, 0)?.or(candidate(&payload, 1)?)
        }
        .ok_or(Error::Persistence)?;
        return if same_payload(prior, bundle)? {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    let slot = slot
        .or_else(|| {
            (0..CAPACITY).find(|i| {
                catalog[32 + i * ENTRY..32 + (i + 1) * ENTRY]
                    .iter()
                    .all(|v| *v == 0)
            })
        })
        .ok_or(Error::ResourceLimit)?;
    let at = 32 + slot * ENTRY;
    catalog[at..at + 240].copy_from_slice(&group_bytes(&bundle.group));
    catalog[at + 240..at + ENTRY].copy_from_slice(&replica_bytes(next));
    let target = usize::from(publish);
    payload[8 + target] = 1;
    payload[64 + target * BUNDLE_BYTES..64 + (target + 1) * BUNDLE_BYTES]
        .copy_from_slice(&bundle.encode()?);
    if publish && next.staged.is_none() {
        payload[8] = 0;
        payload[64..64 + BUNDLE_BYTES].fill(0);
    }
    let (p, n) = path(object);
    store
        .replace_named_state_pair(PATH, &catalog, &p[..n], &payload)
        .map_err(|_| Error::Persistence)
}

// ------------------------=
// FUNC: read_repair_payload
// DESC: Reads one immutable repair child through the bounded native metadata parent, without additional namespace entries.
// ------------------=
pub(crate) fn read_repair_payload<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    object: [u8; 16],
    stage: bool,
    out: &mut [u8],
) -> Result<usize, Error> {
    let b = read_payload(store, object)?;
    let at = if stage { 16 } else { 32 };
    let id = ObjectId(b[at..at + 16].try_into().unwrap());
    if id.0 == [0; 16] {
        return Ok(0);
    }
    store.read(id, None, out).map_err(|_| Error::Persistence)
}
// ------------------------=
// FUNC: write_repair_payload
// DESC: Atomically creates or replaces a linked repair child; existing eight-object metadata bounds limit storage growth.
// ------------------=
pub(crate) fn write_repair_payload<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    object: [u8; 16],
    stage: bool,
    bytes: &[u8],
) -> Result<(), Error> {
    let b = read_payload(store, object)?;
    if b[9] != 1 {
        return Err(Error::Persistence);
    }
    let (p, n) = path(object);
    let id = store.resolve(&p[..n]).map_err(|_| Error::Persistence)?;
    store
        .replace_linked_state(id, &b, if stage { 16 } else { 32 }, bytes)
        .map_err(|_| Error::Persistence)
}

// ------------------------=
// FUNC: read_mutation_payload
// DESC: Recovers the bounded owner mutation intent linked from the certified metadata object.
// ------------------=
pub(crate) fn read_mutation_payload<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    object: [u8; 16],
    out: &mut [u8],
) -> Result<usize, Error> {
    let b = read_payload(store, object)?;
    let id = ObjectId(b[48..64].try_into().unwrap());
    if id.0 == [0; 16] {
        return Ok(0);
    }
    store.read(id, None, out).map_err(|_| Error::Persistence)
}
// ------------------------=
// FUNC: write_mutation_payload
// DESC: Atomically persists an owner mutation intent without adding user namespace records.
// ------------------=
pub(crate) fn write_mutation_payload<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    object: [u8; 16],
    bytes: &[u8],
) -> Result<(), Error> {
    let b = read_payload(store, object)?;
    if b[9] != 1 {
        return Err(Error::Persistence);
    }
    let (p, n) = path(object);
    let id = store.resolve(&p[..n]).map_err(|_| Error::Persistence)?;
    store
        .replace_linked_state(id, &b, 48, bytes)
        .map_err(|_| Error::Persistence)
}
// ------------------------=
// FUNC: is_shared_object
// DESC: Detects any existing shared metadata record and treats malformed records as errors rather than granting unfenced writes.
// ------------------=
pub(crate) fn is_shared_object<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    object: [u8; 16],
) -> Result<bool, Error> {
    let (p, n) = path(object);
    match store.resolve(&p[..n]) {
        Ok(_) => {
            read_payload(store, object)?;
            Ok(true)
        }
        Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => Ok(false),
        Err(_) => Err(Error::Persistence),
    }
}
