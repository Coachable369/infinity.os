//! One-use combined biometric proof; face verification cannot itself open a voice-protected session.
use super::identity::StableId;
pub struct Pending{account:StableId,started:u64,expires:u64}
pub struct Proof{account:StableId,issued:u64}
impl Pending{
    // ------------------------=
    // FUNC: begin
    // DESC: Consumes a fresh face proof and binds the subsequent voice recording to this account and a bounded deadline.
    // ------------------=
    pub(super) fn begin(account:StableId,face:super::face_login::Proof,now:u64)->Option<Self>{
        face.consume(account,now).then_some(Self{account,started:now,expires:now.saturating_add(90_000)})
    }
    // ------------------------=
    // FUNC: valid
    // DESC: Rejects selected-account changes, clock regression and expired two-stage attempts.
    // ------------------=
    pub(super) fn valid(&self,account:StableId,now:u64)->bool{account==self.account&&now>=self.started&&now<self.expires}
    // ------------------------=
    // FUNC: complete
    // DESC: Consumes pending face authority only after a later matching voice capture from the trusted native workflow.
    // ------------------=
    pub(super) fn complete(self,account:StableId,captured:u64,now:u64)->Option<Proof>{
        (self.valid(account,now)&&captured>self.started&&captured<=now).then_some(Proof{account,issued:now})
    }
}
impl Proof{
    // ------------------------=
    // FUNC: consume
    // DESC: Consumes a short-lived combined proof once and rejects account substitution or stale delivery.
    // ------------------=
    pub(super) fn consume(self,account:StableId,now:u64)->bool{self.account==account&&now>=self.issued&&now-self.issued<5000}
}
