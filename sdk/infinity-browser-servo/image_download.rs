//! Explicit user downloads use the same native transport, never host filesystem APIs.
use super::{Provider,Event,download,compression};
use std::{string::String,vec::Vec};

pub struct Transfer {url:String,id:u64,deadline:u64,redirects:u8,body:Vec<u8>,
    metadata:Option<download::Metadata>,encoded:bool,location:Option<String>,expected:Option<usize>}
impl Transfer {
    // ------------------------=
    // FUNC: new
    // DESC: Admits only network images selected by an explicit native user gesture.
    // ------------------=
    pub fn new(url:String)->Result<Self,()> {
        let parsed=servo::ServoUrl::parse(&url).map_err(|_|())?;
        if url.len()>2048 || !matches!(parsed.scheme(),"https"|"http") || !parsed.username().is_empty() || parsed.password().is_some(){return Err(());}
        Ok(Self{url,id:0,deadline:0,redirects:0,body:Vec::new(),metadata:None,encoded:false,location:None,expected:None})
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Releases the request and staged bytes without publishing a partial file.
    // ------------------=
    pub fn cancel<P:Provider>(&mut self,p:&mut P){if self.id!=0 {p.cancel(self.id);self.id=0;}self.body.clear();}
    // ------------------------=
    // FUNC: head
    // DESC: Validates response MIME, negotiated encoding and bounded redirect metadata.
    // ------------------=
    fn head(&mut self,status:u16,headers:&[(String,String)])->Result<(),()> {
        self.encoded=compression::gzip(headers)?;
        self.expected=None;
        for (_,value) in headers.iter().filter(|(n,_)|n.eq_ignore_ascii_case("content-length")) {
            let length=value.trim().parse::<usize>().map_err(|_|())?;
            if length>download::MAX_IMAGE_BYTES || self.expected.is_some_and(|old|old!=length){return Err(());}
            self.expected=Some(length);
        }
        if matches!(status,301|302|303|307|308){
            self.location=headers.iter().find(|(n,_)|n.eq_ignore_ascii_case("location")).map(|(_,v)|v.clone());
            return self.location.as_ref().map(|_|()).ok_or(());
        }
        if !(200..300).contains(&status){return Err(());}
        let media=headers.iter().find(|(n,_)|n.eq_ignore_ascii_case("content-type")).map(|(_,v)|v.as_str()).ok_or(())?;
        self.metadata=Some(download::image_metadata(&self.url,media).map_err(|_|())?);Ok(())
    }
    // ------------------------=
    // FUNC: append
    // DESC: Bounds untrusted response accumulation before allocating another body chunk.
    // ------------------=
    fn append(&mut self,bytes:&[u8])->Result<(),()> {
        let length=self.body.len().saturating_add(bytes.len());
        if length>download::MAX_IMAGE_BYTES || self.expected.is_some_and(|n|length>n){return Err(());}
        self.body.extend_from_slice(bytes);Ok(())
    }
    // ------------------------=
    // FUNC: pump
    // DESC: Advances one transport event; returns terminal success only after the native staging callback accepts complete bytes.
    // ------------------=
    pub fn pump<P:Provider>(&mut self,p:&mut P,now:u64)->Result<bool,()> {
        if self.id==0 {self.id=p.begin_with_headers(&self.url,b"Accept: image/png,image/jpeg,image/webp,image/gif,image/svg+xml\r\nAccept-Encoding: gzip\r\n")?;self.deadline=now.saturating_add(120_000_000_000);}
        if self.id==0 || now>=self.deadline{return Err(());}
        let Some(event)=p.poll_stream(self.id)? else{return Ok(false);};
        match event {
            Event::Head(status,headers)=>{self.head(status,&headers)?;return Ok(false);},
            Event::Data(bytes)=>{self.append(&bytes)?;p.download_progress(4,self.body.len(),"");return Ok(false);},
            Event::Complete(response)=>{self.head(response.status,&response.headers)?;self.append(&response.body)?;},
            Event::Done=>{},
        }
        p.cancel(self.id);self.id=0;
        if self.expected.is_some_and(|n|n!=self.body.len()){return Err(());}
        if let Some(location)=self.location.take(){
            if self.redirects>=5{return Err(());}self.redirects+=1;
            let next=servo::ServoUrl::parse(&self.url).map_err(|_|())?.into_url().join(&location).map_err(|_|())?;
            let next=Self::new(next.into())?;
            self.url=next.url;self.body.clear();self.metadata=None;self.expected=None;return Ok(false);
        }
        if self.encoded {self.body=compression::decode(&self.body,download::MAX_IMAGE_BYTES)?;}
        let metadata=self.metadata.as_ref().ok_or(())?;
        if self.body.is_empty(){return Err(());}
        p.download(metadata,&self.body)?;Ok(true)
    }
}
