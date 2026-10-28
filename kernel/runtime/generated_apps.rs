//! Durable catalog of verified user-created applications, not an execution framework.
pub const CAPACITY:usize=24;
pub const RECORD:usize=288;
pub const BYTES:usize=32+CAPACITY*RECORD;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Language{C,Python}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error{Invalid,Full,Ambiguous,Missing,Owner}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Entry {
    pub name:[u8;48],pub name_len:usize,pub source:[u8;95],pub source_len:usize,
    pub artifact:[u8;95],pub artifact_len:usize,pub source_id:[u8;16],pub artifact_id:[u8;16],
    pub source_version:u32,pub artifact_version:u32,pub language:Language,
}
impl Entry {
    // ------------------------=
    // FUNC: new
    // DESC: Binds a display identity to immutable validated source and executable versions without inferring any AI hooks.
    // ------------------=
    pub fn new(source:&[u8],artifact:&[u8],source_id:[u8;16],artifact_id:[u8;16],source_version:u32,artifact_version:u32,language:Language)->Result<Self,Error>{
        if source.len()>95||artifact.len()>95||!source.starts_with(b"/home/")||!artifact.starts_with(b"/home/")
            ||source_id==[0;16]||artifact_id==[0;16]||source_version==0||artifact_version==0{return Err(Error::Invalid);}
        if !(match language{Language::C=>source.ends_with(b".c")&&artifact.ends_with(b".elf"),Language::Python=>source.ends_with(b".py")&&source==artifact}){return Err(Error::Invalid);}
        let mut entry=Self{name:[0;48],name_len:0,source:[0;95],source_len:source.len(),artifact:[0;95],artifact_len:artifact.len(),source_id,artifact_id,source_version,artifact_version,language};
        entry.source[..source.len()].copy_from_slice(source);entry.artifact[..artifact.len()].copy_from_slice(artifact);
        let file=source.rsplit(|b|*b==b'/').next().ok_or(Error::Invalid)?;
        let mut stem=&file[..file.iter().rposition(|b|*b==b'.').ok_or(Error::Invalid)?];
        if let Some(i)=stem.iter().rposition(|b|*b==b'-'){
            let tail=&stem[i+1..];if tail.len()>=8&&tail.iter().all(|b|b.is_ascii_hexdigit()){stem=&stem[..i];}
        }
        for &b in stem.iter().take(48){entry.name[entry.name_len]=if matches!(b,b'_'|b'-'){b' '}else{b.to_ascii_lowercase()};entry.name_len+=1;}
        if entry.name_len==0||!entry.name[..entry.name_len].iter().all(|b|b.is_ascii_alphanumeric()||matches!(b,b' '|b'.')){return Err(Error::Invalid);}Ok(entry)
    }
    // ------------------------=
    // FUNC: matches
    // DESC: Selects complete name words or an exact source path, never arbitrary substrings or source contents.
    // ------------------=
    pub fn matches(&self,target:&[u8])->bool{
        let target=target.trim_ascii();if target.is_empty(){return false;}
        if target.contains(&b'/'){return target==&self.source[..self.source_len];}
        let target=target.strip_suffix(b" app").or_else(||target.strip_suffix(b" application")).unwrap_or(target);
        let mut count=0;
        for word in target.split(|b|!b.is_ascii_alphanumeric()).filter(|b|!b.is_empty()){
            count+=1;if !self.name[..self.name_len].split(|b|!b.is_ascii_alphanumeric()).any(|part|part.eq_ignore_ascii_case(word)){return false;}
        }count>0
    }
}
#[derive(Clone,Copy)]
pub struct Registry {pub owner:[u8;16],pub entries:[Option<Entry>;CAPACITY]}
impl Registry {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes an empty per-user catalog without implicit registrations or authority.
    // ------------------=
    pub const fn new(owner:[u8;16])->Self{Self{owner,entries:[None;CAPACITY]}}
    // ------------------------=
    // FUNC: register
    // DESC: Replaces only the same source identity, retaining independent applications and rejecting exhausted capacity.
    // ------------------=
    pub fn register(&mut self,owner:[u8;16],entry:Entry)->Result<(),Error>{
        if owner==[0;16]||owner!=self.owner{return Err(Error::Owner);}
        let index=self.entries.iter().position(|e|e.is_some_and(|old|old.source_id==entry.source_id))
            .or_else(||self.entries.iter().position(Option::is_none)).ok_or(Error::Full)?;
        self.entries[index]=Some(entry);Ok(())
    }
    // ------------------------=
    // FUNC: find
    // DESC: Requires one unambiguous user-owned registration for a launch or named source update.
    // ------------------=
    pub fn find(&self,owner:[u8;16],target:&[u8])->Result<Entry,Error>{
        if owner==[0;16]||owner!=self.owner{return Err(Error::Owner);}
        let mut found=None;for entry in self.entries.iter().flatten(){if entry.matches(target){if found.is_some(){return Err(Error::Ambiguous);}found=Some(*entry);}}
        found.ok_or(Error::Missing)
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Serializes bounded versioned object references with no session grants, handles or executable code.
    // ------------------=
    pub fn encode(&self,out:&mut [u8])->Result<usize,Error>{
        if out.len()<BYTES||self.owner==[0;16]{return Err(Error::Invalid);}out[..BYTES].fill(0);out[..8].copy_from_slice(b"INFAPPS1");out[8..24].copy_from_slice(&self.owner);
        for(i,entry)in self.entries.iter().enumerate(){if let Some(e)=entry{let b=&mut out[32+i*RECORD..32+(i+1)*RECORD];
            b[0]=match e.language{Language::C=>1,Language::Python=>2};b[1]=e.name_len as u8;b[2]=e.source_len as u8;b[3]=e.artifact_len as u8;
            b[4..52].copy_from_slice(&e.name);b[52..147].copy_from_slice(&e.source);b[147..242].copy_from_slice(&e.artifact);
            b[242..258].copy_from_slice(&e.source_id);b[258..274].copy_from_slice(&e.artifact_id);
            b[274..278].copy_from_slice(&e.source_version.to_le_bytes());b[278..282].copy_from_slice(&e.artifact_version.to_le_bytes());
        }}Ok(BYTES)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Validates every reference and deterministic display name before trusting an authenticated stored catalog.
    // ------------------=
    pub fn decode(owner:[u8;16],bytes:&[u8])->Result<Self,Error>{
        if owner==[0;16]||bytes.len()!=BYTES||bytes[..8]!=*b"INFAPPS1"||bytes[8..24]!=owner||bytes[24..32].iter().any(|b|*b!=0){return Err(Error::Invalid);}
        let mut registry=Self::new(owner);
        for i in 0..CAPACITY{let b=&bytes[32+i*RECORD..32+(i+1)*RECORD];if b[0]==0{if b.iter().any(|b|*b!=0){return Err(Error::Invalid);}continue;}
            if b[1]==0||b[1]>48||b[2]>95||b[3]>95||b[282..].iter().any(|b|*b!=0){return Err(Error::Invalid);}
            let language=match b[0]{1=>Language::C,2=>Language::Python,_=>return Err(Error::Invalid)};
            let e=Entry::new(&b[52..52+b[2]as usize],&b[147..147+b[3]as usize],b[242..258].try_into().unwrap(),b[258..274].try_into().unwrap(),
                u32::from_le_bytes(b[274..278].try_into().unwrap()),u32::from_le_bytes(b[278..282].try_into().unwrap()),language)?;
            if e.name_len!=b[1]as usize||e.name!=b[4..52]||e.source!=b[52..147]||e.artifact!=b[147..242]{return Err(Error::Invalid);}
            if registry.entries.iter().flatten().any(|old|old.source_id==e.source_id){return Err(Error::Invalid);}registry.entries[i]=Some(e);
        }Ok(registry)
    }
}
