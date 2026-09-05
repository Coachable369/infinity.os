//! Minimal emergency framebuffer scene used after unrecoverable kernel failures.

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::DisplayDevice;
use crate::crash::CrashReport;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const PIRATE_FLAG_BMP: &[u8] =
    include_bytes!("../../../assets/crash/infinity-fatal-pirate-flag-v1.bmp");

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const RED: (u8, u8, u8) = (255, 28, 48);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const WHITE: (u8, u8, u8) = (250, 252, 255);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const MUTED: (u8, u8, u8) = (174, 181, 192);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const PANEL: (u8, u8, u8) = (9, 10, 13);

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: show_fatal_crash
// DESC: Replaces every visible pixel with a deterministic allocation-free fatal report.
// ------------------=
pub fn show_fatal_crash(info: &crate::boot_info::BootInfo, report: &CrashReport) {
    let Some(mut display) = DisplayDevice::from_boot_info(info) else {
        return;
    };
    display.fill_rect(0, 0, display.width, display.height, 0, 0, 0);
    let scale = 1;
    let title_scale = if display.width >= 1400 { 2 } else { 1 };
    let layout =
        crate::ui::crash_layout::CrashLayout::new(display.width, display.height, title_scale);

    display.paint_bitmap_alpha_fit_rect(
        PIRATE_FLAG_BMP,
        layout.flag_left,
        layout.flag_top,
        layout.flag_size,
        layout.flag_size,
    );
    crash_text_centered_strong(
        &mut display,
        layout.gutter,
        layout.content_width,
        layout.title_y,
        b"INFINITYOS HAS STOPPED",
        WHITE,
        title_scale,
    );
    display.fill_rounded_rect_alpha(
        layout.gutter,
        layout.panel_top,
        layout.content_width,
        layout.panel_height,
        18,
        PANEL.0,
        PANEL.1,
        PANEL.2,
        248,
    );
    display.outline_rounded_rect(
        layout.gutter,
        layout.panel_top,
        layout.content_width,
        layout.panel_height,
        18,
        RED.0,
        RED.1,
        RED.2,
    );
    let inner_x = layout.gutter + 34 * scale;
    let inner_width = layout.content_width.saturating_sub(68 * scale);
    let panel_header_y = layout.panel_top + 24 * scale;
    crash_text_strong(
        &mut display,
        inner_x,
        panel_header_y,
        b"FATAL SYSTEM REPORT",
        RED,
        1,
    );
    let classification_width = crash_text_width(&display, report.reason.label(), 1, true);
    let classification_left =
        inner_x + inner_width.saturating_sub(classification_width + 28 * scale);
    display.fill_rounded_rect_alpha(
        classification_left.saturating_sub(14 * scale),
        panel_header_y.saturating_sub(8 * scale),
        classification_width + 28 * scale,
        32 * scale,
        10 * scale,
        45,
        7,
        13,
        255,
    );
    display.outline_rounded_rect(
        classification_left.saturating_sub(14 * scale),
        panel_header_y.saturating_sub(8 * scale),
        classification_width + 28 * scale,
        32 * scale,
        10 * scale,
        RED.0,
        RED.1,
        RED.2,
    );
    crash_text_strong(
        &mut display,
        classification_left,
        panel_header_y,
        report.reason.label(),
        RED,
        1,
    );

    let header_divider_y = layout.panel_top + 62 * scale;
    display.fill_rect(inner_x, header_divider_y, inner_width, 1, 94, 20, 29);
    let body_y = header_divider_y + 26 * scale;
    let split_x = inner_x + inner_width * 61 / 100;
    let column_gap = 28 * scale;
    let left_width = split_x.saturating_sub(inner_x + column_gap);
    display.fill_rect(
        split_x,
        body_y,
        1,
        layout.panel_height.saturating_sub(166 * scale),
        73,
        23,
        31,
    );

    crash_text_strong(&mut display, inner_x, body_y, b"WHAT HAPPENED", RED, 1);
    crash_text_wrapped(
        &mut display,
        inner_x,
        body_y + 34 * scale,
        left_width,
        report.reason.description(),
        WHITE,
        2,
    );
    let summary_y = body_y + 118 * scale;
    crash_text_strong(
        &mut display,
        inner_x,
        summary_y,
        b"DIAGNOSTIC SUMMARY",
        RED,
        1,
    );
    crash_text_wrapped(
        &mut display,
        inner_x,
        summary_y + 34 * scale,
        left_width,
        report.summary(),
        WHITE,
        4,
    );

    let details_x = split_x + column_gap;
    let details_value_x = details_x + 142 * scale;
    let mut y = body_y;
    crash_text_strong(&mut display, details_x, y, b"INCIDENT DETAILS", RED, 1);
    y += 42 * scale;
    crash_text(&mut display, details_x, y, b"PHASE", MUTED, 1);
    crash_text(
        &mut display,
        details_value_x,
        y,
        report.phase.label(),
        WHITE,
        1,
    );
    y += 31 * scale;
    crash_text(&mut display, details_x, y, b"STOP CODE", MUTED, 1);
    let code = hexadecimal(report.code as u64, 8);
    crash_text(&mut display, details_value_x, y, code.as_slice(), WHITE, 1);
    y += 31 * scale;
    crash_text(&mut display, details_x, y, b"FINGERPRINT", MUTED, 1);
    let fingerprint = hexadecimal(report.fingerprint, 16);
    crash_text(
        &mut display,
        details_value_x,
        y,
        fingerprint.as_slice(),
        WHITE,
        1,
    );
    if report.line != 0 {
        y += 31 * scale;
        crash_text(&mut display, details_x, y, b"SOURCE", MUTED, 1);
        let location = decimal_pair(report.line, report.column);
        crash_text(
            &mut display,
            details_value_x,
            y,
            location.as_slice(),
            WHITE,
            1,
        );
    }

    let footer_y = layout.panel_top + layout.panel_height.saturating_sub(48 * scale);
    display.fill_rect(
        inner_x,
        footer_y.saturating_sub(18 * scale),
        inner_width,
        1,
        94,
        20,
        29,
    );
    crash_text(
        &mut display,
        inner_x,
        footer_y,
        b"RESTART THE MACHINE. RECORD THE STOP CODE AND FINGERPRINT IF THE FAILURE REPEATS.",
        MUTED,
        1,
    );
    display.force_full_present();
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: show_fatal_crash
// DESC: Leaves legacy BIOS failures on the already emitted serial diagnostic because no framebuffer is available.
// ------------------=
pub fn show_fatal_crash(_info: &crate::boot_info::BootInfo, _report: &CrashReport) {}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: crash_text
// DESC: Draws anti-aliased regular emergency text using the bundled system typeface.
// ------------------=
fn crash_text(
    display: &mut DisplayDevice,
    x: usize,
    y: usize,
    text: &[u8],
    color: (u8, u8, u8),
    scale: usize,
) {
    display.ui_text(x, y, text, color.0, color.1, color.2, scale);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: crash_text_strong
// DESC: Draws anti-aliased semibold emergency headings with the system typeface.
// ------------------=
fn crash_text_strong(
    display: &mut DisplayDevice,
    x: usize,
    y: usize,
    text: &[u8],
    color: (u8, u8, u8),
    scale: usize,
) {
    display.ui_text_strong(x, y, text, color.0, color.1, color.2, scale);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: crash_text_centered_strong
// DESC: Centers one fatal-screen heading inside a bounded horizontal region.
// ------------------=
fn crash_text_centered_strong(
    display: &mut DisplayDevice,
    left: usize,
    width: usize,
    y: usize,
    text: &[u8],
    color: (u8, u8, u8),
    scale: usize,
) {
    let measured = display.ui_text_width_weighted(text, scale, true);
    crash_text_strong(
        display,
        left + width.saturating_sub(measured) / 2,
        y,
        text,
        color,
        scale,
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: crash_text_width
// DESC: Measures emergency text using the active architecture's actual crash-screen typeface.
// ------------------=
fn crash_text_width(display: &DisplayDevice, text: &[u8], scale: usize, strong: bool) -> usize {
    display.ui_text_width_weighted(text, scale, strong)
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: crash_text_wrapped
// DESC: Wraps emergency prose at word boundaries within a fixed-width region.
// ------------------=
fn crash_text_wrapped(
    display: &mut DisplayDevice,
    x: usize,
    y: usize,
    width: usize,
    text: &[u8],
    color: (u8, u8, u8),
    max_lines: usize,
) {
    display.ui_text_wrapped(x, y, width, text, color.0, color.1, color.2, max_lines);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: hexadecimal
// DESC: Formats a fixed-width uppercase hexadecimal value without allocation.
// ------------------=
fn hexadecimal(mut value: u64, digits: usize) -> HexText {
    let mut output = HexText {
        bytes: [b'0'; 18],
        len: 2 + digits.min(16),
    };
    output.bytes[0] = b'0';
    output.bytes[1] = b'x';
    let digits = digits.min(16);
    for index in 0..digits {
        let nibble = (value & 0x0f) as u8;
        output.bytes[1 + digits - index] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'A' + nibble - 10
        };
        value >>= 4;
    }
    output
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
struct HexText {
    bytes: [u8; 18],
    len: usize,
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
impl HexText {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: as_slice
    // DESC: Returns the populated portion of a hexadecimal projection.
    // ------------------=
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
struct DecimalPair {
    bytes: [u8; 24],
    len: usize,
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
impl DecimalPair {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: as_slice
    // DESC: Returns the populated portion of a source-location projection.
    // ------------------=
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: decimal_pair
// DESC: Formats a line and column pair without heap allocation.
// ------------------=
fn decimal_pair(line: u32, column: u32) -> DecimalPair {
    let mut output = DecimalPair {
        bytes: [0; 24],
        len: 0,
    };
    output.len += write_decimal(line, &mut output.bytes[output.len..]);
    output.bytes[output.len] = b':';
    output.len += 1;
    output.len += write_decimal(column, &mut output.bytes[output.len..]);
    output
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: write_decimal
// DESC: Writes one unsigned decimal number into a caller-owned emergency buffer.
// ------------------=
fn write_decimal(mut value: u32, destination: &mut [u8]) -> usize {
    let mut reversed = [0u8; 10];
    let mut count = 0usize;
    loop {
        reversed[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for index in 0..count.min(destination.len()) {
        destination[index] = reversed[count - index - 1];
    }
    count.min(destination.len())
}
