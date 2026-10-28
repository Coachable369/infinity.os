#[path="../kernel/runtime/generated_apps.rs"]mod catalog;
use catalog::{Entry,Error,Language,Registry,BYTES};
// ------------------------=
// FUNC: main
// DESC: Exercises persisted owner/version identities, human-name matching, ambiguity and malformed-record rejection.
// ------------------=
fn main(){
    let user=[7;16];let other=[8;16];let source=b"/home/default/documents/calculator-0123456789abcdef.c";
    let executable=b"/home/default/documents/calculator-0123456789abcdef.elf";
    let entry=Entry::new(source,executable,[1;16],[2;16],3,4,Language::C).unwrap();
    assert!(entry.matches(b"calculator"));assert!(entry.matches(b"Calculator app"));assert!(entry.matches(source));
    assert!(!entry.matches(b"calc"));assert!(!entry.matches(b"calendar"));
    let mut registry=Registry::new(user);registry.register(user,entry).unwrap();
    assert_eq!(registry.find(other,b"calculator"),Err(Error::Owner));assert_eq!(registry.find(user,b"missing"),Err(Error::Missing));
    let mut bytes=[0;BYTES];assert_eq!(registry.encode(&mut bytes).unwrap(),BYTES);
    let loaded=Registry::decode(user,&bytes).unwrap();assert_eq!(loaded.find(user,b"calculator"),Ok(entry));
    assert!(Registry::decode(other,&bytes).is_err());bytes[32+282]=1;assert!(Registry::decode(user,&bytes).is_err());
    let python=Entry::new(b"/home/default/documents/tax-calculator.py",b"/home/default/documents/tax-calculator.py",[3;16],[3;16],1,1,Language::Python).unwrap();
    registry.register(user,python).unwrap();assert_eq!(registry.find(user,b"calculator"),Err(Error::Ambiguous));assert_eq!(registry.find(user,b"tax calculator"),Ok(python));
    let replacement=Entry{source_version:5,artifact_version:6,..entry};registry.register(user,replacement).unwrap();assert_eq!(registry.entries.iter().flatten().count(),2);
    assert!(Entry::new(b"/system/a.py",b"/system/a.py",[1;16],[1;16],1,1,Language::Python).is_err());
    println!("generated application catalog behavior passed");
}
