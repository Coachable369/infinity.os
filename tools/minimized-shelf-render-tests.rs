//! Runs the production shelf, glass recipe, bitmap and font rasterizers against a host-owned framebuffer.
use super::*;
use crate::primitives::UI_FONT_CELL_HEIGHT;
#[path = "../kernel/core/bootstrap/holographic_card.rs"]
mod holographic_card;
use holographic_card::paint_card;

#[test]
// ------------------------=
// FUNC: pill_text_is_centered_and_density_stable
// DESC: Measures rendered glyph ink and compares exact capsule pixels across display-density boundaries.
// ------------------=
fn pill_text_is_centered_and_density_stable() {
    let mut reference = None;
    for (width, height) in [(1280,720),(2560,1440),(3840,2160)] {
        let mut pixels = vec![0x102336u32; width * height];
        let mut d = DisplayDevice { buffer:pixels.as_mut_ptr(),width,height,stride:width,
            format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false };
        d.glass_label_pill(40,40,240,44,b"File Navigator",true);
        let mut capsule = Vec::new();
        let (mut x0,mut y0,mut x1,mut y1) = (usize::MAX,usize::MAX,0,0);
        for y in 40..84 {
            for x in 40..280 {
                let p = pixels[y * width + x];
                capsule.push(p);
                if p & 255 > 200 && (p >> 8) & 255 > 215 && (p >> 16) & 255 > 225 {
                    x0=x0.min(x);x1=x1.max(x);y0=y0.min(y);y1=y1.max(y);
                }
            }
        }
        assert!(x0 < x1 && y0 < y1);
        assert!(((x0+x1) as isize - 320).abs() <= 6, "horizontal ink: {x0}..{x1}");
        assert!(((y0+y1) as isize - 124).abs() <= 6, "vertical ink: {y0}..{y1}");
        if let Some(expected) = &reference { assert_eq!(&capsule,expected); }
        else { reference=Some(capsule); }
    }
}
use crate::ui::spatial::{overview_bounds, Preview};
#[path = "../kernel/core/bootstrap/spatial_carousel.rs"]
mod carousel_renderer;

impl DisplayDevice {
    // ------------------------=
    // FUNC: set_render_clip
    // DESC: Supplies the host framebuffer clipping boundary used by the production sprite renderer.
    // ------------------=
    fn set_render_clip(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.render_clip = Some(Region {left:x.min(self.width), top:y.min(self.height),
            right:x.saturating_add(width).min(self.width), bottom:y.saturating_add(height).min(self.height)});
    }
}

#[test]
// ------------------------=
// FUNC: interrupted_carousel_cache_matches_fresh_frame
// DESC: Changing the origin geometry with the same destination must match a freshly prepared carousel.
// ------------------=
fn interrupted_carousel_cache_matches_fresh_frame() {
    use crate::ui::spatial::OVERVIEW_COUNT;
    let (width, height) = (960, 640);
    let background = vec![0x102336u32; width * height];
    let mut pixels = background.clone();
    let mut d = DisplayDevice {buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    let mut previews = [Preview::EMPTY; 3];
    for (i, p) in previews.iter_mut().enumerate() { p.app = i as u8; }
    let mut origin = [(0,0,0,0); OVERVIEW_COUNT];
    for i in 0..3 { origin[i] = overview_bounds(i, 0, 0, 3); }
    carousel_renderer::invalidate();
    assert!(carousel_renderer::paint(&mut d, &previews, &origin, 1, 0, 120));
    for i in 0..3 { origin[i] = overview_bounds(i, 2, 255, 3); }
    pixels.copy_from_slice(&background);
    assert!(carousel_renderer::paint(&mut d, &previews, &origin, 1, 0, 120));
    let reused = pixels.clone();
    carousel_renderer::invalidate();
    pixels.copy_from_slice(&background);
    assert!(carousel_renderer::paint(&mut d, &previews, &origin, 1, 0, 120));
    assert_eq!(pixels, reused);
}

#[test]
// ------------------------=
// FUNC: holographic_glass_cards_render_on_isolated_stage
// DESC: Exercises production card pixels and writes a native visual proof without mock application contents.
// ------------------=
fn holographic_glass_cards_render_on_isolated_stage() {
    let (width,height)=(1440,900);
    let mut pixels=vec![0u32;width*height];
    let mut d=DisplayDevice {buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    d.paint_bitmap_cover_rect(include_bytes!("../assets/desktop/spatial-world-2.bmp"),0,0,width,height);
    let before=pixels.clone();
    for (app,p,selected) in [(0,(150,280,330,290),false),(4,(960,280,330,290),false),(2,(470,235,500,395),true)] {
        let mut preview=crate::ui::spatial::Preview::EMPTY; preview.app=app;
        holographic_card::paint_card(&mut d,&preview,p,selected);
    }
    assert_ne!(pixels,before);
    assert_eq!(&pixels[..width*200],&before[..width*200]);
    use std::io::Write;
    let mut file=std::io::BufWriter::new(std::fs::File::create("build/holographic-glass-proof.ppm").unwrap());
    write!(file,"P6\n{width} {height}\n255\n").unwrap();
    for pixel in pixels { file.write_all(&[(pixel&255)as u8,((pixel>>8)&255)as u8,((pixel>>16)&255)as u8]).unwrap(); }
}
use crate::ui::app_launcher::minimized_shelf::{self as shelf, Geometry, State};
#[path = "../kernel/core/bootstrap/glass.rs"]
mod glass;
#[path = "../kernel/core/bootstrap/minimized_shelf.rs"]
mod shelf_renderer;
#[path = "../kernel/core/bootstrap/desktop_widget_menu.rs"]
mod widget_menu_renderer;
#[path = "../kernel/core/bootstrap/app_shortcuts.rs"]
mod app_shortcuts_renderer;

#[test]
// ------------------------=
// FUNC: shortcut_drag_partial_pixels_match_full_composition
// DESC: Pixel-compares production themed shortcut drags, release and cancellation with full reference frames.
// ------------------=
fn shortcut_drag_partial_pixels_match_full_composition() {
    use crate::ui::app_launcher::shortcuts::{self,State};
    let (width,height)=(1200,800);
    let background=vec![0x102336u32;width*height];
    let mut pixels=background.clone();
    let mut display=DisplayDevice {buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    shortcuts::publish(State::new());let _=shortcuts::take_changed();
    let mut state=State::new();state.place(11,100,150);
    for frame in 0..5 {
        if frame==1 {state.begin(11,100,150,true);state.motion(200,250);}
        if frame==2 {state.motion(400,350);}
        if frame==3 {state.drag=None;}
        if frame==4 {state.place(11,450,400);}
        shortcuts::publish(state);
        let (x,y,w,h)=shortcuts::take_damage(width,height).unwrap();
        assert!(w*h<width*height);
        let clip=Region{left:x,top:y,right:x+w,bottom:y+h};
        for row in y..y+h { let range=row*width+x..row*width+x+w;pixels[range.clone()].copy_from_slice(&background[range]); }
        display.render_clip=Some(clip);
        let p=state.stationary_positions()[11];
        if p != [0,0] {display.desktop_app_shortcut(11,width*p[0] as usize/1000,height*p[1] as usize/1000,false);}
        if state.drag.is_some() {display.desktop_app_shortcut(11,width*state.pointer[0] as usize/1000,height*state.pointer[1] as usize/1000,false);}
        display.render_clip=None;
        let mut expected=background.clone();let mut reference=display;reference.buffer=expected.as_mut_ptr();
        if p != [0,0] {reference.desktop_app_shortcut(11,width*p[0] as usize/1000,height*p[1] as usize/1000,false);}
        if state.drag.is_some() {reference.desktop_app_shortcut(11,width*state.pointer[0] as usize/1000,height*state.pointer[1] as usize/1000,false);}
        let mismatch=pixels.iter().zip(&expected).position(|(a,b)|a!=b);
        assert!(mismatch.is_none(),"shortcut damage mismatch in frame {frame}: {:?}, clip {:?}",mismatch.map(|i|(i%width,i/width)),(x,y,w,h));
        if frame==2 {
            use std::io::Write;
            let mut file=std::io::BufWriter::new(std::fs::File::create("build/shortcut-drag-proof.ppm").unwrap());
            write!(file,"P6\n{width} {height}\n255\n").unwrap();
            for pixel in &pixels {file.write_all(&[(*pixel&255) as u8,((*pixel>>8)&255) as u8,((*pixel>>16)&255) as u8]).unwrap();}
        }
    }
    shortcuts::publish(State::new());let _=shortcuts::take_changed();
}

#[test]
// ------------------------=
// FUNC: widget_chooser_partial_pixels_match_full_composition
// DESC: Exercises actual menu rasterization and opening/dismissal damage without stale checkmarks.
// ------------------=
fn widget_chooser_partial_pixels_match_full_composition() {
    use crate::ui::desktop_widgets::{self as widgets,State};
    let (width,height)=(800,600);
    let background=vec![0x102336u32;width*height];
    let mut pixels=background.clone();
    let mut display=DisplayDevice {buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    widgets::publish(State::new()); let _=widgets::take_damage(width,height,1);
    for position in [Some(crate::ui::geometry::Point{x:200,y:180}),Some(crate::ui::geometry::Point{x:780,y:590}),None] {
        let mut state=State::new(); state.menu=position; widgets::publish(state);
        let damage=widgets::take_damage(width,height,1).unwrap();
        let clip=Region{left:damage.x.max(0) as usize,top:damage.y.max(0) as usize,right:(damage.right() as usize).min(width),bottom:(damage.bottom() as usize).min(height)};
        for y in clip.top..clip.bottom { let range=y*width+clip.left..y*width+clip.right; pixels[range.clone()].copy_from_slice(&background[range]); }
        display.render_clip=Some(clip); display.desktop_widget_menu(1); display.render_clip=None;
        let mut expected=background.clone(); let mut reference=display; reference.buffer=expected.as_mut_ptr(); reference.desktop_widget_menu(1);
        assert!(pixels==expected,"chooser partial composition differs at {position:?}");
        if position.is_some_and(|p|p.x==200) {
            use std::io::Write;
            let mut file=std::io::BufWriter::new(std::fs::File::create("build/widget-menu-proof.ppm").unwrap());
            write!(file,"P6\n{width} {height}\n255\n").unwrap();
            for pixel in &pixels { file.write_all(&[(*pixel&255) as u8,((*pixel>>8)&255) as u8,((*pixel>>16)&255) as u8]).unwrap(); }
        }
    }
    let mut state=State::new(); state.positions[1]=[201,201]; widgets::publish(state);
    let layout=crate::ui::system_layout::SystemLayout::new(width,height);
    let chat=layout.ai_chat_geometry(false);
    let x=(chat.composer.x+chat.composer.width as i32/2)*1000/width as i32;
    let y=(chat.composer.y+chat.composer.height as i32/2)*1000/height as i32;
    assert!(layout.ai_chat_target(x,y,false).is_some());
    state.visible &= !2; widgets::publish(state);
    assert!(layout.ai_chat_target(x,y,false).is_none());
    widgets::publish(State::new()); let _=widgets::take_damage(width,height,1);
}

impl DisplayDevice {
    // ------------------------=
    // FUNC: skin_visual_mode
    // DESC: Supplies the default dark skin to the production glass recipe.
    // ------------------=
    fn skin_visual_mode(&self) -> u8 {
        0
    }
    // ------------------------=
    // FUNC: active_accent_surface
    // DESC: Uses the real default skin registry for semantic glass colors.
    // ------------------=
    fn active_accent_surface(&self, surface: crate::ui::skin::AccentSurface) -> (u8, u8, u8) {
        crate::ui::skin::SkinRegistry::new()
            .accent_surface(surface)
            .channels()
    }
    // ------------------------=
    // FUNC: launcher_icon
    // DESC: Draws the same installed high-resolution atlas cells as the production launcher.
    // ------------------=
    fn launcher_icon(&mut self, x: usize, y: usize, role: usize, size: usize) -> bool {
        let roles = [0usize, 1, 4, 8, 9, 10, 12, 19, 23, 25, 26, 28, 32, 49];
        let Some(cell) = roles.iter().position(|r| *r == role) else {
            return self.paint_bitmap_alpha_atlas_cell(
                include_bytes!("../assets/icons/runtime/aurora-harmony-base.bmp"),
                5,
                9,
                role,
                x.saturating_sub(size / 2),
                y.saturating_sub(size / 2),
                size,
            );
        };
        self.paint_bitmap_alpha_atlas_cell(
            include_bytes!("../assets/icons/runtime/aurora-harmony-launcher-256.bmp"),
            4,
            4,
            cell,
            x.saturating_sub(size / 2),
            y.saturating_sub(size / 2),
            size,
        )
    }
}

#[test]
// ------------------------=
// FUNC: shelf_partial_frames_match_complete_frames
// DESC: Pixel-compares every overlay transition with full composition, including menu dismissal and overflow.
// ------------------=
fn shelf_partial_frames_match_complete_frames() {
    for (width, height) in [(800, 600), (1440, 900), (2560, 1440)] {
        let mut pixels = vec![0u32; width * height];
        let mut display = DisplayDevice {
            buffer: pixels.as_mut_ptr(),
            width,
            height,
            stride: width,
            format: 0,
            render_clip: None,
            fast_motion_frame: false,
            submissions: 0,
            recording_surface: false,
        };
        display.paint_bitmap_cover_box(
            include_bytes!("../assets/desktop/infinity-default-dark-wallpaper-v2.bmp"),
            0,
            0,
            width,
            height,
        );
        let background = pixels.clone();
        shelf::publish(State::new());
        let _ = shelf::take_damage(width, height);
        display.minimized_app_shelf();
        for step in 0..11 {
            let mut state = State::new();
            for id in [1, shelf::COMMAND, shelf::EDITOR, shelf::SETTINGS] {
                state.set(id, true);
            }
            if step >= 1 && step <= 4 {
                state.menu = Some(shelf::COMMAND);
                state.row = Some((step - 1) % 3);
                state.hover = Some(shelf::COMMAND);
            }
            if step == 5 {
                state.hover = Some(shelf::EDITOR);
            }
            if step == 6 {
                for id in 0..shelf::COUNT {
                    state.set(id, true);
                }
                state.scroll(2, Geometry::new(width, height, state).capacity);
            }
            if step == 7 {
                state = State::new();
            }
            if step == 8 {
                state.left = true;
                state.menu = Some(shelf::COMMAND);
            }
            if step == 9 {
                state.floating = [401, 221];
            }
            if step == 10 {
                state.drag = Some((600, 300, 10, 10));
            }
            shelf::publish(state);
            if let Some(damage) = shelf::take_damage(width, height) {
                let clip = Region {
                    left: damage.x.max(0) as usize,
                    top: damage.y.max(0) as usize,
                    right: (damage.right().max(0) as usize).min(width),
                    bottom: (damage.bottom().max(0) as usize).min(height),
                };
                for y in clip.top..clip.bottom {
                    let range = y * width + clip.left..y * width + clip.right;
                    pixels[range.clone()].copy_from_slice(&background[range]);
                }
                display.render_clip = Some(clip);
                display.minimized_app_shelf();
                display.render_clip = None;
            }
            let mut expected = background.clone();
            let mut reference = display;
            reference.buffer = expected.as_mut_ptr();
            reference.minimized_app_shelf();
            assert!(
                pixels == expected,
                "incremental pixels differ at {width}x{height} step {step}"
            );
            if width == 1440 && matches!(step, 2 | 8 | 9) {
                use std::io::Write;
                let mut file = std::io::BufWriter::new(
                    std::fs::File::create(match step {
                        8 => "build/minimized-shelf-left.ppm",
                        9 => "build/minimized-shelf-floating.ppm",
                        _ => "build/minimized-shelf-render.ppm",
                    })
                    .unwrap(),
                );
                write!(file, "P6\n{width} {height}\n255\n").unwrap();
                for pixel in &pixels {
                    file.write_all(&[
                        (*pixel & 255) as u8,
                        ((*pixel >> 8) & 255) as u8,
                        ((*pixel >> 16) & 255) as u8,
                    ])
                    .unwrap();
                }
            }
        }
    }
}
