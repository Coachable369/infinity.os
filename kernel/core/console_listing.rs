//! Native namespace metadata projection and aligned terminal columns; no Unix permission emulation.
use crate::{runtime::{automation,console_execution::{Bytes,OUTPUT_BYTES}},storage::object::{ObjectMetadata,ObjectType}};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Kind{Directory,File,Collection,Application,Model,Metadata,System,Identity,Device,Unknown}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Modified{Utc(u64),NotRecorded,Unavailable}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Row<'a>{pub kind:Kind,pub owner:&'a[u8],pub size:Option<u64>,pub modified:Modified,pub name:&'a[u8]}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Widths{pub owner:usize,pub size:usize}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Geometry{pub columns:[usize;5],pub end:usize,pub directory:bool}
impl Widths{
    // ------------------------=
    // FUNC: new
    // DESC: Establishes compact native listing column widths that can expand for complete owner names and real byte counts.
    // ------------------=
    pub const fn new()->Self{Self{owner:5,size:8}}
    // ------------------------=
    // FUNC: observe
    // DESC: Measures visible metadata before rendering so all rows align without shortening an owner identity.
    // ------------------=
    pub fn observe(&mut self,row:Row<'_>){self.owner=self.owner.max(row.owner.len());if let Some(size)=row.size{self.size=self.size.max(decimal_len(size));}}
}
// ------------------------=
// FUNC: project
// DESC: Maps real native metadata into readable columns, excluding internal directory payload sizes and untagged legacy timestamps.
// ------------------=
pub fn project<'a>(metadata:Option<&ObjectMetadata>,owner:&'a[u8],name:&'a[u8])->Row<'a>{
    let Some(m)=metadata else{return Row{kind:Kind::Unknown,owner,size:None,modified:Modified::Unavailable,name};};
    let kind=match m.kind{ObjectType::NamespaceNode=>Kind::Directory,ObjectType::Blob|ObjectType::Text=>Kind::File,ObjectType::Collection|ObjectType::Project=>Kind::Collection,ObjectType::ApplicationData=>Kind::Application,ObjectType::Model=>Kind::Model,ObjectType::Metadata=>Kind::Metadata,ObjectType::SystemComponent=>Kind::System,ObjectType::IdentityData=>Kind::Identity,ObjectType::DeviceData=>Kind::Device};
    let utc=m.modified&!(1u64<<63);let modified=if m.modified>>63==1&&automation::format_utc(utc).is_some(){Modified::Utc(utc)}else{Modified::NotRecorded};
    Row{kind,owner,size:if kind==Kind::Directory{None}else{Some(u64::from(m.logical_size))},modified,name}
}
// ------------------------=
// FUNC: selected
// DESC: Selects exact files or visible directory children while enforcing namespace boundaries and dot-entry options.
// ------------------=
pub fn selected(prefix:&[u8],path:&[u8],file:bool,all:bool,tree:bool)->bool{
    if file{return path==prefix;}if path==prefix||!path.starts_with(prefix){return false;}
    let relative=if prefix==b"/"{&path[1..]}else{if path.get(prefix.len())!=Some(&b'/'){return false;}&path[prefix.len()+1..]};
    !relative.is_empty()&&(tree||!relative.contains(&b'/'))&&(all||!relative.split(|b|*b==b'/').any(|part|part.starts_with(b".")))
}
// ------------------------=
// FUNC: display_name
// DESC: Displays names relative to the requested directory while preserving hierarchy in recursive listings.
// ------------------=
pub fn display_name<'a>(prefix:&[u8],path:&'a[u8],file:bool)->&'a[u8]{if file{path.rsplit(|byte|*byte==b'/').next().unwrap_or(path)}else if prefix==b"/"{&path[1..]}else{&path[prefix.len()+1..]}}
// ------------------------=
// FUNC: decimal_len
// DESC: Measures an actual unsigned size for right-aligned decimal output.
// ------------------=
fn decimal_len(mut value:u64)->usize{let mut n=1;while value>=10{value/=10;n+=1;}n}
// ------------------------=
// FUNC: spaces
// DESC: Emits bounded monospace padding shared by column headers and rows.
// ------------------=
fn spaces(out:&mut Bytes<OUTPUT_BYTES>,count:usize){for _ in 0..count{out.write(b" ");}}
// ------------------------=
// FUNC: cell
// DESC: Writes one complete field and pads it to the measured column width without truncation.
// ------------------=
fn cell(out:&mut Bytes<OUTPUT_BYTES>,value:&[u8],width:usize,right:bool){let padding=width.saturating_sub(value.len());if right{spaces(out,padding);}out.write(value);if !right{spaces(out,padding);}out.write(b"  ");}
// ------------------------=
// FUNC: header
// DESC: Prints the native long-listing headings with explicit UTC and byte units and no invented Unix permission column.
// ------------------=
pub fn header(out:&mut Bytes<OUTPUT_BYTES>,widths:Widths){cell(out,b"Type",11,false);cell(out,b"Owner",widths.owner,false);cell(out,b"Size (B)",widths.size,true);cell(out,b"Modified (UTC)",19,false);out.write_line(b"Name");}
// ------------------------=
// FUNC: render
// DESC: Renders a metadata row and returns its actual column geometry for behavioral layout verification.
// ------------------=
pub fn render(out:&mut Bytes<OUTPUT_BYTES>,row:Row<'_>,widths:Widths)->Geometry{
    let mut columns=[0;5];columns[0]=out.bytes().len();
    cell(out,match row.kind{Kind::Directory=>b"Directory",Kind::File=>b"File",Kind::Collection=>b"Collection",Kind::Application=>b"Application",Kind::Model=>b"Model",Kind::Metadata=>b"Metadata",Kind::System=>b"System",Kind::Identity=>b"Identity",Kind::Device=>b"Device",Kind::Unknown=>b"Unknown"},11,false);
    columns[1]=out.bytes().len();cell(out,row.owner,widths.owner,false);columns[2]=out.bytes().len();
    let mut size=[0;20];let mut at=size.len();if let Some(mut value)=row.size{loop{at-=1;size[at]=b'0'+(value%10)as u8;value/=10;if value==0{break;}}cell(out,&size[at..],widths.size,true);}else{cell(out,b"-",widths.size,true);}
    columns[3]=out.bytes().len();match row.modified{Modified::Utc(value)=>{if let Some(date)=automation::format_utc(value){cell(out,&date.bytes()[..19],19,false);}else{cell(out,b"not recorded",19,false);}},Modified::NotRecorded=>cell(out,b"not recorded",19,false),Modified::Unavailable=>cell(out,b"unavailable",19,false)};
    columns[4]=out.bytes().len();out.write(row.name);let directory=row.kind==Kind::Directory;if directory&&!row.name.ends_with(b"/"){out.write(b"/");}let end=out.bytes().len();out.write(b"\n");Geometry{columns,end,directory}
}
