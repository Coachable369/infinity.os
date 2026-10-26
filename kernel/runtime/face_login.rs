//! One-use kernel proof. Neither applications nor the face worker can construct it.
use super::identity::StableId;
// ------------------------=
// FUNC: greeting
// DESC: Accepts only the two exact login greetings, ignoring case and terminal punctuation; never accepts arbitrary OS commands.
// ------------------=
pub fn greeting(bytes:&[u8])->bool {
    let Ok(text)=core::str::from_utf8(bytes) else{return false;};
    let mut words=text.trim_matches(|c:char|c.is_whitespace()||matches!(c,'.'|'!'|'?')).split_whitespace();
    let (Some(a),Some(b),None)=(words.next(),words.next(),words.next()) else{return false;};
    (a.eq_ignore_ascii_case("hello")&&b.eq_ignore_ascii_case("infinity")) || (a.eq_ignore_ascii_case("infinity")&&b.eq_ignore_ascii_case("hello"))
}
pub struct Proof { account:StableId, expires:u64 }
impl Proof {
    // ------------------------=
    // FUNC: new
    // DESC: Mints a short-lived proof only inside the trusted runtime after the fixed worker has completed fresh comparisons.
    // ------------------=
    pub(super) fn new(account:StableId,now:u64)->Self {Self{account,expires:now.saturating_add(5000)}}
    // ------------------------=
    // FUNC: consume
    // DESC: Consumes the non-cloneable result and rejects expiration or account switching.
    // ------------------=
    pub(super) fn consume(self,account:StableId,now:u64)->bool {self.account==account && now<self.expires && now>=self.expires.saturating_sub(5000)}
}
