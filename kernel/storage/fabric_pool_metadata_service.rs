//! Bounded native metadata payload transport. Authentication belongs to IOP;
//! these checks additionally preserve immutable object and owner boundaries.
#[cfg(not(test))]
use super::{fabric as replicas, fabric_pool_metadata as backing};
use crate::runtime::{
    fabric::{
        metadata::Error,
        metadata_bundle::{Bundle, BUNDLE_BYTES},
    },
    iop::{
        remote::{AuthenticatedStorageRequest, RemoteError},
        storage_protocol::StorageOperationV1,
    },
    storage_metadata::{NativeReply, NativeRequest, CONFIG_BYTES},
};
use crate::storage::{
    object::{ObjectError, ObjectId, ObjectStore},
    BlockDevice,
};
#[cfg(test)]
use crate::{fabric_pool_metadata as backing, native_fabric as replicas};
use sha2::{Digest, Sha256};
const CONFIG: &[u8] = b"/system/storage/pool-metadata-access";
#[path = "fabric_pool_metadata_mutation.rs"]
mod mutation;
#[path="fabric_pool_metadata_copy.rs"]
mod shared_copy;
struct Upload {
    peer: [u8; 32],
    local: [u8; 32],
    session: [u8; 16],
    grant: u64,
    object: [u8; 16],
    generation: u64,
    hash: [u8; 32],
    at: usize,
    expires: u64,
    bytes: [u8; BUNDLE_BYTES],
}
struct Snapshot {
    peer: [u8; 32],
    session: [u8; 16],
    grant: u64,
    object: [u8; 16],
    generation: u64,
    root: u64,
    bytes: [u8; BUNDLE_BYTES],
}
pub(crate) struct Service {
    upload: Option<Upload>,
    snapshot: Option<Snapshot>,
    validated: Option<(u64, Bundle)>,
    cache_key: Option<([u8; 32], [u8; 16], u64, [u8; 32], [u8; 32])>,
    cache: [u8; 16384],
}
// ------------------------=
// FUNC: error
// DESC: Maps native metadata failures without manufacturing success or bypassing authority.
// ------------------=
fn error(e: Error) -> RemoteError {
    match e {
        Error::Persistence => RemoteError::PersistenceFailed,
        _ => RemoteError::AccessDenied,
    }
}
impl Service {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates one bounded metadata upload and one verified immutable chunk cache.
    // ------------------=
    pub(crate) const fn new() -> Self {
        Self {
            upload: None,
            snapshot: None,
            validated: None,
            cache_key: None,
            cache: [0; 16384],
        }
    }
    // ------------------------=
    // FUNC: reader_bundle
    // DESC: Reuses cryptographically checked immutable metadata only while the native root is unchanged, checking principal and lease on every byte request.
    // ------------------=
    fn reader_bundle<D:BlockDevice>(&mut self,store:&mut ObjectStore<D>,object:[u8;16],reader:crate::runtime::node::types::NodeId,principal:[u8;16],now:u64)->Result<Bundle,RemoteError>{
        let bundle=match self.validated{Some((root,b))if root==store.generation()&&b.manifest.object==object=>b,_=>{let b=backing::read_bundle(store,object).map_err(error)?;self.validated=Some((store.generation(),b));b}};
        if !bundle.persistent_authority()||bundle.certificate.is_none()||bundle.value.record.deleted||!bundle.grants.iter().flatten().any(|g|g.reader==reader&&g.principal==principal&&now<g.expires){return Err(RemoteError::AccessDenied)}Ok(bundle)
    }
    // ------------------------=
    // FUNC: execute
    // DESC: Executes only typed service-authorized native metadata operations.
    // ------------------=
    #[inline(never)]
    pub(crate) fn execute<D: BlockDevice>(
        &mut self,
        store: &mut ObjectStore<D>,
        request: NativeRequest,
    ) -> Result<NativeReply, RemoteError> {
        match request {
            NativeRequest::ConfigLoad => Self::config_load(store),
            NativeRequest::ConfigSave(bytes) => Self::config_save(store,&bytes),
            NativeRequest::Load { index } => Self::catalog_load(store,index),
            other => self.execute_heavy(store,other),
        }
    }
    // ------------------------=
    // FUNC: config_load
    // DESC: Isolates the small boot configuration read from transaction and content stack frames.
    // ------------------=
    #[inline(never)]
    fn config_load<D:BlockDevice>(store:&mut ObjectStore<D>)->Result<NativeReply,RemoteError>{
        let mut bytes=[0;CONFIG_BYTES];
        match store.resolve(CONFIG){
            Ok(id)=>{if store.read(id,None,&mut bytes).map_err(|_|RemoteError::PersistenceFailed)?!=CONFIG_BYTES{return Err(RemoteError::PersistenceFailed)}},
            Err(ObjectError::NotFound|ObjectError::NamespaceNotFound)=>{},
            Err(_)=>return Err(RemoteError::PersistenceFailed),
        }
        Ok(NativeReply::Config(bytes))
    }
    // ------------------------=
    // FUNC: config_save
    // DESC: Keeps native configuration persistence outside the lightweight operation dispatcher.
    // ------------------=
    #[inline(never)]
    fn config_save<D:BlockDevice>(store:&mut ObjectStore<D>,bytes:&[u8;CONFIG_BYTES])->Result<NativeReply,RemoteError>{
        store.replace_named_state(CONFIG,bytes).map_err(|_|RemoteError::PersistenceFailed)?;Ok(NativeReply::Done)
    }
    // ------------------------=
    // FUNC: catalog_load
    // DESC: Loads one bounded bundle without reserving unrelated mutation or transfer local variables.
    // ------------------=
    #[inline(never)]
    fn catalog_load<D:BlockDevice>(store:&mut ObjectStore<D>,index:usize)->Result<NativeReply,RemoteError>{
        Ok(NativeReply::Bundle(backing::load_bundle(store,index).map_err(error)?))
    }
    // ------------------------=
    // FUNC: execute_heavy
    // DESC: Separates heavyweight transaction operations from the boot configuration and catalog dispatch paths.
    // ------------------=
    #[inline(never)]
    fn execute_heavy<D: BlockDevice>(
        &mut self,
        store: &mut ObjectStore<D>,
        request: NativeRequest,
    ) -> Result<NativeReply, RemoteError> {
        match request {
            NativeRequest::Copy{anchor,request,overlay}=>shared_copy::copy(store,anchor,request,overlay),
            NativeRequest::MutateOverlay{anchor,request,overlay}=>mutation::mutate_overlay(store,anchor,request,Some(overlay)),
            NativeRequest::PlacementOverlay{anchor,expected,next,overlay}=>mutation::placement_overlay(store,anchor,expected,next,Some(overlay)),
            NativeRequest::Mutate { anchor, request } => mutation::mutate(store, anchor, request),
            NativeRequest::PlacementMutate { anchor, expected, next } => mutation::placement(store, anchor, expected, next),
            NativeRequest::FinalizeMutation {
                object,
                request,
                record,
            } => mutation::finalize(store, object, request, record),
            NativeRequest::PendingMutation { index } => mutation::pending(store, index),
            NativeRequest::Source {
                object,
                owner,
                scope,
            } => Ok(NativeReply::Manifest(
                store
                    .pool_manifest(ObjectId(object), owner, scope)
                    .map_err(|_| RemoteError::NotFound)?,
            )),
            NativeRequest::Load { index } => Ok(NativeReply::Bundle(
                backing::load_bundle(store, index).map_err(error)?,
            )),
            NativeRequest::Stage { bundle } => {
                if !bundle.persistent_authority(){return Err(RemoteError::AccessDenied)}
                backing::stage_bundle(store, bundle).map_err(error)?;
                self.cache_key = None;
                Ok(NativeReply::Done)
            }
            NativeRequest::Publish {
                object,
                certificate,
            } => {
                backing::publish_bundle(store, object, certificate).map_err(error)?;
                self.cache_key = None;
                Ok(NativeReply::Done)
            }
            NativeRequest::ConfigLoad => {
                let mut b = [0; CONFIG_BYTES];
                match store.resolve(CONFIG) {
                    Ok(id) => {
                        if store
                            .read(id, None, &mut b)
                            .map_err(|_| RemoteError::PersistenceFailed)?
                            != CONFIG_BYTES
                        {
                            return Err(RemoteError::PersistenceFailed);
                        }
                    }
                    Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => {}
                    Err(_) => return Err(RemoteError::PersistenceFailed),
                }
                Ok(NativeReply::Config(b))
            }
            NativeRequest::ConfigSave(b) => {
                store
                    .replace_named_state(CONFIG, &b)
                    .map_err(|_| RemoteError::PersistenceFailed)?;
                Ok(NativeReply::Done)
            }
            NativeRequest::Wire { request, now } => {
                self.wire(store, request, now).map(NativeReply::Wire)
            }
            NativeRequest::Read {
                object,
                record,
                principal,
                reader,
                now,
                offset,
                length,
                overlay,
            } => {
                let bundle = self.reader_bundle(store,object,reader,principal,now)?;
                if bundle.value.record != record {
                    return Err(RemoteError::Conflict);
                }
                let effective = if let Some(o) = overlay {
                    if o.grant.expires!=u64::MAX{return Err(RemoteError::AccessDenied)}
                    let anchor = bundle.certificate.ok_or(RemoteError::AccessDenied)?;
                    o.certificate
                        .ok_or(RemoteError::AccessDenied)?
                        .validate(
                            &bundle.group,
                            &anchor,
                            &o.grant,
                            &bundle.manifest,
                            &o.manifest,
                            0,
                        )
                        .map_err(error)?;
                    o.validate_available(&bundle.manifest).map_err(error)?;
                    if reader==bundle.group.owner {
                        let proof=crate::runtime::fabric::metadata_repair::RepairAuthorization{anchor:bundle,repair:o};
                        store.with_pool_mutation_authority(ObjectId(object),|store|store.pool_reconcile_certified_manifest(ObjectId(object),bundle.group.owner,0,&proof)).map_err(|_|RemoteError::Conflict)?;
                    }
                    o.manifest
                } else {
                    bundle.manifest
                };
                let m = &effective;
                let n = length as usize;
                if n == 0 || n > 64 || offset.checked_add(n as u64).is_none_or(|e| e > m.length) {
                    return Err(RemoteError::MalformedRequest);
                }
                let mut data = [0; 64];
                let mut start = 0u64;
                let mut copied = 0;
                for chunk in m.chunks.iter().flatten() {
                    let end = start + chunk.bytes as u64;
                    if offset + (copied as u64) >= start
                        && offset + (copied as u64) < end
                        && copied < n
                    {
                        let key = (reader.0, m.object, m.version, chunk.hash, record.digest());
                        if self.cache_key != Some(key) {
                            self.cache_key = None;
                            if reader == m.authority {
                                let current = store
                                    .pool_manifest(ObjectId(object), m.authority, 0)
                                    .map_err(|_| RemoteError::NotFound)?;
                                if current.object != m.object
                                    || current.version != m.version
                                    || current.length != m.length
                                    || current.hash != m.hash
                                    || current.chunks != m.chunks
                                    || current.policy != m.policy
                                    || current.authority != m.authority
                                    || current.authority_generation != m.authority_generation
                                {
                                    return Err(RemoteError::Conflict);
                                }
                                store
                                    .pool_read(
                                        &current,
                                        start,
                                        &mut self.cache[..chunk.bytes as usize],
                                    )
                                    .map_err(|_| RemoteError::RemoteFailure)?;
                                if <[u8; 32]>::from(Sha256::digest(
                                    &self.cache[..chunk.bytes as usize],
                                )) != chunk.hash
                                {
                                    return Err(RemoteError::RemoteFailure);
                                }
                            } else {
                                replicas::service::delegated_replica_chunk(
                                    store,
                                    m,
                                    reader,
                                    start,
                                    &mut self.cache[..chunk.bytes as usize],
                                    chunk.hash,
                                )?;
                            }
                            self.cache_key = Some(key);
                        }
                        let at = (offset + copied as u64 - start) as usize;
                        let take = (n - copied).min(chunk.bytes as usize - at);
                        data[copied..copied + take].copy_from_slice(&self.cache[at..at + take]);
                        copied += take;
                    }
                    start = end;
                }
                if copied != n {
                    return Err(RemoteError::RemoteFailure);
                }
                Ok(NativeReply::Bytes { data, length })
            }
        }
    }
    // ------------------------=
    // FUNC: wire
    // DESC: Fences bounded upload windows to one authenticated session and persists complete signed payloads before receipt signing.
    // ------------------=
    fn wire<D: BlockDevice>(
        &mut self,
        store: &mut ObjectStore<D>,
        r: AuthenticatedStorageRequest,
        now: u64,
    ) -> Result<StorageOperationV1, RemoteError> {
        let p = r.payload;
        let mut out = p;
        out.data.fill(0);
        out.length = 0;
        if p.operation != crate::runtime::iop::storage_protocol::Operation::PoolMetadata
            || r.grant == 0
            || r.peer == r.local
        {
            return Err(RemoteError::AccessDenied);
        }
        if self.upload.as_ref().is_some_and(|u| now >= u.expires) {
            self.upload = None;
        }
        if p.value == 6 {
            if p.length != 1 || !(1..=64).contains(&p.data[0]) {
                return Err(RemoteError::MalformedRequest);
            }
            let b = self.reader_bundle(store,p.object,r.peer,crate::runtime::storage_metadata::principal(),now)?;
            if p.object_version != b.value.record.generation {
                return Err(RemoteError::Conflict);
            }
            let mut m = b.manifest;
            if !m.placements.iter().flatten().any(|v| v.node == r.local) {
                let mut bytes =
                    [0; crate::runtime::fabric::metadata_repair::REPAIR_AUTHORIZATION_BYTES];
                let n = backing::read_repair_payload(store, p.object, true, &mut bytes)
                    .map_err(error)?;
                if n != bytes.len() {
                    return Err(RemoteError::NotFound);
                }
                let a =
                    crate::runtime::fabric::metadata_repair::RepairAuthorization::decode(0, &bytes)
                        .map_err(error)?;
                if a.anchor.value != b.value || !a.repair.grant.destinations.contains(&r.local) {
                    return Err(RemoteError::AccessDenied);
                }
                m = a.repair.manifest;
            }
            let n = p.data[0] as usize;
            if p.offset.checked_add(n as u64).is_none_or(|e| e > m.length) {
                return Err(RemoteError::MalformedRequest);
            }
            let mut start = 0u64;
            let mut copied = 0;
            for c in m.chunks.iter().flatten() {
                let end = start + c.bytes as u64;
                if p.offset + (copied as u64) >= start
                    && p.offset + (copied as u64) < end
                    && copied < n
                {
                    let key = (
                        r.local.0,
                        m.object,
                        m.version,
                        c.hash,
                        b.value.record.digest(),
                    );
                    if self.cache_key != Some(key) {
                        self.cache_key = None;
                        replicas::service::delegated_replica_chunk(
                            store,
                            &m,
                            r.local,
                            start,
                            &mut self.cache[..c.bytes as usize],
                            c.hash,
                        )?;
                        self.cache_key = Some(key)
                    }
                    let at = (p.offset + copied as u64 - start) as usize;
                    let take = (n - copied).min(c.bytes as usize - at);
                    out.data[copied..copied + take].copy_from_slice(&self.cache[at..at + take]);
                    copied += take;
                }
                start = end;
            }
            if copied != n {
                return Err(RemoteError::RemoteFailure);
            }
            out.length = n as u16;
            return Ok(out);
        }
        if p.value==7 {
            let b=backing::read_bundle(store,p.object).map_err(error)?;
            if !b.group.members.contains(&r.peer)||!b.group.members.contains(&r.local){return Err(RemoteError::AccessDenied)}
            b.certificate.ok_or(RemoteError::Conflict)?.validate(&b.group).map_err(error)?;
            if p.length!=32||p.offset!=0||p.manifest_generation!=b.value.record.generation||p.data[..32]!=b.value.record.digest(){return Err(RemoteError::Conflict)}
            out.data[..32].copy_from_slice(&b.value.record.digest());out.length=32;out.offset=b.group.members.iter().position(|n|*n==r.local).ok_or(RemoteError::AccessDenied)? as u64;
            return Ok(out);
        }
        if p.value == 0 {
            if p.length != 32 || p.offset != BUNDLE_BYTES as u64 || p.manifest_generation == 0 {
                return Err(RemoteError::MalformedRequest);
            }
            if self.upload.is_some() {
                return Err(RemoteError::QueueFull);
            }
            self.upload = Some(Upload {
                peer: r.peer.0,
                local: r.local.0,
                session: r.session_reference,
                grant: r.grant,
                object: p.object,
                generation: p.manifest_generation,
                hash: p.data[..32].try_into().unwrap(),
                at: 0,
                expires: now.saturating_add(120),
                bytes: [0; BUNDLE_BYTES],
            });
            return Ok(out);
        }
        if p.value == 4 || p.value == 5 {
            if p.value == 5 {
                if let Some(s) = self.snapshot.as_ref().filter(|s| {
                    s.peer == r.peer.0
                        && s.session == r.session_reference
                        && s.grant == r.grant
                        && s.object == p.object
                        && s.generation == p.object_version
                        && s.root == store.generation()
                }) {
                    if p.offset >= BUNDLE_BYTES as u64 {
                        return Err(RemoteError::MalformedRequest);
                    }
                    let at = p.offset as usize;
                    let n = 64.min(BUNDLE_BYTES - at);
                    out.data[..n].copy_from_slice(&s.bytes[at..at + n]);
                    out.length = n as u16;
                    out.manifest_generation = s.generation;
                    return Ok(out);
                }
            }
            let b = backing::read_bundle(store, p.object).map_err(error)?;
            if !b.group.members.contains(&r.peer) || !b.group.members.contains(&r.local) {
                return Err(RemoteError::AccessDenied);
            }
            out.manifest_generation = b.value.record.generation;
            out.object_version = b.value.record.generation;
            if p.value == 4 {
                out.data[..32].copy_from_slice(&b.value.record.digest());
                out.length = 32;
                out.offset = BUNDLE_BYTES as u64;
                self.snapshot = Some(Snapshot {
                    peer: r.peer.0,
                    session: r.session_reference,
                    grant: r.grant,
                    object: p.object,
                    generation: b.value.record.generation,
                    root: store.generation(),
                    bytes: b.encode().map_err(error)?,
                });
            } else {
                if p.object_version != b.value.record.generation || p.offset >= BUNDLE_BYTES as u64
                {
                    return Err(RemoteError::Conflict);
                }
                let bytes = b.encode().map_err(error)?;
                let at = p.offset as usize;
                let n = 64.min(BUNDLE_BYTES - at);
                out.data[..n].copy_from_slice(&bytes[at..at + n]);
                out.length = n as u16;
                self.snapshot=Some(Snapshot{peer:r.peer.0,session:r.session_reference,grant:r.grant,object:p.object,generation:b.value.record.generation,root:store.generation(),bytes});
            }
            return Ok(out);
        }
        let u = self.upload.as_mut().ok_or(RemoteError::InvalidState)?;
        if u.peer != r.peer.0
            || u.local != r.local.0
            || u.session != r.session_reference
            || u.grant != r.grant
            || u.object != p.object
            || u.generation != p.manifest_generation
        {
            return Err(RemoteError::AccessDenied);
        }
        if p.value == 1 {
            let n = p.length as usize;
            if n == 0 || n > 64 || p.offset != u.at as u64 || u.at + n > BUNDLE_BYTES {
                return Err(RemoteError::MalformedRequest);
            }
            u.bytes[u.at..u.at + n].copy_from_slice(&p.data[..n]);
            u.at += n;
            return Ok(out);
        }
        if !matches!(p.value, 2 | 3)
            || p.length != 0
            || u.at != BUNDLE_BYTES
            || <[u8; 32]>::from(Sha256::digest(u.bytes)) != u.hash
        {
            return Err(RemoteError::MalformedRequest);
        }
        let b = Bundle::decode(&u.bytes).map_err(error)?;
        if !b.persistent_authority(){return Err(RemoteError::AccessDenied)}
        if b.value.record.object != p.object
            || b.value.record.generation != u.generation
            || !b.group.members.contains(&r.local)
            || !b.group.members.contains(&r.peer)
        {
            return Err(RemoteError::AccessDenied);
        }
        if p.value == 2 {
            if r.peer != b.group.owner {
                return Err(RemoteError::AccessDenied);
            }
            backing::stage_bundle(store, b).map_err(error)?;
        } else {
            backing::publish_received_bundle(store, b).map_err(error)?;
        }
        out.data[..32].copy_from_slice(&b.value.record.digest());
        out.length = 32;
        out.offset = b.group.members.iter().position(|n| *n == r.local).unwrap() as u64;
        self.upload = None;
        self.cache_key = None;
        Ok(out)
    }
}
