//! Bounded, render-only cache of exact themed pixels; never caches the backdrop.
use super::panel_recipe::{self, Material, Rgba, Surface, SurfaceStyle};
use core::{cell::UnsafeCell, sync::atomic::{AtomicBool, Ordering}};

const SLOTS: usize = 4;
const PIXELS: usize = 1_310_720;
const MIN_PIXELS: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub width: usize,
    pub height: usize,
    pub surface: Surface,
    pub material: Material,
    pub style: SurfaceStyle,
}

struct Entry { key: Option<Key>, age: u64, pixels: [Rgba; PIXELS] }
struct Cache { entries: [Entry; SLOTS], age: u64 }
struct Shared { busy: AtomicBool, value: UnsafeCell<Cache> }
// The nonblocking guard is the only route to the mutable cache. A reentrant or
// competing renderer falls back to normal painting instead of waiting.
unsafe impl Sync for Shared {}
static CACHE: Shared = Shared {
    busy: AtomicBool::new(false),
    value: UnsafeCell::new(Cache {
        entries: [const { Entry { key: None, age: 0, pixels: [(0,0,0,0); PIXELS] } }; SLOTS], age: 0,
    }),
};

struct Guard;
impl Drop for Guard {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases cache ownership even when a host verification callback unwinds.
    // ------------------=
    fn drop(&mut self) { CACHE.busy.store(false, Ordering::Release); }
}

// ------------------------=
// FUNC: with_surface
// DESC: Reuses exact foreground RGBA; admits only full-surface cold paints and bounded dimensions, without allocations or waits.
// ------------------=
pub fn with_surface(key: Key, allow_fill: bool, paint: impl FnOnce(&[Rgba])) -> bool {
    let Some(length) = key.width.checked_mul(key.height).filter(|&n| (MIN_PIXELS..=PIXELS).contains(&n)) else { return false; };
    if CACHE.busy.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() { return false; }
    let _guard = Guard;
    let cache = unsafe { &mut *CACHE.value.get() };
    let existing = cache.entries.iter().position(|entry| entry.key == Some(key));
    if existing.is_none() && !allow_fill { return false; }
    let slot = existing.unwrap_or_else(|| cache.entries.iter().position(|entry| entry.key.is_none())
        .unwrap_or_else(|| cache.entries.iter().enumerate().min_by_key(|(_, entry)| entry.age).unwrap().0));
    cache.age = cache.age.wrapping_add(1);
    let entry = &mut cache.entries[slot];
    if existing.is_none() {
        entry.key = None;
        for (index, pixel) in entry.pixels[..length].iter_mut().enumerate() {
            *pixel = panel_recipe::pixel_styled(key.surface, key.width, key.height,
                index % key.width, index / key.width, key.material, key.style);
        }
        entry.key = Some(key);
    }
    entry.age = cache.age;
    paint(&entry.pixels[..length]);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: exact_pixels_and_invalidation
    // DESC: Checks every cached sample against the production shader across dimensions, themes, states, and silhouettes.
    // ------------------=
    #[test]
    fn exact_pixels_and_invalidation() {
        for surface in [Surface::Panel, Surface::Primary, Surface::Field, Surface::WindowFill] {
            for material in 0..=3 {
                let key = Key { width: 257, height: 256, surface,
                    material: Material { fill: (16,27,58), edge: (134,199,255), opacity: 192,
                        edge_opacity: 255, focus: 127, pressed: material == 3, scale: 1 },
                    style: SurfaceStyle { material, corners: 2, radius: 9, fill: None, border: None } };
                for allow_fill in [true, false] {
                    assert!(with_surface(key, allow_fill, |pixels| {
                        for (index, &pixel) in pixels.iter().enumerate() {
                            assert_eq!(pixel, panel_recipe::pixel_styled(surface, key.width, key.height,
                                index % key.width, index / key.width, key.material, key.style));
                        }
                        assert!(!with_surface(key, true, |_| panic!("reentrant cache access")));
                    }));
                }
                assert!(!with_surface(Key { width: 258, ..key }, false, |_| panic!("stale dimensions")));
                // A screen's many small controls must not evict its expensive
                // window/background samples between frames.
                for width in 20..100 {
                    assert!(!with_surface(Key { width, height: 40, ..key }, true, |_| panic!("small control admission")));
                }
                assert!(with_surface(key, false, |_| {}));
            }
        }
    }
}
