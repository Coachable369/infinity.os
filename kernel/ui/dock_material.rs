//! Bounded, allocation-free frosted dock material over the current desktop pixels.
use crate::ui::panel_recipe;
const COLUMNS: usize = 256;
const ROWS: usize = 64;
const CELLS: usize = COLUMNS * ROWS;

// ------------------------=
// FUNC: expand_damage
// DESC: Restores the full sampling source before any dock repaint so clipped redraws match full-frame blur exactly.
// ------------------=
pub fn expand_damage(bounds:(usize,usize,usize,usize),clip:(usize,usize,usize,usize))->(usize,usize,usize,usize) {
    let (left,top,width,height)=bounds;let right=left.saturating_add(width);let bottom=top.saturating_add(height);
    if clip.0>=right || clip.1>=bottom || clip.2<=left || clip.3<=top {return clip;}
    (clip.0.min(left),clip.1.min(top),clip.2.max(right),clip.3.max(bottom))
}

pub struct Backdrop {
    sampled: [u32; CELLS],
    scratch: [u32; CELLS],
    retained: Option<SampleKey>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SampleKey {
    buffer: usize,
    pixels: usize,
    stride: usize,
    bounds: (usize,usize,usize,usize),
    source: (usize,usize,usize,usize),
    blur: u8,
    scale: usize,
    bgr: bool,
}

#[derive(Clone, Copy)]
pub struct Material {
    pub primary: u32,
    pub accent: u32,
    pub opacity: u8,
    pub blur: u8,
    pub scale: usize,
    pub bgr: bool,
}

impl Backdrop {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves one fixed small-grid workspace rather than allocating a desktop-sized blur surface.
    // ------------------=
    pub const fn new() -> Self { Self { sampled: [0; CELLS], scratch: [0; CELLS], retained: None } }

    // ------------------------=
    // FUNC: paint
    // DESC: Captures clean damaged backdrop pixels before a separable blur and one antialiased material composite.
    // ------------------=
    pub fn paint(&mut self, pixels: &mut [u32], stride: usize, bounds: (usize,usize,usize,usize),
        clip: (usize,usize,usize,usize), material: Material) {
        self.paint_with_edge_opacity(pixels,stride,bounds,clip,material,255);
    }

    // ------------------------=
    // FUNC: paint_with_edge_opacity
    // DESC: Reuses drawer frost for authored panels while preserving their independent border alpha.
    // ------------------=
    pub fn paint_with_edge_opacity(&mut self, pixels: &mut [u32], stride: usize,
        bounds: (usize,usize,usize,usize), clip: (usize,usize,usize,usize),
        material: Material, edge_opacity: u8) {
        self.paint_sampled(pixels,stride,bounds,clip,material,edge_opacity,None);
    }

    // ------------------------=
    // FUNC: paint_retained_with_edge_opacity
    // DESC: Refreshes clean full-panel blur samples on full paints and reuses them for clipped control updates without sampling existing foreground ink.
    // ------------------=
    pub fn paint_retained_with_edge_opacity(&mut self, pixels: &mut [u32], stride: usize,
        bounds: (usize,usize,usize,usize), clip: (usize,usize,usize,usize),
        material: Material, edge_opacity: u8, source: (usize,usize,usize,usize)) {
        self.paint_sampled(pixels,stride,bounds,clip,material,edge_opacity,Some(source));
    }

    // ------------------------=
    // FUNC: paint_sampled
    // DESC: Separates the stable backdrop sampling rectangle from the output damage clip while preserving ordinary dock repaint behavior.
    // ------------------=
    fn paint_sampled(&mut self, pixels: &mut [u32], stride: usize,
        bounds: (usize,usize,usize,usize), clip: (usize,usize,usize,usize),
        material: Material, edge_opacity: u8, retained_source: Option<(usize,usize,usize,usize)>) {
        let (left,top,width,height)=bounds;
        if stride==0 || width==0 || height==0 { return; }
        let x0=left.max(clip.0).min(stride);
        let y0=top.max(clip.1).min(pixels.len()/stride);
        let x1=left.saturating_add(width).min(clip.2).min(stride);
        let y1=top.saturating_add(height).min(clip.3).min(pixels.len()/stride);
        if x0>=x1 || y0>=y1 { return; }
        let mut source=(x0,y0,x1,y1);
        let mut reuse=false;
        if let Some(requested)=retained_source {
            let full=(requested.0.max(left).min(stride),requested.1.max(top).min(pixels.len()/stride),
                requested.2.min(left.saturating_add(width)).min(stride),
                requested.3.min(top.saturating_add(height)).min(pixels.len()/stride));
            let key=SampleKey{buffer:pixels.as_ptr() as usize,pixels:pixels.len(),stride,bounds,
                source:full,blur:material.blur,scale:material.scale,bgr:material.bgr};
            let refresh=full.0<full.2 && full.1<full.3
                && x0<=full.0 && y0<=full.1 && x1>=full.2 && y1>=full.3;
            let covers_output=full.0<=x0 && full.1<=y0 && full.2>=x1 && full.3>=y1;
            if covers_output && (refresh || self.retained==Some(key)) {
                source=full;
                reuse=!refresh;
                self.retained=Some(key);
            } else {
                // A partial first paint has no clean parent sample yet. Keep
                // the bounded legacy behavior until a full scene seeds it.
                self.retained=None;
            }
        } else {
            self.retained=None;
        }
        let (sx0,sy0,sx1,sy1)=source;
        let step=4usize.max((sx1-sx0).div_ceil(COLUMNS)).max((sy1-sy0).div_ceil(ROWS));
        let columns=(sx1-sx0).div_ceil(step);let rows=(sy1-sy0).div_ceil(step);
        if material.blur>0 && material.opacity>0 && material.opacity<100 {
          if !reuse {
            for gy in 0..rows {for gx in 0..columns {
                let mut channels=[0u32;3];let mut count=0;
                // Average actual pixels, not four corner taps which alias stars/lines into blocks.
                for y in sy0+gy*step..(sy0+(gy+1)*step).min(sy1) {
                    for x in sx0+gx*step..(sx0+(gx+1)*step).min(sx1) {
                        let p=pixels[y*stride+x];count+=1;
                        for c in 0..3 {channels[c]+=(p>>(8*c))&255;}
                    }
                }
                self.sampled[gy*COLUMNS+gx]=channels[0]/count | (channels[1]/count)<<8 | (channels[2]/count)<<16;
            }}
            let radius=(material.blur as usize*material.scale.max(1)).div_ceil(step).clamp(1,6);
            for y in 0..rows {for x in 0..columns {
                self.scratch[y*COLUMNS+x]=average(&self.sampled,columns,rows,x,y,radius,false);
            }}
            for y in 0..rows {for x in 0..columns {
                self.sampled[y*COLUMNS+x]=average(&self.scratch,columns,rows,x,y,radius,true);
            }}
          }
        } else {
            self.retained=None;
        }
        let primary=encode(material.primary,material.bgr);
        let panel=panel_recipe::Material {
            fill:(16,27,58),edge:((material.accent>>16)as u8,(material.accent>>8)as u8,material.accent as u8),
            opacity:255,edge_opacity:0,focus:0,pressed:false,scale:material.scale,
        };
        let rim=panel_recipe::rim_width(panel_recipe::Surface::Panel,material.scale);
        let opacity=u32::from(material.opacity.min(100));
        // Material-specific optical density: leave enough backdrop to perceive frost,
        // while preserving exact transparent/opaque endpoints of the preference slider.
        let tint=(u64::from(opacity).pow(4)*255/100_000_000)as u32;
        for y in y0..y1 {for x in x0..x1 {
            let dx=x-left;let dy=y-top;
            let on_rim=dx.min(dy).min(width-1-dx).min(height-1-dy)<rim;
            // Only the bounded authored rim needs atlas sampling; interior frost
            // keeps the existing fixed-grid blur and optical-density fast path.
            let coverage=if on_rim {
                panel_recipe::pixel(panel_recipe::Surface::Panel,width,height,dx,dy,panel).3
            }else{255};
            let edge=if on_rim {
                panel_recipe::pixel(panel_recipe::Surface::Panel,width,height,dx,dy,
                    panel_recipe::Material{opacity:0,edge_opacity,..panel})
            }else{(0,0,0,0)};
            if coverage==0 && edge.3==0 {continue;}
            let original=pixels[y*stride+x];
            let backdrop=if material.blur>0 && material.opacity>0 && material.opacity<100 {
                self.sample(columns,rows,(x-sx0)*256/step,(y-sy0)*256/step)
            }else {original};
            let mut color=mix(backdrop,primary,tint);
            let sheen=if opacity==0 || opacity==100 {0}else{(20usize.saturating_sub(dy*16/height)) as u32};
            color=mix(color,0x00ffffff,sheen);
            color=mix(original,color,u32::from(coverage));
            let edge_rgb=encode((u32::from(edge.0)<<16)|(u32::from(edge.1)<<8)|u32::from(edge.2),material.bgr);
            pixels[y*stride+x]=(original&0xff000000)|mix(color,edge_rgb,u32::from(edge.3));
        }}
    }

    // ------------------------=
    // FUNC: sample
    // DESC: Bilinearly reconstructs the defocused grid without hard pixel-block boundaries.
    // ------------------=
    fn sample(&self, columns:usize,rows:usize,x:usize,y:usize)->u32 {
        let gx=(x/256).min(columns-1);let gy=(y/256).min(rows-1);
        let next_x=(gx+1).min(columns-1);let next_y=(gy+1).min(rows-1);
        mix(mix(self.sampled[gy*COLUMNS+gx],self.sampled[gy*COLUMNS+next_x],(x%256)as u32),
            mix(self.sampled[next_y*COLUMNS+gx],self.sampled[next_y*COLUMNS+next_x],(x%256)as u32),(y%256)as u32)
    }
}

// ------------------------=
// FUNC: average
// DESC: Executes one bounded separable blur pass over the captured backdrop grid.
// ------------------=
fn average(data:&[u32;CELLS],columns:usize,rows:usize,x:usize,y:usize,radius:usize,vertical:bool)->u32 {
    let center=if vertical {y}else{x};let extent=if vertical {rows}else{columns};
    let begin=center.saturating_sub(radius);let end=(center+radius+1).min(extent);
    let mut channels=[0;3];
    for at in begin..end {let p=data[if vertical {at*COLUMNS+x}else{y*COLUMNS+at}];
        for c in 0..3 {channels[c]+=(p>>(8*c))&255;}}
    let count=(end-begin)as u32;
    channels[0]/count | (channels[1]/count)<<8 | (channels[2]/count)<<16
}

// ------------------------=
// FUNC: mix
// DESC: Blends packed channels without crossing their lanes or disturbing framebuffer alpha.
// ------------------=
fn mix(a:u32,b:u32,amount:u32)->u32 {
    let t=amount.min(255);let mut result=0;
    for c in 0..3 {result|=((((a>>(8*c))&255)*(255-t)+((b>>(8*c))&255)*t)/255)<<(8*c);}
    result
}

// ------------------------=
// FUNC: encode
// DESC: Converts portable RGB theme colors to the active native framebuffer channel ordering.
// ------------------=
fn encode(rgb:u32,bgr:bool)->u32 {
    if bgr {rgb&0x00ffffff}else{((rgb&255)<<16)|(rgb&0xff00)|((rgb>>16)&255)}
}

// ------------------------=
// FUNC: rounded_coverage
// DESC: Applies four-by-four subpixel coverage at rounded corners while keeping straight spans inexpensive.
// ------------------=
pub fn rounded_coverage(x:usize,y:usize,w:usize,h:usize,radius:usize)->u8 {
    if x>=w || y>=h {return 0;}
    let radius=radius.min(w/2).min(h/2);
    if radius==0 || (x>=radius && x<w-radius) || (y>=radius && y<h-radius) {return 255;}
    let cx=if x<radius {radius*8}else{(w-radius)*8} as i64;
    let cy=if y<radius {radius*8}else{(h-radius)*8} as i64;
    let mut count=0;
    for sy in [1,3,5,7] {for sx in [1,3,5,7] {
        let dx=(x*8+sx) as i64-cx;let dy=(y*8+sy)as i64-cy;
        count+=u16::from(dx*dx+dy*dy<=(radius*8).pow(2)as i64);
    }}
    ((count*255+8)/16)as u8
}

// ------------------------=
// FUNC: launcher_pixel
// DESC: Keeps launcher interiors identical in all states; only the outer rim and external glow respond.
// ------------------=
pub fn launcher_pixel(size:usize,x:i32,y:i32,accent:u32,active:bool)->(u8,u8,u8,u8) {
    if size==0 {return (0,0,0,0);}
    let scale=(size/46).max(1);let radius=(size/5).max(7);
    let coverage=if x>=0 && y>=0 {rounded_coverage(x as usize,y as usize,size,size,radius)}else{0};
    let inner=if x>0 && y>0 {rounded_coverage(x as usize-1,y as usize-1,size.saturating_sub(2),size.saturating_sub(2),radius.saturating_sub(1))}else{0};
    if inner==255 {return (4,28,52,220);}
    let edge=coverage.saturating_sub(inner);
    if coverage>0 {
        let rim=if active {mix(accent,0xffffff,70)}else{mix(accent,0x041c34,145)};
        let rgb=mix(0x041c34,rim,u32::from(edge));
        return ((rgb>>16)as u8,(rgb>>8)as u8,rgb as u8,coverage);
    }
    if active {
        for spread in 1..=5*scale {
            let px=x+spread as i32;let py=y+spread as i32;
            if px>=0 && py>=0 && rounded_coverage(px as usize,py as usize,size+spread*2,size+spread*2,radius+spread)>0 {
                let alpha=((5*scale+1-spread)*36/(5*scale)) as u8;
                return ((accent>>16)as u8,(accent>>8)as u8,accent as u8,alpha);
            }
        }
    }
    (0,0,0,0)
}
