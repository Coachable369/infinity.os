//! One service-owned immutable source extent cache. Authority is checked by the
//! caller against the current manifest before every use, including cache hits.
use crate::runtime::fabric::manifest::Manifest;
use crate::storage::{
    object::{ObjectError, ObjectStore},
    BlockDevice,
};
pub(super) struct SourceCache {
    object: [u8; 16],
    version: u64,
    content: [u8; 16],
    hash: [u8; 32],
    length: usize,
    bytes: [u8; 16384],
}
impl SourceCache {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes a fixed one-extent service cache without changing generic ObjectStore stack size.
    // ------------------=
    pub(super) const fn new() -> Self {
        Self {
            object: [0; 16],
            version: 0,
            content: [0; 16],
            hash: [0; 32],
            length: 0,
            bytes: [0; 16384],
        }
    }
    // ------------------------=
    // FUNC: invalidate
    // DESC: Invalidates identity before any source mutation or service remount.
    // ------------------=
    pub(super) fn invalidate(&mut self) {
        self.version = 0;
        self.length = 0;
    }
    // ------------------------=
    // FUNC: read
    // DESC: Returns a bounded range from verified immutable extents, loading each at most once during sequential transfer.
    // ------------------=
    pub(super) fn read<D: BlockDevice>(
        &mut self,
        store: &mut ObjectStore<D>,
        m: &Manifest,
        offset: u64,
        out: &mut [u8],
    ) -> Result<(), ObjectError> {
        if out.len() > 64
            || offset
                .checked_add(out.len() as u64)
                .is_none_or(|end| end > m.length)
        {
            return Err(ObjectError::InvalidObject);
        }
        let mut start = 0;
        let mut copied = 0;
        for (index, chunk) in m
            .chunks
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.map(|c| (i, c)))
        {
            let end = start + chunk.bytes as u64;
            if offset < end && offset + out.len() as u64 > start {
                if self.object != m.object
                    || self.version != m.version
                    || self.content != chunk.content
                    || self.hash != chunk.hash
                    || self.length != chunk.bytes as usize
                {
                    self.invalidate();
                    let len = store.pool_verified_chunk(m, index, &mut self.bytes)?;
                    self.object = m.object;
                    self.version = m.version;
                    self.content = chunk.content;
                    self.hash = chunk.hash;
                    self.length = len;
                }
                let from = offset.max(start);
                let to = (offset + out.len() as u64).min(end);
                out[(from - offset) as usize..(to - offset) as usize]
                    .copy_from_slice(&self.bytes[(from - start) as usize..(to - start) as usize]);
                copied += (to - from) as usize;
            }
            start = end;
        }
        if copied != out.len() {
            return Err(ObjectError::CorruptContent);
        }
        Ok(())
    }
}
