//! Zero-copy safetensors reader for native image model shards; no language-model loader changes.
use crate::Error;
const MAX_HEADER:usize=16*1024*1024;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Dtype{Bf16,F16,F32}
#[derive(Clone,Copy,Debug)]
pub struct Tensor<'a>{pub name:&'a[u8],pub dtype:Dtype,pub shape:[usize;8],pub rank:usize,pub bytes:&'a[u8]}
pub struct Shard<'a>{header:&'a[u8],data:&'a[u8]}
struct Json<'a>{bytes:&'a[u8],at:usize}
impl<'a> Json<'a>{
    // ------------------------=
    // FUNC: space
    // DESC: Consumes only JSON whitespace, never arbitrary control bytes.
    // ------------------=
    fn space(&mut self){while self.bytes.get(self.at).is_some_and(|b|matches!(b,b' '|b'\n'|b'\r'|b'\t')){self.at+=1;}}
    // ------------------------=
    // FUNC: eat
    // DESC: Consumes one bounded structural token.
    // ------------------=
    fn eat(&mut self,token:u8)->bool{self.space();if self.bytes.get(self.at)==Some(&token){self.at+=1;true}else{false}}
    // ------------------------=
    // FUNC: require
    // DESC: Rejects malformed descriptor punctuation.
    // ------------------=
    fn require(&mut self,token:u8)->Result<(),Error>{if self.eat(token){Ok(())}else{Err(Error::Shape)}}
    // ------------------------=
    // FUNC: string
    // DESC: Reads exporter tensor names and scalar metadata as bounded UTF-8 without allocating.
    // ------------------=
    fn string(&mut self)->Result<&'a[u8],Error>{
        self.require(b'"')?;let start=self.at;
        while let Some(&b)=self.bytes.get(self.at){
            if b==b'"'{let text=&self.bytes[start..self.at];core::str::from_utf8(text).map_err(|_|Error::Shape)?;self.at+=1;return Ok(text);}
            // Tensor identifiers and native package metadata deliberately forbid escaped names.
            if b<32||b==b'\\'{return Err(Error::Shape);}self.at+=1;
        }Err(Error::Bounds)
    }
    // ------------------------=
    // FUNC: number
    // DESC: Parses a nonnegative checked dimension or relative data offset.
    // ------------------=
    fn number(&mut self)->Result<usize,Error>{
        self.space();let start=self.at;let mut n=0usize;
        while let Some(&b)=self.bytes.get(self.at){if !b.is_ascii_digit(){break;}n=n.checked_mul(10).and_then(|n|n.checked_add((b-b'0')as usize)).ok_or(Error::Bounds)?;self.at+=1;}
        if start==self.at||(self.at-start>1&&self.bytes[start]==b'0'){return Err(Error::Shape);}Ok(n)
    }
    // ------------------------=
    // FUNC: array
    // DESC: Reads dimensions or offset pairs into caller-owned fixed storage.
    // ------------------=
    fn array(&mut self,out:&mut[usize])->Result<usize,Error>{
        self.require(b'[')?;let mut n=0;if self.eat(b']'){return Ok(0);}
        loop{if n==out.len(){return Err(Error::Bounds);}out[n]=self.number()?;n+=1;if self.eat(b']'){return Ok(n);}self.require(b',')?;}
    }
    // ------------------------=
    // FUNC: metadata
    // DESC: Validates the format's optional string-only metadata object without interpreting instructions.
    // ------------------=
    fn metadata(&mut self)->Result<(),Error>{
        self.require(b'{')?;if self.eat(b'}'){return Ok(());}
        loop{self.string()?;self.require(b':')?;self.string()?;if self.eat(b'}'){return Ok(());}self.require(b',')?;}
    }
    // ------------------------=
    // FUNC: tensor
    // DESC: Verifies dtype, shape, byte count and shard bounds before exposing immutable tensor bytes.
    // ------------------=
    fn tensor(&mut self,name:&'a[u8],data:&'a[u8])->Result<Tensor<'a>,Error>{
        self.require(b'{')?;let mut dtype=None;let mut shape=[0;8];let mut rank=None;let mut offsets=[0;2];let mut has_offsets=false;
        loop{let field=self.string()?;self.require(b':')?;
            match field{
                b"dtype" if dtype.is_none()=>{dtype=Some(match self.string()?{b"BF16"=>Dtype::Bf16,b"F16"=>Dtype::F16,b"F32"=>Dtype::F32,_=>return Err(Error::Shape)});}
                b"shape" if rank.is_none()=>{rank=Some(self.array(&mut shape)?);}
                b"data_offsets" if !has_offsets=>{if self.array(&mut offsets)?!=2{return Err(Error::Shape);}has_offsets=true;}
                _=>return Err(Error::Shape),
            }
            if self.eat(b'}'){break;}self.require(b',')?;
        }
        let dtype=dtype.ok_or(Error::Shape)?;let rank=rank.ok_or(Error::Shape)?;
        if !has_offsets||shape[..rank].contains(&0){return Err(Error::Shape);}
        let mut size=if dtype==Dtype::F32{4usize}else{2};for d in &shape[..rank]{size=size.checked_mul(*d).ok_or(Error::Bounds)?;}
        if offsets[1].checked_sub(offsets[0])!=Some(size){return Err(Error::Shape);}
        let bytes=data.get(offsets[0]..offsets[1]).ok_or(Error::Bounds)?;
        Ok(Tensor{name,dtype,shape,rank,bytes})
    }
}
impl<'a> Shard<'a>{
    // ------------------------=
    // FUNC: parse
    // DESC: Validates an entire bounded shard index before any tensor is usable; integrity hashes belong to package admission.
    // ------------------=
    pub fn parse(bytes:&'a[u8])->Result<Self,Error>{
        let len=u64::from_le_bytes(bytes.get(..8).ok_or(Error::Bounds)?.try_into().map_err(|_|Error::Bounds)?);
        let len=usize::try_from(len).map_err(|_|Error::Bounds)?;if len==0||len>MAX_HEADER{return Err(Error::Bounds);}
        let end=8usize.checked_add(len).ok_or(Error::Bounds)?;
        let shard=Self{header:bytes.get(8..end).ok_or(Error::Bounds)?,data:bytes.get(end..).ok_or(Error::Bounds)?};
        shard.visit(|_|{})?;Ok(shard)
    }
    // ------------------------=
    // FUNC: visit
    // DESC: Streams bounded descriptors without copying model weights or allocating an index.
    // ------------------=
    pub fn visit(&self,mut accept:impl FnMut(Tensor<'a>))->Result<(),Error>{
        let mut j=Json{bytes:self.header,at:0};j.require(b'{')?;let mut count=0;let mut metadata=false;
        if !j.eat(b'}'){loop{let name=j.string()?;if name.is_empty(){return Err(Error::Shape);}j.require(b':')?;
            if name==b"__metadata__"{if metadata{return Err(Error::Shape);}metadata=true;j.metadata()?;}
            else{count+=1;if count>16384{return Err(Error::Bounds);}accept(j.tensor(name,self.data)?);}
            if j.eat(b'}'){break;}j.require(b',')?;
        }}j.space();if j.at!=j.bytes.len()||count==0{return Err(Error::Shape);}Ok(())
    }
    // ------------------------=
    // FUNC: find
    // DESC: Resolves exactly one named tensor and rejects ambiguous duplicate names.
    // ------------------=
    pub fn find(&self,name:&[u8])->Result<Tensor<'a>,Error>{
        let mut value=None;let mut duplicate=false;self.visit(|t|if t.name==name{if value.is_some(){duplicate=true;}value=Some(t);})?;
        if duplicate{return Err(Error::Shape);}value.ok_or(Error::NotReady)
    }
}

#[cfg(test)]
mod tests{
    extern crate std;
    use super::*;
    // ------------------------=
    // FUNC: shard
    // DESC: Serializes a small independent safetensors fixture, without weights or downloads.
    // ------------------=
    fn shard(header:&[u8],data:&[u8])->std::vec::Vec<u8>{let mut bytes=std::vec::Vec::from((header.len()as u64).to_le_bytes());bytes.extend_from_slice(header);bytes.extend_from_slice(data);bytes}
    #[test]
    // ------------------------=
    // FUNC: real_header_and_tensor_arithmetic
    // DESC: Exercises an actual safetensors BF16 tensor through the native linear kernel.
    // ------------------=
    fn real_header_and_tensor_arithmetic(){
        let bytes=shard(br#"{"__metadata__":{"format":"pt"},"weight":{"dtype":"BF16","shape":[1,2],"data_offsets":[0,4]}}"#,&[0x80,0x3f,0,0x40]);
        let s=Shard::parse(&bytes).unwrap();let t=s.find(b"weight").unwrap();assert_eq!(t.rank,2);assert_eq!(&t.shape[..2],&[1,2]);
        let mut out=[0.];crate::tensor::linear_bf16(&[3.,4.],t.bytes,None,&mut out).unwrap();assert_eq!(out,[11.]);
    }
    #[test]
    // ------------------------=
    // FUNC: malformed_shards_are_rejected
    // DESC: Checks truncation, missing fields, duplicate fields, overflow and offset/shape disagreement.
    // ------------------=
    fn malformed_shards_are_rejected(){
        for header in [
            br#"{"w":{"dtype":"BF16","shape":[2],"data_offsets":[0,2]}}"#.as_slice(),
            br#"{"w":{"dtype":"BF16","dtype":"F32","shape":[1],"data_offsets":[0,2]}}"#,
            br#"{"w":{"dtype":"BF16","shape":[18446744073709551615,8],"data_offsets":[0,2]}}"#,
            br#"{"w":{"shape":[1],"data_offsets":[0,2]}}"#,
        ]{assert!(Shard::parse(&shard(header,&[0,0])).is_err());}
        let bytes=shard(br#"{"w":{"dtype":"BF16","shape":[1],"data_offsets":[0,2]}}"#,&[0,0]);
        for n in 0..bytes.len(){assert!(Shard::parse(&bytes[..n]).is_err());}
    }
}
