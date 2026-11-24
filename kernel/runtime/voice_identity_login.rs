//! One-use ordered biometric proof: voice first, then a fresh face comparison.
use super::identity::StableId;
pub struct Pending{account:StableId,started:u64,expires:u64,voice_verified:Option<u64>,greeting_capture:Option<u64>}
pub struct Proof{account:StableId,issued:u64}
impl Pending{
    // ------------------------=
    // FUNC: begin
    // DESC: Binds an explicit voice-first attempt to one account and a bounded deadline without granting session authority.
    // ------------------=
    pub(super) fn begin(account:StableId,now:u64)->Self{
        Self{account,started:now,expires:now.saturating_add(90_000),voice_verified:None,greeting_capture:None}
    }
    // ------------------------=
    // FUNC: valid
    // DESC: Rejects selected-account changes, clock regression and expired two-stage attempts.
    // ------------------=
    pub(super) fn valid(&self,account:StableId,now:u64)->bool{account==self.account&&now>=self.started&&now<self.expires}
    // ------------------------=
    // FUNC: accept_greeting
    // DESC: Binds either login greeting to the same fresh recording that must subsequently pass speaker matching.
    // ------------------=
    pub(super) fn accept_greeting(&mut self,account:StableId,captured:u64,now:u64,text:&[u8])->bool{
        if !self.valid(account,now)||self.greeting_capture.is_some()||captured<=self.started||captured>now||!super::face_login::greeting(text){return false;}
        self.greeting_capture=Some(captured);true
    }
    // ------------------------=
    // FUNC: accept_voice
    // DESC: Records one trusted speaker match from audio captured after this attempt began; does not issue a login proof.
    // ------------------=
    pub(super) fn accept_voice(&mut self,account:StableId,captured:u64,now:u64)->bool{
        if self.greeting_capture!=Some(captured)||!self.valid(account,now)||self.voice_verified.is_some()||captured<=self.started||captured>now{return false;}
        self.voice_verified=Some(now);true
    }
    // ------------------------=
    // FUNC: complete
    // DESC: Consumes voice authority only with a fresh same-account face proof issued after speaker verification.
    // ------------------=
    pub(super) fn complete(self,account:StableId,face:super::face_login::Proof,now:u64)->Option<Proof>{
        (self.valid(account,now)&&self.voice_verified.is_some_and(|verified|face.consume_after(account,verified,now)))
            .then_some(Proof{account,issued:now})
    }
}
impl Proof{
    // ------------------------=
    // FUNC: consume
    // DESC: Consumes a short-lived combined proof once and rejects account substitution or stale delivery.
    // ------------------=
    pub(super) fn consume(self,account:StableId,now:u64)->bool{self.account==account&&now>=self.issued&&now-self.issued<5000}
}
