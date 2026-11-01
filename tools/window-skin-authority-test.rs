//! Behavioral draft-artwork parity checks against the production native samplers.
use std::sync::atomic::{AtomicU8, Ordering};
#[path="../kernel/ui/geometry.rs"] pub mod geometry;
mod ui { pub use super::geometry; }
#[path="../kernel/ui/assistant_tab.rs"] mod assistant_tab;
#[path="../kernel/ui/window_controls.rs"] mod window_controls;
mod studio_native_render {
    use super::*;
    pub static MODE: AtomicU8 = AtomicU8::new(0);
    static TAB: [u8;112*416*4] = [255;112*416*4];
    static CONTROLS: [u8;3*96*96*4] = [255;3*96*96*4];
    // ------------------------=
    // FUNC: asset_override
    // DESC: Supplies bounded deterministic draft sprites to the unchanged production sampler.
    // ------------------=
    pub fn asset_override(name:&[u8])->Option<&'static [u8]> {
        match MODE.load(Ordering::SeqCst) {
            1=>match name {b"ai-window-tab-v1.rgba"=>Some(&TAB),b"window-controls-v1.rgba"=>Some(&CONTROLS),_=>None},
            2=>Some(&[255]),_=>None,
        }
    }
}
// ------------------------=
// FUNC: main
// DESC: Verifies unsaved Studio sprites replace native pixels, malformed sprites fall back safely and mirrored alpha remains exact.
// ------------------=
fn main() {
    let samples=|| (assistant_tab::pixel(28,104,27,0,false,48),window_controls::pixel(26,0,0,0,0x101b3a,0x86c7ff,false));
    let authored=samples();
    studio_native_render::MODE.store(1,Ordering::SeqCst);
    assert_eq!(samples().0,(255,255,255,255));
    assert_ne!(samples(),authored);
    for state in 0..3 {for y in 0..26 {for x in 0..26 {
        assert_eq!(window_controls::pixel(26,x,y,state,0x101b3a,0x86c7ff,false).3,255);
    }}}
    for y in 0..104 {for x in 0..28 {
        assert_eq!(assistant_tab::pixel(28,104,x,y,false,48),assistant_tab::pixel(28,104,27-x,y,true,48));
    }}
    studio_native_render::MODE.store(2,Ordering::SeqCst);
    assert_eq!(samples(),authored);
}
