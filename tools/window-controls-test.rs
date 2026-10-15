#![allow(dead_code)]
#[path="../kernel/ui/geometry.rs"] pub mod geometry;
mod ui {pub use crate::geometry;}
#[path="../kernel/ui/window_controls.rs"] mod controls;
use geometry::{Point,Rect};

#[test]
// ------------------------=
// FUNC: geometry_states_and_theme_pixels
// DESC: Verifies hit bounds, finite hover damage, authored alpha, and distinct states for every palette combination.
// ------------------=
fn geometry_states_and_theme_pixels() {
    for s in 1..=3 {
        let window=Rect{x:10,y:20,width:600*s as u32,height:400*s as u32};
        let b=controls::bounds(window,48*s as u32,s);
        for (i,r) in b.into_iter().enumerate() {
            assert_eq!(controls::hit(b,Point{x:r.x+1,y:r.y+1}),Some(i));
            assert_eq!(r.width,26*s as u32);
            if i>0 {assert_eq!(r.x-b[i-1].right(),8*s as i32);}
        }
        assert_eq!(b[2].right(),window.right()-12*s as i32);
        assert_eq!(controls::hit(b,Point{x:b[1].x-1,y:b[1].y}),None);
    }
    controls::publish(None);let _=controls::take_damage();
    let a=Rect{x:10,y:20,width:26,height:26};
    let b=Rect{x:1900,y:20,width:26,height:26};
    assert!(controls::publish(Some(controls::Highlight{rect:a,pressed:false})));
    assert_eq!(controls::state(a),1);
    assert_eq!(controls::take_damage(),[None,Some(a)]);
    assert!(!controls::publish(Some(controls::Highlight{rect:a,pressed:false})));
    assert_eq!(controls::take_damage(),[None,None]);
    assert!(controls::publish(Some(controls::Highlight{rect:a,pressed:true})));
    assert_eq!(controls::state(a),2);
    let _=controls::take_damage();
    controls::publish(Some(controls::Highlight{rect:b,pressed:false}));
    assert_eq!(controls::take_damage(),[Some(a),Some(b)]);
    controls::publish(None);let _=controls::take_damage();
    for primary in [0x0d2238,0xe8edf5,0x202020,0x35182f] {
        for accent in [0x38bdf8,0xa78bfa,0x22c55e,0xf59e0b,0xff5577] {
            for close in [false,true] {
                let mut states=Vec::new();
                for state in 0..3 {
                    let mut pixels=Vec::new();
                    for y in 0..26 {for x in 0..26 {
                        pixels.push(controls::pixel(26,x,y,state,primary,accent,close));
                    }}
                    assert_eq!(pixels[0].3,0);
                    assert!(pixels[13*26+13].3>230);
                    assert!(pixels.iter().any(|p|p.3>0&&p.3<255));
                    states.push(pixels);
                }
                assert_ne!(states[0],states[1]);assert_ne!(states[1],states[2]);
            }
        }
    }
}
