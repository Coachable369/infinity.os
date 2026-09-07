//! Bounded user input preferences and deterministic key-repeat timing.
use core::sync::atomic::{AtomicU64, Ordering};
static CURRENT: AtomicU64 = AtomicU64::new(0);
pub const LABELS: [&[u8]; 8] = [
    b"Pointer speed",
    b"Scroll speed",
    b"Scroll direction",
    b"Primary button",
    b"Pointer acceleration",
    b"Key repeat delay",
    b"Key repeat rate",
    b"Reset input defaults",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub speed: u8,
    pub scroll: u8,
    pub natural: bool,
    pub left_handed: bool,
    pub acceleration: bool,
    pub delay: u8,
    pub rate: u8,
}
impl Preferences {
    // ------------------------=
    // FUNC: defaults
    // DESC: Returns conservative desktop defaults compatible with existing profiles.
    // ------------------=
    pub const fn defaults() -> Self {
        Self {
            speed: 4,
            scroll: 3,
            natural: false,
            left_handed: false,
            acceleration: true,
            delay: 1,
            rate: 1,
        }
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Validates persisted preferences; pre-feature records use defaults.
    // ------------------=
    pub fn decode(bytes: [u8; 8]) -> Self {
        if bytes[0] != 0xa1
            || !(1..=10).contains(&bytes[1])
            || !(1..=10).contains(&bytes[2])
            || bytes[3] > 7
            || bytes[4] > 3
            || bytes[5] > 3
        {
            return Self::defaults();
        }
        Self {
            speed: bytes[1],
            scroll: bytes[2],
            natural: bytes[3] & 1 != 0,
            left_handed: bytes[3] & 2 != 0,
            acceleration: bytes[3] & 4 != 0,
            delay: bytes[4],
            rate: bytes[5],
        }
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes input preferences in the reserved desktop-profile bytes without changing its record size.
    // ------------------=
    pub fn encode(self) -> [u8; 8] {
        [
            0xa1,
            self.speed,
            self.scroll,
            self.natural as u8 | (self.left_handed as u8) << 1 | (self.acceleration as u8) << 2,
            self.delay,
            self.rate,
            0,
            0,
        ]
    }
    // ------------------------=
    // FUNC: cycle
    // DESC: Applies the next bounded value for an activated Settings row.
    // ------------------=
    pub fn cycle(&mut self, row: usize) {
        match row {
            0 => self.speed = self.speed % 10 + 1,
            1 => self.scroll = self.scroll % 10 + 1,
            2 => self.natural = !self.natural,
            3 => self.left_handed = !self.left_handed,
            4 => self.acceleration = !self.acceleration,
            5 => self.delay = (self.delay + 1) % 4,
            6 => self.rate = (self.rate + 1) % 4,
            _ => *self = Self::defaults(),
        }
    }
    // ------------------------=
    // FUNC: value
    // DESC: Returns the real active value, not a placeholder setting label.
    // ------------------=
    pub fn value(self, row: usize) -> &'static [u8] {
        const LEVEL: [&[u8]; 10] = [b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9", b"10"];
        match row {
            0 => LEVEL[self.speed as usize - 1],
            1 => LEVEL[self.scroll as usize - 1],
            2 => {
                if self.natural {
                    b"Natural"
                } else {
                    b"Traditional"
                }
            }
            3 => {
                if self.left_handed {
                    b"Right"
                } else {
                    b"Left"
                }
            }
            4 => {
                if self.acceleration {
                    b"On"
                } else {
                    b"Off"
                }
            }
            5 => [b"250 ms".as_slice(), b"400 ms", b"600 ms", b"900 ms"][self.delay as usize],
            6 => [
                b"15 / sec".as_slice(),
                b"25 / sec",
                b"35 / sec",
                b"50 / sec",
            ][self.rate as usize],
            _ => b"Reset",
        }
    }
    // ------------------------=
    // FUNC: wheel
    // DESC: Preserves wheel magnitude and applies the selected scroll speed and direction.
    // ------------------=
    pub fn wheel(self, amount: i8) -> i32 {
        amount as i32 * self.scroll as i32 * if self.natural { -1 } else { 1 }
    }
    // ------------------------=
    // FUNC: buttons
    // DESC: Maps physical primary/secondary buttons consistently for relative and absolute devices.
    // ------------------=
    pub fn buttons(self, value: u8) -> u8 {
        if self.left_handed {
            value & !3 | (value & 1) << 1 | (value & 2) >> 1
        } else {
            value
        }
    }
    // ------------------------=
    // FUNC: delay_ms
    // DESC: Supplies the selected initial repeat delay.
    // ------------------=
    pub fn delay_ms(self) -> u64 {
        [250, 400, 600, 900][self.delay as usize]
    }
    // ------------------------=
    // FUNC: interval_ms
    // DESC: Supplies the selected key-repeat interval.
    // ------------------=
    pub fn interval_ms(self) -> u64 {
        1000 / [15, 25, 35, 50][self.rate as usize]
    }
}
// ------------------------=
// FUNC: current
// DESC: Reads a coherent preference snapshot without service calls in input or paint paths.
// ------------------=
pub fn current() -> Preferences {
    Preferences::decode(CURRENT.load(Ordering::Relaxed).to_le_bytes())
}
// ------------------------=
// FUNC: apply
// DESC: Publishes validated preferences immediately to input handling.
// ------------------=
pub fn apply(preferences: Preferences) {
    CURRENT.store(
        u64::from_le_bytes(Preferences::decode(preferences.encode()).encode()),
        Ordering::Relaxed,
    );
}

pub struct Repeat {
    held: Option<(u16, u8)>,
    due: u64,
}
impl Repeat {
    // ------------------------=
    // FUNC: new
    // DESC: Starts a released keyboard repeat state.
    // ------------------=
    pub const fn new() -> Self {
        Self { held: None, due: 0 }
    }
    // ------------------------=
    // FUNC: press
    // DESC: Starts repeat only on a new press, suppressing duplicate hardware repeats.
    // ------------------=
    pub fn press(
        &mut self,
        code: u16,
        modifiers: u8,
        now_ms: u64,
        preferences: Preferences,
    ) -> bool {
        if self.held.map(|key| key.0) == Some(code) {
            return false;
        }
        self.held = Some((code, modifiers));
        self.due = now_ms.saturating_add(preferences.delay_ms());
        true
    }
    // ------------------------=
    // FUNC: release
    // DESC: Cancels repeats immediately on key-up.
    // ------------------=
    pub fn release(&mut self, code: u16) {
        if self.held.map(|key| key.0) == Some(code) {
            self.held = None;
        }
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Emits at most one due repeat, never flooding input after a slow frame.
    // ------------------=
    pub fn poll(&mut self, now_ms: u64, preferences: Preferences) -> Option<(u16, u8)> {
        if now_ms < self.due {
            return None;
        }
        let key = self.held?;
        self.due = now_ms.saturating_add(preferences.interval_ms());
        Some(key)
    }
}
