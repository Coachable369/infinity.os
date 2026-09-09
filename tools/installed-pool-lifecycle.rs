//! Offline inspection only. Native reads validate sector/root/content checksums;
//! absent objects are not evidence of secure erasure or remote acknowledgement.
use super::*;
use storage::object::{ObjectId,ObjectError};
pub struct Report { pub object_present:bool,pub local_manifest:Option<(u64,u64,[u8;32])>,pub tombstone:Option<(bool,u64,u64,[u8;32])>,pub outbox_present:bool,pub pending:Option<(u64,u8,usize)>,pub audit_sequence:u64,pub audit:Vec<(u64,u8,u64,u64,u64)>,pub content:Vec<([u8;16],bool)> }
// ------------------------=
// FUNC: read_optional
// DESC: Reads a native named object without initializing missing catalogs or accepting corrupt reads as absence.
// ------------------=
fn read_optional<D:BlockDevice>(s:&mut ObjectStore<D>,path:&[u8],out:&mut[u8])->Result<Option<usize>,String>{
    match s.resolve(path){Ok(id)=>s.read(id,None,out).map(Some).map_err(|_|"content_invalid".into()),Err(ObjectError::NotFound|ObjectError::NamespaceNotFound)=>Ok(None),Err(_)=>Err("namespace_invalid".into())}
}
// ------------------------=
// FUNC: identity
// DESC: Formats fixed binary identities for JSON without treating human output as an acceptance oracle.
// ------------------=
fn identity(bytes:&[u8])->String{bytes.iter().map(|b|format!("{b:02x}")).collect()}
// ------------------------=
// FUNC: inspect
// DESC: Inspects canonical committed tombstone, exact owner outbox, retained audit and requested physical identities with no mutation APIs.
// ------------------=
pub fn inspect<D:BlockDevice>(s:&mut ObjectStore<D>,owner:[u8;32],object:[u8;16],content:&[[u8;16]])->Result<Report,String>{
    let mut report=Report{object_present:s.object_exists(ObjectId(object)),local_manifest:None,tombstone:None,outbox_present:false,pending:None,audit_sequence:0,audit:vec![],content:content.iter().map(|id|(*id,s.object_exists(ObjectId(*id)))).collect()};
    let path=format!("/system/storage/pool-quorum-payload/{}",identity(&object));let mut raw=[0;16384];
    let name=format!("@pool-quorum/{}",identity(&object));
    let payload=s.reserved_system_metadata_id(name.as_bytes(),path.as_bytes()).map_err(|_|"payload_identity")?;
    if let Some(id)=payload{let n=s.read(id,None,&mut raw).map_err(|_|"payload_content")?;
        let header=match &raw[..8]{b"INFQPY02"=>64,b"INFQPY01"=>32,_=>return Err("payload_format".into())};
        let size=runtime::fabric::metadata_bundle::BUNDLE_BYTES;
        if n!=header+2*size||raw[9]>1{return Err("payload_bounds".into())}
        if raw[9]==1{
            let b=runtime::fabric::metadata_bundle::Bundle::decode(raw[header+size..n].try_into().map_err(|_|"bundle_bounds")?).map_err(|_|"bundle_invalid")?;
            if b.group.owner.0!=owner||b.manifest.object!=object||b.certificate.is_none(){return Err("bundle_identity".into())}
            report.tombstone=Some((b.value.record.deleted,b.value.record.generation,b.value.record.version,b.manifest.hash));
        }
    }
    let mut catalog=[0;1056];let mut outbox=None;
    if let Some(n)=read_optional(s,b"/system/storage/pool-manifests",&mut catalog)?{
        if n!=1056||&catalog[..8]!=b"INFPOOL1"{return Err("catalog_invalid".into())}
        match s.pool_manifest(ObjectId(object),runtime::node::types::NodeId(owner),0){Ok(m)=>report.local_manifest=Some((m.generation,m.version,m.hash)),Err(ObjectError::NotFound)=>{},Err(_)=>return Err("local_manifest_invalid".into())}
        let id=ObjectId(catalog[16..32].try_into().unwrap());if id.0!=[0;16]{outbox=Some(id)}
    }
    if outbox.is_none(){match s.resolve(b"/system/storage/pool-deletions"){Ok(id)=>outbox=Some(id),Err(ObjectError::NotFound|ObjectError::NamespaceNotFound)=>{},Err(_)=>return Err("outbox_namespace".into())}}
    if let Some(id)=outbox{
        let mut b=[0;9120];if s.read(id,None,&mut b).map_err(|_|"outbox_read")?!=b.len()||&b[..8]!=b"INFPDEL1"{return Err("outbox_invalid".into())}report.outbox_present=true;
        for row in b[32..].chunks_exact(1136){if row.iter().all(|v|*v==0){continue}if row[0]!=1||row[2..8]!=[0;6]{return Err("outbox_record".into())}
            if row[8..24]==object&&row[24..56]==owner{if report.pending.is_some(){return Err("outbox_duplicate".into())}let placements=(0..8).filter(|i|row[112+i*128..144+i*128]!=[0;32]).count();report.pending=Some((number(row,72),row[1],placements));}
        }
    }
    let mut audit=[0;2080];if let Some(id)=s.reserved_system_metadata_id(b"@pool-audit",b"/system/storage/pool-audit").map_err(|_|"audit_identity")?{
        let n=s.read(id,None,&mut audit).map_err(|_|"audit_read")?;
        if n!=audit.len()||&audit[..8]!=b"INFPAD01"{return Err("audit_invalid".into())}report.audit_sequence=number(&audit,8);
        for row in audit[32..].chunks_exact(128){let seq=number(row,120);if seq==0{continue}if seq>report.audit_sequence||report.audit_sequence-seq>=16{return Err("audit_sequence".into())}if row[..16]==object&&row[48..80]==owner{report.audit.push((seq,row[98],number(row,16),number(row,24),number(row,32)));}}
        report.audit.sort_by_key(|r|r.0);
    }Ok(report)
}
impl Report{
    // ------------------------=
    // FUNC: json
    // DESC: Serializes structured read-only observations and clearly distinguishes missing evidence from completed reclamation.
    // ------------------=
    pub fn json(&self)->String{
        let tombstone=self.tombstone.map_or("{\"present\":false}".into(),|(deleted,generation,version,hash)|format!("{{\"present\":true,\"deleted\":{deleted},\"generation\":{generation},\"version\":{version},\"hash\":\"{}\"}}",identity(&hash)));
        let pending=self.pending.map_or("null".into(),|(generation,ack,placements)|format!("{{\"manifest_generation\":{generation},\"acknowledged\":{ack},\"placements\":{placements}}}"));
        let audit=self.audit.iter().map(|(seq,kind,old,new,version)|format!("{{\"sequence\":{seq},\"kind\":{kind},\"previous_generation\":{old},\"generation\":{new},\"version\":{version}}}")).collect::<Vec<_>>().join(",");
        let content=self.content.iter().map(|(id,present)|format!("{{\"id\":\"{}\",\"present\":{present}}}",identity(id))).collect::<Vec<_>>().join(",");
        let local=self.local_manifest.map_or("null".into(),|(generation,version,hash)|format!("{{\"generation\":{generation},\"version\":{version},\"hash\":\"{}\"}}",identity(&hash)));
        format!("{{\"inspected\":true,\"read_only\":true,\"object_present\":{},\"local_manifest\":{local},\"tombstone\":{tombstone},\"outbox\":{{\"present\":{},\"pending\":{pending}}},\"audit\":{{\"sequence\":{},\"entries\":[{audit}]}},\"content_objects\":[{content}]}}",self.object_present,self.outbox_present,self.audit_sequence)
    }
}
