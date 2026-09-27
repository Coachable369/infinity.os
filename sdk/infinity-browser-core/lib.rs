//! Engine-independent lifecycle logic. No renderer, network authority, or host IO.
#![no_std]

const QUEUE_SIZE: usize = 32;
pub mod download;
pub mod layout;
pub mod skin;
pub mod worker;
pub mod mailbox;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Navigation(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure { Dns, Connection, Tls, Http(u16), Engine, Unsupported, Timeout }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Idle, Loading, Ready, Failed(Failure), Closed }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Closed, Full, Stale, Invalid, Exhausted }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Pointer { x: u32, y: u32, buttons: u8 },
    Scroll { x: i32, y: i32 },
    Key { code: u32, pressed: bool, modifiers: u8 },
    Text(char),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Work { pub navigation: Navigation, pub input: Input }

pub struct Session {
    generation: u64,
    phase: Phase,
    deadline: u64,
    queue: [Option<Work>; QUEUE_SIZE],
    head: usize,
    length: usize,
}

impl Session {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an inert session with no implicit network or site authority.
    // ------------------=
    pub const fn new() -> Self {
        Self { generation: 0, phase: Phase::Idle, deadline: 0,
               queue: [None; QUEUE_SIZE], head: 0, length: 0 }
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Supersedes pending navigation and discards inputs targeting its old document.
    // ------------------=
    pub fn begin(&mut self, now: u64, timeout: u64) -> Result<Navigation, Error> {
        if self.phase == Phase::Closed { return Err(Error::Closed); }
        let deadline = now.checked_add(timeout).filter(|_| timeout != 0).ok_or(Error::Invalid)?;
        let generation = self.generation.checked_add(1).ok_or(Error::Exhausted)?;
        self.clear();
        self.generation = generation;
        self.deadline = deadline;
        self.phase = Phase::Loading;
        Ok(Navigation(generation))
    }

    // ------------------------=
    // FUNC: accepts
    // DESC: Rejects late engine/network messages after supersession, failure, or close.
    // ------------------=
    pub fn accepts(&self, navigation: Navigation) -> bool {
        navigation.0 == self.generation && matches!(self.phase, Phase::Loading | Phase::Ready)
    }

    // ------------------------=
    // FUNC: complete
    // DESC: Applies only the active loading navigation's terminal result.
    // ------------------=
    pub fn complete(&mut self, navigation: Navigation, result: Result<(), Failure>, now: u64)
        -> Result<(), Error> {
        self.tick(now);
        if !self.accepts(navigation) || self.phase != Phase::Loading { return Err(Error::Stale); }
        self.phase = match result { Ok(()) => Phase::Ready, Err(error) => Phase::Failed(error) };
        if result.is_err() { self.clear(); }
        Ok(())
    }

    // ------------------------=
    // FUNC: tick
    // DESC: Expires navigation without accepting a late completion as success.
    // ------------------=
    pub fn tick(&mut self, now: u64) {
        if self.phase == Phase::Loading && now >= self.deadline {
            self.phase = Phase::Failed(Failure::Timeout);
            self.clear();
        }
    }

    // ------------------------=
    // FUNC: enqueue
    // DESC: Applies explicit backpressure rather than allocating or silently losing input.
    // ------------------=
    pub fn enqueue(&mut self, navigation: Navigation, input: Input) -> Result<(), Error> {
        if !self.accepts(navigation) { return Err(Error::Stale); }
        if self.length == QUEUE_SIZE { return Err(Error::Full); }
        self.queue[(self.head + self.length) % QUEUE_SIZE] = Some(Work { navigation, input });
        self.length += 1;
        Ok(())
    }

    // ------------------------=
    // FUNC: take
    // DESC: Consumes one input so the caller can enforce its own per-tick work budget.
    // ------------------=
    pub fn take(&mut self) -> Option<Work> {
        if self.length == 0 { return None; }
        let result = self.queue[self.head].take();
        self.head = (self.head + 1) % QUEUE_SIZE;
        self.length -= 1;
        result
    }

    // ------------------------=
    // FUNC: close
    // DESC: Invalidates active results and drops all queued inputs; closure is irreversible.
    // ------------------=
    pub fn close(&mut self) { self.phase = Phase::Closed; self.clear(); }

    // ------------------------=
    // FUNC: phase
    // DESC: Returns structured shell state without exposing raw engine error strings.
    // ------------------=
    pub fn phase(&self) -> Phase { self.phase }

    // ------------------------=
    // FUNC: clear
    // DESC: Releases every queue slot and restores an empty ring.
    // ------------------=
    fn clear(&mut self) { self.queue = [None; QUEUE_SIZE]; self.head = 0; self.length = 0; }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport { pub x: i32, pub y: i32, pub width: u32, pub height: u32 }
impl Viewport {
    // ------------------------=
    // FUNC: local
    // DESC: Maps screen pixels into content coordinates with checked arithmetic and clipping.
    // ------------------=
    pub fn local(&self, x: i32, y: i32) -> Option<(u32, u32)> {
        let x = u32::try_from(i64::from(x) - i64::from(self.x)).ok()?;
        let y = u32::try_from(i64::from(y) - i64::from(self.y)).ok()?;
        (x < self.width && y < self.height).then_some((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: supersession_close_and_timeout_reject_late_results
    // DESC: Exercises lifecycle races without a simulated engine or text oracle.
    // ------------------=
    #[test]
    fn supersession_close_and_timeout_reject_late_results() {
        let mut session = Session::new();
        let old = session.begin(0, 100).unwrap();
        session.enqueue(old, Input::Text('x')).unwrap();
        let current = session.begin(10, 100).unwrap();
        assert!(session.take().is_none());
        assert_eq!(session.complete(old, Ok(()), 20), Err(Error::Stale));
        assert_eq!(session.complete(current, Ok(()), 110), Err(Error::Stale));
        assert_eq!(session.phase(), Phase::Failed(Failure::Timeout));
        let current = session.begin(120, 100).unwrap();
        session.close();
        assert_eq!(session.complete(current, Ok(()), 130), Err(Error::Stale));
        assert_eq!(session.begin(140, 10), Err(Error::Closed));
    }
    // ------------------------=
    // FUNC: queue_is_bounded_ordered_and_reusable
    // DESC: Checks wraparound and backpressure without dropping accepted inputs.
    // ------------------=
    #[test]
    fn queue_is_bounded_ordered_and_reusable() {
        let mut session = Session::new();
        let navigation = session.begin(0, 100).unwrap();
        for _ in 0..3 {
            for code in 0..32 {
                session.enqueue(navigation, Input::Key { code, pressed: true, modifiers: 0 }).unwrap();
            }
            assert_eq!(session.enqueue(navigation, Input::Text('x')), Err(Error::Full));
            for code in 0..32 {
                assert_eq!(session.take().unwrap().input, Input::Key { code, pressed: true, modifiers: 0 });
            }
            assert!(session.take().is_none());
        }
        session.complete(navigation, Ok(()), 50).unwrap();
        session.tick(1000);
        assert_eq!(session.phase(), Phase::Ready);
    }
    // ------------------------=
    // FUNC: viewport_clips_chrome_and_extreme_coordinates
    // DESC: Ensures pointer translation cannot wrap or leak events outside web content.
    // ------------------=
    #[test]
    fn viewport_clips_chrome_and_extreme_coordinates() {
        let viewport = Viewport { x: 100, y: 80, width: 640, height: 480 };
        assert_eq!(viewport.local(100, 80), Some((0, 0)));
        assert_eq!(viewport.local(739, 559), Some((639, 479)));
        assert_eq!(viewport.local(740, 559), None);
        assert_eq!(viewport.local(100, 79), None);
        assert_eq!(viewport.local(i32::MIN, i32::MAX), None);
    }
}
