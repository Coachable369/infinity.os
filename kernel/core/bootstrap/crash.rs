//! Minimal emergency framebuffer scene used after unrecoverable kernel failures.

use super::DisplayDevice;
use crate::crash::CrashReport;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const PIRATE_FLAG_BMP: &[u8] =
    include_bytes!("../../../assets/crash/infinity-fatal-pirate-flag-v1.bmp");

const RED: (u8, u8, u8) = (255, 28, 48);
const WHITE: (u8, u8, u8) = (250, 252, 255);
const MUTED: (u8, u8, u8) = (174, 181, 192);
const PANEL: (u8, u8, u8) = (9, 10, 13);

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
    let gutter = (display.width / 18).max(28);
    let content_width = display.width.saturating_sub(gutter * 2);
    let flag_size = (display.height * 28 / 100)
        .min(display.width * 22 / 100)
        .max(112);
    let flag_left = display.width.saturating_sub(flag_size) / 2;
    let flag_top = (display.height * 4 / 100).max(20);

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    display.paint_bitmap_alpha_fit_rect(PIRATE_FLAG_BMP, flag_left, flag_top, flag_size, flag_size);
    #[cfg(target_arch = "x86")]
    paint_legacy_pirate_flag(&mut display, flag_left, flag_top, flag_size);

    let title_y = flag_top + flag_size.saturating_sub(8);
    crash_text_centered_strong(
        &mut display,
        gutter,
        content_width,
        title_y,
        b"INFINITYOS HAS STOPPED",
        WHITE,
        title_scale,
    );
    let classification_y = title_y + 36 * scale;
    crash_text_centered_strong(
        &mut display,
        gutter,
        content_width,
        classification_y,
        report.reason.label(),
        RED,
        1,
    );

    let panel_top = classification_y + 46 * scale;
    let panel_height = display
        .height
        .saturating_sub(panel_top)
        .saturating_sub((display.height / 18).max(24));
    display.fill_rounded_rect_alpha(
        gutter,
        panel_top,
        content_width,
        panel_height,
        18,
        PANEL.0,
        PANEL.1,
        PANEL.2,
        248,
    );
    display.outline_rounded_rect(
        gutter,
        panel_top,
        content_width,
        panel_height,
        18,
        RED.0,
        RED.1,
        RED.2,
    );
    let inner_x = gutter + 34 * scale;
    let inner_width = content_width.saturating_sub(68 * scale);
    let mut y = panel_top + 26 * scale;
    crash_text_strong(&mut display, inner_x, y, b"WHAT HAPPENED", RED, 1);
    y += 32 * scale;
    crash_text_wrapped(
        &mut display,
        inner_x,
        y,
        inner_width,
        report.reason.description(),
        WHITE,
        2,
    );
    y += 70 * scale;
    display.fill_rect(inner_x, y, inner_width, 1, RED.0, RED.1, RED.2);
    y += 20 * scale;
    crash_text_strong(&mut display, inner_x, y, b"TECHNICAL DETAILS", RED, 1);
    y += 32 * scale;
    crash_text(&mut display, inner_x, y, b"PHASE", MUTED, 1);
    crash_text(
        &mut display,
        inner_x + 150 * scale,
        y,
        report.phase.label(),
        WHITE,
        1,
    );
    y += 31 * scale;
    crash_text(&mut display, inner_x, y, b"STOP CODE", MUTED, 1);
    let code = hexadecimal(report.code as u64, 8);
    crash_text(
        &mut display,
        inner_x + 150 * scale,
        y,
        code.as_slice(),
        WHITE,
        1,
    );
    y += 31 * scale;
    crash_text(&mut display, inner_x, y, b"FINGERPRINT", MUTED, 1);
    let fingerprint = hexadecimal(report.fingerprint, 16);
    crash_text(
        &mut display,
        inner_x + 150 * scale,
        y,
        fingerprint.as_slice(),
        WHITE,
        1,
    );
    if report.line != 0 {
        y += 31 * scale;
        crash_text(&mut display, inner_x, y, b"SOURCE", MUTED, 1);
        let location = decimal_pair(report.line, report.column);
        crash_text(
            &mut display,
            inner_x + 150 * scale,
            y,
            location.as_slice(),
            WHITE,
            1,
        );
    }
    y += 40 * scale;
    crash_text_strong(&mut display, inner_x, y, b"DIAGNOSTIC SUMMARY", RED, 1);
    y += 31 * scale;
    crash_text_wrapped(
        &mut display,
        inner_x,
        y,
        inner_width,
        report.summary(),
        WHITE,
        2,
    );

    let footer_y = panel_top + panel_height.saturating_sub(52 * scale);
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

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: crash_text
// DESC: Draws compact emergency text on the legacy compatibility target.
// ------------------=
fn crash_text(
    display: &mut DisplayDevice,
    x: usize,
    y: usize,
    text: &[u8],
    color: (u8, u8, u8),
    scale: usize,
) {
    display.text_scaled(
        x,
        y,
        text,
        color.0,
        color.1,
        color.2,
        scale.max(1),
        false,
    );
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

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: crash_text_strong
// DESC: Draws compact emergency headings on the legacy compatibility target.
// ------------------=
fn crash_text_strong(
    display: &mut DisplayDevice,
    x: usize,
    y: usize,
    text: &[u8],
    color: (u8, u8, u8),
    scale: usize,
) {
    display.text_scaled(
        x,
        y,
        text,
        color.0,
        color.1,
        color.2,
        scale.max(1),
        true,
    );
}

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
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    let measured = display.ui_text_width_weighted(text, scale, true);
    #[cfg(target_arch = "x86")]
    let measured = text.len().saturating_mul(9).saturating_mul(scale.max(1));
    crash_text_strong(
        display,
        left + width.saturating_sub(measured) / 2,
        y,
        text,
        color,
        scale,
    );
}

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
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        display.ui_text_wrapped(x, y, width, text, color.0, color.1, color.2, max_lines);
    }
    #[cfg(target_arch = "x86")]
    {
        let characters = (width / 9).max(1);
        for (line, chunk) in text.chunks(characters).take(max_lines).enumerate() {
            crash_text(display, x, y + line * 18, chunk, color, 1);
        }
    }
}

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

struct HexText {
    bytes: [u8; 18],
    len: usize,
}

impl HexText {
    // ------------------------=
    // FUNC: as_slice
    // DESC: Returns the populated portion of a hexadecimal projection.
    // ------------------=
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

struct DecimalPair {
    bytes: [u8; 24],
    len: usize,
}

impl DecimalPair {
    // ------------------------=
    // FUNC: as_slice
    // DESC: Returns the populated portion of a source-location projection.
    // ------------------=
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

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

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: paint_legacy_pirate_flag
// DESC: Draws a bounded monochrome flag silhouette when bitmap assets exceed the legacy image budget.
// ------------------=
fn paint_legacy_pirate_flag(display: &mut DisplayDevice, left: usize, top: usize, size: usize) {
    let pole_x = left + size / 5;
    display.fill_rect(pole_x, top + size / 8, 3, size * 3 / 4, 230, 234, 240);
    display.outline_rect(
        pole_x + 3,
        top + size / 8,
        size * 3 / 5,
        size / 2,
        230,
        234,
        240,
    );
    let center_x = pole_x + size * 3 / 10;
    let center_y = top + size * 3 / 8;
    display.outline_rect(
        center_x - size / 10,
        center_y - size / 10,
        size / 5,
        size / 5,
        255,
        255,
        255,
    );
    display.line(
        (center_x - size / 7) as i32,
        (center_y + size / 7) as i32,
        (center_x + size / 7) as i32,
        (center_y - size / 7) as i32,
        255,
        255,
        255,
    );
    display.line(
        (center_x - size / 7) as i32,
        (center_y - size / 7) as i32,
        (center_x + size / 7) as i32,
        (center_y + size / 7) as i32,
        255,
        255,
        255,
    );
}
