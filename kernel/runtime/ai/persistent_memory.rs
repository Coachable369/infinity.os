//! Model-independent AIKit records. The native ObjectStore is the durability authority.
pub const MAX_RECORDS: usize = 16;
pub const MAX_CONTENT: usize = 512;
pub const MAX_SOURCE: usize = 64;
pub const MAX_DIMENSIONS: usize = 256;
pub const MAX_BYTES: usize = 16 * 1024;
pub type Owner = [u8; 16];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unauthorized,
    Invalid,
    Capacity,
    NotFound,
    Corrupt,
    Version,
    Storage,
    Unavailable,
    ModelMismatch,
    DimensionMismatch,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Category {
    Episodic = 1,
    Semantic = 2,
    Procedural = 3,
}
#[derive(Clone, Copy)]
pub struct Record {
    pub id: u64,
    pub owner: Owner,
    pub category: Category,
    pub created_at: u64,
    pub updated_at: u64,
    content: [u8; MAX_CONTENT],
    content_len: u16,
    source: [u8; MAX_SOURCE],
    source_len: u8,
    model: [u8; 32],
    model_len: u8,
    dimensions: u16,
    embedding: [f32; MAX_DIMENSIONS],
}
impl Record {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates an unused bounded record without allocation.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            id: 0,
            owner: [0; 16],
            category: Category::Semantic,
            created_at: 0,
            updated_at: 0,
            content: [0; MAX_CONTENT],
            content_len: 0,
            source: [0; MAX_SOURCE],
            source_len: 0,
            model: [0; 32],
            model_len: 0,
            dimensions: 0,
            embedding: [0.0; MAX_DIMENSIONS],
        }
    }
    // ------------------------=
    // FUNC: new
    // DESC: Validates attributed UTF-8 content independently of conversational model selection.
    // ------------------=
    pub fn new(
        owner: Owner,
        category: Category,
        content: &[u8],
        source: &[u8],
        now: u64,
    ) -> Result<Self, Error> {
        if owner == [0; 16] {
            return Err(Error::Unauthorized);
        }
        if !text_valid(content, MAX_CONTENT) || !text_valid(source, MAX_SOURCE) {
            return Err(Error::Invalid);
        }
        let mut r = Self::empty();
        r.owner = owner;
        r.category = category;
        r.created_at = now;
        r.updated_at = now;
        r.content[..content.len()].copy_from_slice(content);
        r.content_len = content.len() as u16;
        r.source[..source.len()].copy_from_slice(source);
        r.source_len = source.len() as u8;
        Ok(r)
    }
    // ------------------------=
    // FUNC: content
    // DESC: Returns the original attributed memory text.
    // ------------------=
    pub fn content(&self) -> &[u8] {
        &self.content[..self.content_len as usize]
    }
    // ------------------------=
    // FUNC: source
    // DESC: Returns provenance as data, never as authorization.
    // ------------------=
    pub fn source(&self) -> &[u8] {
        &self.source[..self.source_len as usize]
    }
    // ------------------------=
    // FUNC: model
    // DESC: Identifies the embedding space independently of the conversational LLM.
    // ------------------=
    pub fn model(&self) -> &[u8] {
        &self.model[..self.model_len as usize]
    }
    // ------------------------=
    // FUNC: embedding
    // DESC: Returns only validated initialized vector dimensions.
    // ------------------=
    pub fn embedding(&self) -> &[f32] {
        &self.embedding[..self.dimensions as usize]
    }
    // ------------------------=
    // FUNC: set_embedding
    // DESC: Rejects incompatible dimensions, nonfinite values and zero vectors before replacing an embedding.
    // ------------------=
    pub fn set_embedding(
        &mut self,
        model: &[u8],
        vector: &[f32],
        dimensions: usize,
    ) -> Result<(), Error> {
        if !text_valid(model, 32) {
            return Err(Error::Invalid);
        }
        validate_vector(vector, dimensions)?;
        self.model.fill(0);
        self.model[..model.len()].copy_from_slice(model);
        self.model_len = model.len() as u8;
        self.embedding.fill(0.0);
        self.embedding[..vector.len()].copy_from_slice(vector);
        self.dimensions = dimensions as u16;
        Ok(())
    }
}
#[derive(Clone, Copy)]
pub struct Memory {
    owner: Owner,
    next: u64,
    count: usize,
    records: [Record; MAX_RECORDS],
}
impl Memory {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a model-independent owner catalog; session authorization belongs to the native API adapter.
    // ------------------=
    pub const fn new(owner: Owner) -> Self {
        Self {
            owner,
            next: 1,
            count: 0,
            records: [Record::empty(); MAX_RECORDS],
        }
    }
    // ------------------------=
    // FUNC: authorize
    // DESC: Rejects anonymous and cross-owner access even when callers bypass higher-level prompt policy.
    // ------------------=
    fn authorize(&self, owner: Owner) -> Result<(), Error> {
        if owner == [0; 16] || owner != self.owner {
            Err(Error::Unauthorized)
        } else {
            Ok(())
        }
    }
    // ------------------------=
    // FUNC: records
    // DESC: Returns only the authorized catalog's active records.
    // ------------------=
    pub fn records(&self, owner: Owner) -> Result<&[Record], Error> {
        self.authorize(owner)?;
        Ok(&self.records[..self.count])
    }
    // ------------------------=
    // FUNC: inspect
    // DESC: Resolves an immutable stable identifier within one authorized catalog.
    // ------------------=
    pub fn inspect(&self, owner: Owner, id: u64) -> Result<&Record, Error> {
        self.records(owner)?
            .iter()
            .find(|r| r.id == id)
            .ok_or(Error::NotFound)
    }
    // ------------------------=
    // FUNC: store
    // DESC: Inserts an attributed record without evicting other memories and deduplicates exact case-insensitive content.
    // ------------------=
    pub fn store(&mut self, owner: Owner, mut record: Record) -> Result<u64, Error> {
        self.authorize(owner)?;
        if record.owner != owner {
            return Err(Error::Unauthorized);
        }
        if !text_valid(record.content(), MAX_CONTENT) || !text_valid(record.source(), MAX_SOURCE) {
            return Err(Error::Invalid);
        }
        if let Some(old) = self.records[..self.count].iter().find(|r| {
            r.category == record.category && r.content().eq_ignore_ascii_case(record.content())
        }) {
            return Ok(old.id);
        }
        if self.count == MAX_RECORDS || self.next == u64::MAX {
            return Err(Error::Capacity);
        }
        record.id = self.next;
        self.next += 1;
        self.records[self.count] = record;
        self.count += 1;
        Ok(record.id)
    }
    // ------------------------=
    // FUNC: update
    // DESC: Preserves identity and creation time while replacing content and its explicitly supplied embedding together.
    // ------------------=
    pub fn update(&mut self, owner: Owner, id: u64, mut record: Record) -> Result<(), Error> {
        self.authorize(owner)?;
        if record.owner != owner {
            return Err(Error::Unauthorized);
        }
        if !text_valid(record.content(), MAX_CONTENT) || !text_valid(record.source(), MAX_SOURCE) {
            return Err(Error::Invalid);
        }
        let old = self.records[..self.count]
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(Error::NotFound)?;
        record.id = id;
        record.created_at = old.created_at;
        record.updated_at = record.updated_at.max(old.updated_at);
        *old = record;
        Ok(())
    }
    // ------------------------=
    // FUNC: delete
    // DESC: Removes a record from the authoritative snapshot and active index without identifier reuse.
    // ------------------=
    pub fn delete(&mut self, owner: Owner, id: u64) -> Result<(), Error> {
        self.authorize(owner)?;
        let i = self.records[..self.count]
            .iter()
            .position(|r| r.id == id)
            .ok_or(Error::NotFound)?;
        self.records.copy_within(i + 1..self.count, i);
        self.count -= 1;
        self.records[self.count] = Record::empty();
        Ok(())
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Emits a versioned little-endian checkpoint with payload checksum and no rebuildable index state.
    // ------------------=
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, Error> {
        self.authorize(self.owner)?;
        let mut w = Writer { out, at: 0 };
        w.put(b"IMEM\x01\0\0\0")?;
        w.put(&self.owner)?;
        w.put(&self.next.to_le_bytes())?;
        w.put(&(self.count as u16).to_le_bytes())?;
        for r in &self.records[..self.count] {
            w.put(&r.id.to_le_bytes())?;
            w.put(&[r.category as u8])?;
            w.put(&r.created_at.to_le_bytes())?;
            w.put(&r.updated_at.to_le_bytes())?;
            w.put(&r.content_len.to_le_bytes())?;
            w.put(&[r.source_len, r.model_len])?;
            w.put(&r.dimensions.to_le_bytes())?;
            w.put(r.content())?;
            w.put(r.source())?;
            w.put(r.model())?;
            for f in r.embedding() {
                w.put(&f.to_bits().to_le_bytes())?;
            }
        }
        let crc = checksum(&w.out[..w.at]);
        w.put(&crc.to_le_bytes())?;
        Ok(w.at)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Rebuilds the active record index only from a complete validated checkpoint; corrupt bytes are never silently reset.
    // ------------------=
    pub fn decode(bytes: &[u8], owner: Owner) -> Result<Self, Error> {
        if bytes.len() < 38 || bytes.len() > MAX_BYTES {
            return Err(Error::Corrupt);
        }
        let end = bytes.len() - 4;
        let crc = u32::from_le_bytes(bytes[end..].try_into().map_err(|_| Error::Corrupt)?);
        if checksum(&bytes[..end]) != crc {
            return Err(Error::Corrupt);
        }
        let mut rd = Reader {
            bytes: &bytes[..end],
            at: 0,
        };
        if rd.take(4)? != b"IMEM" {
            return Err(Error::Corrupt);
        }
        if rd.take(4)? != [1, 0, 0, 0] {
            return Err(Error::Version);
        }
        if owner == [0; 16] || rd.take(16)? != owner {
            return Err(Error::Unauthorized);
        }
        let mut m = Self::new(owner);
        m.next = rd.u64()?;
        let count = rd.u16()? as usize;
        if count > MAX_RECORDS || m.next == 0 {
            return Err(Error::Corrupt);
        }
        for i in 0..count {
            let id = rd.u64()?;
            if id == 0 || id >= m.next || m.records[..i].iter().any(|r| r.id == id) {
                return Err(Error::Corrupt);
            }
            let category = match rd.take(1)?[0] {
                1 => Category::Episodic,
                2 => Category::Semantic,
                3 => Category::Procedural,
                _ => return Err(Error::Corrupt),
            };
            let created = rd.u64()?;
            let updated = rd.u64()?;
            if updated < created {
                return Err(Error::Corrupt);
            }
            let cn = rd.u16()? as usize;
            let sn = rd.take(1)?[0] as usize;
            let mn = rd.take(1)?[0] as usize;
            let dims = rd.u16()? as usize;
            let mut record = Record::new(owner, category, rd.take(cn)?, rd.take(sn)?, created)
                .map_err(|_| Error::Corrupt)?;
            record.id = id;
            record.updated_at = updated;
            let model = rd.take(mn)?;
            if dims > MAX_DIMENSIONS || (dims == 0) != (mn == 0) {
                return Err(Error::Corrupt);
            }
            if dims > 0 {
                let mut vector = [0.0; MAX_DIMENSIONS];
                for v in &mut vector[..dims] {
                    *v = f32::from_bits(u32::from_le_bytes(
                        rd.take(4)?.try_into().map_err(|_| Error::Corrupt)?,
                    ));
                }
                record
                    .set_embedding(model, &vector[..dims], dims)
                    .map_err(|_| Error::Corrupt)?;
            }
            m.records[i] = record;
        }
        if rd.at != end {
            return Err(Error::Corrupt);
        }
        m.count = count;
        Ok(m)
    }
}
// ------------------------=
// FUNC: text_valid
// DESC: Accepts bounded UTF-8 text while excluding control bytes and empty values.
// ------------------=
fn text_valid(text: &[u8], max: usize) -> bool {
    !text.is_empty()
        && text.len() <= max
        && core::str::from_utf8(text).is_ok()
        && text.iter().all(|&b| b >= 32 || b == b'\n' || b == b'\t')
}
// ------------------------=
// FUNC: validate_vector
// DESC: Validates vector shape and finite nonzero norm without assuming an embedding model is installed.
// ------------------=
pub fn validate_vector(v: &[f32], dimensions: usize) -> Result<(), Error> {
    if dimensions == 0 || dimensions > MAX_DIMENSIONS || v.len() != dimensions {
        return Err(Error::DimensionMismatch);
    }
    if v.iter().any(|x| !x.is_finite())
        || v.iter().map(|&x| f64::from(x) * f64::from(x)).sum::<f64>() == 0.0
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
// ------------------------=
// FUNC: checksum
// DESC: Detects truncated or damaged checkpoint content with IEEE CRC32, independently of native extent checksums.
// ------------------=
fn checksum(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in bytes {
        c ^= b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ (0xedb88320u32 & 0u32.wrapping_sub(c & 1));
        }
    }
    !c
}
struct Writer<'a> {
    out: &'a mut [u8],
    at: usize,
}
impl Writer<'_> {
    // ------------------------=
    // FUNC: put
    // DESC: Bounds every serialized field before writing to caller-owned storage.
    // ------------------=
    fn put(&mut self, b: &[u8]) -> Result<(), Error> {
        let end = self.at.checked_add(b.len()).ok_or(Error::Capacity)?;
        if end > self.out.len() || end > MAX_BYTES {
            return Err(Error::Capacity);
        }
        self.out[self.at..end].copy_from_slice(b);
        self.at = end;
        Ok(())
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    // ------------------------=
    // FUNC: take
    // DESC: Rejects out-of-range fields without unchecked indexing or allocation.
    // ------------------=
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).ok_or(Error::Corrupt)?;
        let result = self.bytes.get(self.at..end).ok_or(Error::Corrupt)?;
        self.at = end;
        Ok(result)
    }
    // ------------------------=
    // FUNC: u16
    // DESC: Decodes an architecture-independent short integer.
    // ------------------=
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| Error::Corrupt)?,
        ))
    }
    // ------------------------=
    // FUNC: u64
    // DESC: Decodes an architecture-independent stable identifier or timestamp.
    // ------------------=
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| Error::Corrupt)?,
        ))
    }
}
