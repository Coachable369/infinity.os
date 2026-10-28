//! Authored configuration geometry checked against actual installed Inter metrics.
#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"] mod ui;
#[path = "../kernel/runtime/font_faces.rs"] mod font_faces;
use ui::installer_template::{InstallerTemplate, InstallerTemplateRole as Role};

// ------------------------=
// FUNC: scaled
// DESC: Independently rounds font units into physical pixels for layout assertions.
// ------------------=
fn scaled(value: usize, pixels: usize) -> usize { (value * pixels + 12) / 24 }

// ------------------------=
// FUNC: width
// DESC: Measures saved copy using installed Inter advances and pair adjustments rather than approximate character counts.
// ------------------=
fn width(text: &[u8], pixels: usize, bold: bool) -> usize {
    let (_, metrics, kern) = font_faces::face(1, bold);
    let mut width = 0usize; let mut previous = None;
    for &byte in text {
        if byte == b'\n' { break; }
        if !(32..=126).contains(&byte) { previous = None; continue; }
        if let Some(previous) = previous {
            let delta = kern[(previous - 32) as usize * 95 + (byte - 32) as usize] as i32 - 128;
            let magnitude = scaled(delta.unsigned_abs() as usize, pixels);
            width = if delta < 0 { width.saturating_sub(magnitude) } else { width + magnitude };
        }
        width += scaled(metrics[(byte - 32) as usize] as usize, pixels); previous = Some(byte);
    }
    width
}

// ------------------------=
// FUNC: wrapped_lines
// DESC: Counts whole-word paragraph lines from actual advances, allowing authored hard line breaks without accepting hidden overflow.
// ------------------=
fn wrapped_lines(text: &[u8], pixels: usize, bold: bool, available: usize) -> usize {
    let mut lines = 0;
    for paragraph in text.split(|byte| *byte == b'\n') {
        if paragraph.is_empty() { lines += 1; continue; }
        let mut start = 0;
        while start < paragraph.len() {
            while start < paragraph.len() && paragraph[start] == b' ' { start += 1; }
            if start == paragraph.len() { break; }
            let mut end = start + 1; let mut space = None;
            while end <= paragraph.len() {
                if width(&paragraph[start..end], pixels, bold) > available {
                    end = space.unwrap_or(end.saturating_sub(1).max(start + 1)); break;
                }
                if end < paragraph.len() && paragraph[end] == b' ' { space = Some(end); }
                if end == paragraph.len() { break; } end += 1;
            }
            assert!(width(&paragraph[start..end], pixels, bold) <= available,
                "line {:?}: width {}, available {available}, font {pixels}",
                String::from_utf8_lossy(&paragraph[start..end]), width(&paragraph[start..end], pixels, bold));
            start = end; lines += 1;
        }
    }
    lines
}

// ------------------------=
// FUNC: main
// DESC: Checks all configuration steps across supported desktop aspects for readable primary typography and complete bounded labels.
// ------------------=
fn main() {
    let template = InstallerTemplate::parse(ui::installer_layout::CONFIGURATION_TEMPLATE_BYTES).unwrap();
    for (display_width, display_height) in [(1024,768), (1280,720), (1440,900), (1920,1080), (2560,1440)] {
        let content = ui::installer_layout::configuration_template_rect(6, Role::Content,
            display_width, display_height).unwrap();
        let primary = ui::installer_layout::configuration_template_rect(6, Role::PrimaryButton,
            display_width, display_height).unwrap();
        for index in 0..3 {
            let row = ui::installer_layout::configuration_network_row_rect(index,
                display_width, display_height).unwrap();
            assert!(row.left >= content.left && row.right() <= content.right());
            assert!(row.top >= content.top && row.bottom() <= content.bottom());
            assert!(row.bottom() < primary.top);
            let unit = |size: usize| (size * display_height / 1000).max(1);
            let detail_pixels = unit(18);
            let detail_width = row.width.saturating_sub(unit(46) + unit(12));
            for detail in [b"Connected wireless link detected".as_slice(),
                b"Set up networking later in Settings", b"No wireless adapter detected"] {
                let lines = wrapped_lines(detail, detail_pixels, false, detail_width);
                assert!(lines <= 2);
                assert!(scaled(28,detail_pixels) + lines.saturating_sub(1) * scaled(32,detail_pixels)
                    <= row.height.saturating_sub(unit(38)));
            }
        }
        for screen in 1..=template.screen_count() as u8 {
            for index in 0..template.element_count(screen).unwrap() {
                let element = template.element_at(screen,index).unwrap();
                if element.hidden || element.text.is_empty() || !matches!(element.kind,3|5) { continue; }
                let rect = ui::installer_layout::scale_template_rect(element.frame,display_width,display_height);
                let bold = element.role == Role::Title as u8 || element.kind == 5;
                let mut pixels = (element.font_size as usize * display_height / 1000).max(6);
                if element.role == Role::Title as u8 && !element.text.contains(&b'\n')
                    && rect.height < scaled(28,pixels)+scaled(32,pixels) {
                    while pixels > 6 && width(element.text,pixels,bold) > rect.width { pixels -= 1; }
                }
                if element.role == Role::Body as u8 { assert!(pixels >= 15); }
                if element.kind == 5 { assert!(pixels >= 17); }
                let lines = if element.kind == 5 {
                    assert!(width(element.text,pixels,bold) <= rect.width, "button {screen}:{index} at {display_width}x{display_height}"); 1
                } else { wrapped_lines(element.text,pixels,bold,rect.width) };
                let required = scaled(28,pixels) + lines.saturating_sub(1) * scaled(32,pixels);
                assert!(required <= rect.height, "text {screen}:{index} at {display_width}x{display_height}: requires {required}, has {}",rect.height);
            }
        }
    }
}
