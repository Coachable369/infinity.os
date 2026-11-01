//! Bounded wallpaper-only snapshot; never captures credentials or foreground UI.
use super::DisplayDevice;
use core::{cell::UnsafeCell, sync::atomic::{AtomicBool,Ordering}};

const CAPACITY:usize=3840*2160;
struct Cache { key:Option<(usize,usize,u32,usize,usize)>, pixels:[u32;CAPACITY] }
struct Shared { busy:AtomicBool, cache:UnsafeCell<Cache> }
unsafe impl Sync for Shared {}
static CACHE:Shared=Shared {busy:AtomicBool::new(false),cache:UnsafeCell::new(Cache{key:None,pixels:[0;CAPACITY]})};
struct Guard;
impl Drop for Guard {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases nonblocking wallpaper ownership even during host test unwinding.
    // ------------------=
    fn drop(&mut self) {CACHE.busy.store(false,Ordering::Release);}
}

// ------------------------=
// FUNC: capture
// DESC: Saves an exact full wallpaper paint before waves and foreground, with a fixed memory bound and no per-frame allocation.
// ------------------=
pub(super) fn capture(display:&DisplayDevice,source:&[u8]) {
    if display.render_clip.is_some() {return;}
    if CACHE.busy.swap(true,Ordering::Acquire) {return;}
    let _guard=Guard;
    let cache=unsafe {&mut *CACHE.cache.get()};
    cache.key=None;
    if display.width.saturating_mul(display.height)>CAPACITY {return;}
    for y in 0..display.height {
        unsafe {core::ptr::copy_nonoverlapping(display.buffer.add(y*display.stride),
            cache.pixels.as_mut_ptr().add(y*display.width),display.width);}
    }
    cache.key=Some((display.width,display.height,display.format,source.as_ptr()as usize,source.len()));
}

// ------------------------=
// FUNC: restore
// DESC: Restores only the caller's clipped wallpaper region and rejects mismatched scene identity or viewport.
// ------------------=
pub(super) fn restore(display:&mut DisplayDevice,source:&[u8])->bool {
    if CACHE.busy.swap(true,Ordering::Acquire) {return false;}
    let _guard=Guard;
    let cache=unsafe {&*CACHE.cache.get()};
    if cache.key!=Some((display.width,display.height,display.format,source.as_ptr()as usize,source.len())) {return false;}
    let Some(clip)=display.clipped_render_region(0,0,display.width,display.height) else {return true;};
    for y in clip.top..clip.bottom {
        unsafe {core::ptr::copy_nonoverlapping(cache.pixels.as_ptr().add(y*display.width+clip.left),
            display.buffer.add(y*display.stride+clip.left),clip.right-clip.left);}
    }
    display.mark_dirty_rect(clip.left,clip.top,clip.right-clip.left,clip.bottom-clip.top);
    true
}
