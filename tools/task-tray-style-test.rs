#[path="../kernel/ui/panel_recipe.rs"] mod panel_recipe;
use panel_recipe::{SurfaceStyle,Surface,Material};
// ------------------------=
// FUNC: main
// DESC: Exercises legacy inheritance, independent tray persistence, malformed payload rejection and distinct native panel pixels.
// ------------------=
fn main(){
 let mut bytes=vec![0u8;108];bytes[..8].copy_from_slice(b"IUISTY01");bytes[8]=1;bytes[10]=8;
 let mut legacy=bytes[..84].to_vec();legacy[10]=6;
 assert_eq!(panel_recipe::decode_styles(&legacy).unwrap()[6],SurfaceStyle::AUTHORED);
 let mut previous=bytes[..96].to_vec();previous[10]=7;
 assert_eq!(panel_recipe::decode_styles(&previous).unwrap()[7],SurfaceStyle::AUTHORED);
 let slot=84;bytes[slot..slot+4].copy_from_slice(&[1,2,10,1]);
 bytes[slot+4..slot+8].copy_from_slice(&0xd8101b3au32.to_le_bytes());
 let styles=panel_recipe::decode_styles(&bytes).unwrap();
 assert_eq!(styles[6].fill,Some(0xd8101b3a));assert_eq!(styles[3],SurfaceStyle::AUTHORED);
 assert_eq!(panel_recipe::resolve_task_tray_style(SurfaceStyle{radius:10,..SurfaceStyle::AUTHORED},styles[6]),styles[6]);
 let m=Material{fill:(16,27,58),edge:(134,199,255),opacity:208,edge_opacity:175,focus:0,pressed:false,scale:1};
 let image=|style| (0..90).flat_map(|y|(0..120).map(move|x|panel_recipe::pixel_styled(Surface::Panel,120,90,x,y,m,style))).collect::<Vec<_>>();
 let smoke=image(styles[6]);let sapphire=image(SurfaceStyle{fill:Some(0xe8204dad),..styles[6]});
 assert_ne!(smoke,sapphire);assert_eq!(smoke[0].3,0);
 bytes[96]=2;bytes[99]=1;bytes[100..104].copy_from_slice(&0xff91bdffu32.to_le_bytes());
 let customized=panel_recipe::decode_styles(&bytes).unwrap();
 assert_eq!(customized[6],styles[6]);assert_eq!(customized[2],styles[2]);
 let frost=panel_recipe::tray_hover_recipe(customized[7]);
 assert_eq!(frost.color,(145,189,255));assert_eq!(frost.opacity,156);
 let glass=panel_recipe::tray_hover_recipe(SurfaceStyle::AUTHORED);
 assert_eq!(glass.opacity,112);assert_eq!(glass.color,(248,250,255));
 assert_ne!(glass,frost);
 assert_eq!(panel_recipe::tray_hover_recipe(SurfaceStyle{material:3,..customized[7]}).opacity,200);
 bytes[10]=6;assert!(panel_recipe::decode_styles(&bytes).is_none());
 bytes[10]=8;bytes[84]=29;assert!(panel_recipe::decode_styles(&bytes).is_none());
}
