//! Bounds-checked, zero-copy GGUF v3 reader. File offsets remain 64-bit.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Truncated,
    Format,
    Unsupported,
    Overflow,
    Missing,
}

#[derive(Clone, Copy)]
pub struct Reader<'a> {
    pub bytes: &'a [u8],
    pub position: usize,
}
impl<'a> Reader<'a> {
    // ------------------------=
    // FUNC: take
    // DESC: Reads a bounded slice without alignment assumptions.
    // ------------------=
    pub fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self.position.checked_add(count).ok_or(Error::Overflow)?;
        let result = self.bytes.get(self.position..end).ok_or(Error::Truncated)?;
        self.position = end;
        Ok(result)
    }
    // ------------------------=
    // FUNC: u32
    // DESC: Reads a little-endian integer.
    // ------------------=
    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    // ------------------------=
    // FUNC: u64
    // DESC: Reads a little-endian integer.
    // ------------------=
    pub fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    // ------------------------=
    // FUNC: string
    // DESC: Reads a GGUF length-prefixed byte string.
    // ------------------=
    pub fn string(&mut self) -> Result<&'a [u8], Error> {
        let count = usize::try_from(self.u64()?).map_err(|_| Error::Overflow)?;
        self.take(count)
    }
    // ------------------------=
    // FUNC: skip
    // DESC: Validates and skips one metadata value; nested arrays are rejected.
    // ------------------=
    pub fn skip(&mut self, kind: u32) -> Result<(), Error> {
        match kind {
            0 | 1 | 7 => {
                self.take(1)?;
            }
            2 | 3 => {
                self.take(2)?;
            }
            4 | 5 | 6 => {
                self.take(4)?;
            }
            10 | 11 | 12 => {
                self.take(8)?;
            }
            8 => {
                self.string()?;
            }
            9 => {
                let element = self.u32()?;
                let count = self.u64()?;
                if element == 9 || count > self.bytes.len() as u64 {
                    return Err(Error::Format);
                }
                for _ in 0..count {
                    self.skip(element)?;
                }
            }
            _ => return Err(Error::Unsupported),
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tensor<'a> {
    pub name: &'a [u8],
    pub dimensions: [usize; 4],
    pub rank: usize,
    pub kind: u32,
    pub data: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct Model<'a> {
    bytes: &'a [u8],
    metadata: usize,
    tensors: usize,
    table: usize,
    data: usize,
}
impl<'a> Model<'a> {
    // ------------------------=
    // FUNC: parse
    // DESC: Validates the GGUF header and complete tensor table before exposing weights.
    // ------------------=
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        let mut reader = Reader { bytes, position: 0 };
        if reader.take(4)? != b"GGUF" || reader.u32()? != 3 {
            return Err(Error::Format);
        }
        let tensors = usize::try_from(reader.u64()?).map_err(|_| Error::Overflow)?;
        let metadata = usize::try_from(reader.u64()?).map_err(|_| Error::Overflow)?;
        if tensors > 1024 || metadata > 1024 {
            return Err(Error::Unsupported);
        }
        let mut alignment = 32usize;
        for _ in 0..metadata {
            let key = reader.string()?;
            let kind = reader.u32()?;
            if key == b"general.alignment" {
                if kind != 4 {
                    return Err(Error::Format);
                }
                alignment = reader.u32()? as usize;
            } else {
                reader.skip(kind)?;
            }
        }
        if !alignment.is_power_of_two() || alignment > 4096 {
            return Err(Error::Format);
        }
        let table = reader.position;
        for _ in 0..tensors {
            reader.string()?;
            let rank = reader.u32()?;
            if rank == 0 || rank > 4 {
                return Err(Error::Format);
            }
            reader.take(rank as usize * 8 + 12)?;
        }
        let data = reader
            .position
            .checked_add(alignment - 1)
            .ok_or(Error::Overflow)?
            & !(alignment - 1);
        let result = Self {
            bytes,
            metadata,
            tensors,
            table,
            data,
        };
        // Validate every offset and type, not just tensors used by the first layer.
        let mut cursor = table;
        for _ in 0..tensors {
            result.read_tensor(&mut cursor)?;
        }
        Ok(result)
    }
    // ------------------------=
    // FUNC: metadata
    // DESC: Returns one typed metadata value without copying its payload.
    // ------------------=
    pub fn metadata(&self, name: &[u8]) -> Result<(u32, Reader<'a>), Error> {
        let mut reader = Reader {
            bytes: self.bytes,
            position: 24,
        };
        for _ in 0..self.metadata {
            let key = reader.string()?;
            let kind = reader.u32()?;
            if key == name {
                return Ok((kind, reader));
            }
            reader.skip(kind)?;
        }
        Err(Error::Missing)
    }
    // ------------------------=
    // FUNC: read_tensor
    // DESC: Resolves one tensor to its checked, aligned model byte range.
    // ------------------=
    fn read_tensor(&self, cursor: &mut usize) -> Result<Tensor<'a>, Error> {
        let mut reader = Reader {
            bytes: self.bytes,
            position: *cursor,
        };
        let name = reader.string()?;
        let rank = reader.u32()? as usize;
        if rank == 0 || rank > 4 {
            return Err(Error::Format);
        }
        let mut dimensions = [1; 4];
        let mut elements = 1usize;
        for dimension in &mut dimensions[..rank] {
            *dimension = usize::try_from(reader.u64()?).map_err(|_| Error::Overflow)?;
            if *dimension == 0 {
                return Err(Error::Format);
            }
            elements = elements.checked_mul(*dimension).ok_or(Error::Overflow)?;
        }
        let kind = reader.u32()?;
        let offset = usize::try_from(reader.u64()?).map_err(|_| Error::Overflow)?;
        let (block, size) = match kind {
            0 => (1, 4),
            1 => (1, 2),
            12 => (256, 144),
            14 => (256, 210),
            _ => return Err(Error::Unsupported),
        };
        if dimensions[0] % block != 0 {
            return Err(Error::Format);
        }
        let length = (elements / block)
            .checked_mul(size)
            .ok_or(Error::Overflow)?;
        let start = self.data.checked_add(offset).ok_or(Error::Overflow)?;
        let end = start.checked_add(length).ok_or(Error::Overflow)?;
        let data = self.bytes.get(start..end).ok_or(Error::Truncated)?;
        *cursor = reader.position;
        Ok(Tensor {
            name,
            dimensions,
            rank,
            kind,
            data,
        })
    }
    // ------------------------=
    // FUNC: tensor
    // DESC: Finds a tensor during model initialization, never during matrix painting or inference.
    // ------------------=
    pub fn tensor(&self, name: &[u8]) -> Result<Tensor<'a>, Error> {
        let mut cursor = self.table;
        for _ in 0..self.tensors {
            let tensor = self.read_tensor(&mut cursor)?;
            if tensor.name == name {
                return Ok(tensor);
            }
        }
        Err(Error::Missing)
    }
}
