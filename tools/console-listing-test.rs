#![allow(dead_code)]
//! Behavioral metadata projection and terminal-column geometry with the production listing renderer.
#[path="../kernel/runtime/mod.rs"]mod runtime;
#[path="../kernel/ui/mod.rs"]mod ui;
mod storage;
#[path="../kernel/core/console_listing.rs"]mod listing;
// ------------------------=
// FUNC: output_text
// DESC: Supplies an inert diagnostic sink without making prose an acceptance oracle.
// ------------------=
fn output_text(_:&[u8]){}
// ------------------------=
// FUNC: metadata
// DESC: Creates typed native metadata fixtures with explicit byte sizes and timestamp provenance.
// ------------------=
fn metadata(kind:storage::object::ObjectType,modified:u64)->storage::object::ObjectMetadata{
    use storage::object::*;ObjectMetadata{object:ObjectRef{id:ObjectId([1;16])},kind,space:Space::Personal,owner:ObjectId([2;16]),content_type:ContentType::Utf8Text,flags:0,current_version:3,logical_size:123456,created:17,modified,tags:[0;8],tags_len:0}
}
// ------------------------=
// FUNC: main
// DESC: Checks actual metadata semantics, dot/tree filtering, aligned geometry, directory distinction and bounded output without text oracles.
// ------------------=
fn main(){
    use listing::*;use storage::object::ObjectType;use runtime::console_execution::{Bytes,OUTPUT_BYTES};
    let epoch=runtime::automation::parse_utc(b"2026-11-09 10:11:12").unwrap();let file=metadata(ObjectType::Text,(1u64<<63)|epoch);let directory=metadata(ObjectType::NamespaceNode,42);
    let file_row=project(Some(&file),b"an-owner-with-a-long-native-handle",b"report.py");assert_eq!(file_row.kind,Kind::File);assert_eq!(file_row.size,Some(123456));assert_eq!(file_row.modified,Modified::Utc(epoch));
    let dir_row=project(Some(&directory),b"system",b"documents");assert_eq!(dir_row.kind,Kind::Directory);assert_eq!(dir_row.size,None);assert_eq!(dir_row.modified,Modified::NotRecorded);
    let absent=project(None,b"unavailable",b"private");assert_eq!(absent.kind,Kind::Unknown);assert_eq!(absent.modified,Modified::Unavailable);assert_eq!(absent.size,None);
    let unrecorded=metadata(ObjectType::Blob,(1u64<<63)|u64::MAX>>1);assert_eq!(project(Some(&unrecorded),b"owner",b"old").modified,Modified::NotRecorded);
    assert!(selected(b"/home",b"/home/file.py",false,false,false));assert!(!selected(b"/home",b"/homepage/file.py",false,true,true));assert!(!selected(b"/home",b"/home/sub/file.py",false,false,false));assert!(selected(b"/home",b"/home/sub/file.py",false,false,true));assert!(!selected(b"/home",b"/home/.keys/file",false,false,true));assert!(selected(b"/home",b"/home/.keys/file",false,true,true));assert!(selected(b"/home/.keys/file",b"/home/.keys/file",true,false,false));assert!(!selected(b"/",b"/",false,true,true));assert!(selected(b"/",b"/home",false,false,false));
    let mut widths=Widths::new();widths.observe(file_row);widths.observe(dir_row);widths.observe(absent);assert_eq!(widths.owner,file_row.owner.len());assert_eq!(widths.size,8);
    let mut out=Bytes::<OUTPUT_BYTES>::new();header(&mut out,widths);let a=render(&mut out,file_row,widths);let b=render(&mut out,dir_row,widths);let c=render(&mut out,absent,widths);
    for geometry in [a,b,c]{assert_eq!(geometry.columns[1]-geometry.columns[0],13);assert_eq!(geometry.columns[2]-geometry.columns[1],widths.owner+2);assert_eq!(geometry.columns[3]-geometry.columns[2],widths.size+2);assert_eq!(geometry.columns[4]-geometry.columns[3],21);}
    assert!(!a.directory);assert!(b.directory);assert_eq!(a.end-a.columns[4],file_row.name.len());assert_eq!(b.end-b.columns[4],dir_row.name.len()+1);assert!(!out.truncated());
    for _ in 0..OUTPUT_BYTES/20+1{render(&mut out,file_row,widths);}assert!(out.truncated());assert_eq!(out.bytes().len(),OUTPUT_BYTES);
}
