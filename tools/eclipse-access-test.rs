//! Behavioral pixel checks for production access surfaces and bitmap filtering.
#![allow(dead_code)]

#[path = "../kernel/core/bootstrap/access_style.rs"]
mod access_style;
#[path = "../kernel/core/bootstrap/access_scale.rs"]
mod access_scale;
#[path = "../kernel/core/bootstrap/access_view.rs"]
mod access_view;
#[path = "../kernel/core/bootstrap/panel_view.rs"]
mod panel_view;
#[path = "../kernel/ui/panel_recipe.rs"]
mod panel_recipe;
#[path = "../kernel/ui/surface_cache.rs"]
mod surface_cache;
use panel_recipe::Surface;
#[path = "../kernel/ui/bitmap.rs"]
mod bitmap;
#[path = "../kernel/ui/geometry.rs"]
mod geometry;
#[path = "../kernel/ui/skin.rs"]
mod skin;
mod ui {
    pub(crate) use crate::{geometry, panel_recipe, surface_cache, skin};
}
pub const INFINITY_UI_ABI_VERSION: u16 = 1;
mod bootstrap {
    pub const SPLASH_BMP: &[u8] = include_bytes!("../assets/boot/infinity-eclipse-header-v1.bmp");
}

// ------------------------=
// FUNC: panel_entrance_and_reduced_motion
// DESC: Verifies centered monotonic entrance, scene scoping, late ticks and motion suppression.
// ------------------=
#[test]
fn panel_entrance_and_reduced_motion() {
    let mut motion=access_style::PanelMotion::new();
    assert!(motion.advance(3,1000,false));
    assert_eq!(motion.rectangle((20,30,400,600)),(220,330,0,0));
    let mut last=0;
    for elapsed in (0..=360).step_by(16) {
        motion.advance(3,1000+elapsed,false);
        assert!(motion.extent>=last);
        let (x,y,w,h)=motion.rectangle((20,30,400,600));
        assert!(x>=20 && y>=30 && x+w<=420 && y+h<=630);
        assert!((2*x+w).abs_diff(440)<=1);
        assert!((2*y+h).abs_diff(660)<=1);
        last=motion.extent;
    }
    motion.advance(3,1360,false);
    assert_eq!(motion.rectangle((20,30,400,600)),(20,30,400,600));
    motion.advance(3,10000,false);
    assert_eq!(motion.extent,1024);
    assert_eq!(motion.started_ms,1000);
    motion.advance(4,10000,false);
    assert_eq!(motion.extent,0);
    motion.advance(4,10016,true);
    assert_eq!((motion.extent,motion.glow),(1024,24));
    assert!(!motion.advance(4,12000,true));
}

// ------------------------=
// FUNC: panel_glow_preserves_content_and_parent_clip
// DESC: Checks production glow pixels leave the content and wallpaper untouched.
// ------------------=
#[test]
fn panel_glow_preserves_content_and_parent_clip() {
    access_style::animate(0x100,0,false,true);
    let mut display=DisplayDevice::new(200,160);
    display.access_panel(20,20,160,120);
    let before=display.pixels.clone();
    display.clip=Some(Region{left:20,top:20,right:180,bottom:80});
    display.access_panel_glow(20,20,160,120);
    assert!((20..22).any(|y|(20..180).any(|x|display.pixels[y*200+x]!=before[y*200+x])));
    assert_eq!(display.pixels[60*200+100],before[60*200+100]);
    for y in 80..160 { assert_eq!(&display.pixels[y*200..(y+1)*200],&before[y*200..(y+1)*200]); }
    for y in 0..160 { assert_eq!(&display.pixels[y*200..y*200+20],&before[y*200..y*200+20]); }
}

// ------------------------=
// FUNC: frost_uses_fixture_framebuffer_and_clip
// DESC: Proves production raw frost writes reach the asserted byte pixels through aligned stable storage without changing exterior pixels.
// ------------------=
#[test]
fn frost_uses_fixture_framebuffer_and_clip() {
    let mut display=DisplayDevice::new(80,60);
    assert_eq!((display.buffer as usize)%core::mem::align_of::<u32>(),0);
    let before=display.pixels.clone();
    let clip=Region{left:12,top:14,right:55,bottom:42};
    display.clip=Some(clip);
    display.access_frost_panel(8,8,64,44,access_style::SURFACE,access_style::BORDER,235,255);
    assert_eq!(display.buffer,display.pixels.0.as_mut_ptr());
    assert_ne!(display.pixels[25*80+30],before[25*80+30]);
    for y in 0..60 {for x in 0..80 {
        if x<clip.left || x>=clip.right || y<clip.top || y>=clip.bottom {
            assert_eq!(display.pixels[y*80+x],before[y*80+x]);
        }
    }}
}

// ------------------------=
// FUNC: panel_focus_does_not_restart_entrance
// DESC: Exercises the actual shared animation state across focus changes and desktop reentry.
// ------------------=
#[test]
fn panel_focus_does_not_restart_entrance() {
    access_style::leave_scene();
    access_style::animate(0x30000,0,false,false);
    access_style::animate(0x30001,400,true,false);
    assert_eq!(access_style::panel_motion().extent,1024);
    assert_eq!(access_style::panel_motion().started_ms,0);
    access_style::leave_scene();
    access_style::animate(0x30001,500,false,false);
    assert_eq!(access_style::panel_motion().extent,0);
    access_style::animate(0x30001,500,false,true);
}

// ------------------------=
// FUNC: panel_motion_render_frames
// DESC: Produces a framebuffer contact sheet of production panel surfaces and clipped entrance states.
// ------------------=
#[test]
fn panel_motion_render_frames() {
    let mut display=DisplayDevice::new(1000,240);
    let mut motion=access_style::PanelMotion::new();
    motion.advance(1,0,false);
    access_style::animate(0x100,0,false,true);
    for (index,time) in [0,64,128,240,360].into_iter().enumerate() {
        motion.advance(1,time,false);
        let (x,y,w,h)=motion.rectangle((index*200+10,10,180,220));
        display.access_panel(index*200+10,10,180,220);
        display.access_surface(index*200+26,80,148,36,Surface::Field,access_style::BACKGROUND,access_style::BORDER,255,false);
        display.access_surface(index*200+26,132,148,36,Surface::Primary,access_style::PRIMARY,access_style::BORDER,255,false);
        display.access_panel_glow(index*200+10,10,180,220);
        let mut source=Vec::new();
        for row in 10..230 { for col in index*200+10..index*200+190 {
            source.push(u32::from_le_bytes(display.pixels[row*1000+col]));
            display.pixels[row*1000+col]=[9,15,38,0];
        }}
        for row in 0..h { for col in 0..w {
            display.pixels[(y+row)*1000+x+col]=access_scale::sample(&source,180,220,col,row,w,h).to_le_bytes();
        }}
    }
    let mut image=b"P6\n1000 240\n255\n".to_vec();
    for pixel in display.pixels { image.extend_from_slice(&pixel[..3]); }
    std::fs::write("build/behavior-tests/access-panel-motion.ppm",image).unwrap();
}

// ------------------------=
// FUNC: panel_filter_preserves_channels_and_endpoints
// DESC: Verifies true scaled panel pixels and degenerate dimensions without prose or source-text oracles.
// ------------------=
#[test]
fn panel_filter_preserves_channels_and_endpoints() {
    let source=[0xff000000,0xff0000ff,0xff00ff00,0xffff0000];
    for y in 0..2 { for x in 0..2 {
        assert_eq!(access_scale::sample(&source,2,2,x,y,2,2),source[y*2+x]);
    }}
    assert_eq!(access_scale::sample(&source,2,2,0,0,1,1),0xff404040);
    assert_eq!(access_scale::sample(&[],0,0,0,0,0,0),0);
    assert_eq!(access_scale::sample(&source[..1],2,2,0,0,2,2),0);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Region { left: usize, top: usize, right: usize, bottom: usize }
#[derive(Clone, Debug, PartialEq, Eq)]
struct Wordmark { original: bool, rectangle: [usize; 4], crop: [u8; 4], opacity: u8, fill: bool }
#[derive(Clone, Debug, PartialEq, Eq)]
struct Label { rectangle: [usize; 4], bytes: Vec<u8>, color: [u8; 3], pixels: usize, centered: bool, strong: bool }

// ------------------------=
// FUNC: neutral_access_and_matching_desktop_palette
// DESC: Checks actual focus pixels, neutral text and default desktop token parity without changing custom themes.
// ------------------=
#[test]
fn neutral_access_and_matching_desktop_palette() {
    let tokens=skin::default_dark_skin().tokens;
    for (role,expected) in [(skin::ColorRole::Canvas,access_style::BACKGROUND),
        (skin::ColorRole::Panel,access_style::SURFACE),(skin::ColorRole::PanelRaised,access_style::ELEVATED),
        (skin::ColorRole::Border,access_style::BORDER),(skin::ColorRole::Accent,access_style::PRIMARY),
        (skin::ColorRole::Focus,access_style::FOCUS),(skin::ColorRole::TextPrimary,access_style::TEXT),
        (skin::ColorRole::TextSecondary,access_style::SECONDARY)] {
        assert_eq!(tokens.color(role).channels(),expected);
    }
    assert_eq!(access_style::TEXT,(255,255,255));
    assert_eq!(access_style::SECONDARY,(190,190,190));
    let mut registry=skin::SkinRegistry::new();
    registry.set_accent(0x3e6fff,skin::AppearanceScope::User).unwrap();
    assert_eq!(registry.color(skin::ColorRole::Focus).channels(),(255,255,255));
    registry.set_accent(0xc08040,skin::AppearanceScope::User).unwrap();
    assert_eq!(registry.color(skin::ColorRole::Accent).rgb24(),0xc08040);
    access_style::animate(123,0,false,true);
    let mut display=DisplayDevice::new(320,140);
    for (i,focused) in [false,true].into_iter().enumerate() {
        display.access_surface(12,12+i*60,296,44,Surface::Panel,access_style::SURFACE,access_style::BORDER,255,focused);
    }
    let mut changed=0;
    let mut normal_light=0;let mut focused_light=0;
    for y in 0..44 { for x in 0..296 {
        if x.min(y).min(295-x).min(43-y)>=2 {continue;}
        let normal=display.pixels[(12+y)*320+12+x];
        let focused=display.pixels[(72+y)*320+12+x];
        let light=|pixel:[u8;4]|pixel[..3].iter().map(|v|u32::from(*v)).sum::<u32>();
        normal_light+=light(normal);focused_light+=light(focused);
        changed+=usize::from(normal!=focused);
    } }
    assert!(changed>0);
    assert!(focused_light>normal_light);
    let mut image=format!("P6\n320 140\n255\n").into_bytes();
    for pixel in display.pixels {image.extend_from_slice(&pixel[..3]);}
    std::fs::write("build/behavior-tests/eclipse-white-focus.ppm",image).unwrap();
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct FramebufferPixels(Box<[u32]>);

impl core::ops::Deref for FramebufferPixels {
    type Target = [[u8; 4]];
    // ------------------------=
    // FUNC: deref
    // DESC: Exposes existing byte-oriented pixel assertions over aligned native framebuffer storage.
    // ------------------=
    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.0.as_ptr().cast::<[u8;4]>(),self.0.len()) }
    }
}

impl core::ops::DerefMut for FramebufferPixels {
    // ------------------------=
    // FUNC: deref_mut
    // DESC: Shares mutable RGBA bytes with the exact allocation used by production raw framebuffer painters.
    // ------------------=
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.0.as_mut_ptr().cast::<[u8;4]>(),self.0.len()) }
    }
}

impl IntoIterator for FramebufferPixels {
    type Item = [u8;4];
    type IntoIter = core::iter::Map<std::vec::IntoIter<u32>,fn(u32)->[u8;4]>;
    // ------------------------=
    // FUNC: into_iter
    // DESC: Preserves existing image export loops without changing the underlying aligned framebuffer allocation.
    // ------------------=
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_vec().into_iter().map(u32::to_le_bytes)
    }
}

impl<'a> IntoIterator for &'a FramebufferPixels {
    type Item = &'a [u8;4];
    type IntoIter = core::slice::Iter<'a,[u8;4]>;
    // ------------------------=
    // FUNC: into_iter
    // DESC: Preserves borrowed pixel comparisons against the native framebuffer bytes.
    // ------------------=
    fn into_iter(self) -> Self::IntoIter { self.iter() }
}

struct DisplayDevice {
    width: usize,
    height: usize,
    stride: usize,
    format: u32,
    buffer: *mut u32,
    pixels: FramebufferPixels,
    clip: Option<Region>,
    damage: Vec<Region>,
    wordmark: Option<Wordmark>,
    labels: Vec<Label>,
}
impl DisplayDevice {
    // ------------------------=
    // FUNC: new
    // DESC: Creates bounded host framebuffer transport for the unchanged production surface and glyph painters.
    // ------------------=
    fn new(width: usize, height: usize) -> Self {
        let mut pixels=FramebufferPixels(vec![u32::from_le_bytes([9,15,38,0]);width*height].into_boxed_slice());
        let buffer=pixels.0.as_mut_ptr();
        Self { width, height, stride:width, format:0, buffer, pixels, clip: None,
            damage: Vec::new(), wordmark: None, labels: Vec::new() }
    }
    // ------------------------=
    // FUNC: ui_scale
    // DESC: Mirrors the production framebuffer scale seam while using the actual shared panel painter and asset sampler.
    // ------------------=
    fn ui_scale(&self) -> usize {
        if self.width >= 2560 && self.height >= 1440 { 2 } else { 1 }
    }
    // ------------------------=
    // FUNC: active_accent_surface
    // DESC: Supplies the native default skin through its semantic API without introducing an alternate panel recipe.
    // ------------------=
    fn active_accent_surface(&self, surface: skin::AccentSurface) -> access_style::Rgb {
        skin::SkinRegistry::new().accent_surface(surface).channels()
    }
    // ------------------------=
    // FUNC: clipped_render_region
    // DESC: Intersects framebuffer and parent damage clips before the production painter may address a pixel.
    // ------------------=
    fn clipped_render_region(&self, left: usize, top: usize, width: usize, height: usize) -> Option<Region> {
        let clip = self.clip.unwrap_or(Region { left: 0, top: 0, right: self.width, bottom: self.height });
        let region = Region { left: left.max(clip.left), top: top.max(clip.top),
            right: left.saturating_add(width).min(self.width).min(clip.right),
            bottom: top.saturating_add(height).min(self.height).min(clip.bottom) };
        (region.left < region.right && region.top < region.bottom).then_some(region)
    }
    // ------------------------=
    // FUNC: blend_color_unchecked
    // DESC: Records actual composited color and coverage while rejecting any painter write outside its declared clip.
    // ------------------=
    fn blend_color_unchecked(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8, alpha: u8) {
        assert!(x < self.width && y < self.height);
        if let Some(clip) = self.clip { assert!(x >= clip.left && x < clip.right && y >= clip.top && y < clip.bottom); }
        let pixel = &mut self.pixels[y * self.width + x];
        for (channel, value) in [r, g, b].into_iter().enumerate() {
            pixel[channel] = ((pixel[channel] as u16 * (255-alpha as u16) + value as u16 * alpha as u16) / 255) as u8;
        }
        pixel[3] = alpha + (pixel[3] as u16 * (255-alpha as u16) / 255) as u8;
    }
    // ------------------------=
    // FUNC: mark_dirty_rect
    // DESC: Retains structured damage geometry independently of human-readable labels.
    // ------------------=
    fn mark_dirty_rect(&mut self, left: usize, top: usize, width: usize, height: usize) {
        self.damage.push(Region { left, top, right: left+width, bottom: top+height });
    }
    // ------------------------=
    // FUNC: paint_bitmap_template_rect
    // DESC: Captures the production wordmark asset binding; actual sampling behavior is exercised separately against RuntimeBitmap below.
    // ------------------=
    fn paint_bitmap_template_rect(&mut self, bytes: &[u8], left: usize, top: usize,
        width: usize, height: usize, crop: [u8; 4], opacity: u8, fill: bool) {
        self.wordmark = Some(Wordmark { original: bytes == bootstrap::SPLASH_BMP,
            rectangle: [left, top, width, height], crop, opacity, fill });
    }
    // ------------------------=
    // FUNC: access_label
    // DESC: Captures the fitted Inter label transport and numeric bounds without using rendered copy as an oracle.
    // ------------------=
    fn access_label(&mut self, (left, top, width, height): (usize, usize, usize, usize),
        label: &[u8], pixels: usize, (r, g, b): access_style::Rgb, centered: bool, strong: bool) {
        self.labels.push(Label { rectangle: [left,top,width,height], bytes: label.to_vec(),
            color: [r,g,b], pixels, centered, strong });
    }
}

// ------------------------=
// FUNC: rgba_bitmap
// DESC: Encodes deterministic top-down BGRA pixels with explicit alpha masks for the real runtime decoder.
// ------------------=
fn rgba_bitmap(width: usize, height: usize, pixels: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(pixels.len(), width*height);
    let mut bytes = vec![0; 70 + width*height*4];
    bytes[..2].copy_from_slice(b"BM");
    for (offset, value) in [(10,70u32),(14,56),(18,width as u32),(22,(-(height as i32)) as u32),
        (30,3),(54,0x00ff0000),(58,0x0000ff00),(62,0x000000ff),(66,0xff000000)] {
        bytes[offset..offset+4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[26..28].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&32u16.to_le_bytes());
    for (index, &[r,g,b,a]) in pixels.iter().enumerate() {
        bytes[70+index*4..74+index*4].copy_from_slice(&[b,g,r,a]);
    }
    bytes
}

// ------------------------=
// FUNC: rounded_coverage_has_smooth_symmetric_transparent_edges
// DESC: Exercises actual coverage at compact and scaled dimensions without comparing rendered text or source tokens.
// ------------------=
#[test]
fn rounded_coverage_has_smooth_symmetric_transparent_edges() {
    let mut partial = 0;
    for (width,height) in [(1,1),(2,3),(40,24),(81,49),(160,96)] {
        for radius in [0,1,3,8,18,200] {
            for y in 0..height { for x in 0..width {
                let value = access_style::rounded_coverage(x,y,width,height,radius);
                assert_eq!(value, access_style::rounded_coverage(width-1-x,y,width,height,radius));
                assert_eq!(value, access_style::rounded_coverage(x,height-1-y,width,height,radius));
                partial += usize::from(value > 0 && value < 255);
            } }
            assert_eq!(access_style::rounded_coverage(width,0,width,height,radius),0);
            assert_eq!(access_style::rounded_coverage(0,height,width,height,radius),0);
        }
    }
    assert!(partial > 20);
    assert_eq!(access_style::rounded_coverage(0,0,40,24,8),0);
    assert_eq!(access_style::rounded_coverage(20,12,40,24,8),255);
}

// ------------------------=
// FUNC: surfaces_preserve_coverage_focus_geometry_and_damage_clips
// DESC: Compares full and clipped production pixels and verifies authored focus lighting changes no hit or damage bounds.
// ------------------=
#[test]
fn surfaces_preserve_coverage_focus_geometry_and_damage_clips() {
    access_style::animate(123,0,false,true);
    assert_eq!(access_style::focus_amount(),255);
    let clip = Region { left: 19, top: 13, right: 62, bottom: 39 };
    for fill in [access_style::SURFACE,access_style::PRIMARY] {
        let mut unfocused = DisplayDevice::new(80,60);
        let mut focused = DisplayDevice::new(80,60);
        for (display,focus) in [(&mut unfocused,false),(&mut focused,true)] {
            display.access_surface(5,6,66,42,if fill==access_style::PRIMARY {Surface::Primary}else{Surface::Panel},fill,access_style::BORDER,230,focus);
            assert_eq!(display.damage,vec![Region{left:5,top:6,right:71,bottom:48}]);
            assert!(display.pixels.iter().any(|pixel|pixel[3]>0 && pixel[3]<230));
            for y in 0..60 { for x in 0..80 {
                if x<5 || x>=71 || y<6 || y>=48 { assert_eq!(display.pixels[y*80+x],[9,15,38,0]); }
            } }
            let mut clipped = DisplayDevice::new(80,60); clipped.clip=Some(clip);
            clipped.access_surface(5,6,66,42,if fill==access_style::PRIMARY {Surface::Primary}else{Surface::Panel},fill,access_style::BORDER,230,focus);
            assert_eq!(clipped.damage,vec![clip]);
            for y in 0..60 { for x in 0..80 {
                let expected = if x>=clip.left && x<clip.right && y>=clip.top && y<clip.bottom {
                    display.pixels[y*80+x]
                } else { [9,15,38,0] };
                assert_eq!(clipped.pixels[y*80+x],expected);
            } }
        }
        assert!(unfocused.pixels.iter().zip(&focused.pixels).any(|(a,b)|a!=b));
        assert_eq!(unfocused.damage,focused.damage);
        // Authored glow changes edge coverage, not the interior's background
        // opacity or any pixel outside the shared control rectangle.
        let center=(6+21)*80+5+33;
        assert_eq!(unfocused.pixels[center][3],focused.pixels[center][3]);
    }
    let mut edge=DisplayDevice::new(80,60);
    edge.access_panel(72,52,60,40);
    assert_eq!(edge.damage,vec![Region{left:72,top:52,right:80,bottom:60}]);
    let before=edge.pixels.clone();
    edge.access_panel(80,60,60,40);
    edge.access_panel(0,0,0,40);
    assert_eq!(edge.pixels,before);
}

// ------------------------=
// FUNC: authored_fill_and_border_alpha_remain_independent
// DESC: Checks independent zero/partial/opaque fill and edge layers against source-over alpha, including authored focus glow.
// ------------------=
#[test]
fn authored_fill_and_border_alpha_remain_independent() {
    access_style::animate(123,0,false,true);
    for focused in [false,true] {
        let mut opaque=DisplayDevice::new(80,60);
        opaque.access_surface_alpha(5,6,66,42,Surface::Panel,access_style::SURFACE,
            access_style::BORDER,255,255,focused);
        assert!(opaque.pixels.iter().any(|pixel|pixel[3]>0));
        let mut fill_only=DisplayDevice::new(80,60);
        fill_only.access_surface_alpha(5,6,66,42,Surface::Panel,access_style::SURFACE,
            access_style::BORDER,255,0,focused);
        let mut edge_only=DisplayDevice::new(80,60);
        edge_only.access_surface_alpha(5,6,66,42,Surface::Panel,access_style::SURFACE,
            access_style::BORDER,0,255,focused);
        for fill_alpha in [0u8,64,127,255] {
            for border_alpha in [0u8,64,127,255] {
                let mut display=DisplayDevice::new(80,60);
                display.access_surface_alpha(5,6,66,42,Surface::Panel,access_style::SURFACE,
                    access_style::BORDER,fill_alpha,border_alpha,focused);
                for y in 0..42 { for x in 0..66 {
                    let index=(y+6)*80+x+5;
                    let fill=u32::from(fill_only.pixels[index][3])*u32::from(fill_alpha)/255;
                    let edge=u32::from(edge_only.pixels[index][3])*u32::from(border_alpha)/255;
                    let expected=(edge*255+fill*(255-edge)+127)/255;
                    assert_eq!(u32::from(display.pixels[index][3]),expected);
                } }
                if fill_alpha==0 && border_alpha==0 {
                    assert!(display.damage.is_empty());
                    assert!(display.pixels.iter().all(|pixel|*pixel==[9,15,38,0]));
                }
                if border_alpha==255 {
                    let mut native=DisplayDevice::new(80,60);
                    native.access_surface(5,6,66,42,Surface::Panel,access_style::SURFACE,
                        access_style::BORDER,fill_alpha,focused);
                    assert_eq!(display.pixels,native.pixels);
                }
            }
        }
    }
}

// ------------------------=
// FUNC: buttons_keep_surface_hit_geometry_and_forward_label_contract
// DESC: Exercises fallback primary and secondary recipes, native label geometry and parent damage clipping.
// ------------------=
#[test]
fn buttons_keep_surface_hit_geometry_and_forward_label_contract() {
    for primary in [false,true] { for focused in [false,true] {
        for height in [20usize,48,80] {
            let mut button=DisplayDevice::new(160,120);
            let clip=Region { left: 16,top: 15,right: 114,bottom: 90 };
            button.clip=Some(clip);
            let bytes=[0x41,0x32,0x3f];
            button.access_button(10,10,120,height,&bytes,primary,focused);
            let mut surface=DisplayDevice::new(160,120);surface.clip=Some(clip);
            surface.access_surface(10,10,120,height,if primary {Surface::Primary}else{Surface::Secondary},
                if primary {access_style::PRIMARY} else {access_style::BACKGROUND},
                access_style::BORDER,255,focused);
            assert_eq!(button.pixels,surface.pixels);
            assert_eq!(button.damage,surface.damage);
            let (r,g,b)=access_style::TEXT;
            let padding = (height / 3).min(30);
            assert_eq!(button.labels,vec![Label { rectangle: [10+padding,10,120-padding*2,height],
                bytes: bytes.to_vec(),color: [r,g,b],pixels: (height*5/12).max(8), centered: true, strong: true }]);
        }
    } }
}

// ------------------------=
// FUNC: filter_rejects_transparent_rgb_fringes_and_respects_crop
// DESC: Proves transparent RGB contributes no color, partial alpha survives, and reduced samples cannot leak outside the crop.
// ------------------=
#[test]
fn filter_rejects_transparent_rgb_fringes_and_respects_crop() {
    for hidden in [[0,0,0,0],[255,0,255,0],[0,255,0,0]] {
        let bytes=rgba_bitmap(2,2,&[[30,170,255,255],hidden,hidden,hidden]);
        let image=bitmap::RuntimeBitmap::parse(&bytes).unwrap();
        let placement=image.placement(1,1,[0;4],false).unwrap();
        assert_eq!(image.filtered_rgba(placement,0,0),[30,170,255,63]);
        let mut display=DisplayDevice::new(1,1);
        let [r,g,b,a]=image.filtered_rgba(placement,0,0);
        display.blend_color_unchecked(0,0,r,g,b,a);
        assert_eq!(display.pixels[0],[14,53,91,63]);
    }
    for alpha in [0,1,128,255] {
        let bytes=rgba_bitmap(8,8,&vec![[200,170,40,alpha];64]);
        let image=bitmap::RuntimeBitmap::parse(&bytes).unwrap();
        let p=image.placement(2,2,[0;4],false).unwrap();
        for y in 0..2 { for x in 0..2 {
            assert_eq!(image.filtered_rgba(p,x,y),if alpha==0 {[0;4]} else {[200,170,40,alpha]});
        } }
    }
    let mut pixels=vec![[0,255,0,255];64];
    for y in 2..6 { for x in 2..6 { pixels[y*8+x]=[240,120,255,255]; } }
    let bytes=rgba_bitmap(8,8,&pixels);
    let image=bitmap::RuntimeBitmap::parse(&bytes).unwrap();
    let p=image.placement(4,2,[25;4],false).unwrap();
    assert_eq!((p.destination_left,p.destination_width,p.destination_height),(1,2,2));
    for y in 0..2 { for x in 0..2 { assert_eq!(image.filtered_rgba(p,x,y),[240,120,255,255]); } }
    let pixels:Vec<_>=(0..64).map(|index|[130,210,255,access_style::rounded_coverage(index%8,index/8,8,8,4)]).collect();
    let bytes=rgba_bitmap(8,8,&pixels);
    let image=bitmap::RuntimeBitmap::parse(&bytes).unwrap();
    let p=image.placement(4,4,[0;4],false).unwrap();
    for y in 0..4 { for x in 0..4 {
        assert_eq!(image.filtered_rgba(p,x,y),image.filtered_rgba(p,3-x,y));
        assert_eq!(image.filtered_rgba(p,x,y),image.filtered_rgba(p,x,3-y));
    } }
}

// ------------------------=
// FUNC: original_wordmark_is_bound_and_visible_after_reduction
// DESC: Verifies the original BMP binding and real cropped downsampling contain transparent, partially covered, and bright visible pixels.
// ------------------=
#[test]
fn original_wordmark_is_bound_and_visible_after_reduction() {
    let mut display=DisplayDevice::new(1024,768);
    display.access_wordmark(500,90,480,110);
    assert_eq!(display.wordmark,Some(Wordmark{original:true,rectangle:[500,90,480,110],crop:[14,1,12,80],opacity:255,fill:false}));
    let image=bitmap::RuntimeBitmap::parse(bootstrap::SPLASH_BMP).unwrap();
    assert_eq!((image.width(),image.height()),(1672,941));
    let p=image.placement(480,110,[14,1,12,80],false).unwrap();
    let (mut clear,mut partial,mut bright)=(0,0,0);
    for y in 0..p.destination_height { for x in 0..p.destination_width {
        let [r,g,b,a]=image.filtered_rgba(p,x,y);
        clear+=usize::from(a==0);partial+=usize::from(a>0&&a<255);
        bright+=usize::from(r as u16*a as u16/255>160 && g as u16*a as u16/255>160 && b as u16*a as u16/255>160);
    } }
    assert!(clear>100 && partial>100 && bright>100,"{:?}",(clear,partial,bright));
}

// ------------------------=
// FUNC: production_glyphs_respect_transparency_and_clip
// DESC: Exercises the actual generated atlas sampler at native and scaled sizes and rejects unsupported icon kinds.
// ------------------=
#[test]
fn production_glyphs_respect_transparency_and_clip() {
    for kind in 1..=16 { for size in [16,32,64,96] {
        let mut full=DisplayDevice::new(128,128);
        full.access_icon(64,64,kind,size,true);
        assert!(full.pixels.iter().any(|pixel|pixel[3]>0));
        assert!(full.pixels.iter().any(|pixel|pixel[3]>0&&pixel[3]<255));
        let clip=Region{left:59,top:61,right:91,bottom:87};
        let mut clipped=DisplayDevice::new(128,128);clipped.clip=Some(clip);
        clipped.access_icon(64,64,kind,size,true);
        for y in 0..128 { for x in 0..128 {
            assert_eq!(clipped.pixels[y*128+x],if x>=clip.left&&x<clip.right&&y>=clip.top&&y<clip.bottom {
                full.pixels[y*128+x]
            } else {[9,15,38,0]});
        } }
    } }
    let mut display=DisplayDevice::new(20,20);
    for kind in [0,17,usize::MAX] {display.access_icon(10,10,kind,16,false);}
    display.access_icon(10,10,1,0,false);
    assert!(display.damage.is_empty());
    assert!(display.pixels.iter().all(|pixel|*pixel==[9,15,38,0]));
}

// ------------------------=
// FUNC: focus_animation_is_bounded_settles_and_respects_reduced_motion
// DESC: Checks numeric easing, focus replacement and pressed-state transitions without triggering any authentication actions.
// ------------------=
#[test]
fn focus_animation_is_bounded_settles_and_respects_reduced_motion() {
    let mut motion=access_style::FocusMotion::new();
    assert!(motion.advance(1,1000,false,false));assert_eq!(motion.amount,0);
    let mut previous=0;
    for time in 1000..=1140 {
        motion.advance(1,time,false,false);
        assert!(motion.amount>=previous);previous=motion.amount;
    }
    assert_eq!(motion.amount,255);
    assert!(!motion.advance(1,1141,false,false));
    assert!(motion.advance(1,1141,true,false));assert!(motion.pressed);
    assert!(motion.advance(2,1142,false,false));assert_eq!(motion.amount,0);
    assert!(motion.advance(3,1142,false,true));assert_eq!(motion.amount,255);
}
