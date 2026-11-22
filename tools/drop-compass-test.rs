#[path="../kernel/ui/geometry.rs"] mod geometry;
#[path="../kernel/ui/drop_compass.rs"] mod drop_compass;
use geometry::Point;
use drop_compass::Compass;
// ------------------------=
// FUNC: hold_and_cancel
// DESC: Verifies a single hold opens, quick clicks and movement cancel, and dismissal clears capture.
// ------------------=
#[test]
fn hold_and_cancel(){
 let p=Point{x:640,y:360};let mut c=Compass::new();c.arm(p,100,false);
 assert!(!c.advance(p,true,649,1280,720));assert!(c.center.is_none());
 assert!(c.advance(p,true,650,1280,720));assert_eq!(c.center,Some(p));
 c.dismiss();assert_eq!(c,Compass::new());c.arm(p,0,false);c.advance(p,false,100,1280,720);c.advance(p,true,800,1280,720);assert!(c.center.is_none());
 c.arm(p,0,true);c.advance(Point{x:650,y:360},true,600,1280,720);assert!(c.center.is_none());
}
// ------------------------=
// FUNC: bounded_targets
// DESC: Checks all screen corners at supported display sizes, distinct card hit regions and inert center.
// ------------------=
#[test]
fn bounded_targets(){for (w,h) in [(640,480),(1280,720),(1920,1080),(2560,1440)]{for p in [Point{x:0,y:0},Point{x:w as i32-1,y:h as i32-1}]{let mut c=Compass::new();c.arm(p,0,true);c.advance(p,true,550,w,h);for (i,r) in c.cards(w,h).into_iter().enumerate(){assert!(r.x>=0&&r.y>=0&&r.right()<=w as i32&&r.bottom()<=h as i32);c.advance(Point{x:r.x+10,y:r.y+10},true,600,w,h);assert_eq!(c.selected,Some(i));}c.advance(c.center.unwrap(),true,600,w,h);assert_eq!(c.selected,None);}}}
