//! Export native UIKit surface pixels for the Python ttk adapter.
#![allow(dead_code)]
#[path="../../kernel/ui/panel_recipe.rs"] mod panel_recipe;
use panel_recipe::{Material,Surface};
use std::io::Write;
// ------------------------=
// FUNC: main
// DESC: Export the production surface sampler's exact default button and field states, without recreating artwork.
// ------------------=
fn main() {
    let mut out=std::io::stdout().lock();
    for kind in [Surface::Secondary,Surface::Primary,Surface::Field] {
        for state in 0..3 {
            for y in 0..40 { for x in 0..80 {
                let fill=if kind==Surface::Primary {(62,111,255)}else{(16,27,58)};
                let m=Material{fill,edge:(134,199,255),opacity:255,edge_opacity:255,
                    focus:if state==1 {255}else{0},pressed:state==2,scale:1};
                let (r,g,b,a)=panel_recipe::pixel(kind,80,40,x,y,m);
                let rgb=[r,g,b].map(|c|c as u32);
                let bg=[16u32,27,58];
                let bytes=std::array::from_fn::<u8,3,_>(|i|((rgb[i]*a as u32+bg[i]*(255-a as u32)+127)/255) as u8);
                out.write_all(&bytes).unwrap();
            }}
        }
    }
}
