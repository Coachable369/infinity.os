//! Shared fixed-point login ribbons. Audio is amplitude only, never retained PCM.
use core::sync::atomic::{AtomicU64, Ordering};

static PRESENTATION: AtomicU64 = AtomicU64::new(0);
pub const PERIOD_MS: u32 = 24_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Frame { pub time_ms: u32, pub level: u16 }

// ------------------------=
// FUNC: current
// DESC: Reads one coherent presentation shared by full and damage-only login paints.
// ------------------=
pub fn current() -> Frame {
    let value = PRESENTATION.load(Ordering::Relaxed);
    Frame { time_ms: value as u32, level: (value >> 32) as u16 }
}

// ------------------------=
// FUNC: publish
// DESC: Publishes a bounded frame without exposing microphone samples to the painter.
// ------------------=
pub fn publish(frame: Frame) {
    PRESENTATION.store(u64::from(frame.time_ms) | (u64::from(frame.level.min(1000)) << 32), Ordering::Relaxed);
}

// ------------------------=
// FUNC: advance
// DESC: Smooths authorized peaks with fast attack and slow release while denial and reduced motion clear energy immediately.
// ------------------=
pub fn advance(previous: Frame, now_ms: u32, peak: Option<u16>, reduced: bool) -> Frame {
    if reduced { return Frame::default(); }
    let dt = now_ms.wrapping_sub(previous.time_ms).min(80);
    let level = match peak {
        None => 0,
        Some(peak) => {
            let target = (u32::from(peak.saturating_sub(160)) * 1000 / 6000).min(1000) as i32;
            let old = i32::from(previous.level.min(1000));
            let tau = if target > old { 40 } else { 300 };
            (old + (target - old) * dt as i32 / (tau + dt as i32)).clamp(0, 1000) as u16
        }
    };
    Frame { time_ms: now_ms, level }
}

// ------------------------=
// FUNC: sine
// DESC: Evaluates a continuous periodic sine approximation with bounded integer arithmetic and no floating-point state.
// ------------------=
pub fn sine(phase: i32) -> i32 {
    let p = phase.rem_euclid(4096);
    let x = i64::from(p % 2048);
    let product = x * (2048 - x);
    let value = (16 * product * 1024 / (5 * 2048 * 2048 - 4 * product)) as i32;
    if p < 2048 { value } else { -value }
}

// ------------------------=
// FUNC: ordinate
// DESC: Places three long sine ribbons and their delayed trails inside the wallpaper's central blue wave band.
// ------------------=
pub fn ordinate(frame: Frame, ribbon: usize, trail: u32, x: usize, width: usize, height: usize) -> i32 {
    let time = (frame.time_ms % PERIOD_MS + PERIOD_MS - trail * 110 % PERIOD_MS) % PERIOD_MS;
    let phase = (u64::from(time) * 4096 / u64::from(PERIOD_MS)) as i32;
    let x = (x.min(width) * 4096 / width.max(1)) as i32;
    let (center, amplitude, offset, speed) = match ribbon {
        0 => (420, 92, 200, 1),
        1 => (485, 72, 1680, -1),
        _ => (730, 75, 600, 1),
    };
    let excursion = amplitude + i32::from(frame.level) * 72 / 1000;
    let y = center * 1024 + sine(x * 3 / 5 + phase * speed + offset) * excursion
        + sine(x + phase * 2 + offset) * 15;
    (y as i64 * height as i64 * 256 / 1_024_000) as i32
}

// ------------------------=
// FUNC: paint
// DESC: Emits clipped antialiased ribbon pixels with fading history, a translucent body and one luminous leading edge.
// ------------------=
pub fn paint(frame: Frame, width: usize, height: usize, clip: (usize, usize, usize, usize),
    mut pixel: impl FnMut(usize, usize, (u8, u8, u8), u8)) {
    if width == 0 || height == 0 { return; }
    let scale = (height * 256 / 941).max(128) as i32;
    for ribbon in (0..3).rev() {
        let color = if ribbon == 1 { (54, 132, 255) } else { (64, 210, 255) };
        for trail in (0..4).rev() {
            for x in clip.0.min(width)..clip.2.min(width) {
                let y = ordinate(frame, ribbon, trail, x, width, height);
                let radius = if trail == 0 { 8 * scale } else { 2 * scale };
                let body = if trail == 0 { 24 * scale } else { 0 };
                let top = ((y - radius) / 256).max(clip.1 as i32).max(0) as usize;
                let bottom = ((y + radius + body) / 256 + 1).max(0) as usize;
                for py in top..bottom.min(clip.3).min(height) {
                    let d = (py as i32 * 256 + 128 - y).abs();
                    let energy = i32::from(frame.level);
                    let glow = ((radius - d).max(0) * (38 + energy / 18) / radius) as u8;
                    let edge = ((scale - d).max(0) * (145 + energy / 10) / scale) as u8;
                    let below = py as i32 * 256 + 128 - y;
                    let fill = if body > 0 && below >= 0 && below < body {
                        ((body - below) * (12 + energy / 60) / body) as u8
                    } else { 0 };
                    let alpha = if trail == 0 { glow.max(fill) } else { glow / (trail as u8 + 1) };
                    if alpha > 0 { pixel(x, py, color, alpha); }
                    let edge = if trail == 0 { edge } else { edge / (3 * trail as u8 + 2) };
                    if edge > 0 { pixel(x, py, (180, 239, 255), edge); }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: envelope_permission_and_motion
    // DESC: Exercises attack, release, silence, denial, clock wrap and reduced-motion state transitions.
    // ------------------=
    #[test]
    fn envelope_permission_and_motion() {
        let mut f = Frame::default();
        for t in (16..=160).step_by(16) { f = advance(f, t, Some(8000), false); }
        assert!(f.level > 950);
        let decay = advance(f, 176, Some(0), false);
        assert!(decay.level < f.level && decay.level > 850);
        assert_eq!(advance(decay, 192, None, false).level, 0);
        assert_eq!(advance(decay, 192, Some(8000), true), Frame::default());
        assert_eq!(advance(Frame::default(), 10, Some(100), false).level, 0);
        assert!(advance(Frame {time_ms: u32::MAX - 5, level: 0}, 10, Some(8000), false).level < 300);
    }
    // ------------------------=
    // FUNC: ribbons_move_and_react_with_bounded_pixels
    // DESC: Verifies visible idle movement, larger audio excursion, periodic continuity and clipped finite paint work.
    // ------------------=
    #[test]
    fn ribbons_move_and_react_with_bounded_pixels() {
        assert_eq!(sine(0), 0); assert_eq!(sine(1024), 1024); assert_eq!(sine(3072), -1024);
        for p in 0..4096 { assert!((sine(p + 1) - sine(p)).abs() <= 3); }
        for ribbon in 0..3 { for trail in 0..4 {
            let last=Frame{time_ms:PERIOD_MS-1,level:1000};
            let first=Frame{time_ms:PERIOD_MS,level:1000};
            for x in (0..1920).step_by(32) {
                assert!((ordinate(last,ribbon,trail,x,1920,1080)
                    -ordinate(first,ribbon,trail,x,1920,1080)).abs()<512);
            }
        }}
        for (w, h) in [(640,480), (1280,800), (1920,1080), (3440,1440)] {
            let a = Frame { time_ms: 4000, level: 0 };
            let b = Frame { time_ms: 4600, level: 0 };
            let loud = Frame { level: 1000, ..a };
            let movement: i64 = (0..w).map(|x| i64::from((ordinate(a,0,0,x,w,h)-ordinate(b,0,0,x,w,h)).abs())).sum();
            let reaction: i64 = (0..w).map(|x| i64::from((ordinate(a,0,0,x,w,h)-ordinate(loud,0,0,x,w,h)).abs())).sum();
            assert!(movement > w as i64 * 256);
            assert!(reaction > w as i64 * 10 * 256);
            let mut pixels = 0;
            paint(loud,w,h,(w/2,h/5,w,h*9/10), |x,y,_,alpha| {
                assert!(x >= w/2 && x < w && y >= h/5 && y < h*9/10 && alpha > 0); pixels += 1;
            });
            assert!(pixels > w && pixels < w*h);
        }
    }
}
