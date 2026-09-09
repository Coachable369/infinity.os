//! Bounded typed storage operations carried by the existing authenticated IOP
//! router. Full application identities and generations are never truncated.
pub const OPERATION_BYTES: usize = 136;
pub const DATA_BYTES: usize = 64;

/// Postcommit replica state; the renderer is not the authoritative subscriber.
pub const EVENT_REPLICA_CHANGED: u32 = 0xe041;
pub const EVENT_RESOURCE_CHANGED: u32 = 0xe040;
pub const EVENT_OBJECT_CHANGED: u32 = 0xe042;
pub const EVENT_POLICY_CHANGED: u32 = 0xe043;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageCommit {
    pub event: u32,
    pub object: [u8; 16], pub generation: u64, pub copied: u64,
    pub state: u8, pub correlation: u64, pub causation: u64,
}
pub type StorageHandler = fn(super::remote::AuthenticatedStorageRequest)
    -> Result<(StorageOperationV1, Option<StorageCommit>), super::remote::RemoteError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Operation {
    ObjectCreate = super::OperationId::ObjectCreate as u32,
    ObjectRead = super::OperationId::ObjectRead as u32,
    ObjectUpdate = super::OperationId::ObjectUpdate as u32,
    ObjectCopy = super::OperationId::ObjectCopy as u32,
    ObjectInspect = super::OperationId::ObjectInspect as u32,
    ObjectDelete = super::OperationId::ObjectDelete as u32,
    PoolUploadBegin = super::OperationId::PoolUploadBegin as u32,
    PoolUploadAppend = super::OperationId::PoolUploadAppend as u32,
    PoolUploadCommit = super::OperationId::PoolUploadCommit as u32,
    PoolUploadAbort = super::OperationId::PoolUploadAbort as u32,
    ResourceAdvertise = super::OperationId::ResourceAdvertise as u32,
    ResourceInspect = super::OperationId::ResourceInspect as u32,
    PoolInspect = super::OperationId::PoolInspect as u32,
    ObjectSetPolicy = super::OperationId::ObjectSetPolicy as u32,
    ReplicaInspect = super::OperationId::ReplicaInspect as u32,
    ReplicaDelete = super::OperationId::ReplicaDelete as u32,
    TransferBegin = super::OperationId::ReplicaTransferBegin as u32,
    TransferChunk = super::OperationId::ReplicaTransferChunk as u32,
    TransferCommit = super::OperationId::ReplicaTransferCommit as u32,
    PoolHeal = super::OperationId::PoolHeal as u32,
}
impl Operation {
    // ------------------------=
    // FUNC: decode
    // DESC: Accepts only registered native storage operation identifiers rather than treating arbitrary integers as service calls.
    // ------------------=
    pub fn decode(value: u32) -> Result<Self, ProtocolError> {
        use Operation::*;
        [ObjectCreate, ObjectRead, ObjectUpdate, ObjectCopy, ObjectInspect, ResourceAdvertise,
            ResourceInspect, PoolInspect, ObjectSetPolicy, ReplicaInspect, TransferBegin,
            TransferChunk, TransferCommit, PoolHeal, ObjectDelete, PoolUploadBegin,
            PoolUploadAppend, PoolUploadCommit, PoolUploadAbort, ReplicaDelete].into_iter().find(|op| *op as u32 == value)
            .ok_or(ProtocolError::Operation)
    }
    // ------------------------=
    // FUNC: read_only
    // DESC: Identifies inspection and read operations without granting authority or bypassing per-object policy.
    // ------------------=
    pub const fn read_only(self) -> bool {
        matches!(self, Self::ObjectRead | Self::ObjectInspect | Self::ResourceInspect | Self::PoolInspect | Self::ReplicaInspect)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolError { Length, Version, Operation, NonCanonical }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageOperationV1 {
    pub operation: Operation,
    pub object: [u8; 16],
    pub authority_generation: u64,
    pub manifest_generation: u64,
    pub object_version: u64,
    pub offset: u64,
    pub scope: u64,
    pub value: u64,
    pub length: u16,
    pub data: [u8; DATA_BYTES],
}
impl StorageOperationV1 {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes canonical little-endian storage IOP data within one existing secure envelope; unused bytes cannot smuggle a second request.
    // ------------------=
    pub fn encode(self) -> Result<[u8; OPERATION_BYTES], ProtocolError> {
        if self.length as usize > DATA_BYTES { return Err(ProtocolError::Length); }
        if self.data[self.length as usize..].iter().any(|b| *b != 0) { return Err(ProtocolError::NonCanonical); }
        let mut out = [0; OPERATION_BYTES];
        out[..2].copy_from_slice(&1u16.to_le_bytes());
        out[2..4].copy_from_slice(&self.length.to_le_bytes());
        out[4..8].copy_from_slice(&(self.operation as u32).to_le_bytes());
        out[8..24].copy_from_slice(&self.object);
        for (index, value) in [self.authority_generation, self.manifest_generation,
            self.object_version, self.offset, self.scope, self.value].iter().enumerate() {
            out[24+index*8..32+index*8].copy_from_slice(&value.to_le_bytes());
        }
        out[72..].copy_from_slice(&self.data);
        Ok(out)
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Rejects truncated, oversized, unsupported and noncanonical storage requests before any service queue or allocation is touched.
    // ------------------=
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() != OPERATION_BYTES { return Err(ProtocolError::Length); }
        if bytes[..2] != 1u16.to_le_bytes() { return Err(ProtocolError::Version); }
        let value = Self { operation: Operation::decode(u32::from_le_bytes(bytes[4..8].try_into().unwrap()))?,
            object: bytes[8..24].try_into().unwrap(), authority_generation: get(bytes, 24),
            manifest_generation: get(bytes, 32), object_version: get(bytes, 40), offset: get(bytes, 48),
            scope: get(bytes, 56), value: get(bytes, 64), length: u16::from_le_bytes(bytes[2..4].try_into().unwrap()),
            data: bytes[72..].try_into().unwrap() };
        value.encode()?;
        Ok(value)
    }
}

// ------------------------=
// FUNC: get
// DESC: Reads a fixed field only after validating the complete wire extent.
// ------------------=
fn get(bytes: &[u8], at: usize) -> u64 { u64::from_le_bytes(bytes[at..at+8].try_into().unwrap()) }

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: storage_wire_preserves_full_ids_and_exact_bounds
    // DESC: Round-trips full-width object identities and generations and rejects every truncation, oversized payload and hidden tail.
    // ------------------=
    #[test]
    fn storage_wire_preserves_full_ids_and_exact_bounds() {
        let mut request = StorageOperationV1 { operation: Operation::TransferChunk, object: [255; 16],
            authority_generation: u64::MAX, manifest_generation: 17, object_version: 23,
            offset: 65536, scope: 91, value: 13, length: 64, data: [37; DATA_BYTES] };
        let bytes = request.encode().unwrap();
        assert_eq!(StorageOperationV1::decode(&bytes), Ok(request));
        assert!(48 + bytes.len() <= 192);
        for n in 0..OPERATION_BYTES { assert_eq!(StorageOperationV1::decode(&bytes[..n]), Err(ProtocolError::Length)); }
        let mut oversized = bytes.to_vec(); oversized.push(0);
        assert_eq!(StorageOperationV1::decode(&oversized), Err(ProtocolError::Length));
        request.length = 65; assert_eq!(request.encode(), Err(ProtocolError::Length));
        request.length = 0; assert_eq!(request.encode(), Err(ProtocolError::NonCanonical));
        request.data = [0; DATA_BYTES]; assert!(request.encode().is_ok());
        let mut unknown = bytes; unknown[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(StorageOperationV1::decode(&unknown), Err(ProtocolError::Operation));
    }
}
