#![allow(dead_code)]
#[path = "../kernel/runtime/font_faces.rs"] pub mod font_faces;
#[path = "../kernel/runtime/font_preferences.rs"] pub mod font_preferences;
#[path = "../kernel/runtime/font.rs"] pub mod font;
mod runtime { pub use super::{font_faces,font_preferences,font}; }
#[path = "../kernel/core/bootstrap/font_library_view.rs"] mod font_library_view;
#[cfg(not(baseline))]
#[path = "../kernel/core/bootstrap/primitives.rs"]
mod primitives;
#[path = "../kernel/core/bootstrap/personalization_view.rs"]
mod personalization_view;
#[path="../kernel/core/bootstrap/app_style.rs"] mod app_style;
#[path="../kernel/core/bootstrap/panel_view.rs"] mod panel_view;
#[path="../kernel/core/bootstrap/ai_widget_view.rs"] mod ai_widget_view;
#[path="ai-widget-tests.rs"] mod ai_widget_tests;
#[path="../kernel/core/bootstrap/assistant_view.rs"] mod assistant_view;
#[path="../kernel/core/bootstrap/editor_view.rs"] mod editor_view;
#[cfg(baseline)]
#[path = "../build/primitives-before.rs"]
mod primitives;
#[path = "../kernel/ui/mod.rs"]
mod ui;
use std::time::Instant;
#[cfg(test)]
#[path = "minimized-shelf-render-tests.rs"]
mod minimized_shelf_fixture;
#[path = "../kernel/core/bootstrap/soft_stroke.rs"]
mod stroke_coverage;

#[test]
// ------------------------=
// FUNC: luminous_strokes_batch_damage_without_changing_pixels
// DESC: Compares the native batched stroke against per-pixel composition, including clipping and retained alpha.
// ------------------=
fn luminous_strokes_batch_damage_without_changing_pixels() {
    for recording_surface in [false, true] {
        for clip in [
            None,
            Some(Region { left: 50, top: 30, right: 200, bottom: 110 }),
        ] {
            let mut actual = vec![0x7f183050u32; 320 * 160];
            let mut expected = actual.clone();
            let mut display = DisplayDevice {
                buffer: actual.as_mut_ptr(),
                width: 320,
                height: 160,
                stride: 320,
                format: 0,
                render_clip: clip,
                fast_motion_frame: false,
                submissions: 0,
                recording_surface,
            };
            let mut reference = display;
            reference.buffer = expected.as_mut_ptr();
            let from = (-8 * 256, 20 * 256 + 128);
            let to = (330 * 256, 140 * 256 + 64);
            stroke_coverage::segment(from, to, 2, |x, y, alpha| {
                let core = u16::from(alpha.saturating_sub(120));
                reference.blend_color(
                    x, y, (38 + core) as u8, (153 + core * 3 / 4) as u8, 255,
                    (u16::from(alpha) * 110 / 255) as u8,
                );
            });
            display.soft_stroke(from, to, 2, 110);
            assert_eq!(actual, expected);
            assert_eq!(display.submissions, 1);
            assert!(reference.submissions > 1000);
        }
    }
}

#[derive(Clone, Copy)]
struct Region {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}
#[derive(Clone, Copy)]
struct DisplayDevice {
    buffer: *mut u32,
    width: usize,
    height: usize,
    stride: usize,
    format: u32,
    render_clip: Option<Region>,
    fast_motion_frame: bool,
    submissions: u64,
    recording_surface: bool,
}
type PresentRegion = Region;
#[path = "../kernel/core/bootstrap/launcher_backdrop.rs"]
mod launcher_backdrop;
#[path = "../kernel/core/bootstrap/retained_windows.rs"]
mod retained_windows;
#[path = "../kernel/core/bootstrap/window_chrome.rs"]
mod window_chrome;
impl DisplayDevice {
    // ------------------------=
    // FUNC: active_primary_rgb
    // DESC: Supplies the deterministic default primary tint for production chrome pixel tests.
    // ------------------=
    fn active_primary_rgb(&self)->u32 { 0x0d2238 }
    // ------------------------=
    // FUNC: active_accent_rgb
    // DESC: Supplies the deterministic default accent tint for production chrome pixel tests.
    // ------------------=
    fn active_accent_rgb(&self)->u32 { 0x38bdf8 }
    // ------------------------=
    // FUNC: active_accent_surface
    // DESC: Resolves fixture tints through the production semantic skin implementation used by application painters.
    // ------------------=
    #[cfg(not(test))]
    fn active_accent_surface(&self,surface:ui::skin::AccentSurface)->(u8,u8,u8) {
        let mut skins=ui::skin::SkinRegistry::new();
        skins.set_primary(self.active_primary_rgb(),ui::skin::AppearanceScope::Session).unwrap();
        skins.set_accent(self.active_accent_rgb(),ui::skin::AppearanceScope::Session).unwrap();
        skins.accent_surface(surface).channels()
    }
    // ------------------------=
    // FUNC: set_render_clip
    // DESC: Supplies the production editor's caret clipping boundary in the framebuffer fixture.
    // ------------------=
    #[cfg(not(test))]
    fn set_render_clip(&mut self,x:usize,y:usize,w:usize,h:usize){self.render_clip=Some(Region{left:x,top:y,right:x+w,bottom:y+h});}
    // ------------------------=
    // FUNC: clear_render_clip
    // DESC: Restores the full framebuffer boundary after a caret-only repaint.
    // ------------------=
    fn clear_render_clip(&mut self){self.render_clip=None;}
    // ------------------------=
    // FUNC: intersect_render_clip
    // DESC: Preserves parent clipping while exercising the production settings label renderer.
    // ------------------=
    fn intersect_render_clip(&mut self,x:usize,y:usize,width:usize,height:usize) {
        let old=self.render_clip.unwrap_or(Region{left:0,top:0,right:self.width,bottom:self.height});
        self.render_clip=Some(Region{left:old.left.max(x),top:old.top.max(y),right:old.right.min(x+width),bottom:old.bottom.min(y+height)});
    }
    // ------------------------=
    // FUNC: active_icon_theme
    // DESC: Supplies mutable theme state for retained-surface appearance regression tests.
    // ------------------=
    fn active_icon_theme(&self) -> u8 {
        TEST_THEME.load(std::sync::atomic::Ordering::Relaxed)
    }
    // ------------------------=
    // FUNC: active_background_effects
    // DESC: Selects no backdrop blur for deterministic surface pixel fixtures.
    // ------------------=
    fn active_background_effects(&self) -> (u8, u8) {
        (100, 0)
    }

    // ------------------------=
    // FUNC: clipped_render_region
    // DESC: Supplies bounded host framebuffer geometry to the actual kernel painter.
    // ------------------=
    fn clipped_render_region(
        &self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) -> Option<Region> {
        let value = Region {
            left: left.max(self.render_clip.map_or(0, |c| c.left)),
            top: top.max(self.render_clip.map_or(0, |c| c.top)),
            right: left
                .saturating_add(width)
                .min(self.width)
                .min(self.render_clip.map_or(self.width, |c| c.right)),
            bottom: top
                .saturating_add(height)
                .min(self.height)
                .min(self.render_clip.map_or(self.height, |c| c.bottom)),
        };
        (value.left < value.right && value.top < value.bottom).then_some(value)
    }
    // ------------------------=
    // FUNC: render_point_visible
    // DESC: Checks a test framebuffer point against its damage clip.
    // ------------------=
    fn render_point_visible(&self, x: usize, y: usize) -> bool {
        x >= self.render_clip.map_or(0, |c| c.left)
            && x < self.render_clip.map_or(self.width, |c| c.right)
            && y >= self.render_clip.map_or(0, |c| c.top)
            && y < self.render_clip.map_or(self.height, |c| c.bottom)
    }
    // ------------------------=
    // FUNC: mark_dirty_rect
    // DESC: Counts actual painter damage submissions without emulating the production damage tracker.
    // ------------------=
    fn mark_dirty_rect(&mut self, _: usize, _: usize, _: usize, _: usize) {
        self.submissions += 1;
    }
    // ------------------------=
    // FUNC: installer_text
    // DESC: Rejects installer-only typography, which is outside this primitive fixture.
    // ------------------=
    fn installer_text(&mut self, _: usize, _: usize, _: &[u8], _: u8, _: u8, _: u8) {
        panic!("installer typography not in fixture");
    }
}

// ------------------------=
// FUNC: scene
// DESC: Exercises the actual glass and bitmap kernels with a full-sized panel and local damage.
// ------------------=
fn scene(display: &mut DisplayDevice) {
    display.paint_bitmap_cover_box(
        include_bytes!("../assets/desktop/infinity-default-dark-wallpaper-v2.bmp"),
        0,
        0,
        2560,
        1600,
    );
    display.fill_rect(50, 50, 2200, 1400, 2, 12, 24);
    display.fill_rounded_rect_alpha(60, 60, 2100, 1300, 18, 4, 30, 55, 173);
    display.outline_rounded_rect(60, 60, 2100, 1300, 18, 100, 180, 230);
}

// ------------------------=
// FUNC: launcher_backdrop_test
// DESC: Verifies cached backdrop restoration is clipped and partial captures are rejected.
// ------------------=
fn launcher_backdrop_test() {
    let mut pixels = vec![0x123456u32; 64 * 48];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 64,
        height: 48,
        stride: 64,
        format: 0,
        render_clip: None,
        fast_motion_frame: false,
        submissions: 0,
        recording_surface: false,
    };
    launcher_backdrop::invalidate();
    assert!(!launcher_backdrop::restore(&mut display));
    launcher_backdrop::capture(&display);
    pixels.fill(0xabcdef);
    display.render_clip = Some(Region {
        left: 3,
        top: 4,
        right: 12,
        bottom: 16,
    });
    assert!(launcher_backdrop::restore(&mut display));
    for y in 0..48 {
        for x in 0..64 {
            assert_eq!(
                pixels[y * 64 + x],
                if (3..12).contains(&x) && (4..16).contains(&y) {
                    0x123456
                } else {
                    0xabcdef
                }
            );
        }
    }
    launcher_backdrop::invalidate();
    launcher_backdrop::capture(&display);
    assert!(!launcher_backdrop::restore(&mut display));
}

#[test]
// ------------------------=
// FUNC: spatial_stage_blurs_to_screen_edges
// DESC: Compares every stage pixel to clamped blur taps and verifies dismissal restores the original.
// ------------------=
fn spatial_stage_blurs_to_screen_edges() {
    let (width, height) = (320usize, 180usize);
    let mut pixels: Vec<u32> = (0..width * height)
        .map(|i| if (i % width / 8 + i / width / 8) % 2 == 0 { 0xffffff } else { 0x304050 })
        .collect();
    let original = pixels.clone();
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(), width, height, stride: width, format: 0,
        render_clip: None, fast_motion_frame: false, submissions: 0, recording_surface: false,
    };
    launcher_backdrop::invalidate();
    launcher_backdrop::capture(&display);
    launcher_backdrop::capture_clean_stage(&display);
    assert!(launcher_backdrop::restore_stage(&mut display));
    for y in 0..height {
        for x in 0..width {
            let mut expected = 0;
            for channel in 0..3 {
                let mut sum = 0;
                for sy in [y.saturating_sub(8), y, (y + 8).min(height - 1)] {
                    for sx in [x.saturating_sub(8), x, (x + 8).min(width - 1)] {
                        sum += (original[sy * width + sx] >> (channel * 8)) & 255;
                    }
                }
                expected |= (sum / 45) << (channel * 8);
            }
            assert_eq!(pixels[y * width + x], expected, "pixel {x},{y}");
        }
    }
    assert!(launcher_backdrop::restore(&mut display));
    assert_eq!(pixels, original);
    launcher_backdrop::capture_wallpaper_stage(&display);
    assert!(launcher_backdrop::restore_stage(&mut display));
    for y in 0..height {
        for x in 0..width {
            let mut expected = 0;
            for channel in 0..3 {
                let mut sum = 0;
                for sy in [y.saturating_sub(8), y, (y + 8).min(height - 1)] {
                    for sx in [x.saturating_sub(8), x, (x + 8).min(width - 1)] {
                        sum += (original[sy * width + sx] >> (channel * 8)) & 255;
                    }
                }
                expected |= (((sum/9)*90 + [8,23,42][channel]*165)/255) << (channel*8);
            }
            assert_eq!(pixels[y*width+x],expected);
        }
    }
    launcher_backdrop::invalidate();
}

// ------------------------=
// FUNC: main
// DESC: Verifies clipped painting against full-render pixels and reports actual painter cost and damage calls.
// ------------------=
fn main() {
    if std::env::args().any(|arg|arg=="--precision-benchmark-only") {precision_surface_benchmark();return;}
    if std::env::args().any(|arg|arg=="--window-substrate-only") {window_substrate_pixels();return;}
    if std::env::args().any(|arg|arg=="--ai-widget-only") {
        ai_widget_tests::verify();return;
    }
    if let Some(label)=std::env::args().find_map(|arg|arg.strip_prefix("--editor-streaming-benchmark=").map(str::to_owned)) {
        editor_streaming_benchmark(&label);return;
    }
    if std::env::args().any(|arg|arg=="--editor-typography-only") {
        editor_typography_pixels();return;
    }
    if std::env::args().any(|arg|arg=="--window-clipping-only") {
        retained_surface_test();retained_left_edge_test();signed_active_outline_test();return;
    }
    if std::env::args().any(|arg|arg=="--window-controls-only") {
        window_controls_test();window_control_states_proof();return;
    }
    window_substrate_pixels();
    precision_surface_benchmark();
    editor_scheme_pixels();
    ai_widget_tests::verify();
    editor_typography_pixels();
    font_library_pixels();
    thinking_text_pixels_test();
    retained_theme_test();
    icon_downscale_test();
    launcher_backdrop_test();
    window_controls_test();
    retained_window_benchmark();
    damage_test();
    retained_surface_test();
    retained_left_edge_test();
    signed_active_outline_test();
    retained_partial_update_test();
    retained_projection_revision_test();
    let mut full = vec![0x102030u32; 2560 * 1600];
    let mut partial = full.clone();
    for format in [0, 1] {
        for (extent, label) in [(32, "local"), (900, "window")] {
            full.fill(0x102030);
            partial.fill(0x102030);
            let mut display = DisplayDevice {
                buffer: full.as_mut_ptr(),
                width: 2560,
                height: 1600,
                stride: 2560,
                format,
                render_clip: Some(Region {
                    left: 0,
                    top: 0,
                    right: 2560,
                    bottom: 1600,
                }),
                fast_motion_frame: false,
                submissions: 0,
                recording_surface: false,
            };
            scene(&mut display);
            display.buffer = partial.as_mut_ptr();
            display.render_clip = Some(Region {
                left: 65,
                top: 65,
                right: 65 + extent,
                bottom: 65 + extent,
            });
            display.submissions = 0;
            let mut times = [0u128; 20];
            for time in &mut times {
                let start = Instant::now();
                scene(&mut display);
                *time = start.elapsed().as_nanos();
            }
            for y in 0..1600 {
                for x in 0..2560 {
                    assert_eq!(
                        partial[y * 2560 + x],
                        if display.render_point_visible(x, y) {
                            full[y * 2560 + x]
                        } else {
                            0x102030
                        },
                        "{x},{y}"
                    );
                }
            }
            times.sort_unstable();
            println!("{{\"fixture\":\"active_painter_{label}\",\"format\":{},\"average_ns\":{},\"p95_ns\":{},\"damage_submissions\":{}}}",format,times.iter().sum::<u128>()/20,times[18],display.submissions);
        }
    }
}

// ------------------------=
// FUNC: precision_surface_benchmark
// DESC: Measures production substrate/frame and focused-rim painting separately from retained translation on the host framebuffer.
// ------------------=
fn precision_surface_benchmark() {
    use ui::{geometry::Rect,panel_recipe::{Material,Surface}};
    let mut pixels=vec![0x261009u32;1600*1000];
    let mut d=DisplayDevice{buffer:pixels.as_mut_ptr(),width:1600,height:1000,stride:1600,format:0,
        render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    let base=Material{fill:(16,27,58),edge:(134,199,255),opacity:224,
        edge_opacity:255,focus:0,pressed:false,scale:1};
    for mode in 0..3 {
        let mut times=[0u128;20];
        for (index,time) in times.iter_mut().enumerate() {
            let start=Instant::now();
            if mode==0 {
                let rect=Rect{x:20,y:20,width:1280,height:900};
                d.window_substrate(rect,base,0);d.precision_window_frame(rect,true,1);
            }else{
                d.precision_surface(Rect{x:20,y:20,width:432,height:74},Surface::Panel,
                    Material{focus:if mode==1 {255}else{(index*255/19) as u8},opacity:0,..base});
            }
            *time=start.elapsed().as_nanos();
        }
        times.sort_unstable();
        println!("{{\"fixture\":\"precision_surface\",\"mode\":{mode},\"samples\":20,\"average_ns\":{},\"p95_ns\":{},\"host_only\":true}}",
            times.iter().sum::<u128>()/20,times[18]);
    }
}

// ------------------------=
// FUNC: window_substrate_expected
// DESC: Independently computes source-over pixels in both firmware channel formats and retained-alpha mode.
// ------------------=
fn window_substrate_expected(base:u32, rgba:(u8,u8,u8,u8), format:u32, retained:bool)->u32 {
    let (r,g,b,a)=rgba;
    if a==0 {return base;}
    let mut out=0u32;
    for (shift,channel) in if format==0 {[(0,r),(8,g),(16,b)]}else{[(16,r),(8,g),(0,b)]} {
        out|=((((base>>shift)&255)*(255-u32::from(a))+u32::from(channel)*u32::from(a))/255)<<shift;
    }
    if retained {out|=(u32::from(a)+(base>>24)*(255-u32::from(a))/255)<<24;}
    out
}

// ------------------------=
// FUNC: window_substrate_pixels
// DESC: Verifies real window substrates never paint a competing border, preserve alpha, clip signed bounds, and leave activation to the shared frame.
// ------------------=
fn window_substrate_pixels() {
    use ui::{geometry::{Rect,Point},panel_recipe::{self,Material,Surface}};
    for scale in 1..=4 {
        let (width,height)=(96*scale,72*scale);
        for format in [0,1] {for retained in [false,true] {
            let card=Rect{x:(8*scale) as i32,y:(8*scale) as i32,
                width:(72*scale) as u32,height:(52*scale) as u32};
            let base=0x142245|if retained {0x4c000000}else{0};
            let mut hidden=vec![base;width*height];let mut filled=hidden.clone();
            let mut card_display=DisplayDevice{buffer:hidden.as_mut_ptr(),width,height,stride:width,format,
                render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:retained};
            card_display.app_card_alpha(card,(18,31,55),(134,199,255),0,scale);
            card_display.buffer=filled.as_mut_ptr();
            card_display.app_card_alpha(card,(18,31,55),(134,199,255),255,scale);
            let mut covered_border=0;
            for dy in 0..card.height as usize {for dx in 0..card.width as usize {
                if dx.min(dy).min(card.width as usize-1-dx).min(card.height as usize-1-dy)<2*scale {
                    let at=(card.y as usize+dy)*width+card.x as usize+dx;
                    assert_eq!(hidden[at],filled[at]);covered_border+=usize::from(hidden[at]!=base);
                }
            }}
            assert!(covered_border>0);
            let card_center=(card.y as usize+card.height as usize/2)*width+card.x as usize+card.width as usize/2;
            assert_eq!(hidden[card_center],base);assert_ne!(filled[card_center],base);
            for (rx,ry) in [(-7,8),(14,-9),(12,6)] {
                let rect=Rect{x:rx*scale as i32,y:ry*scale as i32,
                    width:(78*scale) as u32,height:(56*scale) as u32};
                let clip=Region{left:9*scale,top:7*scale,right:80*scale,bottom:63*scale};
                for opacity in [0,117,255] {
                    let material=Material{fill:(18,31,55),edge:(134,199,255),opacity,
                        edge_opacity:255,focus:0,pressed:false,scale};
                    let original:Vec<u32>=(0..width*height).map(|i|
                        (if (i%width/(7*scale)+i/width/(5*scale))%2==0 {0x142245}else{0x071127})
                        |if retained {0x4c000000}else{0}).collect();
                    let mut pixels=original.clone();
                    let mut d=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,format,
                        render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:retained};
                    d.window_substrate(rect,material,0);
                    for y in 0..height {for x in 0..width {
                        let at=y*width+x;
                        let inside=rect.contains(Point{x:x as i32,y:y as i32});
                        let rgba=if inside {panel_recipe::pixel(Surface::Panel,rect.width as usize,
                            rect.height as usize,(x as i32-rect.x) as usize,(y as i32-rect.y) as usize,
                            Material{edge_opacity:0,..material})}else{(0,0,0,0)};
                        assert_eq!(pixels[at],window_substrate_expected(original[at],rgba,format,retained));
                        if inside {
                            let (dx,dy)=((x as i32-rect.x) as usize,(y as i32-rect.y) as usize);
                            if dx.min(dy).min(rect.width as usize-1-dx).min(rect.height as usize-1-dy)<2*scale {
                                assert_eq!(pixels[at],original[at]);
                            }
                        }
                    }}
                    if opacity==0 {assert_eq!(pixels,original);assert_eq!(d.submissions,0);}
                    let mut partial=original.clone();
                    d.buffer=partial.as_mut_ptr();d.render_clip=Some(clip);
                    d.window_substrate(rect,material,0);
                    for y in 0..height {for x in 0..width {
                        let at=y*width+x;
                        assert_eq!(partial[at],if x>=clip.left&&x<clip.right&&y>=clip.top&&y<clip.bottom {
                            pixels[at]
                        }else{original[at]});
                    }}
                    let mut active=pixels.clone();let mut inactive=pixels.clone();
                    d.render_clip=None;d.buffer=active.as_mut_ptr();d.precision_window_frame(rect,true,scale);
                    d.buffer=inactive.as_mut_ptr();d.precision_window_frame(rect,false,scale);
                    assert_ne!(active,inactive);
                    let center=((rect.y+rect.height as i32/2) as usize)*width
                        +(rect.x+rect.width as i32/2) as usize;
                    assert_eq!(active[center],pixels[center]);assert_eq!(inactive[center],pixels[center]);
                }
            }
        }}
    }
}

#[test]
// ------------------------=
// FUNC: window_substrate_pixel_regression
// DESC: Makes the shared substrate alpha, clipping and single-owner frame contract available to the unit harness.
// ------------------=
fn window_substrate_pixel_regression() { window_substrate_pixels(); }

// ------------------------=
// FUNC: editor_streaming_benchmark
// DESC: Measures real streamed-document append/reflow and production document painting independently of LLM generation or transport speed.
// ------------------=
fn editor_streaming_benchmark(label:&str) {
    use std::{hint::black_box,io::Write};
    use ui::editor_tools::{self,preferences::{self,Preferences}};
    assert!(matches!(label,"before"|"after"));
    preferences::publish(Preferences::new());
    let source=b"    int next_value = current_value + 1; /* bounded streamed source */\n";
    let mut records=Vec::new();
    for length in [1024,4096,16384] {
        let input:Vec<u8>=source.iter().copied().cycle().take(length).collect();
        let mut append_times=Vec::new();
        for _ in 0..9 {
            let mut document=ui::text_editor::TextDocument::new();
            let begin=Instant::now();
            for (index,chunk) in input.chunks(32).enumerate() {
                black_box(ui::text_editor::visual_line_count(document.bytes(),80));
                assert!(document.append_generation_preview(black_box(chunk),index==0));
                black_box(ui::text_editor::visual_line_count(document.bytes(),80));
            }
            append_times.push(begin.elapsed().as_nanos());
            assert_eq!(document.bytes(),input);assert!(document.undo());assert!(document.bytes().is_empty());
        }
        append_times.sort_unstable();
        for scale in [1,2] {
            let (width,height)=(1000*scale,660*scale);
            let window=ui::system_layout::SystemLayout::new(width,height).desktop_app_window_geometry(20,30,960,920,false);
            let g=ui::editor_chrome::Layout::new(window.window,scale,false,editor_tools::Field::None);
            let scroll=ui::text_editor::visual_line_count(&input,g.columns()).saturating_sub(g.rows());
            let mut view=editor_tools::Presentation::new();view.language=editor_tools::Language::C;view.cursor=input.len();editor_tools::publish(view);
            let mut pixels=vec![0u32;width*height];
            let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,
                render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
            let mut times=Vec::new();let mut damage=0;
            for sample in 0..12 {
                display.submissions=0;let begin=Instant::now();
                display.editor_window(window,black_box(&input),scroll,false,false,scale);
                let elapsed=begin.elapsed().as_nanos();black_box(&pixels);
                if sample>=3 {times.push(elapsed);damage=display.submissions;}
            }
            times.sort_unstable();
            let record=format!("{{\"bytes\":{length},\"scale\":{scale},\"frames\":9,\"paint_median_ns\":{},\"paint_max_ns\":{},\"damage_submissions\":{damage},\"append_and_reflow_32byte_chunks_median_ns\":{}}}",times[4],times[8],append_times[4]);
            println!("{record}");records.push(record);
        }
    }
    std::fs::create_dir_all("builds/validation/editor").unwrap();
    let mut file=std::fs::File::create(format!("builds/validation/editor/streaming-{label}.json")).unwrap();
    writeln!(file,"[{}]",records.join(",\n")).unwrap();
    preferences::publish(Preferences::new());editor_tools::publish(editor_tools::Presentation::new());
}

// ------------------------=
// FUNC: editor_typography_pixels
// DESC: Verifies native atlas coverage, independent ink/advance metrics and unchanged chrome at nine zoom levels, then captures real editor pixels.
// ------------------=
fn editor_typography_pixels() {
    use std::io::Write;
    use ui::editor_tools::{self,preferences::{self,Preferences}};
    editor_glyph_damage_pixels();
    for font in 0..4 {
        for size in 0..preferences::SIZES.len() as u8 {
            let p=Preferences{scheme:0,font,size};preferences::publish(p);
            let record=p.record();
            for scale in 1..=2 {
                let mut pixels=vec![0u32;64*72];
                let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width:64,height:72,stride:64,
                    format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
                for byte in 32..=126 {
                    pixels.fill(0);
                    display.editor_glyph(4,4,byte,scale,(255,255,255));
                    for y in 0..72 {
                        for x in 0..64 {
                            let alpha=if x>=4 && y>=4 && x<4+record[3] as usize*scale && y<4+record[1] as usize*scale {
                                record[4+(y-4)/scale*24*95+(byte as usize-32)*24+(x-4)/scale] as u32
                            }else{0};
                            assert_eq!(pixels[y*64+x],alpha*0x010101);
                        }
                    }
                }
            }
        }
    }
    let (width,height)=(1100,700);
    let window=ui::system_layout::SystemLayout::new(width,height).desktop_app_window_geometry(20,30,960,920,false);
    let code=b"// Native C source - precise, compact monospace\n#include <stdio.h>\n\nint main(void) {\n    int total = 42;\n    for (int i = 0; i < 4; ++i) {\n        total += i;\n    }\n    printf(\"Total: %d\\n\", total);\n    return 0;\n}\n";
    let mut view=editor_tools::Presentation::new();view.language=editor_tools::Language::C;
    view.filename[..6].copy_from_slice(b"main.c");view.filename_len=6;
    view.cursor=code.iter().position(|byte|*byte==b'4').unwrap();view.selection=Some((view.cursor,view.cursor+2));
    std::fs::create_dir_all("builds/validation/editor").unwrap();
    let mut title=None;let mut signatures=Vec::new();
    for size in 0..preferences::SIZES.len() as u8 {
        let p=Preferences{scheme:0,font:0,size};preferences::publish(p);editor_tools::publish(view);
        ui::text_input::set_presentation(true,true,view.cursor,4,false);
        let mut pixels=vec![0x20120bu32;width*height];
        let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,
            render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
        display.editor_window(window,code,0,true,false,1);
        let g=ui::editor_chrome::Layout::new(window.window,1,false,editor_tools::Field::None);
        let chrome=&pixels[(window.window.y as usize+5)*width..g.body.y as usize*width];
        if let Some(ref previous)=title {assert_eq!(chrome,previous);}else{title=Some(chrome.to_vec());}
        let signature=pixels[g.body.y as usize*width..g.body.bottom() as usize*width]
            .iter().fold(0u64,|hash,pixel|hash.wrapping_mul(1099511628211)^*pixel as u64);
        assert!(!signatures.contains(&signature));signatures.push(signature);
        let caret=g.caret_damage(code,view.cursor,0).next().unwrap();
        assert_eq!(g.pointer_index(code,0,ui::geometry::Point{x:caret.x,y:caret.y}),view.cursor);
        let (red,green,blue)=p.color(4);let caret_color=red as u32|((green as u32)<<8)|((blue as u32)<<16);
        for y in caret.y..caret.bottom() {
            assert_eq!(pixels[y as usize*width+caret.x as usize],caret_color);
        }
        let (red,green,blue)=p.color(3);let selection_color=red as u32|((green as u32)<<8)|((blue as u32)<<16);
        for x in caret.x..caret.x+2*p.cell() as i32 {
            assert_eq!(pixels[(caret.y as usize-2)*width+x as usize],selection_color);
        }
        let mut file=std::fs::File::create(format!("builds/validation/editor/zoom-{}.ppm",preferences::SIZES[size as usize])).unwrap();
        write!(file,"P6\n{width} {height}\n255\n").unwrap();
        for pixel in pixels {file.write_all(&[pixel as u8,(pixel>>8) as u8,(pixel>>16) as u8]).unwrap();}
    }
    ui::text_input::set_presentation(false,false,0,0,false);
    preferences::publish(Preferences::new());editor_tools::publish(editor_tools::Presentation::new());
}

// ------------------------=
// FUNC: editor_glyph_damage_pixels
// DESC: Compares batched glyphs against original per-pixel composition across clips, zoom, font families, formats and retained-alpha surfaces.
// ------------------=
fn editor_glyph_damage_pixels() {
    use ui::editor_tools::preferences::{self,Preferences};
    for font in 0..4 {
        for size in 0..preferences::SIZES.len() as u8 {
            let p=Preferences{scheme:0,font,size};preferences::publish(p);let record=p.record();
            for scale in [1,2] {
                for format in [0,1] {
                    for recording_surface in [false,true] {
                        for clip in [None,Some(Region{left:7,top:9,right:22,bottom:25}),Some(Region{left:60,top:60,right:64,bottom:72})] {
                            for byte in b" Mj[]" {
                                let mut actual=vec![0x7f203040u32;64*72];let mut expected=actual.clone();
                                let mut display=DisplayDevice{buffer:actual.as_mut_ptr(),width:64,height:72,stride:64,format,
                                    render_clip:clip,fast_motion_frame:false,submissions:0,recording_surface};
                                let mut reference=display;reference.buffer=expected.as_mut_ptr();
                                for row in 0..record[1] as usize*scale {
                                    for column in 0..record[3] as usize*scale {
                                        let alpha=record[4+row/scale*24*95+(*byte as usize-32)*24+column/scale];
                                        if alpha!=0 {reference.blend_color((4+column) as i32,(4+row) as i32,221,232,244,alpha);}
                                    }
                                }
                                display.editor_glyph(4,4,*byte,scale,(221,232,244));
                                assert_eq!(actual,expected);
                                assert_eq!(display.submissions,u64::from(reference.submissions>0));
                            }
                        }
                    }
                }
            }
        }
    }
    preferences::publish(Preferences::new());
}

// ------------------------=
// FUNC: editor_scheme_pixels
// DESC: Captures all ten production editor schemes and asserts unchanged chrome plus changed document pixels.
// ------------------=
fn editor_scheme_pixels(){
    use std::io::Write;
    use ui::editor_tools::{self,preferences::{self,Preferences}};
    let width=1100;let height=700;
    let layout=ui::system_layout::SystemLayout::new(width,height);
    let window=layout.desktop_app_window_geometry(20,30,960,920,false);
    let mut view=editor_tools::Presentation::new();view.language=editor_tools::Language::Rust;
    view.filename[..7].copy_from_slice(b"main.rs");view.filename_len=7;
    let code=b"// Native InfinityOS editor\nfn main() {\n    let answer = 42;\n    println!(\"Hello, Infinity!\");\n}\n";
    view.cursor=code.iter().position(|c|*c==b'4').unwrap();view.selection=Some((view.cursor,view.cursor+2));
    let mut title=None;let mut body_pixels=Vec::new();
    std::fs::create_dir_all("builds/validation/editor").unwrap();
    for scheme in 0..10 {
        preferences::publish(Preferences{scheme,font:0,size:2});editor_tools::publish(view);
        let mut pixels=vec![0x20120bu32;width*height];
        let mut d=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
        d.editor_window(window,code,0,true,false,1);
        let g=ui::editor_chrome::Layout::new(window.window,1,false,editor_tools::Field::None);
        let top=&pixels[(window.window.y as usize+5)*width..(window.window.y as usize+40)*width];
        if let Some(ref old)=title{assert_eq!(top,old)}else{title=Some(top.to_vec())}
        body_pixels.push(pixels[(g.body.y as usize+2)*width+g.body.x as usize+100]);
        let mut f=std::fs::File::create(format!("builds/validation/editor/scheme-{scheme}.ppm")).unwrap();write!(f,"P6\n{width} {height}\n255\n").unwrap();
        for p in pixels {f.write_all(&[p as u8,(p>>8)as u8,(p>>16)as u8]).unwrap();}
    }
    body_pixels.sort_unstable();body_pixels.dedup();assert_eq!(body_pixels.len(),10);
    preferences::publish(Preferences::new());editor_tools::publish(editor_tools::Presentation::new());
}

// ------------------------=
// FUNC: font_library_pixels
// DESC: Exercises the actual Settings font painter and exports production pixels for visual review.
// ------------------=
fn font_library_pixels() {
    use std::io::Write;
    let width=620;let height=520;
    let mut pixels=vec![0x081522u32;width*height];
    let mut display=DisplayDevice {buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,
        render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    ui::font_library::set_page(0);
    let bounds=ui::geometry::Rect{x:16,y:16,width:588,height:488};
    display.settings_font_library(bounds,1);
    assert!(pixels.iter().any(|p|*p!=0x081522));
    assert!(pixels[..width*16].iter().all(|p|*p==0x081522));
    std::fs::create_dir_all("builds/validation/fonts").unwrap();
    let mut file=std::fs::File::create("builds/validation/fonts/settings-fonts.ppm").unwrap();
    write!(file,"P6\n{width} {height}\n255\n").unwrap();
    for p in pixels {file.write_all(&[p as u8,(p>>8) as u8,(p>>16) as u8]).unwrap();}
}

// ------------------------=
// FUNC: thinking_text_pixels_test
// DESC: Verifies font shading changes pixels, preserves glyph edges, and respects header clipping.
// ------------------=
fn thinking_text_pixels_test() {
    let mut pixels = vec![0u32; 320 * 120];
    let mut display = DisplayDevice { buffer: pixels.as_mut_ptr(), width: 320, height: 120,
        stride: 320, format: 0, render_clip: Some(Region { left: 0, top: 0, right: 320, bottom: 48 }),
        fast_motion_frame: false, submissions: 0, recording_surface: false };
    display.ui_text_shaded(20, 10, b"Thinking...", 255, 255, 255, 1, false, Some((0, 96)));
    let first = pixels.clone();
    pixels.fill(0);
    display.ui_text_shaded(20, 10, b"Thinking...", 255, 255, 255, 1, false, Some((32, 96)));
    assert_ne!(first, pixels);
    for (before, after) in first.iter().zip(&pixels) {
        // A one-level antialiased fringe may round to zero at the dim phase.
        if (*before == 0) != (*after == 0) {
            assert!((before & 255).max(after & 255) <= 1);
        }
        assert_eq!(after & 255, (after >> 8) & 255);
        assert_eq!(after & 255, (after >> 16) & 255);
    }
    assert!(pixels[320 * 48..].iter().all(|pixel| *pixel == 0));
}

static TEST_THEME: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

// ------------------------=
// FUNC: retained_theme_test
// DESC: Checks every cached app surface repaints after theme selection without explicit global invalidation.
// ------------------=
fn retained_theme_test() {
    retained_windows::invalidate();
    let mut pixels = vec![0u32; 320 * 200];
    let mut display = DisplayDevice { buffer: pixels.as_mut_ptr(), width: 320, height: 200,
        stride: 320, format: 0, render_clip: None, fast_motion_frame: false,
        submissions: 0, recording_surface: false };
    for theme in 0..4 {
        TEST_THEME.store(theme, std::sync::atomic::Ordering::Relaxed);
        for slot in 0..6 {
            let mut paints = 0;
            display.retained_window(slot, (40, 40, 40, 40), |target| {
                paints += 1;
                target.fill_rect(50, 50, 10, 10, theme + 1, 0, 0);
            });
            assert_eq!(paints, 1);
            assert_eq!(pixels[50 * 320 + 50], theme as u32 + 1);
            display.retained_window(slot, (40, 40, 40, 40), |_| panic!("unchanged theme repainted"));
        }
    }
    TEST_THEME.store(0, std::sync::atomic::Ordering::Relaxed);
    retained_windows::invalidate();
}

// ------------------------=
// FUNC: icon_downscale_test
// DESC: Verifies subpixel icon coverage and transparent edge colors through the production painter.
// ------------------=
fn icon_downscale_test() {
    let mut bitmap = vec![0u8; 54 + 16];
    bitmap[..2].copy_from_slice(b"BM");
    bitmap[10..14].copy_from_slice(&54u32.to_le_bytes());
    bitmap[18..22].copy_from_slice(&2u32.to_le_bytes());
    bitmap[22..26].copy_from_slice(&2i32.to_le_bytes());
    bitmap[28..30].copy_from_slice(&32u16.to_le_bytes());
    bitmap[54..58].copy_from_slice(&[255, 255, 255, 255]);
    let mut pixels = [0u32; 2];
    let mut display = DisplayDevice { buffer: pixels.as_mut_ptr(), width: 2, height: 1,
        stride: 2, format: 0, render_clip: None, fast_motion_frame: false,
        submissions: 0, recording_surface: false };
    assert!(display.paint_bitmap_alpha_atlas_cell(&bitmap, 1, 1, 0, 0, 0, 1));
    display.blend_color(1, 0, 255, 255, 255, 63);
    assert_ne!(pixels[0], 0);
    assert_eq!(pixels[0], pixels[1]);
}

// ------------------------=
// FUNC: window_controls_test
// DESC: Verifies glyphs remain visible inside retained RGBA controls at every supported UI scale.
// ------------------=
fn window_controls_test() {
    for scale in 1..=3 {
        for index in 0..3 {
            for maximized in [false, true] {
                let size = 20 * scale;
                let mut pixels = vec![0u32; 128 * 128];
                let mut display = DisplayDevice {
                    buffer: pixels.as_mut_ptr(),
                    width: 128,
                    height: 128,
                    stride: 128,
                    format: 0,
                    render_clip: None,
                    fast_motion_frame: false,
                    submissions: 0,
                    recording_surface: true,
                };
                display.window_control(8, 8, size, index, maximized);
                let mut glyph_pixels = 0;
                for y in 8 + size / 4..8 + size * 3 / 4 + 1 {
                    for x in 8 + size / 4..8 + size * 3 / 4 + 1 {
                        let pixel = pixels[y * 128 + x];
                        if pixel & 255 > 200 && pixel >> 24 == 255 {
                            glyph_pixels += 1;
                        }
                    }
                }
                assert!(glyph_pixels >= 8 * scale);
                assert_eq!(pixels[0], 0);
            }
        }
    }
    let mut pixels = vec![0x001a0b05u32; 256 * 96];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 256,
        height: 96,
        stride: 256,
        format: 0,
        render_clip: None,
        fast_motion_frame: false,
        submissions: 0,
        recording_surface: false,
    };
    for (slot, (index, maximized)) in [(0, false), (1, false), (1, true), (2, false)]
        .iter()
        .enumerate()
    {
        display.window_control(16 + slot * 60, 28, 40, *index, *maximized);
    }
    let mut ppm = b"P6\n256 96\n255\n".to_vec();
    for pixel in pixels {
        ppm.extend_from_slice(&[pixel as u8, (pixel >> 8) as u8, (pixel >> 16) as u8]);
    }
    std::fs::write("build/window-controls-proof.ppm", ppm).unwrap();
}

// ------------------------=
// FUNC: window_control_states_proof
// DESC: Captures actual normal, hover and pressed native button rendering with clear glyphs and transparent corners.
// ------------------=
fn window_control_states_proof() {
    use ui::{geometry::Rect,app_launcher::window_controls as controls};
    let (width,height)=(512,240);
    let mut pixels=vec![0x001a0b05u32;width*height];
    let mut d=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,
        render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    for state in 0..3 {
        for (i,(index,maximized)) in [(0,false),(1,false),(1,true),(2,false)].into_iter().enumerate() {
            let (x,y)=(32+i*120,20+state*72);
            let rect=Rect{x:x as i32,y:y as i32,width:48,height:48};
            controls::publish(if state==0 {None}else{Some(controls::Highlight{rect,pressed:state==2})});
            d.window_control(x,y,48,index,maximized);
            assert_eq!(pixels[y*width+x],0x001a0b05);
        }
    }
    controls::publish(None);let _=controls::take_damage();
    let mut ppm=format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in pixels {ppm.extend_from_slice(&[pixel as u8,(pixel>>8) as u8,(pixel>>16) as u8]);}
    std::fs::write("build/window-control-states-proof.ppm",ppm).unwrap();
}

// ------------------------=
// FUNC: retained_window_benchmark
// DESC: Measures the actual native cached-window composition path and asserts no application painting during translations.
// ------------------=
fn retained_window_benchmark() {
    retained_windows::invalidate();
    let mut pixels = vec![0x102030u32; 2560 * 1600];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 2560,
        height: 1600,
        stride: 2560,
        format: 0,
        render_clip: None,
        fast_motion_frame: true,
        submissions: 0,
        recording_surface: false,
    };
    let mut painted = 0;
    display.retained_window(1, (80, 80, 1280, 900), |target| {
        painted += 1;
        scene(target);
    });
    let mut times = [0u128; 100];
    for (index, elapsed) in times.iter_mut().enumerate() {
        let start = Instant::now();
        display.retained_window(1, (80 + index, 80, 1280, 900), |_| {
            painted += 1;
        });
        *elapsed = start.elapsed().as_nanos();
    }
    assert_eq!(painted, 1);
    times.sort_unstable();
    println!("{{\"fixture\":\"native_cached_window\",\"frames\":100,\"application_paints\":{},\"average_ns\":{},\"p95_ns\":{}}}",
        painted, times.iter().sum::<u128>() / 100, times[94]);
}

// ------------------------=
// FUNC: damage_test
// DESC: Verifies real presentation damage covers sparse input without full-screen overflow collapse.
// ------------------=
fn damage_test() {
    use ui::present_damage::{insert, Region};
    let mut regions = [Region::default(); 8];
    let mut count = 0;
    for index in 0..9 {
        let x = 10 + index * 250;
        let y = 10 + index * 150;
        insert(
            &mut regions,
            &mut count,
            Region {
                left: x,
                top: y,
                right: x + 8,
                bottom: y + 8,
            },
        );
    }
    assert_eq!(count, 8);
    assert!(regions[..count].iter().map(|r| r.area()).sum::<usize>() < 2560 * 1600 / 20);
    for index in 0..9 {
        let x = 10 + index * 250;
        let y = 10 + index * 150;
        assert!(regions[..count].iter().any(|r| r.contains(Region {
            left: x,
            top: y,
            right: x + 8,
            bottom: y + 8
        })));
    }
    for x in 100..200 {
        count = 0;
        for left in [x, x + 3] {
            insert(
                &mut regions,
                &mut count,
                Region {
                    left,
                    top: 100,
                    right: left + 56,
                    bottom: 156,
                },
            );
        }
        assert!(regions[..count].iter().map(|r| r.area()).sum::<usize>() <= 59 * 56);
    }
}

// ------------------------=
// FUNC: retained_surface_test
// DESC: Verifies translation reuses pixels, preserves transparency on changed backdrops, and invalidation repaints once.
// ------------------=
fn retained_surface_test() {
    retained_windows::invalidate();
    let mut pixels = vec![0x302010u32; 320 * 200];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 320,
        height: 200,
        stride: 320,
        format: 0,
        render_clip: None,
        fast_motion_frame: false,
        submissions: 0,
        recording_surface: false,
    };
    let mut painted = 0;
    display.retained_window(0, (40, 40, 40, 40), |target| {
        painted += 1;
        target.fill_rect_alpha(40, 40, 40, 40, 80, 100, 120, 128);
        target.fill_rect(50, 50, 10, 10, 1, 2, 3);
    });
    assert_eq!(painted, 1);
    assert_eq!(pixels[50 * 320 + 50], 0x030201);
    pixels.fill(0x908070);
    display.retained_window(0, (90, 60, 40, 40), |_| {
        painted += 1;
    });
    assert_eq!(
        painted, 1,
        "Translation must never call the application painter"
    );
    assert_eq!(pixels[70 * 320 + 100], 0x030201);
    assert_eq!(pixels[40 * 320 + 40], 0x908070);
    let expected = retained_windows::over(128 << 24 | 60 << 16 | 50 << 8 | 40, 0x908070);
    assert_eq!(pixels[60 * 320 + 90], expected);
    display.render_clip = Some(Region {
        left: 100,
        top: 70,
        right: 102,
        bottom: 72,
    });
    pixels.fill(0xabcdef);
    display.retained_window(0, (90, 60, 40, 40), |_| {
        panic!("valid cache must be reused")
    });
    for y in 0..200 {
        for x in 0..320 {
            assert_eq!(
                pixels[y * 320 + x],
                if (100..102).contains(&x) && (70..72).contains(&y) {
                    0x030201
                } else {
                    0xabcdef
                }
            );
        }
    }
    retained_windows::invalidate();
    display.retained_window(0, (90, 60, 40, 40), |_| {
        painted += 1;
    });
    assert_eq!(painted, 2);
}

// ------------------------=
// FUNC: retained_left_edge_test
// DESC: Checks signed left-edge cropping pixel-for-pixel, bounded erasure and cached motion back into view.
// ------------------=
fn retained_left_edge_test() {
    retained_windows::invalidate();
    let background=0x302010u32;
    let mut pixels=vec![background;320*200];
    let mut display=DisplayDevice {buffer:pixels.as_mut_ptr(),width:320,height:200,stride:320,
        format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    let mut paints=0;
    let positions:[i32;9]=[40,0,-1,-12,-70,-110,-12,0,40];
    let mut previous=positions[0];
    for (step,left) in positions.into_iter().enumerate() {
        let old_left=previous.saturating_sub(16).max(0) as usize;
        let old_right=(previous+116).clamp(0,320) as usize;
        let new_left=left.saturating_sub(16).max(0) as usize;
        let new_right=(left+116).clamp(0,320) as usize;
        let damages=if step==0 {[(0,320),(0,0)]}
            else {[(old_left,old_right),(new_left,new_right)]};
        for (damage_left,damage_right) in damages {
            if damage_left>=damage_right {continue;}
            display.render_clip=Some(Region {left:damage_left,top:24,right:damage_right,bottom:136});
            display.fill_rect(0,0,320,200,0x10,0x20,0x30);
            display.retained_window_at(0,(left,40,100,80),|target| {
                paints+=1;
                for y in 40..120 {for x in 0..100 {
                    target.fill_rect(left.max(0) as usize+x,y,1,1,(x+1) as u8,(y-39) as u8,90);
                }}
            });
        }
        display.render_clip=None;
        for y in 0..200 {for x in 0..320 {
            let local=x as i32-left;
            let expected=if (0..100).contains(&local)&&(40..120).contains(&y) {
                (90<<16)|(((y-39) as u32)<<8)|(local as u32+1)
            } else {background};
            assert_eq!(pixels[y*320+x],expected,"left={left}, pixel=({x},{y})");
        }}
        previous=left;
    }
    // Only the edge-padding transitions rebuild, never each negative drag step.
    assert_eq!(paints,3);
}

// ------------------------=
// FUNC: signed_active_outline_test
// DESC: Verifies a left-clipped active-window border preserves its off-screen origin exactly like right-edge clipping.
// ------------------=
fn signed_active_outline_test() {
    let background=0x302010u32;
    let mut pixels=vec![background;160*100];
    let mut display=DisplayDevice {buffer:pixels.as_mut_ptr(),width:160,height:100,stride:160,
        format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    display.outline_rounded_rect_signed(-24,10,100,70,8,105,199,245);
    let outline=245u32<<16|199u32<<8|105;
    assert_eq!(pixels[10*160],outline,"visible top edge must retain its signed origin");
    assert_eq!(pixels[45*160],background,"left clipping must not synthesize a border at x=0");
    assert_eq!(pixels[45*160+75],outline,"the original right edge must remain at x=75");
    assert_eq!(pixels[45*160+76],background,"the clipped outline must not retain its unshifted width");
}

// ------------------------=
// FUNC: retained_projection_revision_test
// DESC: Verifies an asynchronous projection invalidates only Settings pixels; unchanged revisions and unrelated window surfaces retain their cached paint.
// ------------------=
fn retained_projection_revision_test() {
    retained_windows::invalidate();
    let mut pixels=vec![0x302010u32;320*200];
    let mut display=DisplayDevice {buffer:pixels.as_mut_ptr(),width:320,height:200,stride:320,format:0,
        render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    let bounds=(40,40,100,80);let mut revision=None;
    assert!(retained_windows::invalidate_revision(4,&mut revision,Some(1)));
    display.retained_window(4,bounds,|target|target.fill_rect(40,40,100,80,1,2,3));
    display.retained_window(0,(180,40,80,80),|target|target.fill_rect(180,40,80,80,7,8,9));
    assert!(!retained_windows::invalidate_revision(4,&mut revision,Some(1)));
    display.retained_window(4,bounds,|_|panic!("unchanged projection repainted"));
    let before=pixels.clone();
    assert!(retained_windows::invalidate_revision(4,&mut revision,Some(2)));
    display.render_clip=Some(Region {left:40,top:40,right:140,bottom:120});
    display.retained_window(4,bounds,|target|target.fill_rect(40,40,100,80,4,5,6));
    for y in 0..200 {for x in 0..320 {
        assert_eq!(pixels[y*320+x],if (40..140).contains(&x)&&(40..120).contains(&y) {0x060504}else{before[y*320+x]});
    }}
    display.render_clip=None;
    display.retained_window(0,(180,40,80,80),|_|panic!("unrelated window invalidated"));
    assert!(!retained_windows::invalidate_revision(4,&mut revision,None));
    display.retained_window(4,bounds,|_|panic!("hidden projection invalidated"));
    assert!(retained_windows::invalidate_revision(4,&mut revision,Some(2)));
}

// ------------------------=
// FUNC: retained_partial_update_test
// DESC: Verifies clipped cache updates equal full repaint pixels, preserve unchanged content, and remain valid for subsequent translation.
// ------------------=
fn retained_partial_update_test() {
    retained_windows::invalidate();
    let mut pixels = vec![0x302010u32; 320 * 200];
    let mut display = DisplayDevice { buffer: pixels.as_mut_ptr(), width: 320,
        height: 200, stride: 320, format: 0, render_clip: None,
        fast_motion_frame: false, submissions: 0, recording_surface: false };
    let bounds = (40, 40, 140, 100);
    display.retained_window(1, bounds, |target| {
        target.fill_rect_alpha(40, 40, 140, 100, 80, 100, 120, 128);
        target.fill_rect(50, 100, 100, 20, 1, 2, 3);
    });
    let prior = pixels.clone();
    let changed = Region { left: 50, top: 100, right: 150, bottom: 120 };
    retained_windows::invalidate_region(1, changed);
    display.render_clip = Some(changed);
    display.retained_window(1, bounds, |target| {
        let clip = target.render_clip.unwrap();
        assert_eq!((clip.left, clip.top, clip.right, clip.bottom), (50, 100, 150, 120));
        target.fill_rect_alpha(40, 40, 140, 100, 80, 100, 120, 128);
        target.fill_rect(50, 100, 100, 20, 4, 5, 6);
    });
    for y in 0..200 { for x in 0..320 {
        assert_eq!(pixels[y * 320 + x], if (50..150).contains(&x) && (100..120).contains(&y) {
            0x060504
        } else { prior[y * 320 + x] });
    } }
    pixels.fill(0x302010);
    display.render_clip = None;
    display.retained_window(1, bounds, |_| panic!("updated cache must remain valid"));
    let partial_result = pixels.clone();
    pixels.fill(0x302010);
    retained_windows::invalidate();
    display.retained_window(1, bounds, |target| {
        target.fill_rect_alpha(40, 40, 140, 100, 80, 100, 120, 128);
        target.fill_rect(50, 100, 100, 20, 4, 5, 6);
    });
    assert_eq!(pixels, partial_result);
    pixels.fill(0x302010);
    display.retained_window(1, (60, 50, 140, 100), |_| panic!("translation must reuse patched cache"));
    assert_eq!(pixels[110 * 320 + 70], 0x060504);
}
