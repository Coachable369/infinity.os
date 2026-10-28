//! Bounded download staging, not a filesystem or ObjectStore implementation.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { TooLarge, LengthMismatch, Closed, InvalidName, InvalidType }

pub const MAX_IMAGE_BYTES:usize=4*1024*1024;
#[derive(Clone,Copy)]
pub struct Receipt {pub name:[u8;63],pub length:usize,pub bytes:u32}
impl Receipt {pub const EMPTY:Self=Self{name:[0;63],length:0,bytes:0};}

// ------------------------=
// FUNC: owner_directory
// DESC: Derives a stable user-owned Downloads namespace without accepting website path components.
// ------------------=
pub fn owner_directory(owner:[u8;16])->[u8;48] {
    let mut path=[0;48];path[..6].copy_from_slice(b"/home/");
    for(i,b)in owner.iter().enumerate(){path[6+i*2]=b"0123456789abcdef"[(b>>4)as usize];path[7+i*2]=b"0123456789abcdef"[(b&15)as usize];}
    path[38..].copy_from_slice(b"/downloads");path
}

// ------------------------=
// FUNC: destination
// DESC: Produces a collision candidate inside one user namespace, preserving the extension within path capacity.
// ------------------=
pub fn destination(owner:[u8;16],name:&[u8],attempt:u8)->([u8;95],usize) {
    let mut path=[0;95];path[..48].copy_from_slice(&owner_directory(owner));path[48]=b'/';
    let extension=name.iter().rposition(|b|*b==b'.').filter(|at|*at>0 && name.len()-*at<=10).unwrap_or(name.len());
    let suffix=if attempt==0 {0}else{3};
    let stem=extension.min(46-(name.len()-extension)-suffix);
    let mut end=49;
    for b in &name[..stem]{path[end]=if b.is_ascii_alphanumeric()||b"-_. ".contains(b){*b}else{b'_'};end+=1;}
    if attempt!=0 {path[end]=b'-';path[end+1]=b'0'+attempt/10;path[end+2]=b'0'+attempt%10;end+=3;}
    for b in &name[extension..]{path[end]=if b.is_ascii_alphanumeric()||*b==b'.'{*b}else{b'_'};end+=1;}
    (path,end)
}

// ------------------------=
// FUNC: image_metadata
// DESC: Creates a safe bounded image basename from a URL and verified response MIME type.
// ------------------=
pub fn image_metadata(url:&str,media:&str)->Result<Metadata,Error> {
    let media=media.split(';').next().unwrap_or("").trim();
    if !media.starts_with("image/"){return Err(Error::InvalidType);}
    let extension=match media {"image/png"=>"png","image/jpeg"=>"jpg","image/gif"=>"gif","image/webp"=>"webp","image/svg+xml"=>"svg","image/avif"=>"avif",_=>"img"};
    let source=url.split(['?','#']).next().unwrap_or("").rsplit('/').next().unwrap_or("");
    let mut value=Metadata{name:[0;63],name_length:0,media_type:[0;127],type_length:media.len()};
    validate_metadata("image",media)?;
    for b in source.bytes().take(35) {value.name[value.name_length]=if b.is_ascii_alphanumeric()||b"-_.".contains(&b){b}else{b'_'};value.name_length+=1;}
    if value.name_length==0 || value.name[..value.name_length].iter().all(|b|*b==b'.') {value.name[..5].copy_from_slice(b"image");value.name_length=5;}
    if !value.name[..value.name_length].contains(&b'.') {value.name[value.name_length]=b'.';value.name_length+=1;value.name[value.name_length..value.name_length+extension.len()].copy_from_slice(extension.as_bytes());value.name_length+=extension.len();}
    value.media_type[..media.len()].copy_from_slice(media.as_bytes());Ok(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub name: [u8; 63],
    pub name_length: usize,
    pub media_type: [u8; 127],
    pub type_length: usize,
}

// ------------------------=
// FUNC: attachment
// DESC: Parses a conservative attachment basename and MIME type without accepting paths or header injection.
// ------------------=
pub fn attachment(disposition: &str, media_type: &str) -> Result<Option<Metadata>, Error> {
    let mut fields = disposition.split(';');
    if !fields.next().unwrap_or("").trim().eq_ignore_ascii_case("attachment") { return Ok(None); }
    let mut filename = None;
    for field in fields {
        let Some((key, value)) = field.trim().split_once('=') else { continue; };
        if key.trim().eq_ignore_ascii_case("filename") {
            if filename.is_some() { return Err(Error::InvalidName); }
            let value = value.trim();
            let value = if value.starts_with('"') {
                value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).ok_or(Error::InvalidName)?
            } else { value };
            if value.contains('"') { return Err(Error::InvalidName); }
            filename = Some(value);
        }
    }
    let name = filename.unwrap_or("download.bin");
    let media_type = media_type.split(';').next().unwrap_or("").trim();
    let media_type = if media_type.is_empty() { "application/octet-stream" } else { media_type };
    validate_metadata(name, media_type)?;
    let mut result = Metadata { name: [0; 63], name_length: name.len(),
        media_type: [0; 127], type_length: media_type.len() };
    result.name[..name.len()].copy_from_slice(name.as_bytes());
    result.media_type[..media_type.len()].copy_from_slice(media_type.as_bytes());
    Ok(Some(result))
}

pub struct Download<const N: usize> {
    bytes: [u8; N],
    length: usize,
    expected: Option<usize>,
    closed: bool,
}

impl<const N: usize> Download<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Rejects declared oversize downloads before accepting any response body.
    // ------------------=
    pub fn new(expected: Option<usize>) -> Result<Self, Error> {
        if expected.is_some_and(|length| length > N) { return Err(Error::TooLarge); }
        Ok(Self { bytes: [0; N], length: 0, expected, closed: false })
    }
    // ------------------------=
    // FUNC: append
    // DESC: Accumulates whole chunks or invalidates the download without silently truncating it.
    // ------------------=
    pub fn append(&mut self, chunk: &[u8]) -> Result<(), Error> {
        if self.closed { return Err(Error::Closed); }
        let length = self.length.checked_add(chunk.len()).ok_or(Error::TooLarge);
        let error = match length {
            Ok(length) if length > N => Some(Error::TooLarge),
            Ok(length) if self.expected.is_some_and(|expected| length > expected) => Some(Error::LengthMismatch),
            Err(error) => Some(error),
            _ => None,
        };
        if let Some(error) = error { self.cancel(); return Err(error); }
        let length = length.unwrap();
        self.bytes[self.length..length].copy_from_slice(chunk);
        self.length = length;
        Ok(())
    }
    // ------------------------=
    // FUNC: finish
    // DESC: Exposes validated complete bytes once, for a separately authorized native object commit.
    // ------------------=
    pub fn finish(&mut self) -> Result<&[u8], Error> {
        if self.closed { return Err(Error::Closed); }
        if self.expected.is_some_and(|expected| expected != self.length) {
            self.cancel();
            return Err(Error::LengthMismatch);
        }
        self.closed = true;
        Ok(&self.bytes[..self.length])
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Discards staged content and prevents subsequent appends or commits.
    // ------------------=
    pub fn cancel(&mut self) {
        self.bytes[..self.length].fill(0);
        self.length = 0;
        self.closed = true;
    }
}

// ------------------------=
// FUNC: validate_metadata
// DESC: Accepts a bounded basename and ASCII media type, never an arbitrary namespace path.
// ------------------=
pub fn validate_metadata(name: &str, media_type: &str) -> Result<(), Error> {
    if name.is_empty() || name.len() > 63 || name == "." || name == ".."
        || name.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')) {
        return Err(Error::InvalidName);
    }
    let Some((kind, subtype)) = media_type.split_once('/') else { return Err(Error::InvalidType); };
    if media_type.len() > 127 || kind.is_empty() || subtype.is_empty()
        || !kind.bytes().chain(subtype.bytes()).all(|c| c.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&c)) {
        return Err(Error::InvalidType);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: image_names_and_destinations_are_bounded_and_distinct
    // DESC: Verifies URL sanitization, extension retention, distinct owners and collision candidates.
    // ------------------=
    #[test] fn image_names_and_destinations_are_bounded_and_distinct(){
        let m=image_metadata("https://cdn.test/path/portrait?x=1","image/jpeg").unwrap();
        assert_eq!(&m.name[..m.name_length],b"portrait.jpg");
        assert!(image_metadata("https://a.test/x","text/html").is_err());
        let (a,n)=destination([1;16],&[b'x';63],0);assert!(n<=95);
        let (b,_)=destination([2;16],&[b'x';63],0);assert_ne!(a,b);
        let (c,k)=destination([1;16],b"portrait.jpg",1);assert_eq!(&c[49..k],b"portrait-01.jpg");
        let (d,k)=destination([1;16],b"../../escape.png",0);assert!(!d[49..k].contains(&b'/'));
    }
    // ------------------------=
    // FUNC: attachment_metadata_preserves_types_and_rejects_paths
    // DESC: Exercises response metadata parsing without giving remote names namespace authority.
    // ------------------=
    #[test]
    fn attachment_metadata_preserves_types_and_rejects_paths() {
        let value = attachment("attachment; filename=\"notes.txt\"", "text/plain; charset=utf-8").unwrap().unwrap();
        assert_eq!(&value.name[..value.name_length], b"notes.txt");
        assert_eq!(&value.media_type[..value.type_length], b"text/plain");
        assert_eq!(attachment("inline; filename=notes.txt", "text/plain"), Ok(None));
        for value in ["attachment; filename=../secret", "attachment; filename=a; filename=b",
            "attachment; filename=\"unterminated", "attachment; filename=a\\b"] {
            assert_eq!(attachment(value, "text/plain"), Err(Error::InvalidName));
        }
        let fallback=attachment("attachment", "").unwrap().unwrap();
        assert_eq!(&fallback.name[..fallback.name_length], b"download.bin");
        assert_eq!(&fallback.media_type[..fallback.type_length], b"application/octet-stream");
    }
    // ------------------------=
    // FUNC: bounded_content_is_complete_or_rejected
    // DESC: Verifies byte preservation, single consumption, overflow and partial-body failure.
    // ------------------=
    #[test]
    fn bounded_content_is_complete_or_rejected() {
        let mut download = Download::<4>::new(Some(4)).unwrap();
        download.append(&[1, 2]).unwrap();
        download.append(&[3, 4]).unwrap();
        assert_eq!(download.finish(), Ok(&[1, 2, 3, 4][..]));
        assert_eq!(download.finish(), Err(Error::Closed));
        let mut download = Download::<4>::new(None).unwrap();
        assert_eq!(download.append(&[0; 5]), Err(Error::TooLarge));
        assert_eq!(download.finish(), Err(Error::Closed));
        let mut download = Download::<4>::new(Some(4)).unwrap();
        download.append(&[1]).unwrap();
        assert_eq!(download.finish(), Err(Error::LengthMismatch));
    }
    // ------------------------=
    // FUNC: metadata_cannot_choose_paths_or_inject_headers
    // DESC: Rejects path components and response metadata control characters.
    // ------------------=
    #[test]
    fn metadata_cannot_choose_paths_or_inject_headers() {
        assert_eq!(validate_metadata("note.txt", "text/plain"), Ok(()));
        for name in ["../note", "..", "C:\\note", "a/b", "a\n"] {
            assert_eq!(validate_metadata(name, "text/plain"), Err(Error::InvalidName));
        }
        assert_eq!(validate_metadata("note", "text/plain\r\nx: y"), Err(Error::InvalidType));
    }
}
