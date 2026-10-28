#[path = "../kernel/ui/panel_recipe.rs"] pub mod panel_recipe;
mod ui { pub use crate::panel_recipe; }
#[path = "../kernel/ui/dock_material.rs"] mod dock_material;
use panel_recipe::{Surface, SurfaceStyle, Material};

// ------------------------=
// FUNC: main
// DESC: Verifies native style decoding, unchanged authored pixels, real material/geometry transitions and antialiased image borders.
// ------------------=
fn main() {
    let frame=(134,199,255);
    for old in [(33,58,85),(41,72,94),(43,80,104)] {
        assert_eq!(panel_recipe::legacy_panel_edge(old,frame),frame);
    }
    assert_eq!(panel_recipe::legacy_panel_edge((255,80,80),frame),(255,80,80));
    let material=Material{fill:(16,27,58),edge:(134,199,255),opacity:235,edge_opacity:255,focus:255,pressed:false,scale:1};
    for kind in [Surface::Frame,Surface::WindowFill,Surface::Panel,Surface::Primary,Surface::Secondary,Surface::Field] {
        for y in 0..67 {for x in 0..131 {
            assert_eq!(panel_recipe::pixel_styled(kind,131,67,x,y,material,SurfaceStyle::AUTHORED),
                panel_recipe::pixel_authored(kind,131,67,x,y,material));
        }}
    }
    let mut bytes=[0u8;panel_recipe::STYLE_BYTES];
    bytes[..8].copy_from_slice(b"IUISTY01");
    bytes[8]=1;bytes[10]=6;
    let slot=12+2*12;
    bytes[slot..slot+4].copy_from_slice(&[2,2,24,3]);
    bytes[slot+4..slot+8].copy_from_slice(&0x80112233u32.to_le_bytes());
    bytes[slot+8..slot+12].copy_from_slice(&0xffabcdefu32.to_le_bytes());
    let decoded=panel_recipe::decode_styles(&bytes).unwrap();
    assert_eq!(decoded[2],SurfaceStyle{material:2,corners:2,radius:24,fill:Some(0x80112233),border:Some(0xffabcdef)});
    bytes[slot]=3;
    assert_eq!(panel_recipe::decode_styles(&bytes).unwrap()[2].material,3);
    for (at,value) in [(0,0),(8,2),(10,5),(12+1,2),(slot,4),(slot+1,3),(slot+2,41),(slot+3,4)] {
        let mut invalid=bytes;invalid[at]=value;
        assert!(panel_recipe::decode_styles(&invalid).is_none());
    }
    assert!(panel_recipe::decode_styles(&bytes[..bytes.len()-1]).is_none());
    let rounded=SurfaceStyle{corners:2,radius:32,..SurfaceStyle::AUTHORED};
    let square=SurfaceStyle{corners:1,..SurfaceStyle::AUTHORED};
    let matte=SurfaceStyle{material:2,..rounded};
    let sample=|style,x,y|panel_recipe::pixel_styled(Surface::Panel,131,67,x,y,material,style);
    assert_eq!(sample(rounded,0,0).3,0);
    assert!(sample(square,0,0).3>0);
    let mut partial=false;
    let mut different=false;
    for y in 0..33 {for x in 0..33 {
        let alpha=sample(rounded,x,y).3;
        partial |= alpha>0 && alpha<235;
        different |= sample(matte,x,y)!=sample(rounded,x,y);
    }}
    assert!(partial && different);
    let transparent=SurfaceStyle{fill:Some(0x00112233),border:Some(0x00112233),..rounded};
    for y in 0..67 {for x in 0..131 {assert_eq!(sample(transparent,x,y).3,0);}}
    let baseline:Vec<u32>=(0..160*90).map(|i|0xff000000|((i*431)&0xffffff)as u32).collect();
    let mut glass=baseline.clone();let mut flat=baseline.clone();
    let frost=dock_material::Material{primary:0x101b3a,accent:0x86c7ff,opacity:90,blur:12,scale:1,bgr:true};
    let mut painter=dock_material::Backdrop::new();
    painter.paint_styled_with_edge_opacity(&mut glass,160,(10,10,131,67),(0,0,160,90),frost,255,rounded);
    painter.paint_styled_with_edge_opacity(&mut flat,160,(10,10,131,67),(0,0,160,90),frost,255,matte);
    assert_ne!(glass,flat);
    assert_eq!(glass[10*160+10],baseline[10*160+10]);
    assert_eq!(flat[10*160+10],baseline[10*160+10]);
    let mut no_blur=baseline.clone();
    painter.paint_styled_with_edge_opacity(&mut no_blur,160,(10,10,131,67),(0,0,160,90),dock_material::Material{blur:0,..frost},255,matte);
    assert_eq!(flat,no_blur);
}
