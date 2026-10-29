//! Conservative conversational memory intents. Never learn from model output, quotations, or hypothetical text.
#[derive(Debug,PartialEq,Eq)]
pub enum Action<'a>{Inspect,Set(&'static[u8],&'a[u8]),Fact(&'a[u8], &'a[u8]),Forget(&'static[u8]),None}
// ------------------------=
// FUNC: prefix
// DESC: Matches ASCII command grammar without changing the user's stored spelling.
// ------------------=
fn prefix<'a>(input:&'a[u8],head:&[u8])->Option<&'a[u8]>{
    (input.len()>=head.len()&&input[..head.len()].eq_ignore_ascii_case(head)).then(||input[head.len()..].trim_ascii())
}
// ------------------------=
// FUNC: field
// DESC: Maps spoken labels to one canonical field so corrections replace facts instead of appending contradictions.
// ------------------=
pub fn field(name:&[u8])->Option<&'static[u8]>{
    for (label,key) in [
        (b"name".as_slice(),b"user_name".as_slice()),(b"address",b"address"),(b"time",b"time"),
        (b"timezone",b"timezone"),(b"time zone",b"timezone"),(b"timrzone",b"timezone"),
        (b"favorite color",b"favorite_color"),(b"favourite colour",b"favorite_color"),
        (b"favorite colour",b"favorite_color"),(b"favourite color",b"favorite_color"),
        (b"birth date",b"birth_date"),(b"birthday",b"birth_date"),
        (b"preferences",b"preferences"),(b"instructions",b"user_instructions"),
        (b"personality",b"personality_style"),(b"attitude",b"attitude"),(b"emergency contact",b"emergency_contact"),
        (b"interests",b"interests"),(b"communication style",b"communication_style"),(b"projects",b"recurring_projects"),
        (b"dislikes",b"dislikes"),(b"assumptions",b"inferred_interests"),
    ]{if name.eq_ignore_ascii_case(label){return Some(key);}}
    super::FIELDS.into_iter().find(|key|name.eq_ignore_ascii_case(key))
}
// ------------------------=
// FUNC: action
// DESC: Parses explicit inspection, correction, deletion, and consent controls; optional learning accepts direct first-person assertions only.
// ------------------=
pub fn action(input:&[u8],learn:bool)->Action<'_>{
    let mut input=input.trim_ascii();
    input=prefix(input,b"Infinity,").or_else(||prefix(input,b"Infinity ")).unwrap_or(input);
    input=input.trim_ascii_end();
    let question=input.ends_with(b"?");
    input=input.strip_suffix(b"?").or_else(||input.strip_suffix(b".")).unwrap_or(input);
    if let Some((key,value))=super::edit_command(input){if let Some(key)=field(key){return Action::Set(key,value);}}
    if [b"what do you remember about me".as_slice(),b"show my memory",b"what do you know about me"].iter().any(|s|input.eq_ignore_ascii_case(s)){return Action::Inspect;}
    if question{return Action::None;}
    for (phrase,key,value) in [
        (b"stop remembering".as_slice(),b"learning_enabled".as_slice(),b"false".as_slice()),
        (b"start remembering",b"learning_enabled",b"true"),
        (b"stop asking personal questions",b"inquiries_enabled",b"false"),
        (b"ask me personal questions",b"inquiries_enabled",b"true"),
        (b"stop inferring",b"inference_enabled",b"false"),
        (b"allow inferences",b"inference_enabled",b"true"),
    ]{if input.eq_ignore_ascii_case(phrase){return Action::Set(key,value);}}
    for head in [b"forget my ".as_slice(),b"delete my ",b"forget "]{
        if let Some(label)=prefix(input,head){return field(label).map(Action::Forget).unwrap_or(Action::None);}
    }
    for head in [b"change my ".as_slice(),b"update my ",b"set my ",b"change ",b"update ",b"set "]{
        if let Some(rest)=prefix(input,head){
            if let Some(at)=rest.windows(4).position(|w|w.eq_ignore_ascii_case(b" to ")){
                if let Some(key)=field(&rest[..at]){return Action::Set(key,rest[at+4..].trim_ascii());}
            }
        }
    }
    let explicit=prefix(input,b"remember ");
    let input=explicit.unwrap_or(input);
    if let Some(equal)=input.iter().position(|b|*b==b'='){
        if explicit.is_some(){if let Some(key)=field(&input[..equal]){return Action::Set(key,input[equal+1..].trim_ascii());}}
    }
    let correction=prefix(input,b"actually, ").or_else(||prefix(input,b"actually "));
    let input=correction.unwrap_or(input);
    if !(learn||explicit.is_some()||correction.is_some()){return Action::None;}
    // Reject ambiguous clauses rather than learning quoted/inferred/hypothetical identities.
    if input.contains(&b'?')||input.contains(&b'"')||input.contains(&b'\n'){return Action::None;}
    let mut lower=[0u8;4096];if input.len()>lower.len(){return Action::None;}
    for (out,b) in lower.iter_mut().zip(input){*out=b.to_ascii_lowercase();}
    let lower=&lower[..input.len()];
    if [b" if ".as_slice(),b" maybe ",b" not ",b" used to ",b" for example ",b" or ",b" and "].iter().any(|term|lower.windows(term.len()).any(|w|w==*term)){return Action::None;}
    if let Some(value)=prefix(input,b"i moved to ").or_else(||prefix(input,b"i live at ")){
        return Action::Set(b"address",value);
    }
    for (head,key) in [(b"i am interested in ".as_slice(),b"interests".as_slice()),
        (b"i now enjoy ",b"interests"),(b"i enjoy ",b"interests"),(b"i like ",b"interests"),
        (b"i no longer like ",b"dislikes"),(b"i dislike ",b"dislikes"),
        (b"i prefer replies that are ",b"communication_style"),
        (b"my recurring project is ",b"recurring_projects")]{
        if let Some(value)=prefix(input,head){if !value.is_empty(){return Action::Set(key,value);}}
    }
    if let Some(rest)=prefix(input,b"my "){
        if let Some(at)=rest.windows(4).position(|w|w.eq_ignore_ascii_case(b" is ")){
            if let Some(key)=field(&rest[..at]){
                if matches!(key,b"birth_date"|b"emergency_contact")&&explicit.is_none(){return Action::None;}
                let value=prefix(&rest[at+4..],b"now ").unwrap_or(&rest[at+4..]).trim_ascii();
                if !value.is_empty(){return Action::Set(key,value);}
            }
            // Store direct nonempty personal assertions as data, not instructions.
            // The receipt writer merges by label rather than replacing all facts.
            let label=rest[..at].trim_ascii();
            let value=rest[at+4..].trim_ascii();
            if !label.is_empty() && label.len()<=64 && !value.is_empty() && value.len()<=256
                && label.iter().all(|b|b.is_ascii_alphabetic()||matches!(*b,b' '|b'\''|b'-')) {
                return Action::Fact(label,value);
            }
        }
    }
    Action::None
}
// ------------------------=
// FUNC: fact_relevant
// DESC: Retrieves labelled facts by their subject without injecting unrelated facts into every prompt.
// ------------------=
pub fn fact_relevant(facts:&[u8], input:&[u8])->bool {
    facts.split(|b|*b==b'\n').filter_map(|line|line.iter().position(|b|*b==b':').map(|end|&line[..end]))
        .any(|label|label.split(|b|!b.is_ascii_alphabetic()).any(|word|
            word.len()>=3 && ![b"name".as_slice(),b"the",b"my"].iter().any(|skip|word.eq_ignore_ascii_case(skip))
            && input.windows(word.len()).any(|part|part.eq_ignore_ascii_case(word))))
}

// ------------------------=
// FUNC: relevant
// DESC: Selects personal context by task without disclosing unrelated address, contact, birthday, or preferences.
// ------------------=
pub fn relevant(key:&[u8],input:&[u8])->bool{
    if matches!(key,b"assistant_name"|b"user_name"|b"personality_style"|b"attitude"|b"user_instructions"|b"communication_style"){return true;}
    let terms:&[&[u8]]=match key{
        b"address"=>&[b"address",b"deliver",b"where i live",b"near me"],
        b"time"|b"timezone"=>&[b"time",b"schedule",b"meeting",b"calendar"],
        b"birth_date"=>&[b"birthday",b"birth date",b"my age"],
        b"favorite_color"=>&[b"color",b"colour",b"theme",b"desktop"],
        b"emergency_contact"=>&[b"emergency contact"],
        b"preferences"=>&[b"prefer",b"recommend",b"suggest"],
        b"interests"|b"dislikes"|b"inferred_interests"=>&[b"recommend",b"suggest",b"interest",b"hobby"],
        b"recurring_projects"=>&[b"project",b"working on"],
        b"facts"=>&[b"remember",b"about me"],_=>&[]
    };
    terms.iter().any(|term|input.windows(term.len()).any(|window|window.eq_ignore_ascii_case(term)))
}
// ------------------------=
// FUNC: infer_interest
// DESC: Produces only a tentative non-sensitive topic hypothesis; never promotes it to an explicit preference.
// ------------------=
pub fn infer_interest(document:&super::Document,input:&[u8])->Option<&'static[u8]>{
    if !document.enabled(b"learning_enabled")||!document.enabled(b"inference_enabled"){return None;}
    let mut value=[0;1024];
    if document.get(b"interests",&mut value)!=Ok(0){return None;}
    for (phrase,topic) in [(b"recommend a hiking".as_slice(),b"Possibly hiking".as_slice()),
        (b"suggest a photography",b"Possibly photography"),(b"recommend a gardening",b"Possibly gardening")]{
        if prefix(input,phrase).is_some(){
            if document.get(b"inferred_interests",&mut value).is_ok_and(|n|value[..n].eq_ignore_ascii_case(topic)){return None;}
            return Some(topic);
        }
    }None
}
// ------------------------=
// FUNC: color
// DESC: Converts supported named and hexadecimal color preferences into native theme RGB values.
// ------------------=
pub fn color(value:&[u8])->Option<u32>{
    for (name,rgb) in [(b"blue".as_slice(),0x3e6fff),(b"red",0xe65050),(b"green",0x35b878),
        (b"purple",0x9966ee),(b"pink",0xe76fa9),(b"orange",0xe89b42),(b"yellow",0xe6c84e),
        (b"cyan",0x22d3ee),(b"white",0xffffff),(b"black",0x101010),
        (b"grey",0x808080),(b"gray",0x808080)]{
        if value.eq_ignore_ascii_case(name){return Some(rgb);}
    }
    let value=value.strip_prefix(b"#")?;
    if value.len()!=6{return None;}
    let mut rgb=0;for b in value{rgb=rgb*16+match b{b'0'..=b'9'=>(b-b'0')as u32,b'a'..=b'f'=>(b-b'a'+10)as u32,b'A'..=b'F'=>(b-b'A'+10)as u32,_=>return None};}
    Some(rgb.max(1))
}
