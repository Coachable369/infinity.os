//! Bounded placement-only durable overlay state and authenticated repair windows.
#[cfg(not(test))]
use super::fabric as replicas;
#[cfg(not(test))]
use super::fabric_pool_metadata as metadata;
#[cfg(test)]
use crate::fabric_pool_metadata as metadata;
#[cfg(test)]
use crate::native_fabric as replicas;
use crate::runtime::{
    fabric::metadata_repair::{RepairAuthorization, REPAIR_AUTHORIZATION_BYTES},
    iop::{
        remote::{AuthenticatedStorageRequest, RemoteError},
        storage_protocol::StorageOperationV1,
    },
    storage_metadata_repair::{NativeReply as R, NativeRequest as N},
};
use crate::storage::{
    object::{ObjectError, ObjectStore},
    BlockDevice,
};
use sha2::{Digest, Sha256};
struct Upload {
    peer: [u8; 32],
    session: [u8; 16],
    grant: u64,
    object: [u8; 16],
    hash: [u8; 32],
    at: usize,
    expires: u64,
    bytes: [u8; REPAIR_AUTHORIZATION_BYTES],
}
pub(crate) struct Service {
    upload: Option<Upload>,
    key: Option<([u8; 32], [u8; 32])>,
    chunk: [u8; 16384],
}
// ------------------------=
// FUNC: load
// DESC: Recovers an exact canonical signed overlay from a native linked child without spending namespace slots.
// ------------------=
fn load<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    object: [u8; 16],
    stage: bool,
    _now: u64,
) -> Result<Option<RepairAuthorization>, RemoteError> {
    let mut b = [0; REPAIR_AUTHORIZATION_BYTES];
    let n = metadata::read_repair_payload(store, object, stage, &mut b)
        .map_err(|_| RemoteError::PersistenceFailed)?;
    if n == 0 {
        return Ok(None);
    }
    let a = RepairAuthorization::decode(0, &b)
        .map_err(|_| RemoteError::AccessDenied)?;
    if n != b.len()
        || a.anchor.manifest.object != object
        || (!stage && a.repair.certificate.is_none())
    {
        return Err(RemoteError::Conflict);
    }
    let current=metadata::read_bundle(store,object).map_err(|_|RemoteError::PersistenceFailed)?;
    if current.value!=a.anchor.value {
        if current.group.digest()==a.anchor.group.digest() && current.value.record.generation>a.anchor.value.record.generation{return Ok(None)}
        return Err(RemoteError::Conflict)
    }
    Ok(Some(a))
}
// ------------------------=
// FUNC: persist
// DESC: Performs bounded CAS staging and certificate publication; a head never references uncommitted payload bytes.
// ------------------=
fn persist<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    a: RepairAuthorization,
    publish: bool,
    now: u64,
) -> Result<(), RemoteError> {
    if !a.anchor.persistent_authority()||a.repair.grant.expires!=u64::MAX{return Err(RemoteError::AccessDenied)}
    let bytes = a
        .encode(if publish { 0 } else { now })
        .map_err(|_| RemoteError::AccessDenied)?;
    a.repair
        .validate_available(&a.anchor.manifest)
        .map_err(|_| RemoteError::AccessDenied)?;
    let object = a.anchor.manifest.object;
    if metadata::read_bundle(store, object)
        .map_err(|_| RemoteError::NotFound)?
        .value
        != a.anchor.value
    {
        return Err(RemoteError::Conflict);
    }
    let old = load(store, object, false, now)?;
    if old
        .as_ref()
        .is_some_and(|v| v.repair.value == a.repair.value)
    {
        return Ok(());
    }
    a.repair
        .value
        .successor(old.map(|v| v.repair.value))
        .map_err(|_| RemoteError::Conflict)?;
    if publish {
        if a.repair.certificate.is_none() {
            return Err(RemoteError::AccessDenied);
        }
    } else {
        if a.repair.certificate.is_some() {
            return Err(RemoteError::MalformedRequest);
        }
        if let Some(staged) = load(store, object, true, now)? {
            if old.is_none_or(|o| staged.repair.value.sequence > o.repair.value.sequence) {
                if staged.repair.value != a.repair.value {
                    return Err(RemoteError::Conflict);
                }
                return Ok(());
            }
        }
    }
    metadata::write_repair_payload(store, object, !publish, &bytes)
        .map_err(|_| RemoteError::PersistenceFailed)?;
    Ok(())
}
impl Service {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates one bounded authenticated repair upload and verified chunk cache.
    // ------------------=
    pub(crate) const fn new() -> Self {
        Self {
            upload: None,
            key: None,
            chunk: [0; 16384],
        }
    }
    // ------------------------=
    // FUNC: read
    // DESC: Serves only owner-delegated immutable bytes from the verified local replica, without granting write or namespace authority.
    // ------------------=
    fn read<D: BlockDevice>(
        &mut self,
        store: &mut ObjectStore<D>,
        a: &RepairAuthorization,
        writer: crate::runtime::node::types::NodeId,
        local: crate::runtime::node::types::NodeId,
        now: u64,
        offset: u64,
        length: u8,
    ) -> Result<R, RemoteError> {
        a.encode(now).map_err(|_| RemoteError::AccessDenied)?;
        if !a.anchor.persistent_authority()||a.repair.grant.expires!=u64::MAX{return Err(RemoteError::AccessDenied)}
        a.repair
            .grant
            .validate(
                &a.anchor.group,
                &a.anchor.certificate.ok_or(RemoteError::AccessDenied)?,
                now,
            )
            .map_err(|_| RemoteError::AccessDenied)?;
        if a.repair.grant.writer != writer {
            return Err(RemoteError::AccessDenied);
        }
        let m = &a.anchor.manifest;
        let n = length as usize;
        if n == 0 || n > 64 || offset.checked_add(n as u64).is_none_or(|e| e > m.length) {
            return Err(RemoteError::MalformedRequest);
        }
        let mut data = [0; 64];
        let mut start = 0;
        let mut copied = 0;
        for c in m.chunks.iter().flatten() {
            let end = start + c.bytes as u64;
            if offset + (copied as u64) >= start && offset + (copied as u64) < end && copied < n {
                let key = (a.anchor.value.record.digest(), c.hash);
                if self.key != Some(key) {
                    self.key = None;
                    replicas::service::delegated_replica_chunk(
                        store,
                        m,
                        local,
                        start,
                        &mut self.chunk[..c.bytes as usize],
                        c.hash,
                    )?;
                    self.key = Some(key)
                }
                let at = (offset + copied as u64 - start) as usize;
                let take = (n - copied).min(c.bytes as usize - at);
                data[copied..copied + take].copy_from_slice(&self.chunk[at..at + take]);
                copied += take;
            }
            start = end;
        }
        if copied != n {
            return Err(RemoteError::RemoteFailure);
        }
        Ok(R::Bytes { data, length })
    }
    // ------------------------=
    // FUNC: execute
    // DESC: Executes one native repair state transition with bounded persistence and explicit signed authorization.
    // ------------------=
    #[inline(never)]
    pub(crate) fn execute<D: BlockDevice>(
        &mut self,
        store: &mut ObjectStore<D>,
        replica: &mut replicas::service::ReplicaService,
        r: N,
    ) -> Result<R, RemoteError> {
        match r {
            N::Load {object,anchor,now}=>Self::load_operation(store,object,anchor,now),
            other=>self.execute_heavy(store,replica,other),
        }
    }
    // ------------------------=
    // FUNC: load_operation
    // DESC: Isolates one durable overlay observation from all transfer and publication stack frames.
    // ------------------=
    #[inline(never)]
    fn load_operation<D:BlockDevice>(store:&mut ObjectStore<D>,object:[u8;16],anchor:[u8;32],now:u64)->Result<R,RemoteError>{
        let a=load(store,object,false,now)?;
        if a.as_ref().is_some_and(|a|a.anchor.value.record.digest()!=anchor){return Err(RemoteError::Conflict)}
        Ok(R::Overlay(a.map(|a|a.repair)))
    }
    // ------------------------=
    // FUNC: execute_heavy
    // DESC: Keeps transfer and durable mutation locals off the metadata-load dispatch stack.
    // ------------------=
    #[inline(never)]
    fn execute_heavy<D: BlockDevice>(
        &mut self,
        store: &mut ObjectStore<D>,
        replica: &mut replicas::service::ReplicaService,
        r: N,
    ) -> Result<R, RemoteError> {
        match r {
            N::Load {
                object,
                anchor,
                now,
            } => {
                let a = load(store, object, false, now)?;
                if a.as_ref()
                    .is_some_and(|a| a.anchor.value.record.digest() != anchor)
                {
                    return Err(RemoteError::Conflict);
                }
                Ok(R::Overlay(a.map(|a| a.repair)))
            }
            N::Stage { authorization, now } => {
                persist(store, authorization, false, now)?;
                Ok(R::Done)
            }
            N::Publish { authorization, now } => {
                persist(store, authorization, true, now)?;
                self.key = None;
                Ok(R::Done)
            }
            N::Read {
                authorization,
                writer,
                offset,
                length,
                now,
            } => self.read(store, &authorization, writer, writer, now, offset, length),
            N::Wire { request, now } => self.wire(store, replica, request, now).map(R::Wire),
        }
    }
    // ------------------------=
    // FUNC: wire
    // DESC: Separates signed metadata upload, quorum voting and bounded replica transfer without impersonating the original owner.
    // ------------------=
    #[inline(never)]
    fn wire<D: BlockDevice>(
        &mut self,
        store: &mut ObjectStore<D>,
        replica: &mut replicas::service::ReplicaService,
        r: AuthenticatedStorageRequest,
        now: u64,
    ) -> Result<StorageOperationV1, RemoteError> {
        let p = r.payload;
        let mut out = p;
        out.data.fill(0);
        out.length = 0;
        if r.grant == 0 {
            return Err(RemoteError::AccessDenied);
        }
        if self.upload.as_ref().is_some_and(|u| now >= u.expires) {
            self.upload = None
        }
        if p.value == 10 {
            if p.length != 32 || p.offset != REPAIR_AUTHORIZATION_BYTES as u64 {
                return Err(RemoteError::MalformedRequest);
            }
            if self.upload.is_some() {
                return Err(RemoteError::QueueFull);
            }
            self.upload = Some(Upload {
                peer: r.peer.0,
                session: r.session_reference,
                grant: r.grant,
                object: p.object,
                hash: p.data[..32].try_into().unwrap(),
                at: 0,
                expires: now.saturating_add(900),
                bytes: [0; REPAIR_AUTHORIZATION_BYTES],
            });
            return Ok(out);
        }
        if matches!(p.value, 14 | 15) {
            let base = metadata::read_bundle(store, p.object).map_err(|_| RemoteError::NotFound)?;
            if p.length != 32
                || !base.group.members.contains(&r.peer)
                || !base.group.members.contains(&r.local)
            {
                return Err(RemoteError::AccessDenied);
            }
            if p.value == 14 && p.data[..32] != base.value.record.digest() {
                return Err(RemoteError::Conflict);
            }
            let a = load(store, p.object, false, now)?;
            if let Some(a) = a {
                if a.anchor.value != base.value {
                    return Err(RemoteError::Conflict);
                }
                if p.value == 14 {
                    out.offset = a.repair.value.sequence;
                    out.length = 32;
                    out.data[..32].copy_from_slice(&a.repair.value.digest())
                } else {
                    let b = a.encode(0).map_err(|_| RemoteError::AccessDenied)?;
                    if p.data[..32] != a.repair.value.digest()
                        || p.manifest_generation != a.repair.value.sequence
                        || p.offset >= b.len() as u64
                    {
                        return Err(RemoteError::Conflict);
                    }
                    let at = p.offset as usize;
                    let n = 64.min(b.len() - at);
                    out.data[..n].copy_from_slice(&b[at..at + n]);
                    out.length = n as u16;
                }
            } else {
                if p.value == 15 {
                    return Err(RemoteError::NotFound);
                }
                out.offset = 0;
                out.length = 32;
            }
            return Ok(out);
        }
        let u = self.upload.as_mut().ok_or(RemoteError::InvalidState)?;
        if u.peer != r.peer.0
            || u.session != r.session_reference
            || u.grant != r.grant
            || u.object != p.object
        {
            return Err(RemoteError::AccessDenied);
        }
        if p.value == 11 {
            let n = p.length as usize;
            if n == 0 || n > 64 || p.offset != u.at as u64 || u.at + n > u.bytes.len() {
                return Err(RemoteError::MalformedRequest);
            }
            u.bytes[u.at..u.at + n].copy_from_slice(&p.data[..n]);
            u.at += n;
            return Ok(out);
        }
        if u.at != u.bytes.len() || <[u8; 32]>::from(Sha256::digest(u.bytes)) != u.hash {
            return Err(RemoteError::MalformedRequest);
        }
        let a = RepairAuthorization::decode(if p.value == 13 { 0 } else { now }, &u.bytes)
            .map_err(|_| RemoteError::AccessDenied)?;
        if !a.anchor.persistent_authority()||a.repair.grant.expires!=u64::MAX{return Err(RemoteError::AccessDenied)}
        if p.object != a.anchor.manifest.object || r.peer != a.repair.grant.writer {
            return Err(RemoteError::AccessDenied);
        }
        if matches!(p.value, 20..=23) {
            let response = replica.repair_transfer(store, r, &a, now)?;
            if p.value == 20 {
                metadata::publish_received_bundle(store, a.anchor)
                    .map_err(|_| RemoteError::PersistenceFailed)?;
                metadata::write_repair_payload(
                    store,
                    p.object,
                    true,
                    &a.encode(now).map_err(|_| RemoteError::AccessDenied)?,
                )
                .map_err(|_| RemoteError::PersistenceFailed)?;
            }
            return Ok(response);
        }
        if p.value == 25 {
            if p.length != 0 {
                return Err(RemoteError::MalformedRequest);
            }
            let mut inspect = r;
            inspect.payload.value = 23;
            let observed = replica.repair_transfer(store, inspect, &a, now)?;
            if observed.value != 1 || observed.offset != a.anchor.manifest.length {
                return Err(RemoteError::InvalidState);
            }
            out.length = 64;
            out.data[..32].copy_from_slice(&a.repair.value.digest());
            out.data[32..].copy_from_slice(&observed.data[..32]);
            self.upload = None;
            return Ok(out);
        }
        if p.value == 24 {
            let length = p.length as u8;
            return match self.read(store, &a, r.peer, r.local, now, p.offset, length)? {
                R::Bytes { data, length } => {
                    out.data = data;
                    out.length = length as u16;
                    Ok(out)
                }
                _ => Err(RemoteError::InvalidState),
            };
        }
        if !matches!(p.value, 12 | 13) || !a.anchor.group.members.contains(&r.local) {
            return Err(RemoteError::AccessDenied);
        }
        persist(store, a, p.value == 13, now)?;
        out.offset = a
            .anchor
            .group
            .members
            .iter()
            .position(|n| *n == r.local)
            .unwrap() as u64;
        out.length = 32;
        out.data[..32].copy_from_slice(&a.repair.value.digest());
        self.upload = None;
        self.key = None;
        Ok(out)
    }
}
