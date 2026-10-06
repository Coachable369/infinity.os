//! Bounded native workplace workflows. No OS services, allocation, or UI dependency.
#![no_std]
use sha2::{Digest,Sha256};
#[path="../../kernel/ui/text_paste.rs"]
mod text_input;
pub const INPUT:usize=512;
pub const HISTORY:usize=32;
pub const CONTENT:usize=16384;

#[derive(Clone,Copy)]
pub struct Entry {pub bytes:[u8;INPUT],pub length:usize}
impl Entry {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates a fully erased history or draft entry.
    // ------------------=
    pub const fn empty()->Self {Self{bytes:[0;INPUT],length:0}}
    // ------------------------=
    // FUNC: text
    // DESC: Returns only initialized command bytes.
    // ------------------=
    pub fn text(&self)->&[u8] {&self.bytes[..self.length]}
}
pub struct History {entries:[Entry;HISTORY],count:usize,position:usize,draft:Entry,private:bool,search_query:Entry,search_before:usize}
impl History {
    // ------------------------=
    // FUNC: new
    // DESC: Creates bounded, memory-only history for one authenticated session.
    // ------------------=
    pub const fn new()->Self {Self{entries:[Entry::empty();HISTORY],count:0,position:0,draft:Entry::empty(),private:false,search_query:Entry::empty(),search_before:HISTORY}}
    // ------------------------=
    // FUNC: clear
    // DESC: Erases command bytes and the unsent draft without changing privacy preference.
    // ------------------=
    pub fn clear(&mut self) {self.entries.fill(Entry::empty());self.count=0;self.position=0;self.draft=Entry::empty();self.search_query=Entry::empty();self.search_before=HISTORY;}
    // ------------------------=
    // FUNC: set_private
    // DESC: Entering private mode immediately destroys retained commands and disables new retention.
    // ------------------=
    pub fn set_private(&mut self,value:bool) {self.clear();self.private=value;}
    // ------------------------=
    // FUNC: private
    // DESC: Exposes the command retention preference without command contents.
    // ------------------=
    pub fn private(&self)->bool {self.private}
    // ------------------------=
    // FUNC: count
    // DESC: Returns retained entry count without exposing command data to diagnostics.
    // ------------------=
    pub fn count(&self)->usize {self.count}
    // ------------------------=
    // FUNC: record
    // DESC: Retains non-sensitive commands, deduplicates adjacent entries, and bounds memory usage.
    // ------------------=
    pub fn record(&mut self,text:&[u8]) {
        self.position=self.count;self.draft=Entry::empty();
        self.search_query=Entry::empty();self.search_before=HISTORY;
        if self.private || text.is_empty() || text.len()>INPUT || sensitive(text) || text.starts_with(b"work history ") {return;}
        if self.count>0 && self.entries[self.count-1].text()==text {return;}
        if self.count==HISTORY {self.entries.copy_within(1..HISTORY,0);self.count-=1;}
        let entry=&mut self.entries[self.count];*entry=Entry::empty();entry.bytes[..text.len()].copy_from_slice(text);entry.length=text.len();
        self.count+=1;self.position=self.count;
    }
    // ------------------------=
    // FUNC: recall
    // DESC: Moves through history and restores the exact unfinished draft after the newest entry.
    // ------------------=
    pub fn recall(&mut self,older:bool,draft:&[u8])->Option<Entry> {
        if self.private || self.count==0 || draft.len()>INPUT {return None;}
        if self.position==self.count && older {
            self.draft=Entry::empty();self.draft.bytes[..draft.len()].copy_from_slice(draft);self.draft.length=draft.len();
        }
        if older {self.position=self.position.saturating_sub(1);}
        else {self.position=(self.position+1).min(self.count);}
        Some(if self.position==self.count {self.draft}else{self.entries[self.position]})
    }
    // ------------------------=
    // FUNC: search
    // DESC: Finds newest matching command before a caller-supplied offset without executing it.
    // ------------------=
    pub fn search(&self,query:&[u8],before:usize)->Option<(usize,Entry)> {
        if self.private {return None;}
        (0..before.min(self.count)).rev().find(|&i|query.is_empty() || self.entries[i].text().windows(query.len()).any(|v|v==query))
            .map(|i|(i,self.entries[i]))
    }
    // ------------------------=
    // FUNC: reverse_search
    // DESC: Cycles older matching drafts on repeated Ctrl/Command-R without executing or truncating a result.
    // ------------------=
    pub fn reverse_search(&mut self,draft:&[u8])->Option<Entry> {
        if self.private || draft.len()>INPUT {return None;}
        if self.search_before>=self.count || self.entries[self.search_before].text()!=draft {
            self.search_query=Entry::empty();self.search_query.bytes[..draft.len()].copy_from_slice(draft);self.search_query.length=draft.len();self.search_before=self.count;
        }
        let (index,entry)=self.search(self.search_query.text(),self.search_before)?;
        self.search_before=index;Some(entry)
    }
}

// ------------------------=
// FUNC: sensitive
// DESC: Excludes credential-bearing command families and explicit secret arguments from echo and history.
// ------------------=
pub fn sensitive(text:&[u8])->bool {
    let words=text.split(|b|b.is_ascii_whitespace() || *b==b'|' || *b==b';');
    for word in words {
        for key in [b"password".as_slice(),b"secret",b"token",b"credential",b"credential-create",b"login",b"pair",b"pairing",b"pin",b"key"] {
            if word.eq_ignore_ascii_case(key) || word.get(..key.len()+1).is_some_and(|v|v[..key.len()].eq_ignore_ascii_case(key) && v[key.len()]==b'=') {return true;}
        }
    }
    false
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Error {Capacity,UnsafeText,Invalid,Storage,Integrity,Conflict}

pub const COMMANDS:&[&[u8]]=&[b"work help",b"work clipboard clear",b"work clipboard status",b"work clipboard ttl ",
    b"work clipboard browser-read ",b"work clipboard browser-write ",b"work history find ",b"work history clear",
    b"work private on",b"work private off",b"work checksum ",b"work compare ",b"work backup ",b"work restore ",b"work report "];
pub struct Words {bytes:[u8;INPUT],ranges:[(usize,usize);8],count:usize}
impl Words {
    // ------------------------=
    // FUNC: parse
    // DESC: Tokenizes bounded quoted arguments and escapes, rejecting incomplete quotes and extra arguments.
    // ------------------=
    pub fn parse(input:&[u8])->Result<Self,Error> {
        if input.len()>INPUT {return Err(Error::Capacity);}
        let mut out=Self{bytes:[0;INPUT],ranges:[(0,0);8],count:0};
        let(mut i,mut n)=(0,0);
        while i<input.len() {
            if input[i].is_ascii_whitespace() {i+=1;continue;}
            if out.count==8 {return Err(Error::Capacity);}
            let start=n;let mut quote=0;
            while i<input.len() {
                let b=input[i];
                if quote==0 && b.is_ascii_whitespace() {break;}
                if b==b'\\' && quote!=b'\'' {
                    i+=1;if i==input.len() {return Err(Error::Invalid);}
                    out.bytes[n]=input[i];n+=1;
                } else if matches!(b,b'\''|b'"') && (quote==0 || quote==b) {quote=if quote==0 {b}else{0};}
                else {out.bytes[n]=b;n+=1;}
                i+=1;
            }
            if quote!=0 {return Err(Error::Invalid);}
            out.ranges[out.count]=(start,n);out.count+=1;
        }Ok(out)
    }
    // ------------------------=
    // FUNC: get
    // DESC: Returns one complete decoded argument without allocation.
    // ------------------=
    pub fn get(&self,index:usize)->Option<&[u8]> {if index>=self.count {None}else{let(a,b)=self.ranges[index];Some(&self.bytes[a..b])}}
    // ------------------------=
    // FUNC: len
    // DESC: Reports exact arity for rejecting surplus command arguments.
    // ------------------=
    pub fn len(&self)->usize {self.count}
}
// ------------------------=
// FUNC: paste_line
// DESC: Inserts a complete single-line ASCII paste atomically; newline, escape, NUL and overflow never mutate input.
// ------------------=
pub fn paste_line<const N:usize>(buffer:&mut [u8;N],length:&mut usize,caret:&mut usize,text:&[u8])->Result<(),Error> {
    if text.iter().any(|b|!(32..=126).contains(b)) {return Err(Error::UnsafeText);}
    if *length>N || text.len()>N-*length {return Err(Error::Capacity);}
    if !text_input::paste_ascii(buffer,length,caret,text) {return Err(Error::Invalid);}Ok(())
}

pub struct Completion {prefix:Entry,common:Entry,pub matches:usize}
impl Completion {
    // ------------------------=
    // FUNC: new
    // DESC: Starts an exact-prefix completion query without retaining unrelated commands.
    // ------------------=
    pub fn new(prefix:&[u8])->Self {
        let mut p=Entry::empty();let n=prefix.len().min(INPUT);p.bytes[..n].copy_from_slice(&prefix[..n]);p.length=n;
        Self{prefix:p,common:Entry::empty(),matches:0}
    }
    // ------------------------=
    // FUNC: offer
    // DESC: Computes the longest common prefix of matching registered commands.
    // ------------------=
    pub fn offer(&mut self,candidate:&[u8]) {
        if candidate.len()>INPUT || !candidate.starts_with(self.prefix.text()) {return;}
        if self.matches==0 {self.common.bytes[..candidate.len()].copy_from_slice(candidate);self.common.length=candidate.len();}
        else {self.common.length=self.common.text().iter().zip(candidate).take_while(|(a,b)|a==b).count();}
        self.matches+=1;
    }
    // ------------------------=
    // FUNC: result
    // DESC: Returns a non-destructive completion only when at least one candidate matched.
    // ------------------=
    pub fn result(&self)->Option<Entry> {(self.matches>0).then_some(self.common)}
}

// ------------------------=
// FUNC: checksum
// DESC: Computes the standard SHA-256 digest over exact file bytes.
// ------------------=
pub fn checksum(bytes:&[u8])->[u8;32] {Sha256::digest(bytes).into()}
// ------------------------=
// FUNC: parse_digest
// DESC: Parses exactly 64 hexadecimal digits without partial or lossy acceptance.
// ------------------=
pub fn parse_digest(text:&[u8])->Result<[u8;32],Error> {
    if text.len()!=64 {return Err(Error::Invalid);}
    let mut out=[0;32];
    for (i,b) in text.iter().enumerate() {
        let n=match b {b'0'..=b'9'=>b-b'0',b'a'..=b'f'=>b-b'a'+10,b'A'..=b'F'=>b-b'A'+10,_=>return Err(Error::Invalid)};
        out[i/2]|=n<<if i%2==0 {4}else{0};
    }Ok(out)
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Comparison {pub same:bool,pub left:usize,pub right:usize,pub first_difference:Option<usize>}
// ------------------------=
// FUNC: compare
// DESC: Reports exact byte equality, lengths, and the first differing or missing byte.
// ------------------=
pub fn compare(left:&[u8],right:&[u8])->Comparison {
    let at=left.iter().zip(right).position(|(a,b)|a!=b).or_else(||(left.len()!=right.len()).then_some(left.len().min(right.len())));
    Comparison{same:at.is_none(),left:left.len(),right:right.len(),first_difference:at}
}

/// The native adapter must fail on truncation, overwrite conflicts and unauthorized paths.
pub trait Files {
    // ------------------------=
    // FUNC: read
    // DESC: Reads one complete regular object, refusing containers and oversized content.
    // ------------------=
    fn read(&mut self,path:&[u8],out:&mut [u8])->Result<usize,Error>;
    // ------------------------=
    // FUNC: copy
    // DESC: Atomically creates an independent object at an unoccupied destination.
    // ------------------=
    fn copy(&mut self,source:&[u8],destination:&[u8])->Result<(),Error>;
    // ------------------------=
    // FUNC: write
    // DESC: Commits one atomic content version while retaining the previous version for rollback.
    // ------------------=
    fn write(&mut self,path:&[u8],bytes:&[u8])->Result<u32,Error>;
}
// ------------------------=
// FUNC: backup
// DESC: Verifies an independent backup against the source digest and returns that digest for later restore.
// ------------------=
pub fn backup(files:&mut impl Files,source:&[u8],destination:&[u8])->Result<[u8;32],Error> {
    let mut bytes=[0;CONTENT];let n=files.read(source,&mut bytes)?;let hash=checksum(&bytes[..n]);
    files.copy(source,destination)?;let n=files.read(destination,&mut bytes)?;
    if checksum(&bytes[..n])!=hash {return Err(Error::Integrity);}Ok(hash)
}
// ------------------------=
// FUNC: restore
// DESC: Checks a caller-retained backup digest before atomically restoring an existing target and verifying its bytes.
// ------------------=
pub fn restore(files:&mut impl Files,backup:&[u8],target:&[u8],expected:[u8;32])->Result<u32,Error> {
    if backup==target {return Err(Error::Conflict);}
    let mut bytes=[0;CONTENT];let n=files.read(backup,&mut bytes)?;
    if checksum(&bytes[..n])!=expected {return Err(Error::Integrity);}
    let mut old=[0;CONTENT];let _=files.read(target,&mut old)?;
    let version=files.write(target,&bytes[..n])?;
    let n=files.read(target,&mut old)?;
    if checksum(&old[..n])!=expected {return Err(Error::Integrity);}Ok(version)
}

/// Deliberate allowlist: no usernames, device serials, addresses, prompts, paths, secrets or clipboard data.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SupportReport {pub architecture:u8,pub installed:bool,pub width:u32,pub height:u32,
    pub ready_devices:u32,pub storage_used:Option<u32>,pub storage_total:Option<u32>,pub tasks:Option<u32>,
    pub browser_state:Option<u32>,pub browser_error:Option<u32>}
impl SupportReport {
    // ------------------------=
    // FUNC: json
    // DESC: Formats the allowlisted numeric support record as UTF-8 JSON with explicit nulls for unavailable probes.
    // ------------------=
    pub fn json(&self,out:&mut [u8])->Result<usize,Error> {
        use core::fmt::Write;
        let mut writer=ReportWriter{bytes:out,length:0};
        write!(&mut writer,"{{\"format\":1,\"architecture\":{},\"installed\":{},\"width\":{},\"height\":{},\"ready_devices\":{}",
            self.architecture,self.installed,self.width,self.height,self.ready_devices).map_err(|_|Error::Capacity)?;
        for (name,value) in [("storage_used_blocks",self.storage_used),("storage_total_blocks",self.storage_total),
            ("tasks",self.tasks),("browser_state",self.browser_state),("browser_error",self.browser_error)] {
            write!(&mut writer,",\"{}\":",name).map_err(|_|Error::Capacity)?;
            match value {Some(v)=>write!(&mut writer,"{}",v),None=>writer.write_str("null")}.map_err(|_|Error::Capacity)?;
        }
        writer.write_str("}\n").map_err(|_|Error::Capacity)?;Ok(writer.length)
    }
}

struct ReportWriter<'a>{bytes:&'a mut [u8],length:usize}
impl core::fmt::Write for ReportWriter<'_> {
    // ------------------------=
    // FUNC: write_str
    // DESC: Appends one bounded report fragment and refuses truncation.
    // ------------------=
    fn write_str(&mut self,text:&str)->core::fmt::Result {
        if text.len()>self.bytes.len()-self.length {return Err(core::fmt::Error);}
        self.bytes[self.length..self.length+text.len()].copy_from_slice(text.as_bytes());self.length+=text.len();Ok(())
    }
}

#[cfg(test)] extern crate std;
#[cfg(test)] #[path="../../tools/storage.rs"] mod storage;
#[cfg(test)] #[path="../../kernel/ui/clipboard.rs"] mod clipboard;
#[cfg(test)] mod tests;
