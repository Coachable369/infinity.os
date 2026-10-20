#[path = "../kernel/runtime/ai/hermes_profile.rs"]
mod profile;
use profile::{Document, Error};
// ------------------------=
// FUNC: round_trip
// DESC: Verifies all fields survive serialization and Unicode edits.
// ------------------=
#[test]
fn round_trip() {
    let mut document = Document::new([1; 16], b"User", b"Infinity").unwrap();
    for key in profile::FIELDS {
        let value: &[u8] = match key {
            b"learning_enabled"|b"inquiries_enabled"|b"inference_enabled"=>b"true",
            b"wake_word" => b"Computer",
            b"birth_date" => b"2000-02-29",
            _ => "Zoë \"example\"\npreferences".as_bytes(),
        };
        document = document.updated([1; 16], key, value).unwrap();
        document = Document::parse(document.bytes(), [1; 16]).unwrap();
        let mut out = [0; 1024];
        let n = document.get(key, &mut out).unwrap();
        assert_eq!(&out[..n], value);
    }
}
// ------------------------=
// FUNC: reject_invalid
// DESC: Rejects foreign owners, invalid dates, unsupported wake words, malformed JSON, and excessive context.
// ------------------=
#[test]
fn reject_invalid() {
    let d = Document::new([1; 16], b"User", b"Infinity").unwrap();
    assert_eq!(
        Document::parse(d.bytes(), [2; 16]).err(),
        Some(Error::Owner)
    );
    assert!(d.updated([1; 16], b"birth_date", b"1900-02-29").is_err());
    assert!(d.updated([1; 16], b"wake_word", b"anything").is_err());
    assert!(d.updated([1; 16], b"owner_id", b"new").is_err());
    for n in 0..d.bytes().len() - 2 {
        assert!(Document::parse(&d.bytes()[..n], [1; 16]).is_err());
    }
    let large = d.updated([1; 16], b"facts", &[b'x'; 1024]).unwrap();
    let expanded=large.updated([1;16],b"preferences",&[b'x';1024]).unwrap();
    let mut selected=[0;1536];
    assert!(expanded.selected_context(b"recommend a preference",&mut selected).unwrap()<=1536);
}

// ------------------------=
// FUNC: correction_inspection_and_forgetting
// DESC: Exercises spoken corrections as canonical replacement, inspection, forgetting, and opt-out semantics.
// ------------------=
#[test]
fn correction_inspection_and_forgetting(){
    use profile::conversation::{action,Action};
    assert_eq!(action(b"My favorite color is blue",true),Action::Set(b"favorite_color",b"blue"));
    assert_eq!(action(b"Actually, my favorite color is now green",true),Action::Set(b"favorite_color",b"green"));
    assert_eq!(action(b"Infinity, what do you remember about me?",true),Action::Inspect);
    assert_eq!(action(b"forget my address",false),Action::Forget(b"address"));
    assert_eq!(action(b"My name is Alex",false),Action::None);
    assert_eq!(action(b"My name is Alex?",true),Action::None);
    assert_eq!(action(b"My name is Alex if I play this role",true),Action::None);
    assert_eq!(action(b"\"my name is Alex\"",true),Action::None);
    let d=Document::new([1;16],b"User",b"Infinity").unwrap()
        .updated([1;16],b"address",b"old").unwrap().updated([1;16],b"address",b"new").unwrap();
    let mut value=[0;1024];assert_eq!(d.get(b"address",&mut value),Ok(3));assert_eq!(&value[..3],b"new");
    let d=d.updated([1;16],b"address",b"").unwrap();assert_eq!(d.get(b"address",&mut value),Ok(0));
    assert!(d.inspect(&mut [0;8192]).is_ok());
    assert!(!profile::conversation::relevant(b"address",b"write a C program"));
    assert!(profile::conversation::relevant(b"address",b"where do I live near me"));
    assert_eq!(profile::conversation::color(b"green"),Some(0x35b878));
    let d=d.updated([1;16],b"inquiries_enabled",b"false").unwrap();
    for turn in 0..100{assert_eq!(profile::conversation::inquiry(&d,turn,turn),None);}
}

#[path="../kernel/storage/user_home.rs"]
mod user_home;
// ------------------------=
// FUNC: display_home
// DESC: Checks Unicode whitespace, exact labels, and traversal rejection without filesystem assumptions.
// ------------------=
#[test]
fn display_home(){
    assert_eq!(user_home::Path::new("Jane\u{2003}Doe".as_bytes()).unwrap().as_bytes(),b"/home/Jane.Doe");
    assert!(user_home::Path::new(b"../other").is_none());
    assert!(user_home::Path::new(b"..").is_none());
    assert!(user_home::Path::new(b"Jane").unwrap().child(b"../secret").is_none());
}
// ------------------------=
// FUNC: relationship_memory
// DESC: Verifies explicit preferences, tentative inference, opt-out and non-interrupting contextual curiosity.
// ------------------=
#[test]
fn relationship_memory(){
    use profile::conversation::{action,Action,infer_interest,contextual_inquiry};
    assert_eq!(action(b"I now enjoy hiking",true),Action::Set(b"interests",b"hiking"));
    assert_eq!(action(b"I prefer replies that are concise",true),Action::Set(b"communication_style",b"concise"));
    assert_eq!(action(b"My recurring project is InfinityOS",true),Action::Set(b"recurring_projects",b"InfinityOS"));
    let d=Document::new([1;16],b"User",b"Infinity").unwrap();
    assert_eq!(infer_interest(&d,b"recommend a hiking trail"),Some(b"Possibly hiking".as_slice()));
    let inferred=d.updated([1;16],b"inferred_interests",b"Possibly hiking").unwrap();
    assert_eq!(infer_interest(&inferred,b"recommend a hiking route"),None);
    assert_eq!(infer_interest(&inferred,b"suggest a photography trip"),Some(b"Possibly photography".as_slice()));
    let explicit=d.updated([1;16],b"interests",b"painting").unwrap();
    assert_eq!(infer_interest(&explicit,b"recommend a hiking trail"),None);
    let disabled=d.updated([1;16],b"inference_enabled",b"false").unwrap();
    assert_eq!(infer_interest(&disabled,b"recommend a hiking trail"),None);
    for turn in 0..100{assert_eq!(contextual_inquiry(&d,b"fix this compilation error",turn,turn),None);}
    assert!( (7..100).any(|turn|contextual_inquiry(&d,b"suggest a desktop color",turn,turn).is_some()));
}
// ------------------------=
// FUNC: spoken_color_and_timezone
// DESC: Accepts supported spoken field spellings, preserves opt-outs, and resolves grey to a native accent.
// ------------------=
#[test]
fn spoken_color_and_timezone(){
    use profile::conversation::{action,Action,color};
    for input in [
        b"My Favorite color is grey".as_slice(),
        b"My favourite colour is grey.",
        b"My favorite colour is grey",
        b"My favourite color is grey",
    ]{
        assert_eq!(action(input,true),Action::Set(b"favorite_color",b"grey"));
        assert_eq!(action(input,false),Action::None);
    }
    for input in [b"Set time zone to CST".as_slice(),b"set timezone to CST",b"set my time zone to CST"]{
        assert_eq!(action(input,false),Action::Set(b"timezone",b"CST"));
    }
    assert_eq!(action(b"Remember my favourite color is grey",false),Action::Set(b"favorite_color",b"grey"));
    assert_eq!(action(b"Set favorite colour to gray",false),Action::Set(b"favorite_color",b"gray"));
    assert_eq!(color(b"grey"),Some(0x808080));
    assert_eq!(color(b"GRAY"),Some(0x808080));
    assert_eq!(action(b"Is my favorite color grey?",true),Action::None);
    assert_eq!(action(b"My favorite color is grey if I change my mind",true),Action::None);
    assert_eq!(action(b"set unknown setting to CST",true),Action::None);
}
// ------------------------=
// FUNC: extension_preservation
// DESC: Preserves future nested data byte-for-byte through typed updates without exposing it to the prompt.
// ------------------=
#[test]
fn extension_preservation() {
    let d = Document::new([1; 16], b"User", b"Infinity").unwrap();
    let mut bytes = d.bytes()[..d.bytes().len() - 2].to_vec();
    let extra = b",\"future\":{\"items\":[1,true,null,{\"n\":-1.5e2}]}}";
    bytes.extend_from_slice(extra);
    let d = Document::parse(&bytes, [1; 16])
        .unwrap()
        .updated([1; 16], b"user_name", b"New")
        .unwrap();
    assert_eq!(&d.bytes()[d.bytes().len() - extra.len()..], extra);
    let mut context = [0; profile::CONTEXT_BYTES];
    assert!(d.context(&mut context).unwrap() > 0);
}
// ------------------------=
// FUNC: explicit_teaching
// DESC: Ordinary conversation cannot mutate memory; only explicit allowed assignments produce edits.
// ------------------=
#[test]
fn explicit_teaching() {
    assert_eq!(profile::edit_command(b"Remember my name is Alex"),Some((b"user_name".as_slice(),b"Alex".as_slice())));
    assert_eq!(
        profile::edit_command(b"remember user_name=Alex"),
        Some((b"user_name".as_slice(), b"Alex".as_slice()))
    );
    assert_eq!(profile::edit_command(b"my name might be Alex"), None);
    assert_eq!(profile::edit_command(b"remember owner_id=changed"), None);
}
