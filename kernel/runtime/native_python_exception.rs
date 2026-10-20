//! Generated-run diagnostic evidence only; never grants execution or repair authority.
pub const OPERATION: u64 = 0x1290;
pub const MAGIC: u32 = 0x4950_5845;
pub const RECORD_BYTES: usize = 352;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Category { Source = 1, OperatingSystem = 2, Resource = 3, Other = 4 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ExceptionClass {
    Value = 1, Type = 2, Index = 3, Key = 4, Attribute = 5, Name = 6,
    ZeroDivision = 7, Assertion = 8, Runtime = 9, Syntax = 10,
    OperatingSystem = 11, Memory = 12, Recursion = 13, Other = 14,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub category: Category,
    pub class: ExceptionClass,
    pub line: u32,
    pub class_name: [u8; 64],
    pub class_name_len: u8,
}

impl Evidence {
    // ------------------------=
    // FUNC: repairable
    // DESC: Selects source-language failures only; callers must still prove exact job authority, native exit and reclamation.
    // ------------------=
    pub fn repairable(&self) -> bool { self.category == Category::Source }

    // ------------------------=
    // FUNC: diagnostics
    // DESC: Prioritizes typed exception identity and source line, then retains the bounded tail of the actual stderr stream for recovery context.
    // ------------------=
    pub fn diagnostics(&self, stderr: &[u8], output: &mut [u8]) -> usize {
        let mut digits = [0; 10];
        let mut cursor = digits.len();
        let mut number = self.line;
        loop {
            cursor -= 1; digits[cursor] = b'0' + (number % 10) as u8; number /= 10;
            if number == 0 { break; }
        }
        let mut written = 0;
        for part in [&self.class_name[..self.class_name_len as usize],
            b" at source line ".as_slice(), &digits[cursor..], b"\n".as_slice()] {
            let count = part.len().min(output.len() - written);
            output[written..written + count].copy_from_slice(&part[..count]);
            written += count;
        }
        let count = stderr.len().min(output.len() - written);
        output[written..written + count].copy_from_slice(&stderr[stderr.len() - count..]);
        written + count
    }
}

// ------------------------=
// FUNC: decode
// DESC: Validates bounded typed evidence against the exact granted source, rejecting out-of-range lines and malformed or foreign records.
// ------------------=
pub fn decode(bytes: &[u8], expected_path: &[u8], source: &[u8]) -> Option<Evidence> {
    if bytes.len() != RECORD_BYTES { return None; }
    let words: [u32; 8] = core::array::from_fn(|i|
        u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()));
    if words[0] != MAGIC || words[1] != 1 || words[7] != 0 { return None; }
    let category = match words[2] {
        1 => Category::Source, 2 => Category::OperatingSystem,
        3 => Category::Resource, 4 => Category::Other, _ => return None,
    };
    let class = match words[3] {
        1 => ExceptionClass::Value, 2 => ExceptionClass::Type, 3 => ExceptionClass::Index,
        4 => ExceptionClass::Key, 5 => ExceptionClass::Attribute, 6 => ExceptionClass::Name,
        7 => ExceptionClass::ZeroDivision, 8 => ExceptionClass::Assertion,
        9 => ExceptionClass::Runtime, 10 => ExceptionClass::Syntax,
        11 => ExceptionClass::OperatingSystem, 12 => ExceptionClass::Memory,
        13 => ExceptionClass::Recursion, 14 => ExceptionClass::Other, _ => return None,
    };
    let required = match class {
        ExceptionClass::OperatingSystem => Category::OperatingSystem,
        ExceptionClass::Memory | ExceptionClass::Recursion => Category::Resource,
        ExceptionClass::Other => Category::Other, _ => Category::Source,
    };
    let path_len = words[5] as usize;
    let name_len = words[6] as usize;
    let lines = source.iter().filter(|byte| **byte == b'\n').count()
        + usize::from(!source.is_empty() && source.last() != Some(&b'\n'));
    if category != required || words[4] == 0 || words[4] as usize > lines
        || path_len == 0 || path_len > 256 || name_len == 0 || name_len > 64
        || bytes[32..32 + path_len] != *expected_path
        || bytes[32 + path_len..288].iter().any(|byte| *byte != 0)
        || bytes[288 + name_len..].iter().any(|byte| *byte != 0)
        || !bytes[288..288 + name_len].iter().all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'.')
    { return None; }
    let mut class_name = [0; 64];
    class_name[..name_len].copy_from_slice(&bytes[288..288 + name_len]);
    Some(Evidence { category, class, line: words[4], class_name, class_name_len: name_len as u8 })
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: record
    // DESC: Constructs binary protocol fixtures independently of prose diagnostics.
    // ------------------=
    fn record(category: u32, class: u32, line: u32) -> [u8; RECORD_BYTES] {
        let mut bytes = [0; RECORD_BYTES];
        for (i, value) in [MAGIC, 1, category, class, line, 10, 10, 0].iter().enumerate() {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[32..42].copy_from_slice(b"/script.py");
        bytes[288..298].copy_from_slice(b"ValueError");
        bytes
    }
    // ------------------------=
    // FUNC: scoped_binary_evidence
    // DESC: Rejects malformed, foreign and inconsistent records and separates source failures from OS/resource errors.
    // ------------------=
    #[test]
    fn scoped_binary_evidence() {
        for (category, class, repairable) in [(1, 1, true), (1, 2, true), (2, 11, false), (3, 12, false), (3, 13, false), (4, 14, false)] {
            let evidence = decode(&record(category, class, 2), b"/script.py", b"one\ntwo\n").unwrap();
            assert_eq!(evidence.line, 2);
            assert_eq!(evidence.repairable(), repairable);
        }
        let valid = record(1, 1, 2);
        assert!(decode(&valid, b"/foreign.py", b"one\ntwo\n").is_none());
        assert!(decode(&valid, b"/script.py", b"one\n").is_none());
        assert!(decode(&valid, b"/script.py", b"").is_none());
        assert!(decode(&valid[..RECORD_BYTES - 1], b"/script.py", b"one\ntwo\n").is_none());
        for (offset, value) in [(0, 0), (4, 2), (8, 3), (12, 99), (16, 0), (20, 257), (24, 65), (28, 1)] {
            let mut bytes = valid;
            bytes[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(value));
            assert!(decode(&bytes, b"/script.py", b"one\ntwo\n").is_none());
        }
        for offset in [42, 298] {
            let mut bytes = valid; bytes[offset] = 1;
            assert!(decode(&bytes, b"/script.py", b"one\ntwo\n").is_none());
        }
    }
    // ------------------------=
    // FUNC: bounded_diagnostic_transport
    // DESC: Preserves exact stderr-tail bytes inside every output bound without treating human-readable formatting as a success oracle.
    // ------------------=
    #[test]
    fn bounded_diagnostic_transport() {
        let evidence = decode(&record(1, 1, 2), b"/script.py", b"one\ntwo\n").unwrap();
        let stderr: [u8; 256] = core::array::from_fn(|i| i as u8);
        let mut output = [0x5a; 384];
        let header = evidence.diagnostics(&[], &mut output);
        for capacity in 0..=output.len() {
            output.fill(0x5a);
            let written = evidence.diagnostics(&stderr, &mut output[..capacity]);
            assert_eq!(written, (header + stderr.len()).min(capacity));
            assert!(output[capacity..].iter().all(|byte| *byte == 0x5a));
            if capacity > header {
                let retained = (capacity - header).min(stderr.len());
                assert_eq!(&output[header..written], &stderr[stderr.len() - retained..]);
            }
        }
    }
}
