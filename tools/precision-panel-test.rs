//! Behavioral pixels from the production panel sampler, composition adapter and glyph filter.
#![allow(dead_code)]
use std::io::Write;
#[path = "rounded-corner-test.rs"] mod rounded_corners;
#[path = "../kernel/ui/geometry.rs"] pub mod geometry;
#[path = "../kernel/ui/panel_recipe.rs"] pub mod panel_recipe;
#[path = "../kernel/ui/surface_cache.rs"] pub mod surface_cache;
#[path = "../kernel/ui/glyph_filter.rs"] mod glyph_filter;
#[path = "../kernel/ui/skin.rs"] pub mod skin;
#[path = "../kernel/runtime/font_faces.rs"] mod font_faces;
const INFINITY_UI_ABI_VERSION: u16 = 1;
mod ui { pub use crate::{geometry, panel_recipe, surface_cache, skin}; }
#[path = "../kernel/core/bootstrap/panel_view.rs"] mod panel_view;
use geometry::Rect;
use panel_recipe::{Material, Rgb, Surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Region { left: usize, top: usize, right: usize, bottom: usize }
struct DisplayDevice {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
    clip: Option<Region>,
    submissions: usize,
    damage: Option<Region>,
    primary: u32,
    accent: u32,
}
impl DisplayDevice {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded host framebuffer seam without replacing the production panel adapter.
    // ------------------=
    fn new(width: usize, height: usize) -> Self {
        Self { width, height, pixels: vec![0x090f26; width * height], clip: None,
            submissions: 0, damage: None, primary: 0x101b3a, accent: 0x86c7ff }
    }
    // ------------------------=
    // FUNC: clipped_render_region
    // DESC: Restricts production painter writes to the framebuffer and its current damage clip.
    // ------------------=
    fn clipped_render_region(&self, left: usize, top: usize, width: usize, height: usize) -> Option<Region> {
        let clip = self.clip.unwrap_or(Region { left: 0, top: 0, right: self.width, bottom: self.height });
        let r = Region { left: left.max(clip.left), top: top.max(clip.top),
            right: left.saturating_add(width).min(self.width).min(clip.right),
            bottom: top.saturating_add(height).min(self.height).min(clip.bottom) };
        (r.left < r.right && r.top < r.bottom).then_some(r)
    }
    // ------------------------=
    // FUNC: blend_color_unchecked
    // DESC: Composes actual sampled RGBA into an opaque framebuffer and detects any out-of-bounds adapter write.
    // ------------------=
    fn blend_color_unchecked(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8, a: u8) {
        assert!(x < self.width && y < self.height);
        let at = y * self.width + x;
        self.pixels[at] = composite(self.pixels[at], (r, g, b, a));
    }
    // ------------------------=
    // FUNC: mark_dirty_rect
    // DESC: Records the production adapter's actual damage rectangle and publication count.
    // ------------------=
    fn mark_dirty_rect(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.submissions += 1;
        self.damage = Some(Region { left: x, top: y, right: x + width, bottom: y + height });
    }
    // ------------------------=
    // FUNC: active_accent_surface
    // DESC: Resolves frame colors through the real semantic skin implementation for theme-change coverage.
    // ------------------=
    fn active_accent_surface(&self, surface: skin::AccentSurface) -> Rgb {
        let mut registry = skin::SkinRegistry::new();
        registry.set_primary(self.primary, skin::AppearanceScope::Session).unwrap();
        registry.set_accent(self.accent, skin::AppearanceScope::Session).unwrap();
        registry.accent_surface(surface).channels()
    }
}

// ------------------------=
// FUNC: composite
// DESC: Supplies an independent integer source-over oracle for sampled alpha against an opaque destination.
// ------------------=
fn composite(background: u32, rgba: (u8, u8, u8, u8)) -> u32 {
    let (r, g, b, a) = rgba;
    let alpha = u32::from(a);
    let mut result = 0;
    for (shift, value) in [(16, r), (8, g), (0, b)] {
        let old = (background >> shift) & 255;
        result |= ((old * (255 - alpha) + u32::from(value) * alpha) / 255) << shift;
    }
    result
}

// ------------------------=
// FUNC: material
// DESC: Uses approved Eclipse tokens while keeping surface and outline opacity independently configurable.
// ------------------=
fn material(scale: usize) -> Material {
    Material { fill: (16, 27, 58), edge: (134, 199, 255), opacity: 224,
        edge_opacity: 255, focus: 0, pressed: false, scale }
}

// ------------------------=
// FUNC: sampler_contract
// DESC: Verifies every surface at four scales, transparent bounds, independent light/fill alpha, theme recoloring and interaction states.
// ------------------=
fn sampler_contract() -> usize {
    let mut covered = 0;
    for scale in 1..=4 {
        for kind in [Surface::Frame, Surface::Panel, Surface::Primary, Surface::Secondary, Surface::Field, Surface::Current, Surface::Inactive] {
            let circle = matches!(kind, Surface::Current | Surface::Inactive);
            let (width, height) = if circle { (28 * scale, 28 * scale) } else { (112 * scale, 64 * scale) };
            let m = material(scale);
            let hidden = Material { opacity: 0, ..m };
            let translucent = Material { opacity: 112, ..m };
            let other = Material { fill: (79, 33, 67), edge: (255, 188, 101), ..m };
            let mut visible = 0;
            let mut partial = 0;
            let mut recolored = 0;
            let mut interior = 0;
            for y in 0..height { for x in 0..width {
                // These invariants describe the original atlas, not a user's
                // independently tinted glass/square/rounded surface overrides.
                let p = panel_recipe::pixel_authored(kind, width, height, x, y, m);
                let h = panel_recipe::pixel_authored(kind, width, height, x, y, hidden);
                let t = panel_recipe::pixel_authored(kind, width, height, x, y, translucent);
                let changed = panel_recipe::pixel_authored(kind, width, height, x, y, other);
                assert_eq!(p.3, changed.3);
                visible += usize::from(p.3 > 0);
                partial += usize::from(p.3 > 0 && p.3 < m.opacity);
                recolored += usize::from(p.3 > 0 && p != changed);
                let edge = x.min(y).min(width - 1 - x).min(height - 1 - y);
                let border = kind == Surface::Frame || (!circle && edge < 2 * scale);
                if border { assert_eq!(p, h); assert_eq!(p, t); }
                else if circle || edge>=panel_recipe::rim_width(kind,scale) {
                    assert_eq!(h.3, 0);
                    assert!(i16::from(p.3) - 2 * i16::from(t.3) <= 1);
                    assert!(2 * u16::from(t.3) <= u16::from(p.3));
                    interior += usize::from(p.3 > 0);
                }
                else { assert!(p.3>=h.3 && p.3>=t.3); }
                if kind == Surface::Frame && edge >= panel_recipe::rim_width(kind,scale) { assert_eq!(p.3, 0); }
                covered += 1;
            }}
            assert!(visible > 0 && partial > 0 && recolored > 0);
            if kind != Surface::Frame { assert!(interior > 0); }
            for (w, h, x, y) in [(0, height, 0, 0), (width, 0, 0, 0),
                (width, height, width, 0), (width, height, 0, height)] {
                assert_eq!(panel_recipe::pixel(kind, w, h, x, y, m).3, 0);
            }
            let center = panel_recipe::pixel(kind, width, height, width / 2, height / 2, m);
            if matches!(kind,Surface::Primary|Surface::Secondary) {
                let pressed = panel_recipe::pixel(kind, width, height, width / 2, height / 2,
                    Material { pressed: true, ..m });
                assert_eq!(center.3, pressed.3);
                assert_ne!(center, pressed);
            }
        }
    }
    covered
}

// ------------------------=
// FUNC: reference_state_contract
// DESC: Checks shared Sapphire tokens, bounded interpolated focus states and pressed-state contrast; visual-reference acceptance is recorded separately.
// ------------------=
fn reference_state_contract() {
    let mut skin=skin::SkinRegistry::new();
    let colors=skin::kit_tokens::colors();
    for persisted_default in [false,true] {
        if persisted_default {skin.set_accent(colors[6]&0xffffff,skin::AppearanceScope::User).unwrap();}
        assert_eq!(skin.accent_surface(skin::AccentSurface::WindowOutline).rgb24(),colors[3]&0xffffff);
        assert_eq!(skin.accent_surface(skin::AccentSurface::Focus).rgb24(),colors[8]&0xffffff);
    }
    skin.set_accent(0xeb9845,skin::AppearanceScope::User).unwrap();
    assert_ne!(skin.accent_surface(skin::AccentSurface::WindowOutline).rgb24(),colors[3]&0xffffff);
    for scale in 1..=4 {
        let (w,h)=(169*scale,38*scale);
        let m=Material{fill:(36,90,212),edge:(54,87,116),opacity:255,edge_opacity:255,focus:0,pressed:false,scale};
        for kind in [Surface::Primary,Surface::Secondary,Surface::Field,Surface::Panel] {
            let mut states=std::collections::BTreeSet::new();
            for focus in [0,64,128,192,255] {
                let mut pixels=Vec::new();
                for y in 0..h {for x in 0..w {
                    let p=panel_recipe::pixel_authored(kind,w,h,x,y,Material{focus,..m});
                    pixels.push(p);
                    assert_eq!(panel_recipe::pixel_authored(kind,w,h,w,y,Material{focus,..m}).3,0);
                }}
                states.insert(pixels);
            }
            assert_eq!(states.len(),5,"focus must interpolate distinct native states without changing bounds");
        }
        let normal=panel_recipe::pixel_authored(Surface::Primary,w,h,w/2,h/2,m);
        let pressed=panel_recipe::pixel_authored(Surface::Primary,w,h,w/2,h/2,Material{pressed:true,..m});
        assert!(normal.2>normal.1 && normal.1>normal.0,"default accent must remain sapphire");
        assert!(u16::from(pressed.0)+u16::from(pressed.1)+u16::from(pressed.2)<
            u16::from(normal.0)+u16::from(normal.1)+u16::from(normal.2));
        for (x,y) in [(0,0),(w-1,0),(0,h-1),(w-1,h-1)] {
            assert_eq!(panel_recipe::pixel_authored(Surface::WindowFill,w,h,x,y,m).3,255);
        }
    }
}

// ------------------------=
// FUNC: adapter_contract
// DESC: Compares every production DisplayDevice panel pixel to signed source placement and clipped source-over sampling, including wholly offscreen frames.
// ------------------=
fn adapter_contract() -> usize {
    let mut covered = 0;
    for scale in 1..=4 {
        let (width, height) = (128 * scale, 96 * scale);
        for (rx, ry) in [(-23, -17), (-90, 15), (77, 63), (160, 120), (-200, -200), (12, 8)] {
            let rect = Rect { x: rx * scale as i32, y: ry * scale as i32,
                width: (112 * scale) as u32, height: (64 * scale) as u32 };
            for kind in [Surface::Frame, Surface::WindowFill, Surface::Panel, Surface::Primary, Surface::Secondary, Surface::Field] {
                for clipped in [false, true] {
                    let mut display = DisplayDevice::new(width, height);
                    for (i, pixel) in display.pixels.iter_mut().enumerate() {
                        *pixel = if (i % width / 11 + i / width / 9) % 2 == 0 { 0x142245 } else { 0x071127 };
                    }
                    if clipped { display.clip = Some(Region { left: 19 * scale, top: 13 * scale,
                        right: 93 * scale, bottom: 81 * scale }); }
                    let original = display.pixels.clone();
                    let m = material(scale);
                    display.precision_surface(rect, kind, m);
                    let clip = display.clip.unwrap_or(Region { left: 0, top: 0, right: width, bottom: height });
                    for y in 0..height { for x in 0..width {
                        let point = geometry::Point { x: x as i32, y: y as i32 };
                        let visible = rect.contains(point) && x >= clip.left && x < clip.right
                            && y >= clip.top && y < clip.bottom;
                        let rgba = if visible { panel_recipe::pixel(kind, rect.width as usize, rect.height as usize,
                            (x as i32 - rect.x) as usize, (y as i32 - rect.y) as usize, m) } else { (0, 0, 0, 0) };
                        assert_eq!(display.pixels[y * width + x], composite(original[y * width + x], rgba));
                        covered += 1;
                    }}
                    let bounds = rect.intersection(Rect { x: clip.left as i32, y: clip.top as i32,
                        width: (clip.right - clip.left) as u32, height: (clip.bottom - clip.top) as u32 });
                    let visible = bounds.width > 0 && bounds.height > 0;
                    assert_eq!(display.submissions, usize::from(visible));
                    if visible { assert_eq!(display.damage, Some(Region { left: bounds.x as usize, top: bounds.y as usize,
                        right: bounds.right() as usize, bottom: bounds.bottom() as usize })); }
                }
            }
        }
        let rect = Rect { x: 5, y: 5, width: (100 * scale) as u32, height: (70 * scale) as u32 };
        let mut active = DisplayDevice::new(width, height);
        let mut inactive = DisplayDevice::new(width, height);
        active.precision_window_frame(rect, true, scale);
        inactive.precision_window_frame(rect, false, scale);
        assert_ne!(active.pixels, inactive.pixels);
        let mut themed = DisplayDevice::new(width, height);
        themed.primary = 0x4f2143;
        themed.accent = 0xffbc65;
        themed.precision_window_frame(rect, true, scale);
        assert_ne!(active.pixels, themed.pixels);
        // Narrow frames have overlapping nominal rims. Source-over must still
        // visit each destination pixel exactly once, including translucent art.
        for narrow in [1, 8, 24] {
            let rect = Rect { x: -2, y: 3, width: narrow * scale as u32, height: (80 * scale) as u32 };
            for kind in [Surface::Frame, Surface::Panel, Surface::Field] {
                let mut d = DisplayDevice::new(width, height);
                let original = d.pixels.clone();
                let m = Material { opacity: 0, edge_opacity: 123, ..material(scale) };
                d.precision_surface(rect, kind, m);
                for y in 0..height { for x in 0..width {
                    let point=geometry::Point{x:x as i32,y:y as i32};
                    let rgba=if rect.contains(point) {panel_recipe::pixel(kind,rect.width as usize,rect.height as usize,
                        (x as i32-rect.x) as usize,(y as i32-rect.y) as usize,m)}else{(0,0,0,0)};
                    assert_eq!(d.pixels[y*width+x],composite(original[y*width+x],rgba));
                    covered+=1;
                }}
                let mut hidden = DisplayDevice::new(width,height);
                hidden.precision_surface(rect,kind,Material{edge_opacity:0,..m});
                assert_eq!(hidden.submissions,0);
            }
        }
    }
    covered
}

// ------------------------=
// FUNC: glyph_contract
// DESC: Checks exact-size coverage, known area and bilinear averages, isolated atlas cells, empty bounds and real Inter antialiasing at four scales.
// ------------------=
fn glyph_contract() -> usize {
    let mut checked = 0;
    let atlas = [255, 0, 0, 255, 255, 0, 255, 255];
    assert_eq!(glyph_filter::coverage(&atlas, 4, 1, 2, 2, 1, 1, 0, 0), 63);
    assert_eq!(glyph_filter::coverage(&atlas, 4, 1, 2, 2, 4, 4, 1, 1), 16);
    assert_eq!(glyph_filter::coverage(&atlas, 4, 1, 2, 2, 2, 2, 1, 1), 255);
    for (width, height) in [(1, 1), (2, 2), (4, 4), (3, 5), (8, 8), (1, 8)] {
        let mut isolated = [255u8; 24];
        for y in 0..4 { isolated[y * 6 + 1..y * 6 + 5].fill(0); }
        for y in 0..height { for x in 0..width {
            assert_eq!(glyph_filter::coverage(&isolated, 6, 1, 4, 4, width, height, x, y), 0);
            checked += 1;
        }}
        for (x, y) in [(width, 0), (0, height)] {
            assert_eq!(glyph_filter::coverage(&isolated, 6, 1, 4, 4, width, height, x, y), 0);
        }
    }
    assert_eq!(glyph_filter::coverage(&atlas, 4, 1, 2, 2, 0, 1, 0, 0), 0);
    assert_eq!(glyph_filter::coverage(&[], 4, 1, 2, 2, 1, 1, 0, 0), 0);
    let (face, _, _) = font_faces::face(1, false);
    for scale in 1..=4 {
        for size in [14 * scale, 19 * scale, 24 * scale] {
            let (width, height) = (size, 28 * size / 24);
            let offset = (b'A' - 32) as usize * 24;
            let mut opaque = 0;
            let mut partial = 0;
            for y in 0..height { for x in 0..width {
                let value = glyph_filter::coverage(face, 24 * 95, offset, 24, 28, width, height, x, y);
                opaque += usize::from(value > 0);
                partial += usize::from(value > 0 && value < 255);
                checked += 1;
            }}
            assert!(opaque > 0 && partial > 0);
        }
    }
    checked
}

// ------------------------=
// FUNC: label
// DESC: Draws immutable native Inter atlas glyphs through the production coverage filter without introducing a host font renderer.
// ------------------=
fn label(display: &mut DisplayDevice, mut x: usize, y: usize, text: &[u8], size: usize, bold: bool, rgb: Rgb) {
    let (atlas, metrics, kerning) = font_faces::face(1, bold);
    let height = 28 * size / 24;
    let mut previous: Option<usize> = None;
    for &byte in text {
        if !(32..=126).contains(&byte) { continue; }
        let glyph = (byte - 32) as usize;
        if let Some(left) = previous {
            x = x.saturating_add_signed((kerning[left * 95 + glyph] as isize - 128) * size as isize / 24);
        }
        for row in 0..height.min(display.height.saturating_sub(y)) {
            for column in 0..size.min(display.width.saturating_sub(x)) {
                let alpha = glyph_filter::coverage(atlas, 24 * 95, glyph * 24, 24, 28, size, height, column, row);
                display.blend_color_unchecked(x + column, y + row, rgb.0, rgb.1, rgb.2, alpha);
            }
        }
        x += (usize::from(metrics[glyph]) * size + 12) / 24;
        previous = Some(glyph);
    }
}

// ------------------------=
// FUNC: render_overview
// DESC: Emits production panel and frame pixels with native Inter labels for human hierarchy, contrast, edge-softness and state review.
// ------------------=
fn render_overview(output: &std::path::Path) {
    let mut d = DisplayDevice::new(1440, 1180);
    let white = (240, 247, 255);
    let muted = (179, 195, 221);
    label(&mut d, 36, 24, b"InfinityOS  |  Eclipse Precision", 30, true, white);
    label(&mut d, 36, 68, b"Production pixels - native Inter - shared panel adapter", 16, false, muted);
    for (index, active) in [true, false].into_iter().enumerate() {
        let x = 36 + index * 704;
        let r = Rect { x: x as i32, y: 134, width: 664, height: 230 };
        d.precision_surface(r, Surface::Panel, Material{edge_opacity:0,..material(1)});
        d.precision_window_frame(r, active, 1);
        label(&mut d, x + 24, 153, if active { b"Active window" } else { b"Inactive window" }, 20, true, white);
        label(&mut d, x + 24, 194, b"Image-backed frame, stable geometry", 16, false, muted);
        label(&mut d, x + 24, 231, b"Independent outline / background opacity", 16, false, muted);
    }
    let states = [(0u8, false, b"Normal" as &[u8]), (255, false, b"Focused"), (255, true, b"Pressed")];
    for (row, kind) in [Surface::Panel, Surface::Primary, Surface::Secondary, Surface::Field].into_iter().enumerate() {
        let y = 420 + row * 120;
        label(&mut d, 36, y - 28, [b"GLASS PANELS" as &[u8], b"PRIMARY BUTTONS", b"SECONDARY BUTTONS", b"TEXT FIELDS"][row], 14, true, muted);
        for (column, &(focus, pressed, name)) in states.iter().enumerate() {
            let x = 36 + column * 468;
            let fill = if kind == Surface::Primary { (62, 111, 255) } else { (16, 27, 58) };
            d.precision_surface(Rect { x: x as i32, y: y as i32, width: 432, height: 74 }, kind,
                Material { fill, focus, pressed, ..material(1) });
            label(&mut d, x + 20, y + 24, name, 18, true, white);
        }
    }
    label(&mut d, 36, 900, b"SCALE 1 / 2 / 3 / 4", 14, true, muted);
    for scale in 1..=4 {
        let x = 36 + (scale - 1) * 174;
        d.precision_surface(Rect { x: x as i32, y: 938, width: (28 * scale) as u32, height: (28 * scale) as u32 },
            Surface::Current, Material { fill: (62, 111, 255), ..material(scale) });
        d.precision_surface(Rect { x: x as i32, y: 1070, width: (20 * scale) as u32, height: (20 * scale) as u32 },
            Surface::Inactive, material(scale));
    }
    label(&mut d, 760, 900, b"THEME / LOW SURFACE OPACITY", 14, true, muted);
    let alternate = Material { fill: (79, 33, 67), edge: (255, 188, 101), opacity: 112,
        focus: 180, ..material(1) };
    let r = Rect { x: 760, y: 938, width: 644, height: 212 };
    d.precision_surface(r, Surface::Panel, Material{edge_opacity:0,..alternate});
    d.precision_surface(r, Surface::Frame, alternate);
    label(&mut d, 784, 962, b"Theme-aware; crisp independent borders", 18, true, white);
    label(&mut d, 784, 1002, b"Inter at 14px", 14, false, muted);
    label(&mut d, 784, 1030, b"Inter at 19px", 19, false, muted);
    label(&mut d, 784, 1066, b"Inter at 24px", 24, false, white);
    let mut file = std::fs::File::create(output).unwrap();
    write!(file, "P6\n{} {}\n255\n", d.width, d.height).unwrap();
    for p in d.pixels { file.write_all(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]).unwrap(); }
}

// ------------------------=
// FUNC: main
// DESC: Runs typed pixel/coverage assertions before emitting a review board and numeric evidence inside the repository build authority.
// ------------------=
fn main() {
    if std::env::args().any(|arg|arg=="--sapphire-proof") {sapphire_proof();return;}
    assert_eq!(std::env::var("INFINITY_BUILD_KIT_ACTIVE").as_deref(), Ok("1"));
    for corners in [0, 1, 2] {
        for pressed in [false, true] {
            let sample=panel_recipe::pixel_styled(Surface::Primary,280,44,140,22,
                Material{fill:(17,30,70),edge:(134,199,255),opacity:255,edge_opacity:255,
                    focus:255,pressed,scale:1},
                panel_recipe::SurfaceStyle{material:1,corners,radius:10,..panel_recipe::SurfaceStyle::AUTHORED});
            assert!(sample.2>sample.1 && sample.1>sample.0 && sample.0>0);
            assert!(sample.3>0 && sample.3<255);
        }
    }
    let surface_pixels = sampler_contract();
    control_pixels();
    reference_state_contract();
    let adapter_pixels = adapter_contract();
    let glyph_samples = glyph_contract();
    let directory = std::path::Path::new("builds/validation/eclipse-precision");
    std::fs::create_dir_all(directory).unwrap();
    rounded_corners::run(directory.join("rounded-corners.ppm").to_str().unwrap(), true);
    render_overview(&directory.join("production-overview.ppm"));
    let evidence = format!("{{\"surface_pixels\":{surface_pixels},\"adapter_pixels\":{adapter_pixels},\"glyph_samples\":{glyph_samples},\"scales\":[1,2,3,4],\"signed_clipping\":true,\"independent_outline_alpha\":true,\"installed_verified\":false}}\n");
    std::fs::write(directory.join("pixel-evidence.json"), &evidence).unwrap();
    println!("{evidence}");
}

// ------------------------=
// FUNC: sapphire_proof
// DESC: Captures actual shared native controls at design-sheet sizes and verifies state, clipping and filled progress behavior.
// ------------------=
fn sapphire_proof() {
    let mut d=DisplayDevice::new(620,450);
    d.pixels.fill(0x091522);
    for (row,kind) in [Surface::Primary,Surface::Secondary,Surface::Field,Surface::Panel].into_iter().enumerate() {
        for state in 0..3 {
            let rect=Rect{x:20+state*200,y:20+row as i32*64,width:169,height:38};
            let m=Material{fill:if kind==Surface::Primary{(36,90,212)}else{(14,28,54)},edge:(54,87,116),
                opacity:255,edge_opacity:255,focus:if state==1{255}else{0},pressed:state==2,scale:1};
            d.precision_surface(rect,kind,m);
        }
    }
    let m=Material{fill:(14,28,54),edge:(54,87,116),opacity:255,edge_opacity:255,focus:0,pressed:false,scale:1};
    for (index,checked,enabled) in [(0,false,true),(1,true,true),(2,true,false)] {
        d.precision_toggle(Rect{x:20+index*200,y:290,width:60,height:30},checked,enabled,m);
        d.precision_slider(Rect{x:20+index*200,y:365,width:169,height:6},
            Rect{x:20+index*200+108,y:358,width:20,height:20},enabled,m);
    }
    assert_ne!(d.pixels[305*620+30],d.pixels[305*620+230]);
    assert_ne!(d.pixels[368*620+60],d.pixels[368*620+175]);
    let output=std::path::Path::new("builds/validation/sapphire-native-controls.ppm");
    let mut file=std::fs::File::create(output).unwrap();
    write!(file,"P6\n{} {}\n255\n",d.width,d.height).unwrap();
    for p in d.pixels {file.write_all(&[(p>>16)as u8,(p>>8)as u8,p as u8]).unwrap();}
}

// ------------------------=
// FUNC: control_pixels
// DESC: Verifies checked/selected/disabled pixels and exact signed clipping for shared controls across four display scales.
// ------------------=
fn control_pixels() {
    for scale in 1..=4 {
        let rect=Rect{x:8,y:8,width:(20*scale)as u32,height:(20*scale)as u32};
        for radio in [false,true] {
            let render=|selected,enabled,clip| {
                let mut display=DisplayDevice::new(120,120);display.clip=clip;
                if radio {display.precision_radio(rect,selected,enabled,material(scale));}
                else {display.precision_toggle(rect,selected,enabled,material(scale));}
                display.pixels
            };
            let off=render(false,true,None);let on=render(true,true,None);
            let disabled=render(true,false,None);
            assert_ne!(off,on);assert_ne!(on,disabled);
            let clip=Region{left:13,top:17,right:30,bottom:32};
            let clipped=render(true,true,Some(clip));
            for y in 0..120 {for x in 0..120 {
                let inside=x>=clip.left&&x<clip.right&&y>=clip.top&&y<clip.bottom;
                assert_eq!(clipped[y*120+x],if inside {on[y*120+x]}else{0x090f26});
            }}
        }
    }
}
