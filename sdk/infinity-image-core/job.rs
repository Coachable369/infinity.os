//! Session-owned, bounded image work. This is not a model implementation or a readiness claim.
use crate::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Idle, Encoding, Denoising, Decoding, Saving, Complete, Cancelled, Failed }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub user: [u8;16],
    pub session: [u8;16],
    pub width: u16,
    pub height: u16,
    pub steps: u16,
    pub seed: u64,
    pub deadline_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub owner: [u8;16],
    pub object: [u8;16],
    pub bytes: u64,
    pub width: u16,
    pub height: u16,
}

pub struct Job {
    request: Option<Request>,
    phase: Phase,
    completed_steps: u16,
    receipt: Option<Receipt>,
    failure: Option<Error>,
}

impl Job {
    // ------------------------=
    // FUNC: new
    // DESC: Creates isolated image job state without acquiring or modifying language-model resources.
    // ------------------=
    pub const fn new() -> Self {Self{request:None,phase:Phase::Idle,completed_steps:0,receipt:None,failure:None}}
    // ------------------------=
    // FUNC: start
    // DESC: Admits a bounded request only after the native caller verifies image weights and reserves independent memory.
    // ------------------=
    pub fn start(&mut self,request:Request,now:u64,required_bytes:u64,reserved_bytes:u64,weights_verified:bool)->Result<(),Error>{
        if matches!(self.phase,Phase::Encoding|Phase::Denoising|Phase::Decoding|Phase::Saving){return Err(Error::Busy);}
        if request.user==[0;16]||request.session==[0;16]{return Err(Error::Unauthorized);}
        if request.width<256||request.height<256||request.width>2048||request.height>2048
            ||request.width%16!=0||request.height%16!=0||request.steps==0||request.steps>100{return Err(Error::Shape);}
        if !weights_verified{return Err(Error::NotReady);}
        if required_bytes==0||reserved_bytes<required_bytes||request.deadline_ms<=now{return Err(Error::Budget);}
        self.request=Some(request);self.phase=Phase::Encoding;self.completed_steps=0;self.receipt=None;self.failure=None;Ok(())
    }
    // ------------------------=
    // FUNC: phase
    // DESC: Returns factual lifecycle state for asynchronous progress presentation.
    // ------------------=
    pub const fn phase(&self)->Phase{self.phase}
    // ------------------------=
    // FUNC: completed_steps
    // DESC: Reports only denoising iterations acknowledged after successful numerical integration.
    // ------------------=
    pub const fn completed_steps(&self)->u16{self.completed_steps}
    // ------------------------=
    // FUNC: receipt
    // DESC: Exposes a persisted image only after native storage has returned a valid owned-object receipt.
    // ------------------=
    pub const fn receipt(&self)->Option<Receipt>{self.receipt}
    // ------------------------=
    // FUNC: failure
    // DESC: Preserves the terminal structured error for diagnostics and user reporting.
    // ------------------=
    pub const fn failure(&self)->Option<Error>{self.failure}
    // ------------------------=
    // FUNC: check
    // DESC: Revokes work at bounded scheduler yield points on session loss or deadline expiration.
    // ------------------=
    pub fn check(&mut self,session:[u8;16],authorized:bool,now:u64)->Result<(),Error>{
        let r=self.request.ok_or(Error::State)?;
        if !matches!(self.phase,Phase::Encoding|Phase::Denoising|Phase::Decoding|Phase::Saving){return Err(Error::State);}
        if !authorized||session!=r.session{self.fail(Error::Unauthorized);return Err(Error::Unauthorized);}
        if now>=r.deadline_ms{self.fail(Error::Budget);return Err(Error::Budget);}
        Ok(())
    }
    // ------------------------=
    // FUNC: encoded
    // DESC: Advances only a completed text-encoding stage; failed stages cannot resume implicitly.
    // ------------------=
    pub fn encoded(&mut self)->Result<(),Error>{if self.phase!=Phase::Encoding{return Err(Error::State);}self.phase=Phase::Denoising;Ok(())}
    // ------------------------=
    // FUNC: denoised
    // DESC: Acknowledges exactly one successful Euler iteration and advances after the bounded step count.
    // ------------------=
    pub fn denoised(&mut self)->Result<(),Error>{
        if self.phase!=Phase::Denoising{return Err(Error::State);}self.completed_steps+=1;
        if self.completed_steps==self.request.ok_or(Error::State)?.steps{self.phase=Phase::Decoding;}Ok(())
    }
    // ------------------------=
    // FUNC: decoded
    // DESC: Rejects decoder output whose dimensions differ from the admitted image before storage is invoked.
    // ------------------=
    pub fn decoded(&mut self,width:u16,height:u16)->Result<(),Error>{
        if self.phase!=Phase::Decoding{return Err(Error::State);}let r=self.request.ok_or(Error::State)?;
        if width!=r.width||height!=r.height{self.fail(Error::Shape);return Err(Error::Shape);}self.phase=Phase::Saving;Ok(())
    }
    // ------------------------=
    // FUNC: saved
    // DESC: Requires a nonempty committed image owned by the initiating user before reporting completion.
    // ------------------=
    pub fn saved(&mut self,receipt:Receipt)->Result<(),Error>{
        if self.phase!=Phase::Saving{return Err(Error::State);}let r=self.request.ok_or(Error::State)?;
        if receipt.owner!=r.user{self.fail(Error::Unauthorized);return Err(Error::Unauthorized);}
        if receipt.object==[0;16]||receipt.bytes==0||receipt.width!=r.width||receipt.height!=r.height{self.fail(Error::Shape);return Err(Error::Shape);}
        self.receipt=Some(receipt);self.phase=Phase::Complete;Ok(())
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Cancels only the exact owner's in-flight request; the caller releases image resources at its next yield.
    // ------------------=
    pub fn cancel(&mut self,session:[u8;16])->Result<(),Error>{
        if self.request.ok_or(Error::State)?.session!=session{return Err(Error::Unauthorized);}
        if !matches!(self.phase,Phase::Encoding|Phase::Denoising|Phase::Decoding|Phase::Saving){return Err(Error::State);}
        self.phase=Phase::Cancelled;self.failure=Some(Error::Cancelled);Ok(())
    }
    // ------------------------=
    // FUNC: fail
    // DESC: Terminates image work without retrying, claiming success or touching any conversational model state.
    // ------------------=
    pub fn fail(&mut self,error:Error){self.phase=Phase::Failed;self.failure=Some(error);self.receipt=None;}
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: request
    // DESC: Constructs a deterministic host fixture without loading models or accessing a network.
    // ------------------=
    fn request()->Request{Request{user:[1;16],session:[2;16],width:256,height:256,steps:2,seed:7,deadline_ms:100}}
    #[test]
    // ------------------------=
    // FUNC: completion_requires_owned_storage_evidence
    // DESC: Exercises the complete lifecycle and rejects early, duplicate and foreign-owner completion.
    // ------------------=
    fn completion_requires_owned_storage_evidence(){
        let mut j=Job::new();j.start(request(),0,10,10,true).unwrap();
        let r=Receipt{owner:[1;16],object:[3;16],bytes:20,width:256,height:256};
        assert_eq!(j.saved(r),Err(Error::State));j.encoded().unwrap();j.denoised().unwrap();assert_eq!(j.phase(),Phase::Denoising);
        j.denoised().unwrap();assert_eq!(j.completed_steps(),2);j.decoded(256,256).unwrap();j.saved(r).unwrap();assert_eq!(j.receipt(),Some(r));
        assert_eq!(j.saved(r),Err(Error::State));
        j.start(request(),0,10,10,true).unwrap();j.encoded().unwrap();j.denoised().unwrap();j.denoised().unwrap();j.decoded(256,256).unwrap();
        assert_eq!(j.saved(Receipt{owner:[9;16],..r}),Err(Error::Unauthorized));assert_eq!(j.receipt(),None);
    }
    #[test]
    // ------------------------=
    // FUNC: limits_cancellation_and_session_loss
    // DESC: Tests unavailable weights, resource admission, cancellation isolation and expired authority.
    // ------------------=
    fn limits_cancellation_and_session_loss(){
        let mut j=Job::new();assert_eq!(j.start(request(),0,10,10,false),Err(Error::NotReady));
        assert_eq!(j.start(request(),0,10,9,true),Err(Error::Budget));assert_eq!(j.phase(),Phase::Idle);
        j.start(request(),0,10,10,true).unwrap();assert_eq!(j.start(request(),0,10,10,true),Err(Error::Busy));
        assert_eq!(j.cancel([9;16]),Err(Error::Unauthorized));j.cancel([2;16]).unwrap();assert_eq!(j.encoded(),Err(Error::State));
        j.start(request(),0,10,10,true).unwrap();assert_eq!(j.check([2;16],true,100),Err(Error::Budget));assert_eq!(j.failure(),Some(Error::Budget));
        j.start(request(),0,10,10,true).unwrap();assert_eq!(j.check([3;16],true,1),Err(Error::Unauthorized));assert_eq!(j.phase(),Phase::Failed);
    }
}
