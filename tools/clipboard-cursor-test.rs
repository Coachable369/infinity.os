#![allow(dead_code)]
#[path="../kernel/ui/geometry.rs"]
pub mod geometry;
#[path="../kernel/ui/input_preferences.rs"]
pub mod input_preferences;
#[path="../kernel/ui/cursor.rs"]
mod cursor;
mod ui {pub use crate::geometry;}
#[path="../kernel/ui/clipboard_menu.rs"]
mod clipboard_menu;
// ------------------------=
// FUNC: main
// DESC: Verifies every clipboard action hit, overlay damage, clipboard erasure and animated cursor pixels.
// ------------------=
fn main() {
    use geometry::Point;
    for (width,height) in [(640,480),(1280,800),(2560,1600)] {for scale in 1..=4 {
        let mut menu=clipboard_menu::Menu::new();menu.open=true;
        menu.place(width as i32,height as i32,width,height,scale);
        assert!(menu.rect.right()<=width as i32 && menu.rect.bottom()<=height as i32);
        for row in 0..6 {
            assert_eq!(menu.row_at(Point{x:menu.rect.x+10*menu.scale as i32,
                y:menu.rect.y+(7+30*row) as i32*menu.scale as i32}),Some(row));
        }
        assert_eq!(menu.row_at(Point{x:menu.rect.x,y:menu.rect.y}),None);
        clipboard_menu::publish(menu);let damage=clipboard_menu::take_damage().unwrap();
        assert!(damage.contains(Point{x:menu.rect.x,y:menu.rect.y}));
        menu.viewer=true;menu.bytes[..6].copy_from_slice(b"secret");menu.length=6;
        menu.place(100,100,width,height,scale);clipboard_menu::publish(menu);
        assert!(menu.rect.right()<=width as i32 && menu.rect.bottom()<=height as i32);
        assert_eq!(menu.rect.width,520*menu.scale as u32);
        assert_eq!(menu.rect.height,350*menu.scale as u32);
        clipboard_menu::close();let closed=clipboard_menu::current();
        assert!(!closed.open);assert_eq!(closed.length,0);assert!(closed.bytes.iter().all(|b|*b==0));
        assert!(clipboard_menu::take_damage().is_some());
    }}
    let prefs=input_preferences::Preferences::defaults();let mut signatures=std::collections::BTreeSet::new();
    for frame in 0..16 {
        assert!(cursor::animate_busy(true,frame*80));assert!(!cursor::animate_busy(true,frame*80+79));
        let bounds=cursor::bounds(200,200,1,prefs,false);assert_eq!(bounds.width,40);
        let mut bytes=Vec::new();let mut visible=0;
        for y in 0..40 {for x in 0..40 {let p=cursor::busy_sample(x,y,40);visible+=usize::from(p[3]>128);bytes.extend_from_slice(&p);}}
        assert!(visible>100 && visible<1200);signatures.insert(bytes);
    }
    assert_eq!(signatures.len(),16);assert!(cursor::animate_busy(false,1280));assert!(!cursor::busy());
    assert_eq!(cursor::bounds(200,200,1,prefs,false).width,28);
}
