use std::fs;

mod boot_info {
    pub struct BootInfo;
}

mod output {
    // ------------------------=
    // FUNC: write
    // DESC: Provides the inert host-side serial boundary required by the crash module.
    // ------------------=
    pub unsafe fn write(_bytes: &[u8]) {}

    // ------------------------=
    // FUNC: quiesce
    // DESC: Provides the inert host-side interrupt boundary required by the crash module.
    // ------------------=
    pub fn quiesce() {}

    // ------------------------=
    // FUNC: idle
    // DESC: Prevents an accidental host invocation of the kernel-only fatal stop loop.
    // ------------------=
    pub fn idle() -> ! {
        panic!("kernel idle is unavailable in host behavior tests")
    }
}

mod bootstrap {
    // ------------------------=
    // FUNC: show_fatal_crash
    // DESC: Provides the host-side display boundary required to compile the exact crash model.
    // ------------------=
    pub fn show_fatal_crash(
        _info: &crate::boot_info::BootInfo,
        _report: &crate::crash::CrashReport,
    ) {
    }
}

#[path = "../kernel/core/crash.rs"]
mod crash;
#[path = "../kernel/ui/crash_layout.rs"]
mod crash_layout;

// ------------------------=
// FUNC: le16
// DESC: Decodes one little-endian sixteen-bit field from the packaged bitmap.
// ------------------=
fn le16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

// ------------------------=
// FUNC: le32
// DESC: Decodes one little-endian thirty-two-bit field from the packaged bitmap.
// ------------------=
fn le32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

// ------------------------=
// FUNC: main
// DESC: Exercises first-failure capture and validates the rendered emblem's pixel behavior.
// ------------------=
fn main() {
    for (width, height, title_scale) in [
        (1024usize, 768usize, 1usize),
        (1920, 1080, 2),
        (2560, 1440, 2),
    ] {
        let layout = crash_layout::CrashLayout::new(width, height, title_scale);
        assert!(layout.flag_top + layout.flag_size < layout.title_y);
        assert!(layout.title_y < layout.panel_top);
        assert!(layout.panel_top + layout.panel_height <= height);
        assert!(layout.content_width + layout.gutter * 2 <= width);
        assert!(layout.panel_height >= 360);
    }

    let oversized_summary = [0x5au8; 400];
    let first = crash::CrashReport::new(
        crash::CrashReason::KernelPanic,
        crash::CrashPhase::Runtime,
        47,
        9,
        &oversized_summary,
    );
    assert_eq!(first.code, 0x1001);
    assert_eq!(first.phase, crash::CrashPhase::Runtime);
    assert_eq!(first.line, 47);
    assert_eq!(first.column, 9);
    assert_eq!(first.summary_len(), 192);

    let second = crash::CrashReport::new(
        crash::CrashReason::ProcessorFault,
        crash::CrashPhase::Drivers,
        0,
        0,
        &[0x33; 8],
    );
    assert_ne!(first.fingerprint, second.fingerprint);
    let mut capture = crash::CrashCapture::new();
    assert!(capture.capture(first));
    assert!(!capture.capture(second));
    let retained = capture
        .report()
        .expect("first failure must remain available");
    assert_eq!(retained.code, 0x1001);
    assert_eq!(retained.fingerprint, first.fingerprint);

    let bitmap = fs::read("assets/crash/infinity-fatal-pirate-flag-v1.bmp")
        .expect("fatal emblem must be readable");
    assert_eq!(&bitmap[0..2], b"BM");
    assert_eq!(le16(&bitmap, 28), 32);
    assert_eq!(le32(&bitmap, 18), 512);
    assert_eq!(le32(&bitmap, 22) as i32, -512);
    let pixel_offset = le32(&bitmap, 10) as usize;
    let pixels = bitmap[pixel_offset..].chunks_exact(4);
    let mut visible = 0usize;
    let mut transparent = 0usize;
    let mut maximum_chroma = 0u8;
    for pixel in pixels {
        let alpha = pixel[3];
        if alpha <= 16 {
            transparent += 1;
            continue;
        }
        visible += 1;
        let low = pixel[0].min(pixel[1]).min(pixel[2]);
        let high = pixel[0].max(pixel[1]).max(pixel[2]);
        maximum_chroma = maximum_chroma.max(high - low);
    }
    assert!(visible > 120_000);
    assert!(transparent > 100_000);
    assert!(maximum_chroma <= 20);
}
