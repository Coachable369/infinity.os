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
    assert!(large
        .updated([1; 16], b"preferences", &[b'x'; 1024])
        .is_err());
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
