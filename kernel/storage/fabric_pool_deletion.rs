//! Bounded native outbox: logical deletion and its remote retirement obligations
//! are committed together. Lost acknowledgements are safe to retry after boot.
use super::*;
use crate::runtime::fabric::deletion::Deletion;
const PATH_DELETE:&[u8]=b"/system/storage/pool-deletions";
const RECORD:usize=1136;
const BYTES:usize=32+8*RECORD;

// ------------------------=
// FUNC: decode
// DESC: Validates a canonical retained object/placement authority record without treating stale copies as current content.
// ------------------=
fn decode(bytes:&[u8])->Result<Option<Deletion>,ObjectError> {
    if bytes.iter().all(|b|*b==0){return Ok(None);}
    if bytes.len()!=RECORD || bytes[0]!=1 || bytes[2..8]!=[0;6] {return Err(ObjectError::CorruptContent);}
    let mut d=Deletion{object:bytes[8..24].try_into().unwrap(),owner:NodeId(bytes[24..56].try_into().unwrap()),
        scope:u64::from_le_bytes(bytes[56..64].try_into().unwrap()),authority_generation:u64::from_le_bytes(bytes[64..72].try_into().unwrap()),
        manifest_generation:u64::from_le_bytes(bytes[72..80].try_into().unwrap()),placements:[None;8],acknowledged:bytes[1]};
    if d.object==[0;16] || d.owner.0==[0;32] || d.authority_generation==0 || d.manifest_generation==0{return Err(ObjectError::CorruptContent);}
    for i in 0..8{let at=112+i*128;if bytes[at]==0 {if bytes[at..at+128]!=[0;128]{return Err(ObjectError::CorruptContent);}continue;}
        let state=match bytes[at]{1=>PlacementState::Staging,2=>PlacementState::Verified,3=>PlacementState::Offline,4=>PlacementState::Stale,5=>PlacementState::Corrupt,_=>return Err(ObjectError::CorruptContent)};
        d.placements[i]=Some(Placement{state,node:NodeId(bytes[at+8..at+40].try_into().unwrap()),resource:ResourceId(bytes[at+40..at+56].try_into().unwrap()),
            device:bytes[at+56..at+72].try_into().unwrap(),generation:u64::from_le_bytes(bytes[at+72..at+80].try_into().unwrap()),
            version:u64::from_le_bytes(bytes[at+80..at+88].try_into().unwrap()),hash:bytes[at+88..at+120].try_into().unwrap(),
            admission_generation:u64::from_le_bytes(bytes[at+120..at+128].try_into().unwrap())});
    }
    if encode(d)!=bytes {return Err(ObjectError::CorruptContent);}Ok(Some(d))
}
// ------------------------=
// FUNC: encode
// DESC: Retains full immutable replica identity and exact admission fence for replay-safe reclamation.
// ------------------=
fn encode(d:Deletion)->[u8;RECORD] {
    let mut b=[0;RECORD];b[0]=1;b[1]=d.acknowledged;b[8..24].copy_from_slice(&d.object);b[24..56].copy_from_slice(&d.owner.0);
    for(at,n)in [(56,d.scope),(64,d.authority_generation),(72,d.manifest_generation)]{b[at..at+8].copy_from_slice(&n.to_le_bytes());}
    for(i,p)in d.placements.iter().enumerate().filter_map(|(i,p)|p.map(|p|(i,p))){let at=112+i*128;b[at]=p.state as u8;
        b[at+8..at+40].copy_from_slice(&p.node.0);b[at+40..at+56].copy_from_slice(&p.resource.0);b[at+56..at+72].copy_from_slice(&p.device);
        b[at+72..at+80].copy_from_slice(&p.generation.to_le_bytes());b[at+80..at+88].copy_from_slice(&p.version.to_le_bytes());
        b[at+88..at+120].copy_from_slice(&p.hash);b[at+120..at+128].copy_from_slice(&p.admission_generation.to_le_bytes());}
    b
}
impl<D:BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: pool_deletion_state
    // DESC: Reads or initializes the bounded typed native outbox; malformed committed records never reset silently.
    // ------------------=
    fn pool_deletion_state(&mut self)->Result<(ObjectId,[u8;BYTES]),ObjectError> {
        let mut bytes=[0;BYTES];bytes[..8].copy_from_slice(b"INFPDEL1");
        self.initialize_pool_catalog()?;
        let mut catalog=Catalog::load(self)?;
        let id=if catalog.deletion.0!=[0;16]{catalog.deletion}else{match self.resolve(PATH_DELETE){
            Ok(id)=>{catalog.deletion=id;self.replace_named_state(PATH,&catalog.encode())?;id},
            Err(ObjectError::NotFound|ObjectError::NamespaceNotFound)=>{
                self.replace_linked_state(catalog.id,&catalog.encode(),16,&bytes)?;
                Catalog::load(self)?.deletion
            },Err(e)=>return Err(e)}};
        if self.read(id,None,&mut bytes)?!=BYTES || &bytes[..8]!=b"INFPDEL1"{return Err(ObjectError::CorruptContent);}
        for i in 0..8{decode(&bytes[32+i*RECORD..32+(i+1)*RECORD])?;}Ok((id,bytes))
    }
    // ------------------------=
    // FUNC: pool_stage_deletion
    // DESC: Stages exact remote obligations before the caller atomically removes the local object and its namespace reference.
    // ------------------=
    pub(super) fn pool_stage_deletion(&mut self,m:&Manifest,scope:u64)->Result<(ObjectId,[u8;BYTES]),ObjectError> {
        let(id,mut bytes)=self.pool_deletion_state()?;
        let mut d=Deletion{object:m.object,owner:m.authority,scope,authority_generation:m.authority_generation,
            manifest_generation:m.generation,placements:m.placements,acknowledged:0};
        for(i,p)in d.placements.iter().enumerate(){if p.is_none_or(|p|p.node==m.authority){d.acknowledged|=1<<i;}}
        if d.acknowledged!=255 {
            let mut slot=None;for i in 0..8{if decode(&bytes[32+i*RECORD..32+(i+1)*RECORD])?.is_none(){slot=Some(i);break;}}
            let slot=slot.ok_or(ObjectError::InsufficientCapacity)?;
            bytes[32+slot*RECORD..32+(slot+1)*RECORD].copy_from_slice(&encode(d));
        }
        Ok((id,bytes))
    }
    // ------------------------=
    // FUNC: pool_deletion
    // DESC: Exposes only one owner/scope-matching durable obligation for the authorized service pump.
    // ------------------=
    pub(crate) fn pool_deletion(&mut self,index:usize,owner:NodeId,scope:u64)->Result<Option<Deletion>,ObjectError>{
        if index>=8{return Err(ObjectError::InvalidObject);}let(_,bytes)=self.pool_deletion_state()?;
        Ok(decode(&bytes[32+index*RECORD..32+(index+1)*RECORD])?.filter(|d|d.owner==owner && d.scope==scope))
    }
    // ------------------------=
    // FUNC: pool_ack_deletion
    // DESC: Records an exact recipient acknowledgement; the outbox slot becomes reclaimable only when every former placement is retired.
    // ------------------=
    pub(crate) fn pool_ack_deletion(&mut self,object:[u8;16],owner:NodeId,scope:u64,generation:u64,placement:usize)->Result<(),ObjectError>{
        if placement>=8{return Err(ObjectError::InvalidObject);}let(id,mut bytes)=self.pool_deletion_state()?;
        for i in 0..8 {let at=32+i*RECORD;if let Some(mut d)=decode(&bytes[at..at+RECORD])?{
            if d.object!=object || d.owner!=owner || d.scope!=scope{continue;}if d.manifest_generation!=generation{return Err(ObjectError::InvalidVersion);}
            d.acknowledged|=1<<placement;bytes[at..at+RECORD].copy_from_slice(&if d.acknowledged==255{[0;RECORD]}else{encode(d)});
            self.replace_state(id,&bytes)?;return Ok(());
        }}Err(ObjectError::NotFound)
    }
}
