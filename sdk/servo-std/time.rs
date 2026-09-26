use crate::time::Duration;
unsafe extern "C" {
    fn infinity_std_clock(clock: u32, seconds: *mut u64, nanoseconds: *mut u32) -> i32;
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Instant(Duration);
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct SystemTime(Duration);
pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::ZERO);
// ------------------------=
// FUNC: read
// DESC: Reads native monotonic or UTC time and rejects unavailable or malformed clock results.
// ------------------=
fn read(clock: u32) -> Duration {
    let mut seconds = 0;
    let mut nanoseconds = 0;
    assert_eq!(unsafe { infinity_std_clock(clock, &mut seconds, &mut nanoseconds) }, 0);
    assert!(nanoseconds < 1_000_000_000);
    Duration::new(seconds, nanoseconds)
}
impl Instant {
    // ------------------------=
    // FUNC: now
    // DESC: Reads the monotonic clock without depending on wall-clock corrections.
    // ------------------=
    pub fn now() -> Self { Self(read(0)) }
    // ------------------------=
    // FUNC: checked_sub_instant
    // DESC: Computes elapsed time without underflow.
    // ------------------=
    pub fn checked_sub_instant(&self, other: &Self) -> Option<Duration> { self.0.checked_sub(other.0) }
    // ------------------------=
    // FUNC: checked_add_duration
    // DESC: Computes a future deadline with overflow checking.
    // ------------------=
    pub fn checked_add_duration(&self, duration: &Duration) -> Option<Self> { self.0.checked_add(*duration).map(Self) }
    // ------------------------=
    // FUNC: checked_sub_duration
    // DESC: Computes an earlier instant with underflow checking.
    // ------------------=
    pub fn checked_sub_duration(&self, duration: &Duration) -> Option<Self> { self.0.checked_sub(*duration).map(Self) }
}
impl SystemTime {
    pub const MAX: Self = Self(Duration::MAX);
    pub const MIN: Self = Self(Duration::ZERO);
    // ------------------------=
    // FUNC: now
    // DESC: Requires native UTC time, which must not be substituted with boot uptime.
    // ------------------=
    pub fn now() -> Self { Self(read(1)) }
    // ------------------------=
    // FUNC: sub_time
    // DESC: Preserves the direction of differences between UTC timestamps.
    // ------------------=
    pub fn sub_time(&self, other: &Self) -> Result<Duration, Duration> {
        self.0.checked_sub(other.0).ok_or_else(|| other.0 - self.0)
    }
    // ------------------------=
    // FUNC: checked_add_duration
    // DESC: Adds a UTC duration without overflow.
    // ------------------=
    pub fn checked_add_duration(&self, duration: &Duration) -> Option<Self> { self.0.checked_add(*duration).map(Self) }
    // ------------------------=
    // FUNC: checked_sub_duration
    // DESC: Subtracts a UTC duration without underflow.
    // ------------------=
    pub fn checked_sub_duration(&self, duration: &Duration) -> Option<Self> { self.0.checked_sub(*duration).map(Self) }
}
