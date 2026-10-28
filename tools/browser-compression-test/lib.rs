#[path="../../sdk/infinity-browser-servo/compression.rs"] pub mod compression;
extern crate self as servo;
pub struct ServoUrl(url::Url);
impl ServoUrl {
    // ------------------------=
    // FUNC: parse
    // DESC: Uses the same URL parser as Servo in the isolated transport harness.
    // ------------------=
    pub fn parse(value:&str)->Result<Self,url::ParseError>{url::Url::parse(value).map(Self)}
    // ------------------------=
    // FUNC: into_url
    // DESC: Returns the parsed URL for redirect resolution.
    // ------------------=
    pub fn into_url(self)->url::Url{self.0}
}
impl core::ops::Deref for ServoUrl {type Target=url::Url;
    // ------------------------=
    // FUNC: deref
    // DESC: Exposes parsed URL fields to production transfer code.
    // ------------------=
    fn deref(&self)->&url::Url{&self.0}
}
#[path="../../sdk/infinity-browser-core/download.rs"] pub mod download;
pub struct Response {pub status:u16,pub headers:Vec<(String,String)>,pub body:Vec<u8>}
pub enum Event {Complete(Response),Head(u16,Vec<(String,String)>),Data(Vec<u8>),Done}
pub trait Provider {
    fn begin_with_headers(&mut self,url:&str,headers:&[u8])->Result<u64,()>;
    fn poll_stream(&mut self,id:u64)->Result<Option<Event>,()>;
    fn cancel(&mut self,id:u64);
    fn download(&mut self,metadata:&download::Metadata,body:&[u8])->Result<(),()>;
    fn download_progress(&mut self,state:u32,bytes:usize,name:&str);
}
#[path="../../sdk/infinity-browser-servo/image_download.rs"] pub mod image_download;
#[cfg(test)] mod transfer_tests {
    use super::*;
    #[derive(Default)]struct Network {events:std::collections::VecDeque<Event>,saved:Vec<Vec<u8>>,cancelled:usize,begun:usize}
    impl Provider for Network {
        // ------------------------=
        // FUNC: begin_with_headers
        // DESC: Records deterministic request admission.
        // ------------------=
        fn begin_with_headers(&mut self,_:&str,_:&[u8])->Result<u64,()>{self.begun+=1;Ok(self.begun as u64)}
        // ------------------------=
        // FUNC: poll_stream
        // DESC: Supplies one fixture transport event.
        // ------------------=
        fn poll_stream(&mut self,_:u64)->Result<Option<Event>,()>{Ok(self.events.pop_front())}
        // ------------------------=
        // FUNC: cancel
        // DESC: Counts releases to verify cleanup.
        // ------------------=
        fn cancel(&mut self,_:u64){self.cancelled+=1;}
        // ------------------------=
        // FUNC: download
        // DESC: Captures only content submitted to native storage.
        // ------------------=
        fn download(&mut self,_:&download::Metadata,b:&[u8])->Result<(),()>{self.saved.push(b.to_vec());Ok(())}
        // ------------------------=
        // FUNC: download_progress
        // DESC: Ignores diagnostic copy rather than treating it as an oracle.
        // ------------------=
        fn download_progress(&mut self,_:u32,_:usize,_:&str){}
    }
    #[test]
    // ------------------------=
    // FUNC: complete_or_no_commit
    // DESC: Runs actual transfer logic through exact, truncated and oversized responses.
    // ------------------=
    fn complete_or_no_commit(){
        for length in [3,4,5] {
            let mut n=Network::default();n.events.extend([Event::Head(200,vec![("Content-Type".into(),"image/png".into()),("Content-Length".into(),"4".into())]),Event::Data(vec![7;length]),Event::Done]);
            let mut t=image_download::Transfer::new("https://example.com/image.png".into()).unwrap();
            assert_eq!(t.pump(&mut n,0),Ok(false));let second=t.pump(&mut n,1);
            let result=if second.is_ok(){t.pump(&mut n,2)}else{second};
            if length==4 {assert_eq!(result,Ok(true));assert_eq!(n.saved,vec![vec![7;4]]);}else{assert!(result.is_err());t.cancel(&mut n);assert!(n.saved.is_empty());}
            assert_eq!(n.cancelled,1);
        }
    }
    #[test]
    // ------------------------=
    // FUNC: redirect_cancel_and_deadline
    // DESC: Rejects unsafe redirects and expiry, and cancellation never publishes partial content.
    // ------------------=
    fn redirect_cancel_and_deadline(){
        let mut n=Network::default();n.events.extend([Event::Head(302,vec![("Location".into(),"file:///secret".into())]),Event::Done]);
        let mut t=image_download::Transfer::new("https://example.com/image".into()).unwrap();
        assert_eq!(t.pump(&mut n,0),Ok(false));assert!(t.pump(&mut n,1).is_err());assert!(n.saved.is_empty());
        let mut t=image_download::Transfer::new("https://example.com/image".into()).unwrap();
        assert_eq!(t.pump(&mut n,0),Ok(false));assert!(t.pump(&mut n,120_000_000_000).is_err());t.cancel(&mut n);assert!(n.saved.is_empty());
        assert!(image_download::Transfer::new("https://user:secret@example.com/image".into()).is_err());
    }
}
#[cfg(test)] mod tests {
    use super::compression::*;
    use std::io::Write;
    // ------------------------=
    // FUNC: encode
    // DESC: Creates actual gzip data for decoder boundary tests.
    // ------------------=
    fn encode(bytes:&[u8])->Vec<u8> {
        let mut out=flate2::write::GzEncoder::new(Vec::new(),flate2::Compression::default());
        out.write_all(bytes).unwrap();out.finish().unwrap()
    }
    #[test]
    // ------------------------=
    // FUNC: bounded_complete_decode
    // DESC: Verifies exact bytes, expansion bounds, truncated streams and CRC rejection.
    // ------------------=
    fn bounded_complete_decode() {
        let original=vec![b'x';100000];let packed=encode(&original);
        assert!(packed.len()<1000);
        assert_eq!(decode(&packed,original.len()).unwrap(),original);
        assert!(decode(&packed,original.len()-1).is_err());
        assert!(decode(&packed[..packed.len()-1],original.len()).is_err());
        let mut corrupt=packed.clone();let n=corrupt.len();corrupt[n-8]^=1;
        assert!(decode(&corrupt,original.len()).is_err());
        let both=[encode(b"first"),encode(b"second")].concat();
        assert_eq!(decode(&both,11).unwrap(),b"firstsecond");
    }
    #[test]
    // ------------------------=
    // FUNC: response_metadata
    // DESC: Preserves security fields while removing wire-only encoding metadata.
    // ------------------=
    fn response_metadata() {
        let mut h=vec![("Content-Encoding".into(),"gzip".into()),("Content-Length".into(),"123".into()),("Content-Security-Policy".into(),"default-src 'self'".into())];
        assert_eq!(gzip(&h),Ok(true));remove_wire_encoding(&mut h);
        assert_eq!(h.len(),1);assert_eq!(h[0].0,"Content-Security-Policy");
        assert_eq!(gzip(&h),Ok(false));
        h.push(("content-encoding".into(),"gzip, br".into()));assert!(gzip(&h).is_err());
        h.pop();h.push(("content-encoding".into(),"gzip".into()));h.push(("content-encoding".into(),"gzip".into()));
        assert!(gzip(&h).is_err());
    }
}
