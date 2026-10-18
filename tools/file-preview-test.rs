//! Behavioral quick-look tests: actual decoding, wrapping, scrolling and teardown.
#[path="../kernel/ui/geometry.rs"] pub mod geometry;
#[path="../kernel/ui/bitmap.rs"] pub mod bitmap;
#[path="../kernel/ui/preview_png.rs"] pub mod preview_png;
#[path="../kernel/ui/file_preview.rs"] pub mod file_preview;
#[path="../kernel/ui/user_wallpaper.rs"] pub mod user_wallpaper;
pub mod ui {pub use crate::{geometry,bitmap,preview_png,file_preview,user_wallpaper};}
pub mod runtime {pub mod object_navigation {
    // ------------------------=
    // FUNC: namespace_basename
    // DESC: Supplies the same path basename to the host-only renderer adapter.
    // ------------------=
    pub fn namespace_basename(path:&[u8])->&[u8]{path.rsplit(|b|*b==b'/').next().unwrap_or(path)}
}}
#[path="file-preview-proof.rs"] mod proof;
// ------------------------=
// FUNC: main
// DESC: Keeps this target runnable while acceptance cases execute through cargo test.
// ------------------=
fn main() {proof::render();}
#[cfg(test)] mod tests {
use super::*;
#[test]
// ------------------------=
// FUNC: wallpaper_pixels_and_session_revocation
// DESC: Verifies decoded wallpaper pixels, bounds rejection and private cache revocation.
// ------------------=
fn wallpaper_pixels_and_session_revocation(){
    let mut out=[0u8;70];
    let n=user_wallpaper::encode(&mut out,2,2,|x,y|Some([x as u8*200,y as u8*120,80,255])).unwrap();
    let b=bitmap::RuntimeBitmap::parse(&out[..n]).unwrap();assert_eq!(b.rgba(1,1),Some([200,120,80,255]));
    assert!(user_wallpaper::encode(&mut out,3,2,|_,_|Some([0;4])).is_none());
    assert!(user_wallpaper::encode(&mut out,0,1,|_,_|Some([0;4])).is_none());
    assert!(user_wallpaper::image_path(b"/home/me/photo.PNG"));assert!(!user_wallpaper::image_path(b"/home/me/photo.txt"));
    assert!(user_wallpaper::install([1;16],1,1,|_,_|Some([70,80,90,255])));
    assert!(user_wallpaper::take_damage(800,600).is_some());assert!(user_wallpaper::take_damage(800,600).is_none());
    user_wallpaper::session([1;16],true);assert!(user_wallpaper::bitmap(|b|b.is_some()));
    user_wallpaper::session([2;16],true);assert!(user_wallpaper::bitmap(|b|b.is_none()));
    assert!(user_wallpaper::install([1;16],1,1,|_,_|Some([70,80,90,255])));
    user_wallpaper::session([1;16],false);assert!(user_wallpaper::bitmap(|b|b.is_none()));
}
// ------------------------=
// FUNC: decode
// DESC: Runs production PNG parsing against the same bounded native zlib adapter.
// ------------------=
fn decode(bytes:&[u8],compressed:&mut [u8],raw:&mut [u8])->Option<preview_png::Png>{preview_png::decode(bytes,compressed,raw,|input,output|{
    let mut inflater=miniz_oxide::inflate::core::DecompressorOxide::new();let (status,n,m)=miniz_oxide::inflate::core::decompress(&mut inflater,input,output,0,5);
    status==miniz_oxide::inflate::TINFLStatus::Done&&n==input.len()&&m==output.len()
})}
// ------------------------=
// FUNC: crc
// DESC: Creates a fixture chunk checksum using an independent bitwise encoder.
// ------------------=
fn crc(bytes:&[u8])->u32{let mut c=u32::MAX;for b in bytes{c^=*b as u32;for _ in 0..8{c=if c&1!=0{(c>>1)^0xedb88320}else{c>>1};}}c^u32::MAX}
// ------------------------=
// FUNC: chunk
// DESC: Encodes fixture-only PNG chunks with exact lengths and integrity checks.
// ------------------=
fn chunk(out:&mut Vec<u8>,tag:&[u8;4],data:&[u8]){out.extend_from_slice(&(data.len() as u32).to_be_bytes());out.extend_from_slice(tag);out.extend_from_slice(data);let start=out.len()-data.len()-4;out.extend_from_slice(&crc(&out[start..]).to_be_bytes());}
// ------------------------=
// FUNC: fixture
// DESC: Builds a one-pixel red PNG using a stored DEFLATE block, independent of the production decoder.
// ------------------=
fn fixture(filter:u8)->Vec<u8>{let mut out=b"\x89PNG\r\n\x1a\n".to_vec();chunk(&mut out,b"IHDR",&[0,0,0,1,0,0,0,1,8,2,0,0,0]);
    let bytes=[filter,255,0,0];let mut z=vec![0x78,0x01,1,4,0,251,255];z.extend_from_slice(&bytes);
    let (mut a,mut b)=(1u32,0u32);for n in bytes{a=(a+n as u32)%65521;b=(b+a)%65521;}z.extend_from_slice(&((b<<16)|a).to_be_bytes());
    chunk(&mut out,b"IDAT",&z);chunk(&mut out,b"IEND",&[]);out}
#[test]
// ------------------------=
// FUNC: png_pixels_and_rejection
// DESC: Requires actual red output for all PNG filters and rejects damaged, truncated and oversized streams.
// ------------------=
fn png_pixels_and_rejection(){for filter in 0..5{let bytes=fixture(filter);let mut compressed=[0;64];let mut raw=[0;32];
    let p=decode(&bytes,&mut compressed,&mut raw).unwrap();assert_eq!(p.rgba(&raw,0,0),Some([255,0,0,255]));assert_eq!(p.rgba(&raw,1,0),None);
    for n in 0..bytes.len(){assert!(decode(&bytes[..n],&mut compressed,&mut raw).is_none());}
    let mut damaged=bytes.clone();damaged[20]^=1;assert!(decode(&damaged,&mut compressed,&mut raw).is_none());
    assert!(decode(&bytes,&mut compressed,&mut [0;3]).is_none());
}assert!(decode(&fixture(5),&mut [0;64],&mut [0;32]).is_none());}
#[test]
// ------------------------=
// FUNC: preview_authority
// DESC: Checks session, space and owner decisions used by every preview entry point.
// ------------------=
fn preview_authority(){assert!(file_preview::can_read(true,true,[7;16],[7;16]));assert!(file_preview::can_read(true,true,[0;16],[7;16]));assert!(!file_preview::can_read(false,true,[7;16],[7;16]));assert!(!file_preview::can_read(true,false,[7;16],[7;16]));assert!(!file_preview::can_read(true,true,[8;16],[7;16]));}
#[test]
// ------------------------=
// FUNC: text_wrapping_and_scroll
// DESC: Verifies CRLF, blank lines and Unicode survive wrapping while scroll and geometry stay bounded.
// ------------------=
fn text_wrapping_and_scroll(){let mut rows=Vec::new();file_preview::text_rows("abcé\r\n\nend".as_bytes(),3,|_,b|rows.push(b.to_vec()));
    assert_eq!(rows,vec![b"abc".to_vec(),"é".as_bytes().to_vec(),vec![],b"end".to_vec()]);
    for (w,h,s) in [(640,480,1),(1920,1080,2),(3840,2160,3)]{let mut p=file_preview::Preview::new();p.place(w,h,s);
        assert!(p.rect.contains(geometry::Point{x:p.body().x,y:p.body().y}));assert!(p.body().bottom()<p.rect.bottom());
        p.lines=500;p.scroll_by(usize::MAX as isize);p.scroll_to_pointer(geometry::Point{x:0,y:i32::MAX});assert_eq!(p.scroll,500-p.rows());
        p.scroll_by(-10000);assert_eq!(p.scroll,0);
    }
    assert_eq!(file_preview::classify(b"%PDF-1.7"),file_preview::Kind::Unsupported);
    assert_eq!(file_preview::classify(&[0,1,2]),file_preview::Kind::Unsupported);
    assert_eq!(file_preview::classify(b"# Markdown\nhello"),file_preview::Kind::Text);
}
#[test]
// ------------------------=
// FUNC: dismissal_clears_owned_data_and_damage
// DESC: Exercises shared state load, movement damage union and complete session teardown.
// ------------------=
fn dismissal_clears_owned_data_and_damage(){file_preview::close();let _=file_preview::take_damage();let mut p=file_preview::Preview::new();p.place(1024,768,1);p.open=true;p.owner=[7;16];
    let mut files=[file_preview::EMPTY_FILE;16];files[0].path[..6].copy_from_slice(b"a.txtx");files[0].length=6;
    file_preview::set_desktop_files(files);assert!(file_preview::desktop_damage(1024,768).is_some());
    file_preview::set_desktop_files(files);assert!(file_preview::desktop_damage(1024,768).is_none());
    file_preview::set_desktop_files([file_preview::EMPTY_FILE;16]);assert_eq!(file_preview::desktop_files()[0].length,0);
    p.length=file_preview::load(|b|{b[..6].copy_from_slice(b"secret");Some(6)}).unwrap();file_preview::publish(p);assert_eq!(file_preview::content(|b|b.to_vec()),b"secret");
    let old=p.rect;p.rect.x+=10;file_preview::publish(p);assert_eq!(file_preview::take_damage(),Some(old.union(p.rect)));
    file_preview::close();assert!(!file_preview::current().open);assert_eq!(file_preview::current().owner,[0;16]);assert!(file_preview::content(|b|b.is_empty()));
}
}
