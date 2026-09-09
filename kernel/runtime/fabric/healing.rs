//! Generation-fenced healing coordination. The owning native service supplies
//! the durable manifest commit and authenticates every resource/replica report.
use super::{manifest::{Manifest, ManifestError, HealingClaim, Placement, PlacementState},
    placement::{Availability, select}, resources::{Directory, Resource, ResourceId, Health},
    replica::{Checkpoint, ReplicaState}};
use crate::runtime::node::types::NodeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealError { AccessDenied, Stale, Busy, NoSource, NoCapacity, NoWork, Invalid, Persistence }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Work { pub source: Placement, pub destination: Resource, pub claim: HealingClaim }

// ------------------------=
// FUNC: eligible
// DESC: Requires the exact current physical generation and an unexpired healthy resource without inferring availability from mere discovery.
// ------------------=
fn eligible(directory: &Directory, placement: Placement, now: u64) -> bool {
    directory.entries().iter().flatten().any(|r| r.id == placement.resource
        && r.owner == placement.node && r.device == placement.device
        && r.generation == placement.generation && r.online && r.expires > now
        && r.health == Health::Healthy)
}

// ------------------------=
// FUNC: authorize
// DESC: Fences coordinator actions to the persisted owner and exact manifest generation; peer grants are separately checked at IOP admission and execution.
// ------------------=
fn authorize(manifest: &Manifest, actor: NodeId, expected: u64) -> Result<(), HealError> {
    manifest.validate().map_err(|_| HealError::Invalid)?;
    if actor != manifest.authority { return Err(HealError::AccessDenied); }
    if expected != manifest.generation { return Err(HealError::Stale); }
    Ok(())
}

// ------------------------=
// FUNC: advance
// DESC: Produces one checked manifest successor; state is not published or mutated until its owning transaction succeeds.
// ------------------=
fn advance(manifest: &Manifest) -> Result<Manifest, HealError> {
    let mut next = *manifest;
    next.generation = next.generation.checked_add(1).ok_or(HealError::Invalid)?;
    Ok(next)
}

// ------------------------=
// FUNC: commit
// DESC: Validates a single successor and waits for the native durable commit before changing any visible manifest state.
// ------------------=
fn commit(current: &mut Manifest, next: Manifest,
    persist: impl FnOnce(u64, &Manifest) -> Result<(), ManifestError>) -> Result<(), HealError> {
    current.successor(&next).map_err(|_| HealError::Invalid)?;
    persist(current.generation, &next).map_err(|error| match error {
        ManifestError::Stale | ManifestError::Conflict => HealError::Stale,
        _ => HealError::Persistence,
    })?;
    *current = next;
    Ok(())
}

// ------------------------=
// FUNC: observe_loss
// DESC: Durably marks expired or lost replicas unavailable while retaining object identity, namespace-independent content and physical placement history.
// ------------------=
pub(crate) fn observe_loss(manifest: &mut Manifest, directory: &Directory, actor: NodeId,
    expected: u64, now: u64, persist: impl FnOnce(u64, &Manifest) -> Result<(), ManifestError>) -> Result<bool, HealError> {
    authorize(manifest, actor, expected)?;
    let mut next = advance(manifest)?;
    let mut changed = false;
    for record in next.placements.iter_mut().flatten() {
        if record.state == PlacementState::Verified && !eligible(directory, *record, now) {
            record.state = PlacementState::Offline;
            changed = true;
        }
    }
    if !changed { return Ok(false); }
    commit(manifest, next, persist)?;
    Ok(true)
}

// ------------------------=
// FUNC: begin
// DESC: Claims one bounded repair and reserves an independent destination atomically with respect to publication; failed commits cannot consume inventory capacity.
// ------------------=
pub(crate) fn begin(manifest: &mut Manifest, directory: &mut Directory, actor: NodeId,
    expected: u64, now: u64, expires: u64,
    persist: impl FnOnce(u64, &Manifest) -> Result<(), ManifestError>) -> Result<Work, HealError> {
    authorize(manifest, actor, expected)?;
    if expires <= now || expires - now > 3600 { return Err(HealError::Invalid); }
    // An expired claim still owns its reservation until explicitly cancelled or
    // recovered. A second coordinator must never silently spend it again.
    if manifest.healing.is_some() { return Err(HealError::Busy); }
    if manifest.availability() == Availability::Healthy { return Err(HealError::NoWork); }
    let source = manifest.placements.iter().flatten().copied().find(|p|
        p.state == PlacementState::Verified && p.version == manifest.version
        && p.hash == manifest.hash && eligible(directory, *p, now)).ok_or(HealError::NoSource)?;
    let mut occupied = [NodeId([0; 32]); 8];
    let mut excluded = [ResourceId([0; 16]); 8];
    let mut count = 0;
    let mut seen = 0;
    for p in manifest.placements.iter().flatten() {
        excluded[seen] = p.resource; seen += 1;
        if p.state == PlacementState::Verified && eligible(directory, *p, now) {
            occupied[count] = p.node; count += 1;
        }
    }
    if manifest.placements.iter().all(Option::is_some) { return Err(HealError::NoCapacity); }
    // Native extents allocate 4-KiB blocks, including empty content, plus a
    // checkpoint block. Reserve actual allocation rather than logical bytes.
    let blocks = manifest.length.checked_add(4095).ok_or(HealError::Invalid)? / 4096;
    let reserved = blocks.max(1).checked_add(1).and_then(|n| n.checked_mul(4096)).ok_or(HealError::Invalid)?;
    let destination = select(directory, &occupied[..count], &excluded[..seen], reserved, now).ok_or(HealError::NoCapacity)?;
    let mut staged = directory.clone();
    staged.reserve(destination.id, destination.generation, reserved, now).map_err(|_| HealError::NoCapacity)?;
    let mut next = advance(manifest)?;
    let claim = HealingClaim { owner: actor, token: next.generation, expires, destination: destination.id,
        destination_generation: destination.generation, reserved };
    next.healing = Some(claim);
    commit(manifest, next, persist)?;
    *directory = staged;
    Ok(Work { source, destination, claim })
}

// ------------------------=
// FUNC: complete
// DESC: Publishes only an authenticated durably Available exact-version receipt under a live matching claim; reservation release follows the authoritative manifest commit.
// ------------------=
pub(crate) fn complete(manifest: &mut Manifest, directory: &mut Directory, actor: NodeId,
    expected: u64, now: u64, token: u64, receipt: Checkpoint,
    persist: impl FnOnce(u64, &Manifest) -> Result<(), ManifestError>) -> Result<(), HealError> {
    let d = receipt.descriptor;
    if actor != manifest.authority { return Err(HealError::AccessDenied); }
    // A lost reply may repeat the already committed publication. Its complete
    // identity and generation must match; no further commit or release occurs.
    if manifest.healing.is_none() && expected.checked_add(1) == Some(manifest.generation)
        && token != 0 && d.job == token && receipt.state == ReplicaState::Available
        && d.object == manifest.object && d.version == manifest.version && d.hash == manifest.hash
        && d.bytes == manifest.length && receipt.copied == manifest.length
        && manifest.placements.iter().flatten().any(|p| p.resource == d.resource
            && p.generation == d.generation && p.version == d.version && p.hash == d.hash
            && p.admission_generation == token
            && p.state == PlacementState::Verified) { return Ok(()); }
    authorize(manifest, actor, expected)?;
    let claim = manifest.healing.ok_or(HealError::Stale)?;
    if claim.token != token || d.job != token || now >= claim.expires { return Err(HealError::Stale); }
    if receipt.state != ReplicaState::Available || receipt.copied != manifest.length
        || d.object != manifest.object || d.version != manifest.version || d.hash != manifest.hash
        || d.bytes != manifest.length || d.resource != claim.destination
        || d.generation != claim.destination_generation { return Err(HealError::Invalid); }
    let resource = directory.entries().iter().flatten().find(|r| r.id == d.resource
        && r.generation == d.generation).copied().ok_or(HealError::Stale)?;
    let placement = Placement { node: resource.owner, resource: resource.id, device: resource.device,
        generation: d.generation, version: d.version, hash: d.hash, state: PlacementState::Verified,
        admission_generation: claim.token };
    if !eligible(directory, placement, now) { return Err(HealError::Stale); }
    if manifest.placements.iter().flatten().any(|p| p.resource == placement.resource
        || (p.node == placement.node && p.state == PlacementState::Verified)) { return Err(HealError::Invalid); }
    let mut next = advance(manifest)?;
    let slot = next.placements.iter_mut().find(|p| p.is_none()).ok_or(HealError::NoCapacity)?;
    *slot = Some(placement); next.healing = None;
    let mut staged = directory.clone();
    staged.release(d.resource, d.generation, claim.reserved).map_err(|_| HealError::Invalid)?;
    commit(manifest, next, persist)?;
    *directory = staged;
    Ok(())
}

// ------------------------=
// FUNC: cancel
// DESC: Releases only the exact authoritative claim after its cancellation is durable; stale or competing cancellations cannot free another repair's capacity.
// ------------------=
pub(crate) fn cancel(manifest: &mut Manifest, directory: &mut Directory, actor: NodeId,
    expected: u64, token: u64,
    persist: impl FnOnce(u64, &Manifest) -> Result<(), ManifestError>) -> Result<(), HealError> {
    authorize(manifest, actor, expected)?;
    let claim = manifest.healing.ok_or(HealError::Stale)?;
    if claim.token != token { return Err(HealError::Stale); }
    let mut staged = directory.clone();
    staged.release(claim.destination, claim.destination_generation, claim.reserved).map_err(|_| HealError::Invalid)?;
    let mut next = advance(manifest)?; next.healing = None;
    commit(manifest, next, persist)?;
    *directory = staged;
    Ok(())
}

// ------------------------=
// FUNC: recover
// DESC: Rebuilds reservations from a bounded complete authoritative manifest set without double counting duplicate objects or discarding expired pending repairs.
// ------------------=
pub(crate) fn recover(directory: &mut Directory, manifests: &[Manifest]) -> Result<(), HealError> {
    if manifests.len() > 16 { return Err(HealError::Invalid); }
    for (index, manifest) in manifests.iter().enumerate() {
        manifest.validate().map_err(|_| HealError::Invalid)?;
        if manifests[..index].iter().any(|prior| prior.object == manifest.object) { return Err(HealError::Invalid); }
    }
    directory.restore_reservations(manifests.iter().filter_map(|m| m.healing)
        .map(|c| (c.destination, c.destination_generation, c.reserved))).map_err(|_| HealError::Stale)
}

// ------------------------=
// FUNC: reconcile
// DESC: Reconciles an authenticated durable replica report against its known physical placement, never promoting old versions or mismatched hashes into the current object.
// ------------------=
pub(crate) fn reconcile(manifest: &mut Manifest, directory: &Directory, actor: NodeId,
    expected: u64, now: u64, mut report: Placement,
    persist: impl FnOnce(u64, &Manifest) -> Result<(), ManifestError>) -> Result<PlacementState, HealError> {
    authorize(manifest, actor, expected)?;
    let index = manifest.placements.iter().position(|p| p.is_some_and(|p|
        p.resource == report.resource && p.node == report.node && p.device == report.device)).ok_or(HealError::Invalid)?;
    let old = manifest.placements[index].unwrap();
    if report.generation < old.generation || !eligible(directory, report, now) { return Err(HealError::Stale); }
    if report.state != PlacementState::Verified { return Err(HealError::Invalid); }
    report.state = if report.version != manifest.version { PlacementState::Stale }
        else if report.hash != manifest.hash { PlacementState::Corrupt } else { PlacementState::Verified };
    if report == old { return Ok(report.state); }
    let mut next = advance(manifest)?; next.placements[index] = Some(report);
    commit(manifest, next, persist)?;
    Ok(report.state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::fabric::{manifest::{Chunk, MAX_CHUNKS, MAX_PLACEMENTS},
        placement::StorageClass, resources::ResourceKind, replica::ReplicaDescriptor};
    use sha2::{Digest, Sha256};

    // ------------------------=
    // FUNC: fixture
    // DESC: Creates explicitly host-fixture resources and a Critical object with three verified independent replicas.
    // ------------------=
    fn fixture() -> (Manifest, Directory) {
        let hash = Sha256::digest([8; 32]).into();
        let mut m = Manifest { object: [9; 16], version: 1, length: 32, hash, policy: StorageClass::Critical,
            minimum_available: 1, generation: 1, authority: NodeId([1; 32]), authority_generation: 1,
            chunks: [None; MAX_CHUNKS], placements: [None; MAX_PLACEMENTS], healing: None };
        m.chunks[0] = Some(Chunk { content: [8; 16], bytes: 32, hash });
        let mut directory = Directory::new();
        for n in 1..=4 {
            let r = Resource { id: ResourceId([n; 16]), owner: NodeId([n; 32]), device: [n; 16], kind: ResourceKind::Storage,
                capacity: 65536, available: 65536, reserved: 0, health: Health::Healthy, online: true,
                capabilities: 1, generation: 1, sequence: 1, expires: 100 };
            directory.apply(r, 1).unwrap();
            if n < 4 { m.placements[n as usize - 1] = Some(Placement { node: r.owner, resource: r.id,
                device: r.device, generation: 1, version: 1, hash, state: PlacementState::Verified, admission_generation: 1 }); }
        }
        (m, directory)
    }

    // ------------------------=
    // FUNC: lose_third
    // DESC: Applies an actual resource transition and exercises the production committed degradation path.
    // ------------------=
    fn lose_third(m: &mut Manifest, d: &mut Directory) {
        let mut resource = d.entries()[2].unwrap(); resource.online = false; resource.sequence += 1;
        d.apply(resource, 2).unwrap();
        observe_loss(m, d, m.authority, m.generation, 2, |_, _| Ok(())).unwrap();
        assert_eq!(m.availability(), Availability::Degraded);
    }

    // ------------------------=
    // FUNC: claim_and_publication_are_fenced_and_commit_ordered
    // DESC: Exercises competing owners, commit failure, incomplete and corrupt receipts, successful recovery, and idempotent lost-ACK completion.
    // ------------------=
    #[test]
    fn claim_and_publication_are_fenced_and_commit_ordered() {
        let (mut m, mut d) = fixture(); let actor = m.authority;
        lose_third(&mut m, &mut d); let before = m;
        assert_eq!(begin(&mut m, &mut d, NodeId([5; 32]), before.generation, 3, 90, |_, _| Ok(())), Err(HealError::AccessDenied));
        assert_eq!(begin(&mut m, &mut d, actor, before.generation, 3, 90, |_, _| Err(ManifestError::Storage)), Err(HealError::Persistence));
        assert_eq!(m, before); assert_eq!(d.usable(3, 3), 65536);
        let work = begin(&mut m, &mut d, actor, before.generation, 3, 90, |_, _| Ok(())).unwrap();
        assert_eq!(work.destination.owner, NodeId([4; 32]));
        assert_eq!(d.usable(3, 3), 65536 - work.claim.reserved);
        let claimed = m;
        assert_eq!(begin(&mut m, &mut d, actor, claimed.generation, 4, 80, |_, _| Ok(())), Err(HealError::Busy));
        let receipt = Checkpoint { descriptor: ReplicaDescriptor { job: work.claim.token, object: m.object,
            version: m.version, resource: work.destination.id, generation: work.destination.generation,
            bytes: m.length, hash: m.hash }, copied: m.length, state: ReplicaState::Available };
        for bad in [Checkpoint { state: ReplicaState::Verifying, ..receipt }, Checkpoint { copied: 0, ..receipt },
            Checkpoint { descriptor: ReplicaDescriptor { hash: [0; 32], ..receipt.descriptor }, ..receipt }] {
            assert_eq!(complete(&mut m, &mut d, actor, claimed.generation, 5, work.claim.token, bad, |_, _| Ok(())), Err(HealError::Invalid));
            assert_eq!(m, claimed);
        }
        assert_eq!(complete(&mut m, &mut d, actor, claimed.generation, 5, work.claim.token, receipt,
            |_, _| Err(ManifestError::Storage)), Err(HealError::Persistence));
        assert_eq!(m, claimed); assert_eq!(d.usable(3, 5), 65536 - work.claim.reserved);
        complete(&mut m, &mut d, actor, claimed.generation, 5, work.claim.token, receipt, |_, _| Ok(())).unwrap();
        assert_eq!(m.availability(), Availability::Healthy); assert!(m.healing.is_none());
        assert_eq!(d.usable(3, 5), 65536);
        complete(&mut m, &mut d, actor, claimed.generation, 6, work.claim.token, receipt, |_, _| panic!("duplicate commit")).unwrap();
    }

    // ------------------------=
    // FUNC: reboot_restores_claim_capacity_and_stale_owner_cannot_release_it
    // DESC: Recovers persisted claim encoding into fresh accounting and proves expiry, duplicate input and failed cancellation preserve exact reservations.
    // ------------------=
    #[test]
    fn reboot_restores_claim_capacity_and_stale_owner_cannot_release_it() {
        let (mut m, mut d) = fixture(); let actor = m.authority;
        lose_third(&mut m, &mut d); let expected = m.generation;
        let w = begin(&mut m, &mut d, actor, expected, 3, 10, |_, _| Ok(())).unwrap();
        let mut bytes = [0; super::super::manifest::MANIFEST_BYTES]; m.encode(&mut bytes).unwrap();
        let mut restored = Manifest::decode(&bytes).unwrap();
        let (_, mut inventory) = fixture(); recover(&mut inventory, &[restored]).unwrap();
        assert_eq!(inventory.usable(3, 12), 65536 - w.claim.reserved);
        assert_eq!(recover(&mut inventory, &[restored, restored]), Err(HealError::Invalid));
        let expected = restored.generation;
        assert_eq!(begin(&mut restored, &mut inventory, actor, expected, 12, 30, |_, _| Ok(())), Err(HealError::Busy));
        assert_eq!(cancel(&mut restored, &mut inventory, actor, expected, w.claim.token + 1, |_, _| Ok(())), Err(HealError::Stale));
        assert_eq!(cancel(&mut restored, &mut inventory, actor, expected, w.claim.token, |_, _| Err(ManifestError::Storage)), Err(HealError::Persistence));
        assert_eq!(inventory.usable(3, 12), 65536 - w.claim.reserved);
        cancel(&mut restored, &mut inventory, actor, expected, w.claim.token, |_, _| Ok(())).unwrap();
        assert_eq!(inventory.usable(3, 12), 65536); assert!(restored.healing.is_none());
    }

    // ------------------------=
    // FUNC: stale_return_never_replaces_current_content
    // DESC: Classifies authenticated old-version and corrupt reports without changing current identity/content and promotes only the exact verified current version.
    // ------------------=
    #[test]
    fn stale_return_never_replaces_current_content() {
        let (mut m, mut d) = fixture(); let actor = m.authority;
        lose_third(&mut m, &mut d);
        let old = m.placements[2].unwrap();
        m.version += 1; m.generation += 1;
        for p in m.placements.iter_mut().flatten() { if p.state == PlacementState::Verified { p.version = m.version; } }
        let mut r = d.entries()[2].unwrap(); r.online = true; r.sequence += 1; d.apply(r, 4).unwrap();
        let gen = m.generation;
        assert_eq!(reconcile(&mut m, &d, actor, gen, 5, Placement { state: PlacementState::Verified, ..old }, |_, _| Ok(())), Ok(PlacementState::Stale));
        assert_eq!(m.version, 2); assert_eq!(m.object, [9; 16]); assert_eq!(m.availability(), Availability::Degraded);
        let gen = m.generation;
        assert_eq!(reconcile(&mut m, &d, actor, gen, 5, Placement { version: 2, hash: [0; 32], state: PlacementState::Verified, ..old }, |_, _| Ok(())), Ok(PlacementState::Corrupt));
        let gen = m.generation;
        assert_eq!(reconcile(&mut m, &d, actor, gen, 5, Placement { version: 2, state: PlacementState::Verified, ..old }, |_, _| Ok(())), Ok(PlacementState::Verified));
        assert_eq!(m.availability(), Availability::Healthy);
    }
}
