//! Export native UIKit surface pixels for the Python ttk adapter.
#![allow(dead_code)]
#[path="../../kernel/ui/panel_recipe.rs"] mod panel_recipe;
#[path="../../kernel/ui/kit_tokens.rs"] mod kit_tokens;
use panel_recipe::{Material,Surface};
use std::io::Write;
// ------------------------=
// FUNC: main
// DESC: Export the production surface sampler's exact default button and field states, without recreating artwork.
// ------------------=
fn main() {
    let mut out=std::io::stdout().lock();
    let colors=kit_tokens::colors();
    let channels=|value:u32|((value>>16)as u8,(value>>8)as u8,value as u8);
    for kind in [Surface::Secondary,Surface::Primary,Surface::Field] {
        for state in 0..3 {
            for y in 0..40 { for x in 0..80 {
                let fill=channels(colors[if kind==Surface::Primary {6}else{1}]);
                let m=Material{fill,edge:channels(colors[3]),opacity:255,edge_opacity:255,
                    focus:if state==1 {255}else{0},pressed:state==2,scale:1};
                let (r,g,b,a)=panel_recipe::pixel(kind,80,40,x,y,m);
                let rgb=[r,g,b].map(|c|c as u32);
                let panel=channels(colors[1]);
                let bg=[panel.0 as u32,panel.1 as u32,panel.2 as u32];
                let bytes=std::array::from_fn::<u8,3,_>(|i|((rgb[i]*a as u32+bg[i]*(255-a as u32)+127)/255) as u8);
                out.write_all(&bytes).unwrap();
            }}
        }
    }
}
