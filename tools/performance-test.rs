#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "storage.rs"]
mod storage;
#[path = "../kernel/ui/mod.rs"]
mod ui;
use std::time::Instant;
use ui::compositor::{SoftwareCompositor, SurfaceFrame};
use ui::geometry::{Rect, Size};
use ui::scene::DamageTracker;
use ui::surface::{PixelFormat, SurfaceRegistry, SurfaceSecurityClass};
use ui::window::{ContextId, ZOrderClass};

// ------------------------=
// FUNC: output_text
// DESC: Supplies the host runtime diagnostic sink.
// ------------------=
fn output_text(_: &[u8]) {}

// ------------------------=
// FUNC: main
// DESC: Measures actual retained composition and verifies protected staging and offscreen sampling.
// ------------------=
fn main() {
    telemetry_history();
    damage_merging();
    let size = Size {
        width: 640,
        height: 480,
    };
    let mut registry = SurfaceRegistry::new(4 * 1024 * 1024);
    let id = registry
        .create(
            ContextId(1),
            Size {
                width: 320,
                height: 240,
            },
            PixelFormat::Xrgb8888,
            SurfaceSecurityClass::Application,
            false,
        )
        .unwrap();
    let descriptor = *registry.inspect(id).unwrap();
    let pixels: Vec<u32> = (0..320 * 240).map(|x| 0xff000000 | x as u32).collect();
    let mut layer = SurfaceFrame {
        descriptor,
        pixels: &pixels,
        bounds: Rect {
            x: 0,
            y: 0,
            width: 320,
            height: 240,
        },
        opacity: 255,
        z_class: ZOrderClass::Normal,
        visible: true,
    };
    let mut compositor = SoftwareCompositor::new(size, 640, 0xff010203);
    let mut back = vec![0xff010203u32; 640 * 480];
    let mut front = back.clone();
    let mut damage = DamageTracker::new();
    let mut durations = Vec::new();
    for frame in 0..300 {
        damage.begin_frame();
        let old = layer.bounds;
        layer.bounds.x = frame % 200;
        damage.add(old);
        damage.add(layer.bounds);
        let start = Instant::now();
        compositor.compose(&mut back, &[layer], &damage).unwrap();
        compositor.present(&mut front, &back, &damage).unwrap();
        durations.push(start.elapsed().as_nanos() as u64);
        assert_eq!(
            front[0],
            if layer.bounds.x == 0 {
                pixels[0]
            } else {
                0xff010203
            }
        );
        assert_eq!(layer.descriptor.content_generation, 0);
        assert!(
            damage
                .regions()
                .iter()
                .map(|rect| u64::from(rect.width) * u64::from(rect.height))
                .sum::<u64>()
                < 640 * 480
        );
        for y in 0..480usize {
            for x in 0..640usize {
                let expected =
                    if y < 240 && x >= layer.bounds.x as usize && x < layer.bounds.x as usize + 320
                    {
                        pixels[y * 320 + x - layer.bounds.x as usize]
                    } else {
                        0xff010203
                    };
                assert_eq!(front[y * 640 + x], expected, "frame {frame}, pixel {x},{y}");
            }
        }
    }
    durations.sort_unstable();
    println!("{{\"fixture\":\"retained_drag\",\"frames\":300,\"average_ns\":{},\"p95_ns\":{},\"worst_ns\":{},\"pixels\":{}}}",
        durations.iter().sum::<u64>() / 300, durations[284], durations[299], compositor.metrics().presented_pixels);
    if std::env::args().any(|arg| arg == "--baseline") {
        return;
    }
    layer.bounds.x = -2;
    damage.begin_frame();
    damage.add(Rect {
        x: 0,
        y: 0,
        width: 640,
        height: 480,
    });
    compositor.compose(&mut back, &[layer], &damage).unwrap();
    assert_eq!(back[0], pixels[2]);
    let saved = back.clone();
    layer.z_class = ZOrderClass::Trusted;
    assert!(compositor.compose(&mut back, &[layer], &damage).is_err());
    assert_eq!(back, saved);
    compositor.present(&mut front, &back, &damage).unwrap();
    layer.z_class = ZOrderClass::Normal;
    compositor.compose(&mut back, &[layer], &damage).unwrap();
    assert!(compositor.compose(&mut [], &[layer], &damage).is_err());
    front.fill(0);
    compositor.present(&mut front, &back, &damage).unwrap();
    assert!(front.iter().all(|pixel| *pixel == 0));
    compositor.compose(&mut back, &[layer], &damage).unwrap();
    assert_eq!(
        compositor.present_prioritized(&mut front, &back, &damage, 0, &mut []),
        Err(ui::compositor::CompositorError::DeferredBufferTooSmall)
    );
    assert!(front.iter().all(|pixel| *pixel == 0));
    compositor.present(&mut front, &back, &damage).unwrap();
    assert_eq!(front, back);
    layer.descriptor.format = PixelFormat::Argb8888;
    layer.opacity = 128;
    compositor.compose(&mut back, &[layer], &damage).unwrap();
    let source = pixels[2];
    let background = 0xff010203u32;
    let mut expected = 0xff000000;
    for shift in [0, 8, 16] {
        expected |= ((((source >> shift) & 255) * 128 + ((background >> shift) & 255) * 127) / 255)
            << shift;
    }
    assert_eq!(back[0], expected);
}

// ------------------------=
// FUNC: telemetry_history
// DESC: Verifies bounded retention, measured statistics, expiration, and unavailable-clock behavior.
// ------------------=
fn telemetry_history() {
    use ui::performance::*;
    let mut history = PerformanceHistory::new();
    assert_eq!(history.summary().average_ns, None);
    for duration in 1..=100 {
        history.record(FramePerformanceSample {
            frame_ns: Some(duration),
            timestamp_ns: Some(0),
            budget_ns: 50,
            ..Default::default()
        });
    }
    let summary = history.summary();
    assert_eq!(summary.average_ns, Some(50));
    assert_eq!(summary.p95_ns, Some(95));
    assert_eq!(summary.worst_ns, Some(100));
    assert_eq!(history.aggregate(0).unwrap().missed, 50);
    for second in 1..=FRAME_HISTORY as u64 {
        history.record(FramePerformanceSample {
            timestamp_ns: Some(second * 1_000_000_000),
            ..Default::default()
        });
    }
    assert!(history.aggregate(0).is_none());
    assert_eq!(history.aggregate(FRAME_HISTORY as u64).unwrap().samples, 1);
    assert_eq!(history.summary().samples, 60);
    assert_eq!(history.summary().overwritten_samples, 100);
    assert_eq!(history.summary().average_ns, None);
    assert_eq!(history.recent(FRAME_HISTORY), None);
    assert_eq!(elapsed(Some(12), Some(11)), None);
    assert_eq!(elapsed(None, Some(11)), None);
    assert_eq!(ticks_to_ns(24_000_000, 24_000_000), Some(1_000_000_000));
    assert_eq!(ticks_to_ns(1, 0), None);
}

// ------------------------=
// FUNC: damage_merging
// DESC: Tests transitive overlap merging and bounded flood behavior without privilege promotion.
// ------------------=
fn damage_merging() {
    use ui::scene::DamageClass;
    let mut damage = DamageTracker::new();
    for x in [0, 18, 9] {
        damage.add(Rect {
            x,
            y: 0,
            width: 10,
            height: 10,
        });
    }
    assert_eq!(damage.regions().len(), 1);
    assert_eq!(damage.regions()[0].width, 28);
    damage.begin_frame();
    for x in 0..1000 {
        damage.add_semantic(
            Rect {
                x: x * 3,
                y: 0,
                width: 1,
                height: 1,
            },
            DamageClass::Content,
            1,
            20,
        );
    }
    assert!(damage.regions().len() <= ui::scene::MAX_DAMAGE_REGIONS);
    assert!(damage.records().iter().all(|record| record.priority == 20));
}
