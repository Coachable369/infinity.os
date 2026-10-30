#![allow(dead_code)]
#[path="../kernel/ui/mod.rs"]mod ui;
use ui::installer_template::{InstallerTemplate,navigation::{Destination,Menu,View}};
// ------------------------=
// FUNC: main
// DESC: Reads actual Studio-produced runtime bytes and verifies native routing, hierarchy, malformed-data rejection and stable scene linkage.
// ------------------=
fn main(){
    let bytes=std::fs::read("builds/validation/studio-menu/navigation-fixture.iuit").unwrap();
    let template=InstallerTemplate::parse_desktop(&bytes).unwrap();
    assert_eq!(template.screen_count(),6);
    let menu=template.navigation().unwrap();assert_eq!(menu.count,11);
    for i in 0..10 {assert_eq!(menu.destination(i),Some(Destination::System(i as u8)));}
    assert_eq!(menu.destination(10),Some(Destination::Window(6)));
    assert_eq!(template.element_count(6),Some(2));
    for (width,height) in [(1280,800),(1920,1080),(3840,2160)] {
        let dock_bounds=ui::desktop_template::scene_bounds_in(template,4,width,height);
        assert!(dock_bounds.x>0 && dock_bounds.y>0);
        let layout=ui::system_layout::SystemLayout::new(width,height);
        let launcher=layout.apply_launcher_template(layout.app_launcher_geometry(),template,5);
        for (role,actual) in [(81,launcher.panel),(82,launcher.search),(83,launcher.close),
            (84,launcher.grid_viewport),(86,launcher.scrollbar_track)] {
            let element=(0..template.element_count(5).unwrap()).filter_map(|i|template.element_at(5,i))
                .find(|e|e.role==role).unwrap();
            assert_eq!(actual,ui::desktop_template::mapped(element,width,height,None));
        }
        assert!(launcher.grid_cell_width>0 && launcher.grid_row_height>0);
        for index in 0..8 {
            let element=(0..template.element_count(4).unwrap()).filter_map(|i|template.element_at(4,i))
                .find(|e|e.role==73+index).unwrap();
            let rect=ui::desktop_template::mapped(element,width,height,None);
            let point=ui::geometry::Point{x:rect.x+rect.width as i32/2,y:rect.y+rect.height as i32/2};
            assert!(dock_bounds.contains(point));
            assert_eq!(ui::desktop_template::dock_hit_in(template,4,point,width,height),Some(index as usize));
        }
        assert_eq!(ui::desktop_template::dock_hit_in(template,4,ui::geometry::Point{x:0,y:0},width,height),None);
    }
    let mut view=View::new();view.step(&menu,false);assert_eq!(view.selected,10);
    view.step(&menu,true);assert_eq!(view.selected,0);
    for drop in 1..64{assert!(InstallerTemplate::parse_desktop(&bytes[..bytes.len()-drop]).is_err());}
    let mut bad=bytes.clone();bad.push(0);assert!(InstallerTemplate::parse_desktop(&bad).is_err());
    let mut raw=vec![73,78,65,86,1,4,0,1];
    for (i,action,target,flags,children) in [(0,5,0,1,3),(1,2,1,0,0),(2,6,0,1,0),(3,1,7,3,0)] {
        let mut id=[0;16];id[0]=i;raw.extend_from_slice(&id);raw.extend_from_slice(&[action,target,flags,children,1,b'X']);
    }
    let menu=Menu::parse(&raw,3).unwrap();assert_eq!(menu.count,4);
    assert_eq!(menu.destination(1),None);assert_eq!(menu.destination(2),None);
    assert_eq!(menu.destination(3),Some(Destination::Settings(7)));
    view=View::new();view.enter(&menu,0);assert_eq!(view.count(&menu),4);
    view.step(&menu,true);assert_eq!(view.selected,3);assert!(menu.items[3].checked);
    assert!(view.back(&menu));assert_eq!(view.parent,None);assert_eq!(view.selected,0);
    for length in 0..raw.len(){assert!(Menu::parse(&raw[..length],3).is_none());}
    let mut invalid=raw.clone();invalid[8+18]=0;let disabled=Menu::parse(&invalid,3).unwrap();assert_eq!(disabled.destination(3),None);
    let legacy=std::fs::read("assets/boot/desktop-screens.iuit").unwrap();
    assert!(InstallerTemplate::parse_desktop(&legacy).is_ok());
    println!("Studio-to-native menu navigation checks passed");
}
