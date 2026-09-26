//! Deterministic elapsed-time choreography for successful authentication.

pub const SCENE_MS: u16 = 1_500;
pub const COMMIT_MS: u16 = 1_600;
pub const DURATION_MS: u16 = COMMIT_MS;
const IMPACT_FRAME_MS: u16 = 550;
const RIPPLE_FRAME_MS: u16 = 1_000;
const FINAL_RIPPLE_FRAME_MS: u16 = 1_300;

pub const ARTWORK_CENTER_X: u16 = 1_335;
pub const ARTWORK_WATER_Y: u16 = 865;
pub const DAMAGE_LEFT: u16 = 790;
pub const DAMAGE_TOP: u16 = 570;
pub const DAMAGE_RIGHT: u16 = 1_920;
pub const DAMAGE_BOTTOM: u16 = 1_080;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Presentation {
    pub progress: u16,
    pub orb_y_per_mille: u16,
    pub orb_opacity: u8,
    pub orb_width_per_mille: u16,
    pub orb_height_per_mille: u16,
    pub primary_ripple_scale: u16,
    pub primary_ripple_opacity: u8,
    pub secondary_ripple_scale: u16,
    pub secondary_ripple_opacity: u8,
    pub splash_scale: u16,
    pub splash_opacity: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    Idle,
    Frame(Presentation),
    CommitDesktop,
    Finished,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timeline {
    elapsed_ms: u16,
    presented_ms: u16,
    active: bool,
    committed: bool,
    checkpoint: u8,
}

impl Timeline {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an inactive authentication-success timeline.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            elapsed_ms: 0,
            presented_ms: 0,
            active: false,
            committed: false,
            checkpoint: 0,
        }
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Starts the animation from its authored resting frame.
    // ------------------=
    pub fn begin(&mut self) {
        self.elapsed_ms = 0;
        self.presented_ms = 0;
        self.active = true;
        self.committed = false;
        self.checkpoint = 0;
    }

    // ------------------------=
    // FUNC: active
    // DESC: Reports whether authentication input must remain gated by the success transition.
    // ------------------=
    pub const fn active(&self) -> bool {
        self.active
    }

    // ------------------------=
    // FUNC: presentation
    // DESC: Returns the current deterministic visual keyframe without advancing time.
    // ------------------=
    pub fn presentation(&self) -> Presentation {
        presentation_at(self.presented_ms)
    }

    // ------------------------=
    // FUNC: opacity
    // DESC: Keeps the authentication scene fully opaque through the direct desktop handoff.
    // ------------------=
    pub const fn opacity(&self) -> u8 {
        scene_opacity(self.presented_ms)
    }

    // ------------------------=
    // FUNC: advance
    // DESC: Advances by bounded elapsed milliseconds and signals the exact desktop-commit boundary.
    // ------------------=
    pub fn advance(&mut self, elapsed_ms: u16) -> Advance {
        if !self.active {
            return Advance::Idle;
        }
        // Follow real elapsed time so software-rendering stalls cannot turn the
        // authored 2.2-second transition into a long apparent freeze. Crossing
        // a checkpoint still emits its keyframe once before catching up.
        self.elapsed_ms = self.elapsed_ms.saturating_add(elapsed_ms);
        if !self.committed {
            let checkpoint_ms = match self.checkpoint {
                0 => IMPACT_FRAME_MS,
                1 => RIPPLE_FRAME_MS,
                2 => FINAL_RIPPLE_FRAME_MS,
                _ => 0,
            };
            if checkpoint_ms != 0 && self.elapsed_ms >= checkpoint_ms {
                self.checkpoint += 1;
                self.presented_ms = checkpoint_ms;
                return Advance::Frame(self.presentation());
            }
        }
        if self.elapsed_ms >= COMMIT_MS && !self.committed {
            self.elapsed_ms = COMMIT_MS;
            self.presented_ms = COMMIT_MS;
            self.committed = true;
            return Advance::CommitDesktop;
        }
        if self.elapsed_ms >= DURATION_MS {
            self.elapsed_ms = DURATION_MS;
            self.presented_ms = DURATION_MS;
            self.active = false;
            return Advance::Finished;
        }
        self.presented_ms = self.elapsed_ms;
        Advance::Frame(self.presentation())
    }
}

// ------------------------=
// FUNC: scene_opacity
// DESC: Keeps both authentication and desktop presentation fully opaque across the direct handoff.
// ------------------=
pub const fn scene_opacity(_elapsed_ms: u16) -> u8 {
    255
}

// ------------------------=
// FUNC: fade_pixel
// DESC: Fades packed RGB/BGR equally while retaining its unused or alpha byte.
// ------------------=
pub const fn fade_pixel(pixel: u32, opacity: u8) -> u32 {
    let a = opacity as u32;
    (pixel & 0xff000000)
        | ((pixel & 255) * a / 255)
        | ((((pixel >> 8) & 255) * a / 255) << 8)
        | ((((pixel >> 16) & 255) * a / 255) << 16)
}

// ------------------------=
// FUNC: lerp
// DESC: Interpolates bounded integer keyframe values without floating point.
// ------------------=
const fn lerp(from: u16, to: u16, progress: u16) -> u16 {
    if to >= from {
        from + ((to - from) as u32 * progress as u32 / 1_000) as u16
    } else {
        from - ((from - to) as u32 * progress as u32 / 1_000) as u16
    }
}

// ------------------------=
// FUNC: segment
// DESC: Normalizes one master progress interval to zero through one thousand.
// ------------------=
const fn segment(progress: u16, start: u16, end: u16) -> u16 {
    if progress <= start {
        0
    } else if progress >= end {
        1_000
    } else {
        ((progress - start) as u32 * 1_000 / (end - start) as u32) as u16
    }
}

// ------------------------=
// FUNC: ease_in_quad
// DESC: Applies a gravity-like quadratic acceleration curve in fixed point.
// ------------------=
const fn ease_in_quad(progress: u16) -> u16 {
    (progress as u32 * progress as u32 / 1_000) as u16
}

// ------------------------=
// FUNC: opacity_between
// DESC: Fades one layer between two master progress values.
// ------------------=
const fn opacity_between(progress: u16, start: u16, peak: u16, end: u16, maximum: u8) -> u8 {
    if progress <= start || progress >= end {
        0
    } else if progress < peak {
        (maximum as u32 * segment(progress, start, peak) as u32 / 1_000) as u8
    } else {
        (maximum as u32 * (1_000 - segment(progress, peak, end)) as u32 / 1_000) as u8
    }
}

// ------------------------=
// FUNC: presentation_at
// DESC: Resolves orb, splash, and dual-ripple keyframes for one elapsed time.
// ------------------=
pub const fn presentation_at(elapsed_ms: u16) -> Presentation {
    let bounded_elapsed = if elapsed_ms < SCENE_MS {
        elapsed_ms
    } else {
        SCENE_MS
    };
    let progress = ((bounded_elapsed as u32 * 1_000) / SCENE_MS as u32) as u16;
    let orb_y_per_mille = if progress < 80 {
        lerp(725, 660, segment(progress, 0, 80))
    } else if progress < 350 {
        lerp(660, 790, ease_in_quad(segment(progress, 80, 350)))
    } else if progress < 500 {
        lerp(790, 830, segment(progress, 350, 500))
    } else {
        830
    };
    let orb_opacity = if progress < 350 {
        255
    } else {
        (255u32 * (1_000 - segment(progress, 350, 560)) as u32 / 1_000) as u8
    };
    let compression = opacity_between(progress, 290, 365, 470, 255) as u16;
    let orb_width_per_mille = 1_000 + (compression as u32 * 180 / 255) as u16;
    let orb_height_per_mille = 1_000 - (compression as u32 * 300 / 255) as u16;
    let primary_ripple_scale = if progress < 310 {
        0
    } else {
        lerp(160, 1_650, segment(progress, 310, 920))
    };
    let primary_ripple_opacity = opacity_between(progress, 295, 340, 920, 235);
    let secondary_ripple_scale = if progress < 400 {
        0
    } else {
        lerp(120, 1_450, segment(progress, 400, 1_000))
    };
    Presentation {
        progress,
        orb_y_per_mille,
        orb_opacity,
        orb_width_per_mille,
        orb_height_per_mille,
        primary_ripple_scale,
        primary_ripple_opacity,
        secondary_ripple_scale,
        secondary_ripple_opacity: opacity_between(progress, 385, 435, 1_000, 200),
        splash_scale: lerp(420, 1_220, segment(progress, 285, 620)),
        splash_opacity: opacity_between(progress, 275, 350, 650, 255),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------=
    // FUNC: successful_authentication_runs_complete_motion_sequence
    // DESC: Verifies gated start, gravity, impact, expanding waves, and the exact completion transition.
    // ------------------=
    #[test]
    fn successful_authentication_runs_complete_motion_sequence() {
        let mut timeline = Timeline::new();
        assert_eq!(timeline.advance(16), Advance::Idle);
        timeline.begin();
        let resting = timeline.presentation();
        assert!(timeline.active());
        assert_eq!(resting.orb_y_per_mille, 725);
        let lifted = presentation_at(100);
        let falling = presentation_at(420);
        let impact = presentation_at(550);
        let wake = presentation_at(1_000);
        assert!(lifted.orb_y_per_mille < resting.orb_y_per_mille);
        assert!(falling.orb_y_per_mille > resting.orb_y_per_mille);
        assert!(impact.splash_opacity > 0);
        assert_eq!(presentation_at(300).primary_ripple_opacity, 0);
        assert!(wake.primary_ripple_scale > impact.primary_ripple_scale);
        assert!(wake.orb_opacity < impact.orb_opacity);
        for _ in 0..31 {
            assert!(matches!(timeline.advance(50), Advance::Frame(_)));
        }
        assert_eq!(timeline.advance(50), Advance::CommitDesktop);
        assert!(timeline.active());
        assert_eq!(timeline.advance(1), Advance::Finished);
        assert!(!timeline.active());
    }

    // ------------------------=
    // FUNC: direct_handoff_preserves_checkpoints_when_frames_stall
    // DESC: Verifies full opacity, pixel endpoints, and checkpoint-preserving delayed-frame catch-up.
    // ------------------=
    #[test]
    fn direct_handoff_preserves_checkpoints_when_frames_stall() {
        assert_eq!(scene_opacity(0), 255);
        assert_eq!(scene_opacity(900), 255);
        assert!(presentation_at(900).primary_ripple_opacity > 0);
        assert_eq!(scene_opacity(COMMIT_MS), 255);
        assert_eq!(scene_opacity(DURATION_MS), 255);
        assert_eq!(fade_pixel(0xff987654, 255), 0xff987654);
        assert_eq!(fade_pixel(0xff987654, 0), 0xff000000);
        let mut timeline = Timeline::new();
        timeline.begin();
        assert_eq!(
            timeline.advance(u16::MAX),
            Advance::Frame(presentation_at(IMPACT_FRAME_MS))
        );
        assert_eq!(
            timeline.advance(1),
            Advance::Frame(presentation_at(RIPPLE_FRAME_MS))
        );
        assert_eq!(
            timeline.advance(1),
            Advance::Frame(presentation_at(FINAL_RIPPLE_FRAME_MS))
        );
        assert_eq!(timeline.advance(1), Advance::CommitDesktop);
        assert_eq!(timeline.advance(1), Advance::Finished);
        assert!(!timeline.active());
    }
}
