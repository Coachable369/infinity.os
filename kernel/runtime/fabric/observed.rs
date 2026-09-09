//! Read-only health projection; never changes the certified manifest.
use super::{manifest::{Manifest, Placement, PlacementState}, resources::Resource};
// ------------------------=
// FUNC: state
// DESC: Combines immutable placement evidence with actually observed resource expiry without inventing unknown reachability.
// ------------------=
pub fn state(m: &Manifest, p: &Placement, resources: &[Option<Resource>;32], now:u64)->PlacementState {
    if p.state==PlacementState::Verified && (p.version!=m.version || p.hash!=m.hash) {return PlacementState::Stale;}
    if p.state==PlacementState::Verified && resources.iter().flatten().find(|r|r.owner==p.node && r.id==p.resource).is_some_and(|r|!r.online || r.expires<=now){PlacementState::Offline}else{p.state}
}
// ------------------------=
// FUNC: summary
// DESC: Produces the existing bounded PoolInspect summary layout from certified content and live resource observations.
// ------------------=
pub fn summary(m:&Manifest,resources:&[Option<Resource>;32],now:u64)->[u8;64]{
    let mut data=[0;64];data[..16].copy_from_slice(&m.object);data[16..48].copy_from_slice(&m.hash);data[48]=m.policy.replicas() as u8;data[56..].copy_from_slice(&m.length.to_le_bytes());
    let mut nodes=[[0;32];8];let mut count=0;
    for p in m.placements.iter().flatten(){let index=match state(m,p,resources,now){PlacementState::Verified=>{if nodes[..count].contains(&p.node.0){continue;}nodes[count]=p.node.0;count+=1;49},PlacementState::Offline=>50,PlacementState::Stale=>51,PlacementState::Corrupt=>52,PlacementState::Staging=>54};data[index]+=1;}
    data[53]=u8::from(data[49]<data[48]);data
}
