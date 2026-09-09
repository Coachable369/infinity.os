//! Explicit until-revoked Pool approvals, not restored traffic keys or live grants.
use super::*;
pub const CAPACITY:usize=32;
pub const TAG:u64=1<<63;
const OFFSET:usize=9728;
const BYTES:usize=72;
const _:()=assert!(OFFSET+CAPACITY*BYTES<=NODE_STATE_BYTES-4);
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Approval{pub id:u64,pub peer:NodeId,pub operation:u32,pub scope:u64,pub rights:u32,pub revoked:bool}
// ------------------------=
// FUNC: allowed
// DESC: Restricts persistent approval to existing typed Pool data and metadata operations, never general node control.
// ------------------=
pub fn allowed(op:u32)->bool{use crate::runtime::iop::OperationId as O;matches!(op,x if [O::ResourceAdvertise,O::ReplicaInspect,O::ReplicaDelete,O::ReplicaTransferBegin,O::ReplicaTransferChunk,O::ReplicaTransferCommit,O::ObjectRead,O::PoolMetadata].iter().any(|o|o.machine_id()==x))}
impl NodeRuntime{
    // ------------------------=
    // FUNC: durable_approval
    // DESC: Resolves a separate tagged policy identity without manufacturing a live grant.
    // ------------------=
    pub fn durable_approval(&self,id:u64)->Option<Approval>{self.durable.iter().flatten().find(|a|a.id==id).copied()}
    // ------------------------=
    // FUNC: grant_durable
    // DESC: Stages exactly one explicitly approved until-revoked scope; caller must atomically persist the ordinary node control transaction.
    // ------------------=
    pub fn grant_durable(&mut self,peer:NodeId,operation:u32,scope:u64,rights:u32,now:u64,correlation:u64)->Result<u64,NodeError>{
        if !allowed(operation)||rights!=1||self.next_id>=TAG{return Err(NodeError::CapabilityDenied)}
        if !self.discovered.iter().flatten().any(|n|n.id==peer&&n.trust==TrustState::Trusted){return Err(NodeError::NotTrusted)}
        let slot=self.durable.iter().position(Option::is_none).ok_or(NodeError::ResourceLimit)?;
        let id=TAG|self.take_id();self.durable[slot]=Some(Approval{id,peer,operation,scope,rights,revoked:false});self.record(AUDIT_REMOTE_GRANT_CREATED,peer,now,correlation,1);Ok(id)
    }
    // ------------------------=
    // FUNC: authorize_durable
    // DESC: Checks current trusted identity and exact durable scope on every freshly authenticated remote request; no uptime lease is renewed.
    // ------------------=
    pub fn authorize_durable(&self,id:u64,peer:NodeId,op:u32,scope:u64,rights:u32)->Result<(),NodeError>{
        let a=self.durable_approval(id).ok_or(NodeError::CapabilityDenied)?;
        if a.revoked{return Err(NodeError::CapabilityRevoked)}
        if !self.discovered.iter().flatten().any(|n|n.id==peer&&n.trust==TrustState::Trusted){return Err(NodeError::NotTrusted)}
        if a.peer!=peer||a.operation!=op||a.scope!=scope||rights==0||rights&!a.rights!=0{return Err(NodeError::CapabilityDenied)}Ok(())
    }
    // ------------------------=
    // FUNC: revoke_durable
    // DESC: Retains a permanent policy tombstone in the same transactional journal so reboot cannot resurrect revoked authority.
    // ------------------=
    pub fn revoke_durable(&mut self,id:u64,now:u64,correlation:u64)->Result<(),NodeError>{let a=self.durable.iter_mut().flatten().find(|a|a.id==id).ok_or(NodeError::CapabilityDenied)?;a.revoked=true;let peer=a.peer;self.record(AUDIT_REMOTE_GRANT_REVOKED,peer,now,correlation,1);Ok(())}
    // ------------------------=
    // FUNC: encode_durable
    // DESC: Writes bounded canonical approvals under the existing atomic node-state checksum.
    // ------------------=
    pub(super) fn encode_durable(&self,out:&mut[u8;NODE_STATE_BYTES]){for(i,a)in self.durable.iter().enumerate(){if let Some(a)=a{let b=&mut out[OFFSET+i*BYTES..OFFSET+(i+1)*BYTES];b[..8].copy_from_slice(&a.id.to_le_bytes());b[8..40].copy_from_slice(&a.peer.0);b[40..44].copy_from_slice(&a.operation.to_le_bytes());b[44..48].copy_from_slice(&a.rights.to_le_bytes());b[48..56].copy_from_slice(&a.scope.to_le_bytes());b[56]=u8::from(a.revoked);}}}
    // ------------------------=
    // FUNC: decode_durable
    // DESC: Rejects malformed, duplicate, unsupported or ambiguous approval records before restoring any authority.
    // ------------------=
    pub(super) fn decode_durable(&mut self,input:&[u8])->Result<(),NodeError>{
        self.durable=[None;CAPACITY];for i in 0..CAPACITY{let b=&input[OFFSET+i*BYTES..OFFSET+(i+1)*BYTES];if b.iter().all(|v|*v==0){continue}let id=u64::from_le_bytes(b[..8].try_into().unwrap());let peer=NodeId(b[8..40].try_into().unwrap());let operation=u32::from_le_bytes(b[40..44].try_into().unwrap());let rights=u32::from_le_bytes(b[44..48].try_into().unwrap());
            if id&TAG==0||id==TAG||(id&!TAG)>=self.next_id||peer.0==[0;32]||!allowed(operation)||rights!=1||b[56]>1||b[57..].iter().any(|v|*v!=0)||self.durable_approval(id).is_some(){return Err(NodeError::StateCorrupt)}
            self.durable[i]=Some(Approval{id,peer,operation,rights,scope:u64::from_le_bytes(b[48..56].try_into().unwrap()),revoked:b[56]!=0});}Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: durable_scope_survives_cold_restore_without_live_grants_and_revocation_sticks
    // DESC: Exercises separate persistent authority, scope denial, lifetime independence and permanent revocation through the real canonical journal.
    // ------------------=
    #[test]
    fn durable_scope_survives_cold_restore_without_live_grants_and_revocation_sticks(){
        let mut n=NodeRuntime::new();n.initialize(&[31;32],true).unwrap();let mut p=NodeRuntime::new();let peer=p.initialize(&[32;32],true).unwrap();n.discover(p.advertise(1,1,1).unwrap(),1).unwrap();let pair=n.begin_pairing(peer,2).unwrap();n.confirm_pairing(pair.id,pair.verification_code,true,3,1).unwrap();
        let op=crate::runtime::iop::OperationId::PoolMetadata.machine_id();let id=n.grant_durable(peer,op,17,1,5000,1).unwrap();n.grant_remote(peer,op,0,1,5100,5000,2).unwrap();
        let mut cold=NodeRuntime::new();cold.restore_state(&n.encode_state().unwrap()).unwrap();assert!(cold.remote_grants().iter().all(Option::is_none));assert!(cold.sessions.iter().all(Option::is_none));assert!(cold.authorize_durable(id,peer,op,17,1).is_ok());assert!(cold.authorize_durable(id,peer,op,18,1).is_err());assert!(cold.authorize_durable(id,peer,op+1,17,1).is_err());assert!(cold.authorize_durable(id,NodeId([9;32]),op,17,1).is_err());
        cold.revoke_durable(id,1,3).unwrap();let mut again=NodeRuntime::new();again.restore_state(&cold.encode_state().unwrap()).unwrap();assert_eq!(again.authorize_durable(id,peer,op,17,1),Err(NodeError::CapabilityRevoked));assert!(again.grant_durable(peer,crate::runtime::iop::OperationId::NodePolicyUpdate.machine_id(),0,1,1,4).is_err());
    }
}
