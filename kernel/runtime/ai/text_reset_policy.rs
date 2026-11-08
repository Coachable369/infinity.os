//! Read-only typed-history reset admission; this module never advances or cancels audio work.
use super::{voice_conversation::State,voice_output::OutputState};
// ------------------------=
// FUNC: allows
// DESC: Permits passive listener preparation while preserving every active capture handoff, response and output lease.
// ------------------=
pub fn allows(conversation:State,output:OutputState)->bool{
    matches!(conversation,State::Off|State::Starting|State::Listening|State::Failed)
        && matches!(output,OutputState::Idle|OutputState::Failed|OutputState::Cancelled|OutputState::Complete)
}
