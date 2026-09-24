#![allow(dead_code)]
#[path="../kernel/ui/mod.rs"] mod ui;
use ui::{cursor,input_preferences::Preferences,personalization::{Layout,Target},geometry::{Rect,Point},spatial::SpatialState};

#[test]
// ------------------------=
// FUNC: user_preferences_survive_binary_roundtrip
// DESC: Exercises legacy migration, every speed/size/style and per-owner tint persistence with corrupt-state rejection.
// ------------------=
fn user_preferences_survive_binary_roundtrip() {
    let mut legacy=Preferences::defaults().encode();legacy[6]=0;legacy[7]=0;
    assert_eq!(Preferences::decode(legacy).cursor_size,2);
    for speed in 1..=10 {for size in 1..=6 {for style in 0..10 {
        let mut p=Preferences::defaults();p.speed=speed;p.cursor_size=size;p.cursor_style=style;
        assert_eq!(Preferences::decode(p.encode()),p);
    }}}
    let owner=[7;16];let mut state=SpatialState::new(owner);
    state.backdrop_tint=[33,90,177,214];
    let bytes=state.encode(owner).unwrap();
    assert_eq!(SpatialState::decode(owner,&bytes).unwrap(),state);
    assert!(SpatialState::decode([8;16],&bytes).is_err());
    let mut broken=bytes;broken[7410]^=1;
    assert!(SpatialState::decode(owner,&broken).is_err());
    let mut legacy=bytes;legacy[7408..7413].fill(0);
    let checksum=legacy[..legacy.len()-4].iter().fold(2166136261u32,|sum,byte|(sum^*byte as u32).wrapping_mul(16777619));
    let end=legacy.len();legacy[end-4..].copy_from_slice(&checksum.to_le_bytes());
    assert_eq!(SpatialState::decode(owner,&legacy).unwrap().backdrop_tint,[8,23,42,165]);
}

#[test]
// ------------------------=
// FUNC: personalization_controls_are_disjoint_and_reachable
// DESC: Verifies responsive gallery targets, clipped hit rejection, slider endpoints and following-row separation.
// ------------------=
fn personalization_controls_are_disjoint_and_reachable() {
    for width in [260,400,600,1000] {for pointer in [false,true] {
        let rect=Rect{x:30,y:80,width,height:620};
        let layout=Layout::new(rect,1,pointer);
        let count=if pointer {10}else{5};
        for i in 0..count {
            let r=layout.choices[i];let p=Point{x:r.x+4,y:r.y+4};
            assert_eq!(layout.hit(p,rect),Some(Target::Choice(i)));
            assert_eq!(layout.hit(p,Rect{x:0,y:0,width:1,height:1}),None);
            assert!(r.right()<=rect.right() && r.bottom()<=rect.bottom());
            for j in i+1..count {assert!(!r.intersects(layout.choices[j]));}
        }
        for i in 0..if pointer {2}else{4} {
            let r=layout.sliders[i];
            assert_eq!(layout.value(i,r.x-100,255),0);
            assert_eq!(layout.value(i,r.right()+100,255),255);
            assert_eq!(layout.hit(Point{x:r.x,y:r.y},rect),Some(Target::Slider(i)));
        }
    }}
    for (width,height) in [(1024,768),(1920,1080),(2560,1440)] {
        let layout=ui::system_layout::SystemLayout::new(width,height);
        for (section,row) in [(5,2),(1,7)] {
            let mut state=ui::system_layout::SettingsWindowState {x:145,y:155,width:690,height:500,maximized:false,expanded_row:None,scroll_offset:0,control_focus:0,row_count:8};
            state.expanded_row=Some(row);state.row_count=if section==5{5}else{8};
            let detail=layout.settings_row_geometry_for_section(state,row,section).detail;
            let controls=Layout::new(detail,layout.scale(),section==5);
            let last=controls.choices[if section==5 {9}else{4}];
            assert!(last.bottom()<=detail.bottom());
            if section==5 {
                let next=layout.settings_row_geometry_for_section(state,row+1,section).summary;
                assert!(detail.bottom()<=next.y);
            }
            let window=layout.settings_window_geometry_for_section(state,section);
            state.scroll_offset=window.maximum_scroll;
            let scrolled=layout.settings_row_geometry_for_section(state,row,section).detail;
            assert_eq!(detail.y-scrolled.y,(window.maximum_scroll*layout.scale()) as i32);
            let controls=Layout::new(scrolled,layout.scale(),section==5);
            assert!(controls.choices[if section==5{9}else{4}].bottom()<=window.viewport.bottom());
        }
    }
}

#[test]
// ------------------------=
// FUNC: cursor_assets_have_opaque_hotspots_and_bounded_restoration
// DESC: Checks all actual linked RGBA sprites and sizes rather than filenames or source-text declarations.
// ------------------=
fn cursor_assets_have_opaque_hotspots_and_bounded_restoration() {
    for style in 0..10 {
        let sprite=cursor::SPRITES[style];let (x,y)=cursor::HOTSPOTS[style];
        assert!(sprite[(y*128+x)*4+3]>200);
        assert!(sprite.chunks_exact(4).filter(|p|p[3]>128).count()>100);
        assert_eq!(sprite[3],0);
        for other in 0..style {assert_ne!(sprite,cursor::SPRITES[other]);}
        for size in 1..=6 {for scale in [1,2,3] {for special in [false,true] {
            let mut p=Preferences::defaults();p.cursor_style=style as u8;p.cursor_size=size;
            let b=cursor::bounds(0,0,scale,p,special);
            assert!(b.width<=128 && b.height<=128);
            if !special {assert_eq!(b.x+(x*b.width as usize/128) as i32,0);assert_eq!(b.y+(y*b.height as usize/128) as i32,0);}
        }}}
    }
}
