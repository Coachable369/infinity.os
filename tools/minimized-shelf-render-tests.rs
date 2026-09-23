//! Runs the production shelf, glass recipe, bitmap and font rasterizers against a host-owned framebuffer.
use super::*;
use crate::ui::app_launcher::minimized_shelf::{self as shelf, Geometry, State};
#[path = "../kernel/core/bootstrap/glass.rs"]
mod glass;
#[path = "../kernel/core/bootstrap/minimized_shelf.rs"]
mod shelf_renderer;

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
