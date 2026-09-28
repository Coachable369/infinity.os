//! UIKIT attached glass fin. Shared pixel recipe for native paint and visual QA.

// ------------------------=
// FUNC: smooth_min
// DESC: Rounds the meeting of a shoulder and the outer rim without polygon corners.
// ------------------=
fn smooth_min(a: f32, b: f32, radius: f32) -> f32 {
    let h = (radius - (a-b).abs()).max(0.0) / radius;
    a.min(b) - h*h*radius*0.25
}

// ------------------------=
// FUNC: distance
// DESC: Measures the attached fin with a straight root and smoothly joined sloped shoulders.
// ------------------=
fn distance(w:f32,h:f32,x:f32,y:f32,s:f32)->f32 {
    let slope = 0.48;
    let top = (y-x*slope)*0.9;
    let bottom = (h-y-x*slope)*0.9;
    smooth_min(top.min(bottom),w-x,4.0*s)
}

// ------------------------=
// FUNC: pixel
// DESC: Samples antialiased navy glass and a blue-violet rim; no outline separates the tab root from its window.
// ------------------=
pub fn pixel(width:u32,height:u32,x:i32,y:i32,left:bool,emphasis:u8)->(u8,u8,u8,u8) {
    if width==0 || height==0 {return (0,0,0,0);}
    let s=(width as f32/48.0).max(1.0);
    let x=if left {width as f32-x as f32-1.0}else{x as f32};
    if x<0.0 {return (0,0,0,0);}
    let mut coverage=0.0;
    for dy in [0.25,0.75] {for dx in [0.25,0.75] {
        if distance(width as f32,height as f32,x+dx,y as f32+dy,s)>=0.0 {coverage+=0.25;}
    }}
    let d=distance(width as f32,height as f32,x+0.5,y as f32+0.5,s);
    let t=(y as f32/height as f32).clamp(0.0,1.0);
    let rim=(75.0+115.0*t,155.0-65.0*t,255.0);
    if coverage==0.0 {
        let glow=(1.0+d/(4.0*s)).clamp(0.0,1.0);
        return (rim.0 as u8,rim.1 as u8,255,(glow*glow*(35.0+emphasis as f32*0.25)) as u8);
    }
    let edge=(1.0-d/(2.0*s)).clamp(0.0,1.0);
    let sheen=(x/width as f32).clamp(0.0,1.0);
    let base=(10.0+sheen*10.0,15.0+sheen*13.0,30.0+sheen*35.0);
    let mix=|a:f32,b:f32| (a+(b-a)*edge) as u8;
    (mix(base.0,rim.0),mix(base.1,rim.1),mix(base.2,rim.2),(coverage*245.0) as u8)
}
