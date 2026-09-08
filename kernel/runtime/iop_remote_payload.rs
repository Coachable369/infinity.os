//! Versioned service payloads share one authenticated remote IOP lifecycle.
use super::{NodeOperationV1, RemoteError};
use super::super::storage_protocol::StorageOperationV1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Payload { Node(NodeOperationV1), Storage(StorageOperationV1) }
impl Payload {
    // ------------------------=
    // FUNC: operation
    // DESC: Returns the exact native operation used for capability admission and reply correlation.
    // ------------------=
    pub fn operation(self) -> u32 { match self { Self::Node(p) => p.operation, Self::Storage(p) => p.operation as u32 } }
    // ------------------------=
    // FUNC: scope
    // DESC: Preserves the explicit grant scope independently of full object identity and physical placement.
    // ------------------=
    pub fn scope(self) -> u64 { match self { Self::Node(p) => p.scope, Self::Storage(p) => p.scope } }
    // ------------------------=
    // FUNC: same_target
    // DESC: Binds replies to the exact payload family, operation and full subject identity without cross-schema aliases.
    // ------------------=
    pub fn same_target(self, other: Self) -> bool {
        match (self, other) {
            (Self::Node(a), Self::Node(b)) => a.operation == b.operation && a.node_id == b.node_id,
            (Self::Storage(a), Self::Storage(b)) => a.operation == b.operation && a.object == b.object,
            _ => false,
        }
    }
    // ------------------------=
    // FUNC: node
    // DESC: Extracts only the node-service schema; storage bytes cannot be interpreted as node authority.
    // ------------------=
    pub fn node(self) -> Result<NodeOperationV1, RemoteError> {
        match self { Self::Node(value) => Ok(value), _ => Err(RemoteError::UnsupportedOperation) }
    }
    // ------------------------=
    // FUNC: storage
    // DESC: Extracts only the storage-service schema while preserving native typed errors.
    // ------------------=
    pub fn storage(self) -> Result<StorageOperationV1, RemoteError> {
        match self { Self::Storage(value) => Ok(value), _ => Err(RemoteError::UnsupportedOperation) }
    }
}
impl From<NodeOperationV1> for Payload {
    // ------------------------=
    // FUNC: from
    // DESC: Wraps the existing node contract without changing its encoding or authority semantics.
    // ------------------=
    fn from(value: NodeOperationV1) -> Self { Self::Node(value) }
}
