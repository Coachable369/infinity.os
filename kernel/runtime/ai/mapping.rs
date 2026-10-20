//! Bounded editable AI instruction mappings backed by the native /system namespace.

use super::control::Command;

pub const PATH: &[u8] = b"/system/ai/instruction-mappings.json";
pub const MAX_BYTES: usize = 16 * 1024;
pub const DEFAULT: &[u8] = include_bytes!("../../../system/ai/instruction-mappings.json");
const SCHEMA: &[u8] = b"infinityos.ai-instruction-map";
const ARGUMENT: &[u8] = b"{argument}";
const MAX_ALIASES: usize = 256;
pub const CAPABILITY_TARGETS: [&[u8]; 103] = [
    b"OS_OPEN:text_editor", b"OS_OPEN:file_navigator", b"OS_OPEN:browser", b"OS_OPEN:settings",
    b"OS_OPEN:terminal", b"OS_OPEN:task_manager", b"OS_OPEN:launcher", b"OS_OPEN:ai_instruction_mappings",
    b"OS_LAUNCH:documents", b"OS_LAUNCH:projects", b"OS_LAUNCH:media", b"OS_LAUNCH:recycle_bin",
    b"OS_FOCUS:text_editor", b"OS_FOCUS:file_navigator", b"OS_FOCUS:browser", b"OS_FOCUS:settings",
    b"OS_FOCUS:terminal", b"OS_FOCUS:task_manager", b"OS_FOCUS:launcher", b"OS_CLOSE:active",
    b"OS_CLOSE:text_editor", b"OS_CLOSE:file_navigator", b"OS_CLOSE:browser", b"OS_CLOSE:settings",
    b"OS_CLOSE:terminal", b"OS_CLOSE:task_manager", b"OS_CLOSE:launcher", b"OS_SETTINGS:general",
    b"OS_SETTINGS:themes", b"OS_SETTINGS:accounts", b"OS_SETTINGS:ai_voice", b"OS_SETTINGS:privacy",
    b"OS_SETTINGS:devices", b"OS_SETTINGS:network", b"OS_SETTINGS:nodes_mesh", b"OS_SETTINGS:storage",
    b"OS_SETTINGS:about", b"OS_SETTINGS:input", b"OS_SURFACE:spatial", b"OS_SURFACE:holographic",
    b"OS_SURFACE:world_shift", b"OS_MENU:audio", b"OS_MENU:network", b"OS_MENU:bluetooth",
    b"OS_MENU:power", b"OS_MENU:window", b"OS_MENU:search", b"OS_MENU:account", b"OS_MENU:calendar",
    b"OS_MENU:help", b"OS_SHELL:snap_left", b"OS_SHELL:snap_right", b"OS_SHELL:center",
    b"OS_SHELL:next_window", b"OS_SHELL:grow", b"OS_SHELL:shrink", b"OS_SHELL:focus_lens",
    b"OS_SHELL:peek", b"OS_SHELL:previous_month", b"OS_SHELL:next_month", b"OS_SHELL:today",
    b"OS_SHELL:show_overview", b"OS_SHELL:hide_overview", b"OS_CONTEXT:maximize", b"OS_CONTEXT:restore",
    b"OS_CONTEXT:minimize", b"OS_CONTEXT:search google for {argument}", b"OS_CONTEXT:go to {argument}",
    b"OS_CONTEXT:back", b"OS_CONTEXT:forward", b"OS_CONTEXT:new tab", b"OS_CONTEXT:close tab",
    b"OS_CONTEXT:zoom in", b"OS_CONTEXT:zoom out", b"OS_CONTEXT:refresh",
    b"OS_CONTEXT:find file {argument}", b"OS_CONTEXT:open folder {argument}", b"OS_CONTEXT:list view",
    b"OS_CONTEXT:grid view", b"OS_CONTEXT:find {argument}", b"OS_CONTEXT:insert {argument}", b"OS_CONTEXT:undo",
    b"OS_CONTEXT:redo", b"OS_CONTEXT:select all", b"OS_CONTEXT:save file", b"OS_CONTEXT:clear text",
    b"OS_CONTEXT:save and compile my code", b"OS_CONTEXT:compile my code", b"OS_CONTEXT:type {argument}",
    b"OS_CONTEXT:next task", b"OS_CONTEXT:previous task", b"OS_CONTEXT:next theme",
    b"OS_CONTEXT:next icon set", b"OS_CONTEXT:next model", b"OS_CONTEXT:search apps {argument}",
    b"OS_CONTEXT:next item", b"OS_CONTEXT:previous item", b"OS_CONTEXT:open selected",
    b"OS_CONTEXT:add idea", b"OS_CONTEXT:new category",
    b"OS_CLOCK:read",
    b"OS_CLOCK:set:{argument}",
    b"OS_CLOCK:zone:{argument}",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidJson,
    UnsupportedSchema,
    UnsupportedVersion,
    MissingField,
    InvalidIdentifier,
    InvalidScope,
    InvalidTarget,
    InvalidAlias,
    DuplicateAlias,
    DuplicateTarget,
    MissingCapability,
    Capacity,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates one bounded JSON cursor over immutable UTF-8-compatible bytes.
    // ------------------=
    fn new(bytes: &'a [u8]) -> Self { Self { bytes, at: 0 } }

    // ------------------------=
    // FUNC: whitespace
    // DESC: Advances over JSON whitespace without accepting non-ASCII control data.
    // ------------------=
    fn whitespace(&mut self) {
        while self.bytes.get(self.at).is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t')) { self.at += 1; }
    }

    // ------------------------=
    // FUNC: take
    // DESC: Consumes one required punctuation byte after bounded whitespace.
    // ------------------=
    fn take(&mut self, expected: u8) -> Result<(), Error> {
        self.whitespace();
        if self.bytes.get(self.at).copied() != Some(expected) { return Err(Error::InvalidJson); }
        self.at += 1;
        Ok(())
    }

    // ------------------------=
    // FUNC: string
    // DESC: Reads one plain printable JSON string; escapes are deliberately excluded from the editable command format.
    // ------------------=
    fn string(&mut self) -> Result<&'a [u8], Error> {
        self.take(b'"')?;
        let start = self.at;
        while let Some(byte) = self.bytes.get(self.at).copied() {
            if byte == b'"' {
                let value = &self.bytes[start..self.at];
                self.at += 1;
                return Ok(value);
            }
            if byte == b'\\' || !(0x20..=0x7e).contains(&byte) { return Err(Error::InvalidJson); }
            self.at += 1;
        }
        Err(Error::InvalidJson)
    }

    // ------------------------=
    // FUNC: number
    // DESC: Reads one non-negative schema integer with overflow rejection.
    // ------------------=
    fn number(&mut self) -> Result<u32, Error> {
        self.whitespace();
        let mut value = 0u32;
        let start = self.at;
        while let Some(byte @ b'0'..=b'9') = self.bytes.get(self.at).copied() {
            value = value.checked_mul(10).and_then(|n| n.checked_add((byte-b'0') as u32)).ok_or(Error::InvalidJson)?;
            self.at += 1;
        }
        if self.at == start { return Err(Error::InvalidJson); }
        Ok(value)
    }

    // ------------------------=
    // FUNC: skip
    // DESC: Skips one unknown JSON value while retaining strict syntax and bounded nesting.
    // ------------------=
    fn skip(&mut self, depth: usize) -> Result<(), Error> {
        if depth > 8 { return Err(Error::Capacity); }
        self.whitespace();
        match self.bytes.get(self.at).copied() {
            Some(b'"') => { self.string()?; Ok(()) },
            Some(b'{') => {
                self.at += 1; self.whitespace();
                if self.bytes.get(self.at) == Some(&b'}') { self.at += 1; return Ok(()); }
                loop {
                    self.string()?; self.take(b':')?; self.skip(depth+1)?; self.whitespace();
                    match self.bytes.get(self.at).copied() { Some(b',') => self.at += 1, Some(b'}') => {self.at += 1; return Ok(());}, _ => return Err(Error::InvalidJson) }
                }
            },
            Some(b'[') => {
                self.at += 1; self.whitespace();
                if self.bytes.get(self.at) == Some(&b']') { self.at += 1; return Ok(()); }
                loop {
                    self.skip(depth+1)?; self.whitespace();
                    match self.bytes.get(self.at).copied() { Some(b',') => self.at += 1, Some(b']') => {self.at += 1; return Ok(());}, _ => return Err(Error::InvalidJson) }
                }
            },
            Some(b't') if self.bytes.get(self.at..self.at+4) == Some(b"true") => {self.at += 4; Ok(())},
            Some(b'f') if self.bytes.get(self.at..self.at+5) == Some(b"false") => {self.at += 5; Ok(())},
            Some(b'n') if self.bytes.get(self.at..self.at+4) == Some(b"null") => {self.at += 4; Ok(())},
            Some(b'0'..=b'9') => self.number().map(|_|()),
            _ => Err(Error::InvalidJson),
        }
    }
}

#[derive(Clone, Copy)]
struct AliasMatch {
    argument_start: usize,
    argument_end: usize,
    dynamic: bool,
}

// ------------------------=
// FUNC: trim_instruction
// DESC: Removes transcript whitespace and one sentence terminator without changing argument bytes.
// ------------------=
fn trim_instruction(mut bytes: &[u8]) -> &[u8] {
    bytes = bytes.trim_ascii();
    if bytes.last().is_some_and(|byte| matches!(byte, b'.' | b'?' | b'!')) { bytes = bytes[..bytes.len()-1].trim_ascii_end(); }
    bytes
}

// ------------------------=
// FUNC: normalized_equal
// DESC: Compares exact aliases case-insensitively while treating repeated transcript whitespace as one space.
// ------------------=
fn normalized_equal(left: &[u8], right: &[u8]) -> bool {
    let left = trim_instruction(left);
    let right = trim_instruction(right);
    let mut a = 0usize;
    let mut b = 0usize;
    loop {
        while left.get(a).is_some_and(|byte| byte.is_ascii_whitespace()) { a += 1; }
        while right.get(b).is_some_and(|byte| byte.is_ascii_whitespace()) { b += 1; }
        if a == left.len() || b == right.len() { return a == left.len() && b == right.len(); }
        while a < left.len() && b < right.len() && !left[a].is_ascii_whitespace() && !right[b].is_ascii_whitespace() {
            if !left[a].eq_ignore_ascii_case(&right[b]) { return false; }
            a += 1; b += 1;
        }
        if a < left.len() && !left[a].is_ascii_whitespace() { return false; }
        if b < right.len() && !right[b].is_ascii_whitespace() { return false; }
    }
}

// ------------------------=
// FUNC: match_alias
// DESC: Matches an exact alias or one trailing argument placeholder and returns only user-supplied argument bounds.
// ------------------=
fn match_alias(alias: &[u8], input: &[u8]) -> Option<AliasMatch> {
    let input = trim_instruction(input);
    if let Some(prefix) = alias.strip_suffix(ARGUMENT) {
        let prefix = prefix.trim_ascii_end();
        if input.len() <= prefix.len() || !input.get(..prefix.len()).is_some_and(|value| value.eq_ignore_ascii_case(prefix))
            || !input[prefix.len()].is_ascii_whitespace() { return None; }
        let argument = input[prefix.len()..].trim_ascii();
        if argument.is_empty() { return None; }
        let start = argument.as_ptr() as usize - input.as_ptr() as usize;
        return Some(AliasMatch { argument_start:start, argument_end:start+argument.len(), dynamic:true });
    }
    normalized_equal(alias,input).then_some(AliasMatch{argument_start:0,argument_end:0,dynamic:false})
}

// ------------------------=
// FUNC: alias_hash
// DESC: Produces a stable case-folded collision-conservative alias key for ambiguity rejection.
// ------------------=
fn alias_hash(alias: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in alias.trim_ascii() { hash = (hash ^ byte.to_ascii_lowercase() as u64).wrapping_mul(0x100000001b3); }
    hash
}

// ------------------------=
// FUNC: identifier_valid
// DESC: Restricts action identifiers to readable dotted lower-camel tokens.
// ------------------=
fn identifier_valid(id: &[u8]) -> bool {
    !id.is_empty() && id.len() <= 72 && id[0].is_ascii_lowercase()
        && id.iter().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte,b'.'|b'_'))
        && !id.windows(2).any(|pair| pair == b"..")
}

// ------------------------=
// FUNC: command_for_target
// DESC: Expands only the declared trailing argument slot and decodes the result through the typed control allowlist.
// ------------------=
fn command_for_target(target: &[u8], matched: AliasMatch, input: &[u8]) -> Result<Command, Error> {
    if let Some(prefix) = target.strip_suffix(ARGUMENT) {
        if !matched.dynamic { return Err(Error::InvalidTarget); }
        let argument = &trim_instruction(input)[matched.argument_start..matched.argument_end];
        if prefix.len()+argument.len() > 256 { return Err(Error::Capacity); }
        let mut expanded = [0u8;256];
        expanded[..prefix.len()].copy_from_slice(prefix);
        expanded[prefix.len()..prefix.len()+argument.len()].copy_from_slice(argument);
        return Command::decode(&expanded[..prefix.len()+argument.len()]).ok_or(Error::InvalidTarget);
    }
    if matched.dynamic { return Err(Error::InvalidTarget); }
    Command::decode(target).ok_or(Error::InvalidTarget)
}

// ------------------------=
// FUNC: parse_action
// DESC: Validates one action inventory entry and resolves at most one matching alias.
// ------------------=
fn parse_action<'a>(cursor: &mut Cursor<'a>, input: Option<&[u8]>, hashes: &mut [u64;MAX_ALIASES],
    alias_count: &mut usize, covered: &mut [bool;CAPABILITY_TARGETS.len()]) -> Result<Option<Command>,Error> {
    cursor.take(b'{')?;
    let mut id=None;
    let mut scope=None;
    let mut target=None;
    let mut matched=None;
    let mut aliases=0usize;
    loop {
        cursor.whitespace();
        if cursor.bytes.get(cursor.at) == Some(&b'}') { cursor.at += 1; break; }
        let key=cursor.string()?; cursor.take(b':')?;
        match key {
            b"id" => id=Some(cursor.string()?),
            b"scope" => scope=Some(cursor.string()?),
            b"target" => target=Some(cursor.string()?),
            b"aliases" => {
                cursor.take(b'[')?; cursor.whitespace();
                if cursor.bytes.get(cursor.at)==Some(&b']') {cursor.at+=1;} else {loop {
                    let alias=cursor.string()?;
                    if alias.is_empty() || alias.len()>128 || alias.windows(ARGUMENT.len()).filter(|part|*part==ARGUMENT).count()>1
                        || (alias.contains(&b'{') && !alias.ends_with(ARGUMENT)) {return Err(Error::InvalidAlias);}
                    if *alias_count>=hashes.len() {return Err(Error::Capacity);}
                    let hash=alias_hash(alias);
                    if hashes[..*alias_count].contains(&hash) {return Err(Error::DuplicateAlias);}
                    hashes[*alias_count]=hash;*alias_count+=1;aliases+=1;
                    if matched.is_none() {if let Some(value)=input.and_then(|text|match_alias(alias,text)){matched=Some(value);}}
                    cursor.whitespace();
                    match cursor.bytes.get(cursor.at).copied(){Some(b',')=>cursor.at+=1,Some(b']')=>{cursor.at+=1;break;},_=>return Err(Error::InvalidJson)}
                }}
            },
            _ => cursor.skip(1)?,
        }
        cursor.whitespace();
        match cursor.bytes.get(cursor.at).copied(){Some(b',')=>cursor.at+=1,Some(b'}')=>{cursor.at+=1;break;},_=>return Err(Error::InvalidJson)}
    }
    let id=id.ok_or(Error::MissingField)?;
    if !identifier_valid(id){return Err(Error::InvalidIdentifier);}
    if !matches!(scope.ok_or(Error::MissingField)?,b"os"|b"application"){return Err(Error::InvalidScope);}
    if aliases==0{return Err(Error::MissingField);}
    let target=target.ok_or(Error::MissingField)?;
    let target_index=CAPABILITY_TARGETS.iter().position(|candidate|*candidate==target).ok_or(Error::InvalidTarget)?;
    if covered[target_index] {return Err(Error::DuplicateTarget);}
    covered[target_index]=true;
    let dynamic=target.ends_with(ARGUMENT);
    if target.contains(&b'{') && !dynamic{return Err(Error::InvalidTarget);}
    let probe=AliasMatch{argument_start:0,argument_end:1,dynamic};
    if dynamic {
        let mut sample=[b'x';1];
        command_for_target(target,probe,&mut sample)?;
    } else { command_for_target(target,probe,b"")?; }
    matched.map(|value|command_for_target(target,value,input.unwrap_or(b""))).transpose()
}

// ------------------------=
// FUNC: walk
// DESC: Validates the complete schema before returning a resolved typed command.
// ------------------=
fn walk(bytes:&[u8],input:Option<&[u8]>)->Result<Option<Command>,Error>{
    if bytes.is_empty()||bytes.len()>MAX_BYTES{return Err(Error::Capacity);}
    let mut cursor=Cursor::new(bytes);cursor.take(b'{')?;
    let mut schema=false;let mut version=false;let mut actions=false;let mut resolved=None;
    let mut hashes=[0u64;MAX_ALIASES];let mut alias_count=0usize;
    let mut covered=[false;CAPABILITY_TARGETS.len()];
    loop{
        cursor.whitespace();if cursor.bytes.get(cursor.at)==Some(&b'}'){cursor.at+=1;break;}
        let key=cursor.string()?;cursor.take(b':')?;
        match key{
            b"schema"=>{schema=cursor.string()?==SCHEMA;if !schema{return Err(Error::UnsupportedSchema);}},
            b"version"=>{version=cursor.number()?==1;if !version{return Err(Error::UnsupportedVersion);}},
            b"actions"=>{actions=true;cursor.take(b'[')?;cursor.whitespace();
                if cursor.bytes.get(cursor.at)==Some(&b']'){cursor.at+=1;}else{loop{
                    let command=parse_action(&mut cursor,input,&mut hashes,&mut alias_count,&mut covered)?;
                    if resolved.is_none(){resolved=command;}
                    cursor.whitespace();match cursor.bytes.get(cursor.at).copied(){Some(b',')=>cursor.at+=1,Some(b']')=>{cursor.at+=1;break;},_=>return Err(Error::InvalidJson)}
                }}},
            _=>cursor.skip(1)?,
        }
        cursor.whitespace();match cursor.bytes.get(cursor.at).copied(){Some(b',')=>cursor.at+=1,Some(b'}')=>{cursor.at+=1;break;},_=>return Err(Error::InvalidJson)}
    }
    cursor.whitespace();if cursor.at!=bytes.len(){return Err(Error::InvalidJson);}
    if !schema||!version||!actions||alias_count==0{return Err(Error::MissingField);}
    if covered.iter().any(|present|!*present){return Err(Error::MissingCapability);}
    Ok(resolved)
}

// ------------------------=
// FUNC: validate
// DESC: Validates an edited mapping as complete, unambiguous JSON containing only typed implemented targets.
// ------------------=
pub fn validate(bytes:&[u8])->Result<(),Error>{walk(bytes,None).map(|_|())}

// ------------------------=
// FUNC: resolve
// DESC: Resolves one voice or chat instruction from the current editable mapping without granting arbitrary execution.
// ------------------=
pub fn resolve(bytes:&[u8],input:&[u8])->Result<Option<Command>,Error>{walk(bytes,Some(input))}

pub struct ConfiguredMap{bytes:[u8;MAX_BYTES],length:usize}

impl ConfiguredMap{
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty cache which will load the installed mapping before first use.
    // ------------------=
    pub const fn new()->Self{Self{bytes:[0;MAX_BYTES],length:0}}

    // ------------------------=
    // FUNC: refresh
    // DESC: Reloads the editable System object and falls back to the build-time mapping only when storage is unavailable or invalid.
    // ------------------=
    pub fn refresh(&mut self){
        #[cfg(target_os="none")]
        let loaded=crate::storage::ai_instruction_mapping_load(&mut self.bytes).ok()
            .filter(|length|validate(&self.bytes[..*length]).is_ok());
        #[cfg(not(target_os="none"))]
        let loaded:Option<usize>=None;
        self.length=if let Some(length)=loaded{length}else{
            self.bytes[..DEFAULT.len()].copy_from_slice(DEFAULT);DEFAULT.len()
        };
    }

    // ------------------------=
    // FUNC: resolve
    // DESC: Resolves against the latest validated installed mapping.
    // ------------------=
    pub fn resolve(&self,input:&[u8])->Option<Command>{resolve(&self.bytes[..self.length],input).ok().flatten()}
}
