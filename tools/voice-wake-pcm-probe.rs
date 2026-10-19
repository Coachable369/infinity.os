//! Sends immutable synthetic speech through production calibrated VAD before native ASR.
#[path = "../kernel/runtime/ai/voice_vad.rs"] mod voice_vad;
#[path = "../kernel/runtime/ai/voice_noise_floor.rs"] mod voice_noise_floor;

// ------------------------=
// FUNC: main
// DESC: Emits three bounded PCM fixtures after actual calibrated segmentation, retaining measured onset and sample counts for native recognizer verification.
// ------------------=
fn main() {
    assert_eq!(std::env::var("INFINITY_BUILD_KIT_ACTIVE").as_deref(), Ok("1"));
    let args: Vec<_> = std::env::args_os().collect();
    assert_eq!(args.len(), 3);
    let bytes = std::fs::read(&args[1]).unwrap();
    assert_eq!(bytes.len() % 2, 0);
    let original: Vec<i16> = bytes.chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]])).collect();
    assert!(!original.is_empty() && original.len() < 8 * voice_vad::RATE);
    let onset = original.iter().position(|sample| sample.unsigned_abs() > 50)
        .expect("fixture has no speech above its ambient level");
    let out = std::path::Path::new(&args[2]);
    std::fs::create_dir_all(out).unwrap();
    for (name, divisor, soft_onset) in [("normal", 1i32, false), ("quiet", 4, false), ("soft-onset", 1, true)] {
        let mut floor = voice_noise_floor::NoiseFloor::new();
        floor.push(&[50; 8000]);
        assert!(floor.ready());
        let mut vad = voice_vad::Utterance::new(floor.threshold());
        vad.clear_with_continuation(floor.threshold(), floor.continuation_threshold());
        let mut pcm = vec![50; voice_vad::RATE * 2];
        let mut changed_samples = 0;
        for (index, &sample) in original.iter().enumerate() {
            let mut value = i32::from(sample) / divisor;
            // Model a microphone onset ramp, without fabricating phonemes or
            // changing speech timing. The original fixture remains immutable.
            if soft_onset && index >= onset && index - onset < 3200 {
                value = value * (800 + (index - onset) as i32) / 4000;
            }
            changed_samples += usize::from(value != i32::from(sample));
            pcm.push(value.clamp(-32768, 32767) as i16);
        }
        if soft_onset { assert!(changed_samples > 0); }
        pcm.extend_from_slice(&[50; voice_vad::END_SILENCE + 320]);
        let unsegmented: Vec<_> = pcm[voice_vad::RATE * 2..].iter()
            .flat_map(|sample| sample.to_le_bytes()).collect();
        std::fs::write(out.join(format!("{name}-unsegmented.raw")), unsegmented).unwrap();
        let mut consumed = 0;
        for chunk in pcm.chunks(160) {
            consumed += vad.push(chunk);
            if vad.state() == voice_vad::VadState::Complete { break; }
        }
        let captured = vad.speech().expect("complete wake-command speech was not segmented");
        let peak = captured.iter().map(|sample| sample.unsigned_abs()).max().unwrap();
        let energy = captured.iter().map(|&sample| i64::from(sample).pow(2)).sum::<i64>() / captured.len() as i64;
        let encoded: Vec<_> = captured.iter().flat_map(|sample| sample.to_le_bytes()).collect();
        std::fs::write(out.join(format!("{name}.raw")), encoded).unwrap();
        println!("{{\"variant\":\"{name}\",\"onset_sample\":{onset},\"changed_samples\":{changed_samples},\"onset_threshold\":{},\"continuation_threshold\":{},\"source_samples\":{},\"consumed_samples\":{consumed},\"captured_samples\":{},\"peak\":{peak},\"mean_square\":{energy}}}",
            floor.threshold(), floor.continuation_threshold(), original.len(), captured.len());
    }
}
