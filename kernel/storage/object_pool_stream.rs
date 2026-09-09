//! Bounded transaction primitives for multi-extent Pool ownership.
use super::*;

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: pool_reserve_extents
    // DESC: Reserves up to sixteen independent unpublished extents and their restart descriptor atomically.
    // ------------------=
    pub(crate) fn pool_reserve_extents<const N: usize>(
        &mut self,
        checkpoint: ObjectId,
        length: u32,
        encode: impl FnOnce([ObjectId; 16]) -> [u8; N],
    ) -> Result<(), ObjectError> {
        if length > 262144 || N > MAX_CONTENT {
            return Err(ObjectError::InsufficientCapacity);
        }
        let before = self.begin()?;
        let result = (|| {
            let mut ids = [ObjectId([0; 16]); 16];
            for (i, id) in ids
                .iter_mut()
                .enumerate()
                .take((length as usize + 16383) / 16384)
            {
                *id = self.staging_extent_record((length as usize - i * 16384).min(16384) as u32)?;
            }
            self.replace_state_record(checkpoint, &encode(ids))?;
            Ok(())
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: pool_publish_extents
    // DESC: Seals verified extents and atomically publishes stable application content, manifest, catalog and cleared upload descriptor.
    // ------------------=
    pub(crate) fn pool_publish_extents<const N: usize, const M: usize, const K: usize>(
        &mut self,
        catalog: ObjectId,
        checkpoint: ObjectId,
        ids: &[ObjectId; 16],
        crcs: &[u32; 16],
        target: Option<ObjectId>,
        backing: Option<ObjectId>,
        content: &[u8],
        encode: impl FnOnce(ObjectId, ObjectId, u32) -> Result<([u8; N], [u8; M], [u8; K]), ObjectError>,
    ) -> Result<(), ObjectError> {
        if N > MAX_CONTENT || M > MAX_CONTENT || K > MAX_CONTENT {
            return Err(ObjectError::InsufficientCapacity);
        }
        let before = self.begin()?;
        let result = (|| {
            for (i, id) in ids.iter().enumerate().filter(|(_, id)| id.0 != [0; 16]) {
                let slot = self
                    .state
                    .versions
                    .iter()
                    .position(|v| v.used && v.object == *id && v.storage_role == 1)
                    .ok_or(ObjectError::InvalidVersion)?;
                let oi = self.object_index(*id)?;
                self.state.versions[slot].storage_role = 2;
                self.state.versions[slot].number = 1;
                self.state.versions[slot].content_crc = crcs[i];
                self.state.objects[oi].current_version = 1;
            }
            let object = match target {
                Some(id) => id,
                None => {
                    self.create_record(b"Pool Object", ObjectType::Metadata, Space::Personal)?
                }
            };
            let version = self.write_record(object, content)?;
            let vi = self
                .state
                .versions
                .iter()
                .position(|v| v.used && v.object == object && v.number == version)
                .ok_or(ObjectError::InvalidVersion)?;
            self.state.versions[vi].storage_role = 3;
            let manifest = match backing {
                Some(id) => id,
                None => {
                    self.create_record(b"object-manifest", ObjectType::Metadata, Space::System)?
                }
            };
            let (catalog_bytes, manifest_bytes, receipt) = encode(object, manifest, version)?;
            self.replace_state_record(manifest, &manifest_bytes)?;
            self.replace_state_record(catalog, &catalog_bytes)?;
            self.replace_state_record(checkpoint, &receipt)?;
            Ok(())
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: pool_version_numbers
    // DESC: Returns bounded retained version identities for reachability accounting without exposing mutable storage metadata.
    // ------------------=
    pub(crate) fn pool_version_numbers(&self, object: ObjectId, out: &mut [u32; 64]) -> usize {
        let mut n = 0;
        for v in self
            .state
            .versions
            .iter()
            .filter(|v| v.used && v.object == object && v.storage_role == 3)
        {
            out[n] = v.number;
            n += 1;
        }
        n
    }
    // ------------------------=
    // FUNC: pool_is_index
    // DESC: Uses typed on-disk storage role rather than user-controlled content bytes to distinguish immutable extent indexes.
    // ------------------=
    pub(crate) fn pool_is_index(&self, object: ObjectId, version: u32) -> bool {
        self.state
            .versions
            .iter()
            .any(|v| v.used && v.object == object && v.number == version && v.storage_role == 3)
    }
    // ------------------------=
    // FUNC: pool_index_objects
    // DESC: Enumerates all live native index owners including copies made outside the Pool catalog, preserving namespace-independent reachability.
    // ------------------=
    pub(crate) fn pool_index_objects(&self, out: &mut [ObjectId; 52]) -> usize {
        let mut n = 0;
        for o in self.state.objects.iter().filter(|o| o.used && !o.tombstone) {
            if self
                .state
                .versions
                .iter()
                .any(|v| v.used && v.object == o.id && v.storage_role == 3)
            {
                out[n] = o.id;
                n += 1;
            }
        }
        n
    }
    // ------------------------=
    // FUNC: pool_retire_owned
    // DESC: Atomically removes authorized private ownership records and references; shared physical version extents remain allocated until every native reference disappears.
    // ------------------=
    pub(crate) fn pool_retire_owned(
        &mut self,
        records: &[ObjectId],
        catalog: ObjectId,
        bytes: &[u8],
    ) -> Result<(), ObjectError> {
        self.pool_retire_owned_with_state(records,catalog,bytes,None)
    }
    // ------------------------=
    // FUNC: pool_retire_owned_with_state
    // DESC: Atomically retires owned records while preserving an optional durable remote-reclamation outbox under the same root.
    // ------------------=
    pub(crate) fn pool_retire_owned_with_state(&mut self, records:&[ObjectId], catalog:ObjectId, bytes:&[u8],
        extra:Option<(ObjectId,&[u8])>)->Result<(),ObjectError> {
        let before = self.begin()?;
        let result = (|| {
            for id in records.iter().filter(|id| id.0 != [0; 16]) {
                if *id == catalog {
                    return Err(ObjectError::InvalidObject);
                }
                let oi = self.object_index(*id)?;
                if self
                    .state
                    .relationships
                    .iter()
                    .any(|r| r.used && (r.source == *id || r.target == *id))
                {
                    return Err(ObjectError::Unauthorized);
                }
                for e in self
                    .state
                    .entries
                    .iter_mut()
                    .filter(|e| e.used && e.target == *id)
                {
                    e.used = false;
                }
                // Keep old blocks allocated until replacement metadata has been written.
                self.state.objects[oi].tombstone = true;
            }
            self.replace_state_record(catalog, bytes)?;
            if let Some((id,bytes))=extra {
                if id==catalog || records.contains(&id) {return Err(ObjectError::InvalidObject);}
                self.replace_state_record(id,bytes)?;
            }
            for id in records.iter().filter(|id| id.0 != [0; 16]) {
                for vi in 0..MAX_VERSIONS {
                    let v = self.state.versions[vi];
                    if v.used && v.object == *id {
                        self.state.versions[vi].used = false;
                        for block in v.extent as usize..v.extent as usize + v.blocks as usize {
                            self.release_unreferenced_block(block);
                        }
                    }
                }
                let oi = self
                    .state
                    .objects
                    .iter()
                    .position(|o| o.used && o.id == *id)
                    .ok_or(ObjectError::NotFound)?;
                self.state.objects[oi].used = false;
            }
            self.compact_objects();
            Ok(())
        })();
        self.finish(before, result)
    }
}
