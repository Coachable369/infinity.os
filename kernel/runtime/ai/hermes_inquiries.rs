//! Bounded conversational follow-ups; only the next direct answer may teach a missing fact.
use super::Document;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question { Communication, Interests, Color, Preference }
impl Question {
    // ------------------------=
    // FUNC: key
    // DESC: Identifies the single non-sensitive profile field owned by this question.
    // ------------------=
    pub const fn key(self) -> &'static [u8] {
        match self { Self::Communication=>b"communication_style", Self::Interests=>b"interests",
            Self::Color=>b"favorite_color", Self::Preference=>b"preferences" }
    }
    // ------------------------=
    // FUNC: text
    // DESC: Uses one concise optional question without requesting sensitive personal data.
    // ------------------=
    pub const fn text(self) -> &'static [u8] {
        match self {
            Self::Communication=>b"Would you prefer concise or detailed replies? You can skip this.",
            Self::Interests=>b"What is a hobby or interest you would like me to remember? You can skip this.",
            Self::Color=>b"What is your favorite color? You can skip this.",
            Self::Preference=>b"Is there a preference you would like me to remember? You can skip this.",
        }
    }
}

pub struct Inquiries { turns:u64, last:u64, offered:u8, candidate:Option<Question>, pending:Option<Question> }
impl Inquiries {
    // ------------------------=
    // FUNC: new
    // DESC: Creates account-local question state with no inherited consent or pending answers.
    // ------------------=
    pub const fn new()->Self { Self{turns:0,last:0,offered:0,candidate:None,pending:None} }
    // ------------------------=
    // FUNC: begin
    // DESC: Schedules at most one missing-field follow-up every six admitted casual conversation turns.
    // ------------------=
    pub fn begin(&mut self,document:Option<&Document>,input:&[u8],conversation:bool) {
        self.candidate=None;
        self.pending=None;
        if !conversation {return;}
        self.turns=self.turns.saturating_add(1);
        let Some(document)=document else{return;};
        if !enabled(document) || self.turns<3 || (self.last!=0 && self.turns.saturating_sub(self.last)<6)
            || !casual(input) {return;}
        let mut value=[0u8;1024];
        let order=if super::conversation::relevant(b"favorite_color",input) {
            [Question::Color,Question::Communication,Question::Interests,Question::Preference]
        } else {[Question::Communication,Question::Interests,Question::Color,Question::Preference]};
        self.candidate=order.into_iter().find(|q| self.offered&(1<<(*q as u8))==0 && document.get(q.key(),&mut value)==Ok(0));
    }
    // ------------------------=
    // FUNC: complete
    // DESC: Publishes a question only after verified conversational completion and marks it offered once per session.
    // ------------------=
    pub fn complete(&mut self,document:Option<&Document>,success:bool)->Option<Question> {
        let candidate=self.candidate.take()?;
        let document=document?;
        let mut value=[0;1024];
        if !success || !enabled(document) || document.get(candidate.key(),&mut value)!=Ok(0) {return None;}
        self.last=self.turns;
        self.offered|=1<<(candidate as u8);
        self.pending=Some(candidate);
        Some(candidate)
    }
    // ------------------------=
    // FUNC: answer
    // DESC: Accepts only a short unambiguous next-turn answer; unrelated requests, opt-outs and quotes are never stored.
    // ------------------=
    pub fn answer<'a>(&mut self,document:Option<&Document>,input:&'a[u8])->Option<(&'static[u8],&'a[u8])> {
        let question=self.pending.take()?;
        if !document.is_some_and(enabled) {return None;}
        let input=input.trim_ascii();
        let input=input.strip_suffix(b".").unwrap_or(input);
        if input.is_empty() || input.len()>120 || input.iter().any(|b| matches!(b,b'\n'|b'?'|b'"'|b'='|b';'|b'`'|b':')) {return None;}
        if super::conversation::action(input,true)!=super::conversation::Action::None {return None;}
        let first=input.split(|b| b.is_ascii_whitespace()).next().unwrap_or(input);
        if [b"skip".as_slice(),b"no",b"yes",b"stop",b"forget",b"don't",b"do",b"open",b"launch",b"create",b"build",b"write",b"generate",b"set",b"change",b"update",b"delete",b"run",b"what",b"how",b"why",b"please",b"thanks",b"thank",b"hello",b"hi",b"infinity",b"computer",b"my",b"i",b"tell",b"show",b"fix",b"edit",b"compile",b"restart",b"switch",b"enable",b"disable",b"play",b"search",b"find",b"schedule",b"save",b"is",b"are"].iter().any(|s|first.eq_ignore_ascii_case(s)) {return None;}
        if input.split(|b|b.is_ascii_whitespace()).any(|word| [b"password".as_slice(),b"secret",b"address",b"birthday",b"ssn",b"credit",b"contact"].iter().any(|s|word.eq_ignore_ascii_case(s))) {return None;}
        if matches!(question,Question::Interests|Question::Preference)
            && !input.iter().all(|b| b.is_ascii_alphabetic()||matches!(b,b' '|b'-'|b','|b'\'')) {return None;}
        match question {
            Question::Communication if ![b"concise".as_slice(),b"detailed",b"brief",b"short",b"balanced",b"friendly",b"formal"].iter().any(|s| input.eq_ignore_ascii_case(s))=>None,
            Question::Color if super::conversation::color(input).is_none()=>None,
            _=>Some((question.key(),input)),
        }
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Clears in-flight questions and answer ownership on cancellation without resetting rate limits.
    // ------------------=
    pub fn cancel(&mut self) {self.candidate=None;self.pending=None;}
}
// ------------------------=
// FUNC: enabled
// DESC: Requires both user-controlled learning and inquiry consent.
// ------------------=
fn enabled(document:&Document)->bool {document.enabled(b"learning_enabled")&&document.enabled(b"inquiries_enabled")}
// ------------------------=
// FUNC: casual
// DESC: Restricts unsolicited questions to conversational boundaries rather than operational or sensitive requests.
// ------------------=
fn casual(input:&[u8])->bool {
    let input=input.trim_ascii();
    if input.len()>256 || input.contains(&b'\n') {return false;}
    [b"hello".as_slice(),b"hi",b"thanks",b"thank you",b"how are you",b"let's chat",b"tell me about yourself",b"suggest a desktop",b"recommend a color",b"suggest a color",b"recommend a hobby",b"what do you like",b"what do you think",b"can you recommend",b"can you suggest"].iter().any(|head| {
        input.len()>=head.len()&&input[..head.len()].eq_ignore_ascii_case(head)
            && input.get(head.len()).is_none_or(|b|b.is_ascii_whitespace()||matches!(b,b'!'|b'.'|b'?'|b','))
    })
}
