//! Production rounded-rim coverage and visual regression fixture.
#![allow(dead_code)]
#[path = "../kernel/ui/panel_recipe.rs"] mod panel_recipe;
use panel_recipe::{Material, Surface, SurfaceStyle};
// ------------------------=
// FUNC: main
// DESC: Checks opaque rim centers across scales and radii and captures native surface pixels for visual review.
// ------------------=
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let check = !args.iter().any(|a| a == "--capture-only");
    run(&args[1], check);
}
// ------------------------=
// FUNC: run
// DESC: Shares the rounded-rim regression with the release painter suite.
// ------------------=
pub fn run(output: &str, check: bool) {
    assert_eq!(std::env::var("INFINITY_BUILD_KIT_ACTIVE").as_deref(), Ok("1"));
    let mut image = vec![12u8; 840*480*3];
    let mut samples = 0;
    for (row, kind) in [Surface::Panel, Surface::Field, Surface::Frame].into_iter().enumerate() {
        for focus in [0,128,255] { for scale in 1..=3 {
            let m = Material {fill:(14,28,54),edge:(54,87,116),opacity:255,edge_opacity:255,focus,pressed:false,scale};
            for radius in [6,12,24,40] {
                let s = SurfaceStyle {corners:2,radius,..SurfaceStyle::AUTHORED};
                let p = panel_recipe::pixel_styled(kind,240*scale,96*scale,120*scale,0,m,s);
                if check {
                    assert_eq!(p.3,255,"opaque outer rim must survive radius and density changes");
                    let logical=panel_recipe::pixel_styled(kind,240,96,120,0,Material{scale:1,..m},s);
                    assert_eq!(p,logical,"the first rim texel must retain its color at every density");
                }
                samples += 1;
            }
        }}
        for column in 0..3 {
            let (width,height,scale) = if column==2 {(240,136,2)}else{(240,136,1)};
            let style = if matches!(kind,Surface::Field|Surface::Panel) {SurfaceStyle::AUTHORED} else {SurfaceStyle {corners:2,radius:if column==0{12}else{32},..SurfaceStyle::AUTHORED}};
            let m = Material {fill:(14,28,54),edge:(89,149,245),opacity:220,edge_opacity:255,focus:[0,128,255][column],pressed:false,scale};
            for y in 0..height {for x in 0..width {
                let p=panel_recipe::pixel_styled(kind,width,height,x,y,m,style);
                if check && matches!(kind,Surface::Field|Surface::Panel) {
                    let continuous=panel_recipe::pixel_styled(kind,width,height,x,y,m,
                        SurfaceStyle{corners:2,radius:if kind==Surface::Field {8}else{12},..style});
                    assert_eq!(p,continuous,"default field and panel must retain continuous corners in every focus state");
                }
                let at=((row*160+12+y)*840+column*280+16+x)*3;
                for (c,value) in [p.0,p.1,p.2].into_iter().enumerate() {
                    image[at+c]=((u32::from(value)*u32::from(p.3)+12*(255-u32::from(p.3)))/255)as u8;
                }
            }}
        }
    }
    let mut out=b"P6\n840 480\n255\n".to_vec();out.extend(image);
    std::fs::write(output,out).unwrap();
    println!("{samples} rounded-rim cases; production render captured");
}
