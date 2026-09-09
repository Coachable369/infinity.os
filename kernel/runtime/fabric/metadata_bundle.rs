//! Canonical signed metadata payloads for quorum replication. Payload possession
//! alone is not freshness evidence or authority to serve application content.
use super::{metadata::{Group,SignedRecord,Record,Certificate,Receipt,ReaderGrant,Error},manifest::{Manifest,MANIFEST_BYTES}};
use crate::runtime::{crypto::NodeCrypto,node::types::NodeId};
use sha2::{Digest,Sha256};
pub const BUNDLE_BYTES:usize=7680;
#[derive(Clone,Copy)]
pub struct Bundle {
    pub group:Group,
    pub value:SignedRecord,
    pub certificate:Option<Certificate>,
    pub manifest:Manifest,
    pub path:[u8;95],
    pub path_len:u8,
    pub grants:[Option<ReaderGrant>;3],
    pub repair_grants:[Option<super::metadata_repair::RepairGrant>;2],
}
// ------------------------=
// FUNC: namespace_bytes
// DESC: Encodes a stable native path/ObjectId relationship independently of physical placement.
// ------------------=
fn namespace_bytes(object:[u8;16],path:&[u8])->Result<[u8;128],Error> {
    if object==[0;16] || path.len()<2 || path.len()>95 || path[0]!=b'/' || path.last()==Some(&b'/')
        || core::str::from_utf8(path).is_err() || path.iter().any(|b|*b<32 || *b==127)
        || path[1..].split(|b|*b==b'/').any(|s|s.is_empty() || s==b"." || s==b"..") {return Err(Error::Invalid);}
    let mut b=[0;128];b[..8].copy_from_slice(b"INFQNS01");b[8..24].copy_from_slice(&object);b[24]=path.len() as u8;b[32..32+path.len()].copy_from_slice(path);Ok(b)
}
// ------------------------=
// FUNC: namespace_digest
// DESC: Binds canonical native namespace bytes into the owner-signed record.
// ------------------=
pub fn namespace_digest(object:[u8;16],path:&[u8])->Result<[u8;32],Error> {Ok(Sha256::digest(namespace_bytes(object,path)?).into())}
// ------------------------=
// FUNC: policy_bytes
// DESC: Encodes explicit group, protection policy and revocation fences without granting ambient access.
// ------------------=
fn policy_bytes(group:&Group,m:&Manifest,revocation:u64)->[u8;64] {
    let mut b=[0;64];b[..8].copy_from_slice(b"INFQPL01");b[8..40].copy_from_slice(&group.digest());b[40]=m.policy.replicas() as u8;b[41]=m.minimum_available;
    b[48..56].copy_from_slice(&revocation.to_le_bytes());b[56..64].copy_from_slice(&m.authority_generation.to_le_bytes());b
}
// ------------------------=
// FUNC: policy_digest
// DESC: Binds native policy payload to its signed metadata generation.
// ------------------=
pub fn policy_digest(group:&Group,m:&Manifest,revocation:u64)->[u8;32] {Sha256::digest(policy_bytes(group,m,revocation)).into()}
impl Bundle {
    // ------------------------=
    // FUNC: persistent_authority
    // DESC: Rejects uptime-based finite delegation for persisted native use until a restart-stable trusted clock exists; finite cryptographic primitives remain available separately.
    // ------------------=
    pub fn persistent_authority(&self)->bool{self.grants.iter().flatten().all(|g|g.expires==u64::MAX)&&self.repair_grants.iter().flatten().all(|g|g.expires==u64::MAX)}
    // ------------------------=
    // FUNC: path
    // DESC: Returns bounded namespace bytes without allocation.
    // ------------------=
    pub fn path(&self)->&[u8] {&self.path[..(self.path_len as usize).min(95)]}
    // ------------------------=
    // FUNC: validate
    // DESC: Verifies actual manifest/namespace/policy bytes against owner signatures and exact certificates; validates every explicit delegation independently.
    // ------------------=
    pub fn validate(&self)->Result<(),Error> {
        self.value.validate(&self.group)?;self.manifest.validate().map_err(|_|Error::Invalid)?;
        if self.path_len as usize>95 || self.path[self.path_len as usize..].iter().any(|b|*b!=0) {return Err(Error::Invalid);}
        let r=self.value.record;let mut encoded=[0;MANIFEST_BYTES];self.manifest.encode(&mut encoded).map_err(|_|Error::Invalid)?;
        if self.manifest.object!=r.object || self.manifest.version!=r.version || self.manifest.authority!=self.group.owner
            || r.manifest!=<[u8;32]>::from(Sha256::digest(encoded)) || r.namespace!=namespace_digest(r.object,self.path())?
            || r.policy!=policy_digest(&self.group,&self.manifest,r.revocation) {return Err(Error::Conflict);}
        if let Some(c)=self.certificate {c.validate(&self.group)?;if c.value!=self.value {return Err(Error::Conflict);}}
        for (i,grant) in self.grants.iter().enumerate() {if let Some(grant)=grant {
            self.validate_grant(grant)?;
            if self.grants[..i].iter().flatten().any(|old|old.reader==grant.reader && old.principal==grant.principal) {return Err(Error::Conflict);}
        }}
        for(i,grant)in self.repair_grants.iter().enumerate(){if let Some(grant)=grant{grant.validate_anchor(&self.group,&self.value,0)?;if self.repair_grants[..i].iter().flatten().any(|old|old.writer==grant.writer){return Err(Error::Conflict)}}}Ok(())
    }
    // ------------------------=
    // FUNC: validate_grant
    // DESC: Verifies an explicit owner-issued object/read delegation without treating group membership as a grant.
    // ------------------=
    fn validate_grant(&self,grant:&ReaderGrant)->Result<(),Error> {
        let r=self.value.record;
        if grant.group!=self.group.digest() || grant.object!=r.object || grant.policy!=r.policy || grant.revocation!=r.revocation
            || grant.principal==[0;16] || grant.expires==0 || !self.group.members.contains(&grant.reader) {return Err(Error::Denied);}
        let owner=self.group.members.iter().position(|id|*id==self.group.owner).ok_or(Error::Denied)?;
        NodeCrypto::verify(&self.group.keys[owner],&grant.transcript(),&grant.signature).map_err(|_|Error::Signature)
    }
    // ------------------------=
    // FUNC: authorize
    // DESC: Requires an exact unexpired delegation and nondeleted certified payload; the caller still must establish fresh quorum and write-back.
    // ------------------=
    pub fn authorize(&self,reader:NodeId,principal:[u8;16],now:u64)->Result<ReaderGrant,Error> {
        self.validate()?;if self.value.record.deleted {return Err(Error::Deleted);}if self.certificate.is_none(){return Err(Error::Quorum);}
        self.grants.iter().flatten().find(|g|g.reader==reader && g.principal==principal && now<g.expires).copied().ok_or(Error::Denied)
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Serializes one fixed payload with canonical reserved bytes and explicit certificate/grant presence.
    // ------------------=
    pub fn encode(&self)->Result<[u8;BUNDLE_BYTES],Error> {
        self.validate()?;let mut b=[0;BUNDLE_BYTES];b[..8].copy_from_slice(b"INFQBL01");b[8]=u8::from(self.certificate.is_some());
        b[16..24].copy_from_slice(&self.group.epoch.to_le_bytes());b[24..56].copy_from_slice(&self.group.owner.0);
        for i in 0..3 {b[56+i*32..88+i*32].copy_from_slice(&self.group.members[i].0);b[152+i*32..184+i*32].copy_from_slice(&self.group.keys[i]);}
        b[256..512].copy_from_slice(&self.value.record.encode());b[512..576].copy_from_slice(&self.value.signature);
        if let Some(c)=self.certificate {for i in 0..2 {let at=576+i*112;b[at..at+48].copy_from_slice(&c.prepared[i].transcript());b[at+48..at+112].copy_from_slice(&c.prepared[i].signature);}}
        self.manifest.encode((&mut b[800..6176]).try_into().unwrap()).map_err(|_|Error::Invalid)?;
        b[6176..6304].copy_from_slice(&namespace_bytes(self.manifest.object,self.path())?);b[6304..6368].copy_from_slice(&policy_bytes(&self.group,&self.manifest,self.value.record.revocation));
        for i in 0..3 {if let Some(g)=self.grants[i] {b[9+i]=1;let at=6368+i*224;b[at..at+160].copy_from_slice(&g.transcript());b[at+160..at+224].copy_from_slice(&g.signature);}}
        for i in 0..2{if let Some(g)=self.repair_grants[i]{b[12+i]=1;let at=7040+i*320;b[at..at+256].copy_from_slice(&g.transcript());b[at+256..at+320].copy_from_slice(&g.signature);}}
        Ok(b)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Recovers only fully canonical signed metadata payloads; flags, padding and independently signed fields are never silently normalized.
    // ------------------=
    pub fn decode(b:&[u8])->Result<Self,Error> {
        if b.len()!=BUNDLE_BYTES || &b[..8]!=b"INFQBL01" || b[8..14].iter().any(|v|*v>1) {return Err(Error::Invalid);}
        let group=Group {epoch:u64::from_le_bytes(b[16..24].try_into().unwrap()),owner:NodeId(b[24..56].try_into().unwrap()),
            members:core::array::from_fn(|i|NodeId(b[56+i*32..88+i*32].try_into().unwrap())),keys:core::array::from_fn(|i|b[152+i*32..184+i*32].try_into().unwrap())};
        let value=SignedRecord {record:Record::decode(&b[256..512])?,signature:b[512..576].try_into().unwrap()};
        let certificate=if b[8]==1 {Some(Certificate {value,prepared:core::array::from_fn(|i|{let at=576+i*112;Receipt {member:b[at+8],published:b[at+9]!=0,digest:b[at+16..at+48].try_into().unwrap(),signature:b[at+48..at+112].try_into().unwrap()}})})}else{None};
        let path_len=b[6200];if path_len>95{return Err(Error::Invalid);}let path=b[6208..6303].try_into().unwrap();
        let grants=core::array::from_fn(|i|if b[9+i]==1 {let at=6368+i*224;Some(ReaderGrant {group:b[at+8..at+40].try_into().unwrap(),object:b[at+40..at+56].try_into().unwrap(),
            reader:NodeId(b[at+56..at+88].try_into().unwrap()),principal:b[at+88..at+104].try_into().unwrap(),policy:b[at+104..at+136].try_into().unwrap(),
            revocation:u64::from_le_bytes(b[at+136..at+144].try_into().unwrap()),expires:u64::from_le_bytes(b[at+144..at+152].try_into().unwrap()),signature:b[at+160..at+224].try_into().unwrap()})}else{None});
        let mut repair_grants=[None;2];for i in 0..2{if b[12+i]==1{repair_grants[i]=Some(super::metadata_repair::RepairGrant::decode(&b[7040+i*320..7360+i*320])?);}}
        let bundle=Self {group,value,certificate,manifest:Manifest::decode(&b[800..6176]).map_err(|_|Error::Invalid)?,path,path_len,grants,repair_grants};
        if bundle.encode()?.as_slice()!=b {return Err(Error::Invalid);}Ok(bundle)
    }
}
