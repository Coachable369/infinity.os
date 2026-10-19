//! Deterministic elapsed-time choreography for successful authentication.

pub const SCENE_MS: u16 = 2_400;
pub const COMMIT_MS: u16 = SCENE_MS;
pub const DURATION_MS: u16 = COMMIT_MS + 600;
const FADE_START_MS: u16 = 1_200;
const IMPACT_FRAME_MS: u16 = 880;
const RIPPLE_FRAME_MS: u16 = 1_500;
const FINAL_RIPPLE_FRAME_MS: u16 = 2_000;

pub const ARTWORK_CENTER_X: u16 = 1_335;
pub const ARTWORK_WATER_Y: u16 = 865;
pub const DAMAGE_LEFT: u16 = 790;
pub const DAMAGE_TOP: u16 = 570;
pub const DAMAGE_RIGHT: u16 = 1_920;
pub const DAMAGE_BOTTOM: u16 = 1_080;
pub const IDLE_PERIOD_MS: u64 = 3_200;
const IDLE_MAX_RIPPLE_SCALE: u16 = 900;

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

// ------------------------=
// FUNC: idle_presentation
// DESC: Expands one quiet water ring on a fixed elapsed-time loop without moving the resting orb or starting success choreography.
// ------------------=
pub const fn idle_presentation(elapsed_ms: u64, reduced_motion: bool) -> Presentation {
    let resting = presentation_at(0);
    if reduced_motion { return resting; }
    let progress = ((elapsed_ms % IDLE_PERIOD_MS) * 1_000 / IDLE_PERIOD_MS) as u16;
    Presentation {
        primary_ripple_scale: lerp(180, IDLE_MAX_RIPPLE_SCALE, progress),
        primary_ripple_opacity: opacity_between(progress, 0, 180, 1_000, 64),
        ..resting
    }
}

// ------------------------=
// FUNC: idle_water_regions
// DESC: Bounds old and new idle rings to water pixels and subtracts the login card and utility tray without touching branding.
// ------------------=
pub fn idle_water_regions(width: usize, height: usize) -> [(usize, usize, usize, usize); 6] {
    if width == 0 || height == 0 { return [(0, 0, 0, 0); 6]; }
    let scale = (width.saturating_mul(1_000) / 1_920)
        .max(height.saturating_mul(1_000) / 1_080).max(1);
    let offset_x = (width as i64 - (1_920 * scale / 1_000) as i64) / 2;
    let offset_y = (height as i64 - (1_080 * scale / 1_000) as i64) / 2;
    let center_x = offset_x + (ARTWORK_CENTER_X as usize * scale / 1_000) as i64;
    let water_y = offset_y + (ARTWORK_WATER_Y as usize * scale / 1_000) as i64;
    let half_width = (768 * scale / 1_000 * IDLE_MAX_RIPPLE_SCALE as usize / 1_000) / 2 + 2;
    let half_height = (344 * scale / 1_000 * IDLE_MAX_RIPPLE_SCALE as usize / 1_000) / 2 + 2;
    let left = (center_x - half_width as i64).clamp(0, width as i64) as usize;
    let top = (water_y - half_height as i64).clamp(0, height as i64) as usize;
    let right = (center_x + half_width as i64 + 1).clamp(0, width as i64) as usize;
    let bottom = (water_y + half_height as i64 + 1).clamp(0, height as i64) as usize;
    let fit = (width.saturating_mul(1_000) / 1_536)
        .min(height.saturating_mul(1_000) / 1_024).max(1);
    let ui_x = width.saturating_sub(1_536 * fit / 1_000) / 2;
    let ui_y = height.saturating_sub(1_024 * fit / 1_000) / 2;
    let card_right = ui_x + 54 * fit / 1_000 + 521 * fit / 1_000;
    let card_top = ui_y + 123 * fit / 1_000;
    let card_bottom = card_top + 754 * fit / 1_000;
    let tray_left = ui_x + 529 * fit / 1_000;
    let tray_top = ui_y + 914 * fit / 1_000;
    let tray_right = tray_left + 478 * fit / 1_000;
    let tray_bottom = tray_top + 90 * fit / 1_000;
    let mut regions = [
        (left, top, right, bottom.min(card_top)),
        (left.max(card_right), top.max(card_top), right, bottom.min(card_bottom)),
        (left, top.max(card_bottom), right, bottom.min(tray_top)),
        (left, top.max(tray_top), right.min(tray_left), bottom.min(tray_bottom)),
        (left.max(tray_right), top.max(tray_top), right, bottom.min(tray_bottom)),
        (left, top.max(tray_bottom), right, bottom),
    ];
    for region in &mut regions {
        if region.0 >= region.2 || region.1 >= region.3 { *region = (0, 0, 0, 0); }
    }
    regions
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    Idle,
    Frame(Presentation),
    DesktopFade(u8),
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
    // DESC: Returns the eased login fade-out or desktop fade-in opacity.
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
        // Bound visual catch-up so a slow render cannot skip straight from
        // impact to an invisible ripple. Audio runs independently in DMA.
        self.elapsed_ms = self.elapsed_ms.saturating_add(elapsed_ms.min(50));
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
        if self.committed { Advance::DesktopFade(self.opacity()) }
        else { Advance::Frame(self.presentation()) }
    }
}

// ------------------------=
// FUNC: scene_opacity
// DESC: Fades the rippling login scene to zero, then reveals the cached desktop with smooth endpoint slopes.
// ------------------=
pub const fn scene_opacity(elapsed_ms: u16) -> u8 {
    let t = if elapsed_ms < COMMIT_MS {
        segment(elapsed_ms, FADE_START_MS, COMMIT_MS)
    } else {
        segment(elapsed_ms, COMMIT_MS, DURATION_MS)
    } as u32;
    let eased = t * t * (3_000 - 2 * t) / 1_000_000;
    let value = (255 * eased / 1_000) as u8;
    if elapsed_ms < COMMIT_MS { 255 - value } else { value }
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
    // FUNC: idle_ripple_is_subtle_periodic_and_does_not_authenticate
    // DESC: Checks actual wave growth, smooth invisible endpoints, elapsed-time repetition and reduced-motion suppression.
    // ------------------=
    #[test]
    fn idle_ripple_is_subtle_periodic_and_does_not_authenticate() {
        let resting = presentation_at(0);
        let mut previous_scale = 0;
        let mut visible = 0;
        for time in 0..IDLE_PERIOD_MS {
            let frame = idle_presentation(time, false);
            assert!(frame.primary_ripple_scale >= previous_scale);
            assert!(frame.primary_ripple_scale <= IDLE_MAX_RIPPLE_SCALE);
            assert!(frame.primary_ripple_opacity <= 64);
            assert_eq!(frame.orb_y_per_mille, resting.orb_y_per_mille);
            assert_eq!(frame.orb_opacity, resting.orb_opacity);
            assert_eq!(frame.orb_width_per_mille, resting.orb_width_per_mille);
            assert_eq!(frame.orb_height_per_mille, resting.orb_height_per_mille);
            assert_eq!(frame.splash_opacity, 0);
            assert_eq!(frame.secondary_ripple_opacity, 0);
            assert_eq!(frame.progress, 0);
            assert_eq!(frame, idle_presentation(time + IDLE_PERIOD_MS * 17, false));
            assert_eq!(idle_presentation(time, true), resting);
            visible += usize::from(frame.primary_ripple_opacity > 0);
            previous_scale = frame.primary_ripple_scale;
        }
        assert!(visible > 2_000);
        assert_eq!(idle_presentation(0, false).primary_ripple_opacity, 0);
        assert_eq!(idle_presentation(IDLE_PERIOD_MS - 1, false).primary_ripple_opacity, 0);
    }

    // ------------------------=
    // FUNC: idle_damage_protects_wordmark_controls_and_display_bounds
    // DESC: Verifies compact and scaled water-only damage excludes branding, credentials and tray, without full-screen redraw.
    // ------------------=
    #[test]
    fn idle_damage_protects_wordmark_controls_and_display_bounds() {
        for (width, height) in [(640, 480), (1024, 768), (2560, 1440), (3440, 1440)] {
            let fit = (width * 1_000 / 1_536).min(height * 1_000 / 1_024);
            let x = (width - 1_536 * fit / 1_000) / 2;
            let y = (height - 1_024 * fit / 1_000) / 2;
            let protected = [
                (0, 0, width, (64 * fit / 1_000).max(38)),
                (x + 900 * fit / 1_000, y + 92 * fit / 1_000,
                    x + 900 * fit / 1_000 + 570 * fit / 1_000,
                    y + 92 * fit / 1_000 + 92 * fit / 1_000),
                (x + 54 * fit / 1_000, y + 123 * fit / 1_000,
                    x + 54 * fit / 1_000 + 521 * fit / 1_000,
                    y + 123 * fit / 1_000 + 754 * fit / 1_000),
                (x + 529 * fit / 1_000, y + 914 * fit / 1_000,
                    x + 529 * fit / 1_000 + 478 * fit / 1_000,
                    y + 914 * fit / 1_000 + 90 * fit / 1_000),
            ];
            let regions = idle_water_regions(width, height);
            let mut area = 0;
            for (index, (left, top, right, bottom)) in regions.into_iter().enumerate() {
                assert!(left <= right && top <= bottom && right <= width && bottom <= height);
                if left == right || top == bottom { continue; }
                area += (right - left) * (bottom - top);
                assert!(top >= height / 2);
                for (l, t, r, b) in protected.into_iter().chain(regions[..index].iter().copied()) {
                    assert!(left >= r || right <= l || top >= b || bottom <= t);
                }
            }
            assert!(area > 0 && area < width * height / 7);
        }
        assert_eq!(idle_water_regions(0, 768), [(0, 0, 0, 0); 6]);
    }

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
        let lifted = presentation_at(160);
        let falling = presentation_at(672);
        let impact = presentation_at(IMPACT_FRAME_MS);
        let wake = presentation_at(RIPPLE_FRAME_MS);
        assert!(lifted.orb_y_per_mille < resting.orb_y_per_mille);
        assert!(falling.orb_y_per_mille > resting.orb_y_per_mille);
        assert!(impact.splash_opacity > 0);
        assert_eq!(presentation_at(300).primary_ripple_opacity, 0);
        assert!(wake.primary_ripple_scale > impact.primary_ripple_scale);
        assert!(wake.orb_opacity < impact.orb_opacity);
        for _ in 0..47 {
            assert!(matches!(timeline.advance(50), Advance::Frame(_)));
        }
        assert_eq!(timeline.advance(50), Advance::CommitDesktop);
        assert!(timeline.active());
        for _ in 0..11 { assert!(matches!(timeline.advance(50), Advance::DesktopFade(_))); }
        assert_eq!(timeline.advance(50), Advance::Finished);
        assert!(!timeline.active());
    }

    // ------------------------=
    // FUNC: fade_preserves_visible_ripples_when_frames_stall
    // DESC: Verifies monotonic fade phases, expanding visible waves and bounded catch-up under rendering stalls.
    // ------------------=
    #[test]
    fn fade_preserves_visible_ripples_when_frames_stall() {
        assert_eq!(scene_opacity(0), 255);
        assert_eq!(scene_opacity(900), 255);
        assert!(presentation_at(900).primary_ripple_opacity > 0);
        assert_eq!(scene_opacity(COMMIT_MS), 0);
        assert_eq!(scene_opacity(DURATION_MS), 255);
        assert_eq!(fade_pixel(0xff987654, 255), 0xff987654);
        assert_eq!(fade_pixel(0xff987654, 0), 0xff000000);
        let mut timeline = Timeline::new();
        timeline.begin();
        let mut visible_waves = 0;
        let mut committed = false;
        for _ in 0..60 {
            match timeline.advance(u16::MAX) {
                Advance::Frame(p) => {
                    assert!(!committed);
                    if p.primary_ripple_opacity > 30 && timeline.opacity() > 30 { visible_waves += 1; }
                }
                Advance::CommitDesktop => { assert!(!committed); committed = true; }
                Advance::DesktopFade(_) | Advance::Finished => assert!(committed),
                Advance::Idle => panic!("transition ended early"),
            }
        }
        assert!(visible_waves >= 12);
        assert!(!timeline.active());
        for t in FADE_START_MS..COMMIT_MS { assert!(scene_opacity(t + 1) <= scene_opacity(t)); }
        for t in COMMIT_MS..DURATION_MS { assert!(scene_opacity(t + 1) >= scene_opacity(t)); }
    }
}
