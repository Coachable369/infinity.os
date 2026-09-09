//! Transaction composition independent of runtime services and wire protocols.
//! Owning services provide their bounded encoded metadata before root commit.
use super::*;

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: reserved_system_metadata_id
    // DESC: Resolves one internal System metadata identity without namespace slots; the public object service forbids reserved System names and access.
    // ------------------=
    pub(crate) fn reserved_system_metadata_id(&self,name:&[u8],legacy_path:&[u8])->Result<Option<ObjectId>,ObjectError>{
        if name.first()!=Some(&b'@')||name.len()>47||core::str::from_utf8(name).is_err(){return Err(ObjectError::InvalidObject)}
        let legacy=match self.resolve(legacy_path){Ok(id)=>Some(id),Err(ObjectError::NotFound|ObjectError::NamespaceNotFound)=>None,Err(e)=>return Err(e)};
        if let Some(id)=legacy{let o=&self.state.objects[self.object_index(id)?];if o.tombstone||o.kind!=ObjectType::Metadata as u8||o.space!=Space::System as u8||o.owner.0!=[0;16]{return Err(ObjectError::CorruptContent)}}
        let mut found=None;
        for o in self.state.objects.iter().filter(|o|o.used&&!o.tombstone&&o.name_len as usize==name.len()&&o.name[..name.len()]==*name&&o.space==Space::System as u8){
            if o.kind!=ObjectType::Metadata as u8||o.owner.0!=[0;16]||found.is_some()||legacy.is_some_and(|id|id!=o.id){return Err(ObjectError::CorruptContent)}found=Some(o.id);
        }
        Ok(found.or(legacy))
    }
    // ------------------------=
    // FUNC: reserve_system_metadata_record
    // DESC: Creates an unnamespaced reserved identity only within the caller's native transaction, preserving existing named identities when present.
    // ------------------=
    pub(super) fn reserve_system_metadata_record(&mut self,name:&[u8],legacy_path:&[u8])->Result<ObjectId,ObjectError>{
        match self.reserved_system_metadata_id(name,legacy_path)?{Some(id)=>Ok(id),None=>self.create_record(name,ObjectType::Metadata,Space::System)}
    }
    // ------------------------=
    // FUNC: replace_linked_state
    // DESC: Atomically publishes one bounded unnamespaced metadata child through its parent reference without spending a user namespace slot.
    // ------------------=
    pub(crate) fn replace_linked_state(&mut self,parent:ObjectId,parent_bytes:&[u8],reference:usize,bytes:&[u8])->Result<(),ObjectError>{
        if parent_bytes.len()>MAX_CONTENT||bytes.len()>MAX_CONTENT||reference.checked_add(16).is_none_or(|n|n>parent_bytes.len()){return Err(ObjectError::InvalidObject)}
        let before=self.begin()?;let result=(||{let mut p=[0;MAX_CONTENT];p[..parent_bytes.len()].copy_from_slice(parent_bytes);let mut child=ObjectId(p[reference..reference+16].try_into().unwrap());
            if child.0==[0;16]{child=self.create_record(b"pool-linked-state",ObjectType::Metadata,Space::System)?;p[reference..reference+16].copy_from_slice(&child.0);}
            self.replace_state_record(child,bytes)?;self.replace_state_record(parent,&p[..parent_bytes.len()]).map(|_|())})();self.finish(before,result)
    }
    // ------------------------=
    // FUNC: replace_named_state
    // DESC: Atomically creates or replaces a bounded native settings object.
    // ------------------=
    pub(crate) fn replace_named_state(&mut self,path:&[u8],bytes:&[u8])->Result<(),ObjectError>{
        if bytes.len()>MAX_CONTENT{return Err(ObjectError::InvalidObject)}let before=self.begin()?;
        let result=(||{let id=match self.resolve(path){Ok(id)=>id,Err(ObjectError::NotFound|ObjectError::NamespaceNotFound)=>{let id=self.create_record(b"pool-settings",ObjectType::Metadata,Space::System)?;self.attach_record(path,id)?;id},Err(e)=>return Err(e)};self.replace_state_record(id,bytes).map(|_|())})();self.finish(before,result)
    }
    // ------------------------=
    // FUNC: replace_named_state_pair
    // DESC: Creates or replaces two native named metadata records under one root, including first publication; callers supply already validated bounded payloads.
    // ------------------=
    pub(crate) fn replace_named_state_pair(&mut self,first_path:&[u8],first:&[u8],second_path:&[u8],second:&[u8])->Result<(),ObjectError> {
        if first_path==second_path || first.len()>MAX_CONTENT || second.len()>MAX_CONTENT {return Err(ObjectError::InvalidObject);}
        let before=self.begin()?;
        let result=(||{
            for (path,bytes) in [(first_path,first),(second_path,second)] {
                let id=match self.resolve(path) {Ok(id)=>id,Err(ObjectError::NotFound|ObjectError::NamespaceNotFound)=>{
                    let id=self.create_record(b"pool-quorum-state",ObjectType::Metadata,Space::System)?;self.attach_record(path,id)?;id
                },Err(e)=>return Err(e)};
                self.replace_state_record(id,bytes)?;
            }Ok(())
        })();self.finish(before,result)
    }
    // ------------------------=
    // FUNC: append_pool_audit_record
    // DESC: Appends one nonsecret fixed record inside the caller's transaction, creating the bounded ring under the same root when absent.
    // ------------------=
    pub(super) fn append_pool_audit_record(&mut self, mut record:[u8;128])->Result<(),ObjectError> {
        let path=b"/system/storage/pool-audit";
        let mut bytes=[0;2080];bytes[..8].copy_from_slice(b"INFPAD01");
        let id=match self.reserved_system_metadata_id(b"@pool-audit",path)? {
            Some(id)=>{if self.read(id,None,&mut bytes)?!=2080 || &bytes[..8]!=b"INFPAD01" {return Err(ObjectError::CorruptContent);}id},
            None=>self.reserve_system_metadata_record(b"@pool-audit",path)?,
        };
        let sequence=u64::from_le_bytes(bytes[8..16].try_into().unwrap()).checked_add(1).ok_or(ObjectError::InvalidVersion)?;
        bytes[8..16].copy_from_slice(&sequence.to_le_bytes());record[120..128].copy_from_slice(&sequence.to_le_bytes());
        let at=32+((sequence-1)%16) as usize*128;bytes[at..at+128].copy_from_slice(&record);
        self.replace_state_record(id,&bytes).map(|_|())
    }
    // ------------------------=
    // FUNC: replace_state_audited
    // DESC: Publishes state and its audit transition atomically including first ring creation.
    // ------------------=
    pub(crate) fn replace_state_audited(&mut self,id:ObjectId,bytes:&[u8],audit:[u8;128])->Result<(),ObjectError> {
        let before=self.begin()?;
        let result=(||{self.replace_state_record(id,bytes)?;self.append_pool_audit_record(audit)})();
        self.finish(before,result)
    }
    // ------------------------=
    // FUNC: replace_state_pair
    // DESC: Commits authoritative state and its durable audit successor under one root; neither record becomes visible alone after interruption.
    // ------------------=
    pub(crate) fn replace_state_pair(&mut self, first:ObjectId, first_bytes:&[u8], second:ObjectId, second_bytes:&[u8]) -> Result<(),ObjectError> {
        if first==second || first_bytes.len()>MAX_CONTENT || second_bytes.len()>MAX_CONTENT {return Err(ObjectError::InvalidObject);}
        let before=self.begin()?;
        let result=(|| {self.replace_state_record(first,first_bytes)?;self.replace_state_record(second,second_bytes)?;Ok(())})();
        self.finish(before,result)
    }
    // ------------------------=
    // FUNC: copy_owned_bundle
    // DESC: Atomically creates an independent object sharing the source's current immutable extent and commits its distinct manifest and catalog binding without copying payload bytes.
    // ------------------=
    pub(crate) fn copy_owned_bundle<const N: usize, const M: usize>(&mut self, source: ObjectId,
        expected_version: u64, catalog: ObjectId,
        encode: impl FnOnce(ObjectId, ObjectId) -> Result<([u8; N], [u8; M], [u8;128]), ObjectError>) -> Result<ObjectId, ObjectError> {
        if N > MAX_CONTENT || M > MAX_CONTENT { return Err(ObjectError::InsufficientCapacity); }
        if self.metadata(source)?.current_version as u64 != expected_version { return Err(ObjectError::InvalidVersion); }
        let before = self.begin()?;
        let result = (|| {
            let object = self.copy_record(source, b"Pool Copy")?;
            let backing = self.create_record(b"object-manifest", ObjectType::Metadata, Space::System)?;
            let (catalog_bytes, manifest, audit) = encode(object, backing)?;
            self.replace_state_record(backing, &manifest)?;
            self.replace_state_record(catalog, &catalog_bytes)?;
            self.append_pool_audit_record(audit)?;
            Ok(object)
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: create_owned_bundle
    // DESC: Atomically creates application content, a distinct private metadata record and its catalog binding, with rollback on any encoding or sector failure.
    // ------------------=
    pub(crate) fn create_owned_bundle<const N: usize, const M: usize>(&mut self, catalog: ObjectId,
        content: &[u8], encode: impl FnOnce(ObjectId, ObjectId) -> Result<([u8; N], [u8; M], [u8;128]), ObjectError>)
        -> Result<ObjectId, ObjectError> {
        if N > MAX_CONTENT || M > MAX_CONTENT || content.len() > MAX_CONTENT { return Err(ObjectError::InsufficientCapacity); }
        let before = self.begin()?;
        let result = (|| {
            let object = self.create_record(b"Pool Object", ObjectType::Metadata, Space::Personal)?;
            self.write_record(object, content)?;
            let backing = self.create_record(b"object-manifest", ObjectType::Metadata, Space::System)?;
            let (catalog_bytes, manifest, audit) = encode(object, backing)?;
            self.replace_state_record(backing, &manifest)?;
            self.replace_state_record(catalog, &catalog_bytes)?;
            self.append_pool_audit_record(audit)?;
            Ok(object)
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: update_owned_bundle
    // DESC: Commits a content successor and its owning metadata under one native transaction root, preserving the previous version on failed validation or persistence.
    // ------------------=
    pub(crate) fn update_owned_bundle<const N: usize>(&mut self, object: ObjectId, backing: ObjectId,
        content: &[u8], encode: impl FnOnce(u32) -> Result<([u8; N],[u8;128]), ObjectError>) -> Result<u32, ObjectError> {
        if object == backing || N > MAX_CONTENT || content.len() > MAX_CONTENT { return Err(ObjectError::InvalidObject); }
        let before = self.begin()?;
        let result = (|| {
            let version = self.write_record(object, content)?;
            let (bytes,audit)=encode(version)?;
            self.replace_state_record(backing, &bytes)?;
            self.append_pool_audit_record(audit)?;
            Ok(version)
        })();
        self.finish(before, result)
    }
}
