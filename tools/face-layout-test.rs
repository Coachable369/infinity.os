#[path="../kernel/ui/geometry.rs"] mod geometry;
#[path="../kernel/ui/face_setup_layout.rs"] mod face_setup_layout;

// ------------------------=
// FUNC: main
// DESC: Exercises exact capture hit targets and enlarged artwork separation at supported VM resolutions.
// ------------------=
fn main() {
    for (width,height) in [(1024,768),(1280,800),(1920,1080),(2560,1440)] {
        let l=face_setup_layout::Layout::new(width,height);
        let point=|r:geometry::Rect|geometry::Point{x:r.x+r.width as i32/2,y:r.y+r.height as i32/2};
        for (r,action) in [(l.primary,1),(l.back,0),(l.skip,3),(l.field,2)] {
            assert_eq!(l.hit(point(r)),Some(action));
        }
        assert!(l.mask.x>=l.field.right());
        assert!(l.mask.bottom()<=l.primary.y);
        assert!(l.mask.right()<=width as i32);
        assert!(l.status.right()<=l.mask.x);
        let old_column=(width*84/100-(32*height/1000).max(1)*3)/2;
        let old_size=old_column.min((420*height/1000).max(1)).min(480);
        assert_eq!(l.mask.width as usize,old_size*170/100);
    }
}
