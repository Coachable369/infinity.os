//! Allocation-free PNG preview: checked CRCs, bounded inflate and eight-bit scanlines.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Png {pub width:usize,pub height:usize,pub channels:usize,pub stride:usize}
// ------------------------=
// FUNC: be32
// DESC: Reads a checked big-endian PNG integer.
// ------------------=
fn be32(b:&[u8],n:usize)->Option<u32>{Some(u32::from_be_bytes(b.get(n..n+4)?.try_into().ok()?))}
// ------------------------=
// FUNC: crc
// DESC: Verifies chunk integrity before treating compressed file bytes as image data.
// ------------------=
fn crc(b:&[u8])->u32{let mut c=!0u32;for &v in b{c^=v as u32;for _ in 0..8{c=(c>>1)^((0u32.wrapping_sub(c&1))&0xedb88320);}}!c}
// ------------------------=
// FUNC: paeth
// DESC: Implements the PNG Paeth predictor without unsigned subtraction overflow.
// ------------------=
fn paeth(a:u8,b:u8,c:u8)->u8{let p=a as i32+b as i32-c as i32;let pa=(p-a as i32).abs();let pb=(p-b as i32).abs();let pc=(p-c as i32).abs();if pa<=pb&&pa<=pc{a}else if pb<=pc{b}else{c}}
// ------------------------=
// FUNC: decode
// DESC: Decodes bounded noninterlaced eight-bit grayscale/RGB PNGs; unsupported formats and malformed streams fail closed.
// ------------------=
pub fn decode(bytes:&[u8],compressed:&mut [u8],raw:&mut [u8],inflate:impl FnOnce(&[u8],&mut [u8])->bool)->Option<Png>{
    if bytes.get(..8)?!=b"\x89PNG\r\n\x1a\n"{return None;}
    let mut at=8usize;let mut used=0usize;let mut info=None;let mut ended=false;
    while at<bytes.len(){let n=be32(bytes,at)? as usize;let end=at.checked_add(12)?.checked_add(n)?;
        if end>bytes.len(){return None;}let tag=bytes.get(at+4..at+8)?;let data=bytes.get(at+8..end-4)?;
        if crc(&bytes[at+4..end-4])!=be32(bytes,end-4)?{return None;}
        match tag {
            b"IHDR"=>{if at!=8||n!=13||data[8]!=8||data[10..13]!=[0,0,0]{return None;}
                let w=be32(data,0)? as usize;let h=be32(data,4)? as usize;
                let channels=match data[9]{0=>1,2=>3,4=>2,6=>4,_=>return None};
                let stride=w.checked_mul(channels)?.checked_add(1)?;
                if w==0||h==0||w>8192||h>8192||stride.checked_mul(h)?>raw.len(){return None;}
                info=Some(Png{width:w,height:h,channels,stride});},
            b"IDAT"=>{info?;let next=used.checked_add(n)?;compressed.get_mut(used..next)?.copy_from_slice(data);used=next;},
            b"IEND"=>{if n!=0||end!=bytes.len(){return None;}ended=true;break;},
            // Unknown critical chunks and transparency keys need a renderer with those semantics.
            b"tRNS"=>return None,
            _=>if tag[0]&32==0{return None;},
        }at=end;
    }
    let p=info?;if !ended||used==0{return None;}
    let count=p.stride*p.height;if !inflate(&compressed[..used],&mut raw[..count]){return None;}
    for y in 0..p.height {let offset=y*p.stride;let filter=raw[offset];if filter>4{return None;}
        for x in 1..p.stride {let a=if x>p.channels{raw[offset+x-p.channels]}else{0};let b=if y>0{raw[offset+x-p.stride]}else{0};
            let c=if y>0&&x>p.channels{raw[offset+x-p.stride-p.channels]}else{0};
            let predictor=match filter{0=>0,1=>a,2=>b,3=>((a as u16+b as u16)/2) as u8,_=>paeth(a,b,c)};
            raw[offset+x]=raw[offset+x].wrapping_add(predictor);
        }
    }Some(p)
}
impl Png {
    // ------------------------=
    // FUNC: rgba
    // DESC: Samples a decoded scanline without allocation or an unchecked caller coordinate.
    // ------------------=
    pub fn rgba(self,raw:&[u8],x:usize,y:usize)->Option<[u8;4]>{if x>=self.width||y>=self.height{return None;}let at=y*self.stride+1+x*self.channels;
        let p=raw.get(at..at+self.channels)?;Some(match self.channels{1=>[p[0],p[0],p[0],255],2=>[p[0],p[0],p[0],p[1]],3=>[p[0],p[1],p[2],255],4=>[p[0],p[1],p[2],p[3]],_=>return None})}
}
