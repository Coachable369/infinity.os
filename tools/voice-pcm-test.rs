#[path = "../kernel/runtime/ai/voice_pcm.rs"] mod pcm;
#[path = "../kernel/runtime/ai/voice_vad.rs"] mod vad;
use pcm::Resampler;

// ------------------------=
// FUNC: convert
// DESC: Exercises streaming conversion with independently bounded input and output chunks.
// ------------------=
fn convert(rate: u32, input: &[i16], chunk: usize, capacity: usize) -> Vec<i16> {
    let mut r = Resampler::empty();
    assert!(r.configure(rate));
    let mut result = Vec::new();
    let mut output = vec![0; capacity];
    for block in input.chunks(chunk) {
        let mut offset = 0;
        while offset < block.len() {
            let (used, count) = r.process(&block[offset..], &mut output);
            assert!(used > 0);
            offset += used;
            result.extend_from_slice(&output[..count]);
        }
    }
    result
}

// ------------------------=
// FUNC: tone
// DESC: Creates numerical PCM fixtures at a known source frequency and amplitude.
// ------------------=
fn tone(rate: u32, hz: f64) -> Vec<i16> {
    (0..rate).map(|n| (12_000.0 * (n as f64 * hz * 2.0 * std::f64::consts::PI / rate as f64).sin()) as i16).collect()
}

// ------------------------=
// FUNC: rms
// DESC: Measures steady-state signal energy after excluding the finite filter startup transient.
// ------------------=
fn rms(samples: &[i16]) -> f64 {
    let s = &samples[200..];
    (s.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / s.len() as f64).sqrt()
}

// ------------------------=
// FUNC: main
// DESC: Verifies duration, chunk invariance, passband gain, alias rejection, clipping and reset behavior.
// ------------------=
fn main() {
    for rate in [44_100, 48_000] {
        let input = tone(rate, 1_000.0);
        let reference = convert(rate, &input, input.len(), 16_000);
        assert_eq!(reference.len(), 16_000);
        assert!((rms(&reference) / (12_000.0 / 2f64.sqrt()) - 1.0).abs() < 0.01);
        // Check fractional sample timing, not just energy: a reversed fractional
        // phase produces periodic distortion while retaining roughly the same RMS.
        let error: f64 = reference.iter().enumerate().skip(200).map(|(i, actual)| {
            let source_time = (i + 1) as f64 * rate as f64 / 16_000.0 - 1.0 - 47.5;
            let expected = 12_000.0 * (source_time * 1_000.0 * 2.0 * std::f64::consts::PI / rate as f64).sin();
            (*actual as f64 - expected).powi(2)
        }).sum();
        assert!((error / 15_800.0).sqrt() < 3.0);
        for (chunk, capacity) in [(1, 1), (17, 3), (441, 31), (4_800, 160)] {
            assert_eq!(convert(rate, &input, chunk, capacity), reference);
        }
        let alias = convert(rate, &tone(rate, 12_000.0), 701, 320);
        assert!(rms(&alias) / rms(&reference) < 0.001);
        for level in [i16::MIN, i16::MAX, 0] {
            let dc = convert(rate, &vec![level; rate as usize], 319, 97);
            assert!(dc[200..].iter().all(|s| (*s as i32 - level as i32).abs() <= 1));
        }
        let mut r = Resampler::empty();
        assert!(r.configure(rate));
        assert_eq!(r.process(&input, &mut []), (0, 0));
        r.process(&input[..1000], &mut [0; 400]);
        r.clear();
        let mut silent = [1; 320];
        assert_eq!(r.process(&vec![0; rate as usize / 50], &mut silent).1, 320);
        assert_eq!(silent, [0; 320]);
        // Feed converted capture-format PCM into the actual utterance segmenter.
        let mut recording = vec![0; rate as usize / 5];
        recording.extend_from_slice(&input[..rate as usize / 2]);
        recording.extend_from_slice(&vec![0; rate as usize]);
        let converted = convert(rate, &recording, 441, 160);
        let mut utterance = vad::Utterance::new(300);
        for block in converted.chunks(160) { utterance.push(block); }
        assert_eq!(utterance.state(), vad::VadState::Complete);
        assert!((11_000..14_000).contains(&utterance.speech().unwrap().len()));
        utterance.cancel();
        assert_eq!(utterance.speech(), None);
        utterance.clear(300);
        assert_eq!(utterance.state(), vad::VadState::Waiting);
    }
    let passthrough = vec![i16::MIN, -1, 0, 1, i16::MAX];
    assert_eq!(convert(16_000, &passthrough, 2, 1), passthrough);
    println!("Native voice sample-rate conversion: PASS");
}
