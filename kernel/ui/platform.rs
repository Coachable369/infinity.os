//! Frame pacing, animation policy, cache budgets, and renderer diagnostics.

pub const MAX_CACHE_ENTRIES: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MotionPreference {
    Full,
    Reduced,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QualityLevel {
    Full,
    Balanced,
    Reduced,
    Safe,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct QualityPolicy {
    pub level: QualityLevel,
    pub blur_enabled: bool,
    pub shadow_radius: u8,
    pub animation_rate_divisor: u8,
}

pub struct AdaptiveQualityController {
    level: QualityLevel,
    pressure_frames: u8,
    stable_frames: u16,
}

impl AdaptiveQualityController {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an adaptive renderer policy at full quality with bounded hysteresis counters.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            level: QualityLevel::Full,
            pressure_frames: 0,
            stable_frames: 0,
        }
    }

    // ------------------------=
    // FUNC: observe_frame
    // DESC: Degrades optional effects under sustained pressure and recovers them only after stable frames.
    // ------------------=
    pub fn observe_frame(&mut self, missed_deadline: bool, memory_pressure: bool) -> QualityPolicy {
        if missed_deadline || memory_pressure {
            self.pressure_frames = self.pressure_frames.saturating_add(1);
            self.stable_frames = 0;
            if self.pressure_frames >= 3 {
                self.level = match self.level {
                    QualityLevel::Full => QualityLevel::Balanced,
                    QualityLevel::Balanced => QualityLevel::Reduced,
                    QualityLevel::Reduced | QualityLevel::Safe => QualityLevel::Safe,
                };
                self.pressure_frames = 0;
            }
        } else {
            self.pressure_frames = 0;
            self.stable_frames = self.stable_frames.saturating_add(1);
            if self.stable_frames >= 120 {
                self.level = match self.level {
                    QualityLevel::Safe => QualityLevel::Reduced,
                    QualityLevel::Reduced => QualityLevel::Balanced,
                    QualityLevel::Balanced | QualityLevel::Full => QualityLevel::Full,
                };
                self.stable_frames = 0;
            }
        }
        self.policy()
    }

    // ------------------------=
    // FUNC: policy
    // DESC: Projects the current quality level into deterministic optional-effect controls.
    // ------------------=
    pub const fn policy(&self) -> QualityPolicy {
        match self.level {
            QualityLevel::Full => QualityPolicy {
                level: QualityLevel::Full,
                blur_enabled: true,
                shadow_radius: 24,
                animation_rate_divisor: 1,
            },
            QualityLevel::Balanced => QualityPolicy {
                level: QualityLevel::Balanced,
                blur_enabled: true,
                shadow_radius: 12,
                animation_rate_divisor: 1,
            },
            QualityLevel::Reduced => QualityPolicy {
                level: QualityLevel::Reduced,
                blur_enabled: false,
                shadow_radius: 6,
                animation_rate_divisor: 2,
            },
            QualityLevel::Safe => QualityPolicy {
                level: QualityLevel::Safe,
                blur_enabled: false,
                shadow_radius: 0,
                animation_rate_divisor: 4,
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct FrameClock {
    pub refresh_hz: u16,
    pub frame_number: u64,
    pub next_deadline: u64,
    pub dropped_frames: u32,
}

impl FrameClock {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a monotonic frame clock with bounded refresh-rate selection.
    // ------------------=
    pub const fn new(refresh_hz: u16) -> Self {
        let bounded = if refresh_hz < 30 {
            30
        } else if refresh_hz > 240 {
            240
        } else {
            refresh_hz
        };
        Self {
            refresh_hz: bounded,
            frame_number: 0,
            next_deadline: 0,
            dropped_frames: 0,
        }
    }

    // ------------------------=
    // FUNC: advance
    // DESC: Advances frame pacing from monotonic ticks and accounts for missed presentation deadlines.
    // ------------------=
    pub fn advance(&mut self, now: u64, ticks_per_second: u64) -> u64 {
        let interval = (ticks_per_second / self.refresh_hz as u64).max(1);
        if self.next_deadline != 0 && now > self.next_deadline.saturating_add(interval) {
            self.dropped_frames = self
                .dropped_frames
                .saturating_add(((now - self.next_deadline) / interval) as u32);
        }
        self.frame_number = self.frame_number.wrapping_add(1);
        self.next_deadline = now.saturating_add(interval);
        self.next_deadline
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheKind {
    Glyph,
    Icon,
    Wallpaper,
    Surface,
}

#[derive(Clone, Copy)]
pub struct CacheBudget {
    pub total_bytes: u32,
    pub used_bytes: u32,
    pub evictions: u32,
}

impl CacheBudget {
    // ------------------------=
    // FUNC: reserve
    // DESC: Reserves bounded cache memory and requests eviction instead of unbounded growth.
    // ------------------=
    pub fn reserve(&mut self, bytes: u32) -> bool {
        if bytes > self.total_bytes.saturating_sub(self.used_bytes) {
            self.evictions = self.evictions.saturating_add(1);
            return false;
        }
        self.used_bytes += bytes;
        true
    }

    // ------------------------=
    // FUNC: release
    // DESC: Returns cache bytes after an entry is invalidated or evicted.
    // ------------------=
    pub fn release(&mut self, bytes: u32) {
        self.used_bytes = self.used_bytes.saturating_sub(bytes);
    }
}
