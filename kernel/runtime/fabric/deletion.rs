//! Durable reclamation intent retains exact former replica authority until each
//! independent recipient has acknowledged retirement. It grants no authority.
use super::manifest::Placement;
use crate::runtime::node::types::NodeId;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Deletion {
    pub object:[u8;16], pub owner:NodeId, pub scope:u64,
    pub authority_generation:u64, pub manifest_generation:u64,
    pub placements:[Option<Placement>;8], pub acknowledged:u8,
}
