#![allow(dead_code)]
#[path="../kernel/ui/motion.rs"] pub mod motion;
mod ui {pub mod app_launcher {pub use crate::motion;}}
#[path="../kernel/ui/desktop_effects.rs"] mod desktop_effects;
#[path="../kernel/drivers/input/desktop_shortcuts.rs"] mod shortcuts;

// ------------------------=
// FUNC: lens_settles_reverses_and_stops_work
// DESC: Verifies actual finite presentation samples, interruption, and reduced-motion endpoints.
// ------------------=
#[test]
fn lens_settles_reverses_and_stops_work() {
    let mut effects=desktop_effects::Effects::new();
    effects.toggle_lens(0,false);
    assert!(effects.advance(90,Some(1),false));
    assert_eq!(effects.view.dim,72);
    effects.toggle_lens(90,false);
    assert!(effects.advance(270,Some(1),false));
    assert_eq!(effects.view.dim,0);
    assert!(!effects.advance(10000,Some(1),false));
    effects.toggle_lens(10000,true);
    assert!(effects.advance(10000,Some(1),true));
    assert_eq!(effects.view.dim,144);
    effects.advance(10001,None,true);
    assert_eq!(effects.view.dim,0);
}
// ------------------------=
// FUNC: peek_restores_without_focus_mutation
// DESC: Verifies automatic restoration, active-window changes, immediate input dismissal and dim suspension.
// ------------------=
#[test]
fn peek_restores_without_focus_mutation() {
    let mut effects=desktop_effects::Effects::new();
    effects.toggle_lens(0,true);
    effects.peek(2,0,false);
    effects.advance(140,Some(2),false);
    assert_eq!(effects.view,desktop_effects::Presentation{dim:0,peek:Some(2),opacity:0});
    assert!(!effects.advance(200,Some(2),false));
    effects.advance(2500,Some(2),false);
    effects.advance(2640,Some(2),false);
    assert_eq!(effects.view,desktop_effects::Presentation{dim:144,peek:None,opacity:255});
    effects.peek(2,3000,true);
    effects.advance(3000,Some(1),true);
    assert_eq!(effects.view.peek,None);
    effects.peek(1,4000,true);
    effects.advance(4000,Some(1),true);
    assert!(effects.dismiss_peek());
    effects.advance(4000,Some(1),true);
    assert_eq!(effects.view.opacity,255);
}
// ------------------------=
// FUNC: fade_preserves_premultiplication_and_endpoints
// DESC: Exercises alpha and channel scaling across all opacities and both framebuffer channel orders.
// ------------------=
#[test]
fn fade_preserves_premultiplication_and_endpoints() {
    for pixel in [0,0xffffffff,0x80402010,0x80102040] {
        assert_eq!(desktop_effects::fade_pixel(pixel,0),0);
        assert_eq!(desktop_effects::fade_pixel(pixel,255),pixel);
        for opacity in 0..=255 {
            let p=desktop_effects::fade_pixel(pixel,opacity);
            for shift in [0,8,16] {assert!((p>>shift)&255 <= p>>24);}
        }
    }
}
