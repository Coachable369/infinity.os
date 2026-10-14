#![allow(dead_code)]
#[path="../kernel/runtime/font_preferences.rs"] mod preferences;
#[path="../kernel/runtime/font_faces.rs"] mod faces;
#[path="../kernel/runtime/font.rs"] mod catalog;
#[path="../kernel/ui/geometry.rs"] mod geometry;
#[path="../kernel/ui/font_library.rs"] mod library;

#[test]
// ------------------------=
// FUNC: packaged_faces
// DESC: Checks every catalog ID has usable pixels and matching bounded metrics for both native weights.
// ------------------=
fn packaged_faces() {
    let catalog=catalog::FontCatalog::new(); assert_eq!(catalog.count(),46);
    let mut distinct=std::collections::HashSet::new();
    for i in 0..catalog.count() {
        let font=catalog.nth(i).unwrap(); assert_eq!(font.id as usize,i+1);
        for bold in [false,true] {
            let (pixels,metrics,kern)=faces::face(font.id as u8,bold);
            assert_eq!(pixels.len(),24*28*95);assert_eq!(metrics.len(),95);assert_eq!(kern.len(),95*95);
            assert!(metrics.iter().all(|v|*v>0 && *v<=32));
            for glyph in 1..95 {assert!((0..28).any(|row|pixels[row*24*95+glyph*24..row*24*95+(glyph+1)*24].iter().any(|v|*v>0)));}
            if !bold {distinct.insert(pixels.to_vec());}
        }
    }
    assert_eq!(distinct.len(),46);
    assert_eq!(faces::face(2,false).0,include_bytes!("../assets/fonts/InfinityUI-Regular-24.atlas"));
    assert_eq!(faces::face(0,false),faces::face(2,false));
}

#[test]
// ------------------------=
// FUNC: hit_regions
// DESC: Verifies every scaled action target, viewport clipping, fallback protection and final-page exclusion.
// ------------------=
fn hit_regions() {
    use geometry::{Point,Rect}; use library::{Action,Layout};
    for scale in 1..=3 {
        let detail=Rect{x:20,y:10,width:520*scale as u32,height:464*scale as u32};
        let l=Layout::new(detail,scale);
        for page in 0..8 {
            for row in 0..6 {
                let id=(page*6+row+1) as u8;
                let a=l.select[row];let b=l.toggle[row];
                assert!(!a.intersects(b));
                let p=Point{x:a.x+1,y:a.y+1};
                assert_eq!(l.hit(p,detail,page),if id<=46 {Some(Action::Use(id))}else{None});
                assert_eq!(l.hit(p,Rect::default(),page),None);
                assert_eq!(l.hit(Point{x:b.x+1,y:b.y+1},detail,page),if id<=46 && id!=2 {Some(Action::Toggle(id))}else{None});
            }
        }
    }
}
