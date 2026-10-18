//! Bounded compiler-feedback projection; raw diagnostics remain in the compiler job.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Projection {
    pub bytes: usize,
    pub errors: usize,
    pub notes: usize,
    pub warnings: usize,
    pub truncated: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Severity { Error, Note, Warning, Other }

// ------------------------=
// FUNC: contains
// DESC: Matches a bounded compiler diagnostic marker without allocating or interpreting diagnostic text as authority.
// ------------------=
fn contains(line:&[u8],marker:&[u8])->bool {
    line.windows(marker.len()).any(|window|window==marker)
}

// ------------------------=
// FUNC: severity
// DESC: Recognizes Clang and LLD severity records while leaving unknown formats available for fallback.
// ------------------=
fn severity(line:&[u8])->Severity {
    if contains(line,b": error:") || contains(line,b": fatal error:")
        || line.starts_with(b"error:") || line.starts_with(b"fatal error:") {Severity::Error}
    else if contains(line,b": note:") || line.starts_with(b"note:") {Severity::Note}
    else if contains(line,b": warning:") || line.starts_with(b"warning:") {Severity::Warning}
    else {Severity::Other}
}

// ------------------------=
// FUNC: append
// DESC: Appends one complete diagnostic line when possible and reports bounded truncation explicitly.
// ------------------=
fn append(line:&[u8],output:&mut [u8],projection:&mut Projection)->bool {
    let remaining=output.len().saturating_sub(projection.bytes);
    if remaining==0 {projection.truncated=true;return false;}
    let length=line.len().min(remaining.saturating_sub(1));
    output[projection.bytes..projection.bytes+length].copy_from_slice(&line[..length]);
    projection.bytes+=length;
    output[projection.bytes]=b'\n';projection.bytes+=1;
    projection.truncated|=length<line.len();
    true
}

// ------------------------=
// FUNC: prioritize
// DESC: Preserves error records before notes and warnings so a warning-heavy prefix cannot hide the actual failed compilation.
// ------------------=
pub fn prioritize(input:&[u8],output:&mut [u8])->Projection {
    let mut result=Projection::default();
    let known=input.split(|byte|*byte==b'\n').any(|line|severity(line)!=Severity::Other);
    if !known {
        result.bytes=input.len().min(output.len());
        output[..result.bytes].copy_from_slice(&input[..result.bytes]);
        result.truncated=result.bytes<input.len();
        return result;
    }
    for wanted in [Severity::Error,Severity::Note,Severity::Warning] {
        for line in input.split(|byte|*byte==b'\n') {
            if severity(line)!=wanted {continue;}
            if append(line,output,&mut result) {
                match wanted {Severity::Error=>result.errors+=1,Severity::Note=>result.notes+=1,
                    Severity::Warning=>result.warnings+=1,Severity::Other=>{}}
            }
        }
    }
    // Source and caret excerpts are intentionally omitted: the repair prompt
    // already carries the full failed source and diagnostic location records.
    result
}
