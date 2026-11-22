//! Native behavior and rendered artwork proof for the Studio surface collection.
#![allow(dead_code)]
#[path="../kernel/ui/panel_recipe.rs"] mod panel_recipe;
#[path="../kernel/ui/bitmap.rs"] mod bitmap;
#[path="../kernel/ui/installer_template.rs"] mod installer_template;
use panel_recipe::{Material,Surface,SurfaceStyle};
// ------------------------=
// FUNC: main
// DESC: Validates native decoding of Studio-authored family selections and exports production-rendered state pixels.
// ------------------=
fn main() {
    let root=std::path::Path::new(env!("PWD"));
    let folder=root.join("builds/validation/style-collection");std::fs::create_dir_all(&folder).unwrap();
    for id in (4u8..=14).chain(25..=28) {
        let data=std::fs::read(root.join(format!("builds/modern-button-{id}.iuit"))).unwrap();
        let template=installer_template::InstallerTemplate::parse(&data).unwrap();
        assert!((0..32).filter_map(|i|template.elements(i)).flatten().any(|e|e.surface_material==id));
    }
    let data=std::fs::read(root.join("builds/modern-menu.iuit")).unwrap();
    let menu=installer_template::InstallerTemplate::parse_desktop(&data).unwrap();
    assert!(menu.navigation().is_some());
    let radii=[10,32,12,8,10,5,14,8,12,8,8,10,32,24,10];
    for id in 4u8..=28 {
        let frame=(15..=24).contains(&id);let kind=if frame{Surface::Frame}else{Surface::Primary};
        let style=SurfaceStyle{material:id,corners:if frame{1}else{2},radius:if frame{0}else{radii[(if id>=25 {id-14}else{id-4})as usize]},fill:None,border:None};
        let (w,h)=if frame{(280,130)}else{(250,52)};
        let mut images=Vec::new();
        for state in 0..3 {
            let m=Material{fill:(255,255,255),edge:(255,255,255),opacity:255,edge_opacity:255,focus:if state==1{255}else{0},pressed:state==2,scale:1};
            let mut bytes=Vec::new();
            for y in 0..h {for x in 0..w {let p=panel_recipe::pixel_styled(kind,w,h,x,y,m,style);bytes.extend_from_slice(&[p.0,p.1,p.2,p.3]);}}
            if frame {assert_eq!(bytes[(h/2*w+w/2)*4+3],0);}
            std::fs::write(folder.join(format!("{id}-{state}.rgba")),&bytes).unwrap();images.push(bytes);
        }
        assert_ne!(images[0],images[1]);
        if !frame {assert_ne!(images[0],images[2]);}
        let mut record=Vec::from(&b"IUISTY01\x01\x00\x06\x00"[..]);
        for index in 0..6 {record.extend_from_slice(&[if index==if frame{0}else{3}{id}else{0},0,0,0]);record.extend_from_slice(&[0;8]);}
        assert_eq!(panel_recipe::decode_styles(&record).unwrap()[if frame{0}else{3}].material,id);
    }
    for id in 15u8..=24 {for radius in [0u8,12] {for focus in [0u8,255] {
        let w=240;let h=140;
        let style=SurfaceStyle{material:id,corners:if radius==0 {1}else{2},radius,fill:None,border:None};
        let m=Material{fill:(16,29,48),edge:(255,255,255),opacity:255,edge_opacity:255,focus,pressed:false,scale:1};
        let mut rgba=Vec::new();
        for y in 0..h {for x in 0..w {
            let coverage=u32::from(panel_recipe::rounded_shape(x,y,w,h,usize::from(radius)).0);
            let p=panel_recipe::pixel_styled(Surface::Frame,w,h,x,y,m,style);
            let base=if y<30 {(22u32,39u32,59u32)}else{(12,25,42)};
            for (b,c,bg) in [(base.0,p.0,9u32),(base.1,p.1,21),(base.2,p.2,34)] {
                let fill=(b*coverage+bg*(255-coverage))/255;
                rgba.push(((u32::from(c)*u32::from(p.3)+fill*(255-u32::from(p.3)))/255)as u8);
            }
            rgba.push(255);
        }}
        std::fs::write(folder.join(format!("edge-{id}-{radius}-{focus}.rgba")),rgba).unwrap();
        let mut binary=Vec::from(&b"IUISTY01\x01\x00\x06\x00"[..]);
        binary.extend_from_slice(&[id,style.corners,radius,0]);binary.extend_from_slice(&[0;8]);
        binary.extend_from_slice(&[0;60]);
        assert_eq!(panel_recipe::decode_styles(&binary).unwrap()[0],style);
    }}}
    println!("25 native families, state pixels and Studio binary persistence verified");
}
