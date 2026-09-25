#[path = "../kernel/runtime/ai/voice_vad.rs"] mod vad;
use vad::*;
// ------------------------=
// FUNC: main
// DESC: Tests actual PCM segmentation and privacy cleanup without using text as an oracle.
// ------------------=
fn main() {
    let mut detector = Detector::new(300);
    assert_eq!(detector.push(&[0; MAX_SAMPLES + 1]), MAX_SAMPLES);
    assert_eq!(detector.state(), VadState::NoSpeech);
    assert_eq!(detector.push(&[32767; FRAME]), 0);
    assert_eq!(detector.segment(), None);
    let mut clip = Detector::new(300);
    clip.push(&[i16::MIN; FRAME * 3]);
    assert_eq!(clip.state(), VadState::Speech);
    assert_eq!(clip.push(&[i16::MAX; MAX_SAMPLES]), MAX_SAMPLES - FRAME * 3);
    assert_eq!(clip.state(), VadState::Complete);
    assert_eq!(clip.segment(), Some(Segment { start: 0, end: MAX_SAMPLES }));
    let mut transient = Detector::new(300);
    transient.push(&[1000; FRAME * 2]); transient.push(&[0; FRAME]);
    assert_eq!(transient.state(), VadState::Waiting);
    let mut input = vec![0; RATE];
    input.extend_from_slice(&[1000; RATE / 2]);
    input.extend_from_slice(&[0; RATE]);
    let mut whole = Detector::new(300);
    let accepted = whole.push(&input);
    assert_eq!(accepted, RATE + RATE / 2 + RATE * 3 / 5);
    let segment = whole.segment().unwrap();
    assert_eq!(segment, Segment { start: RATE - RATE / 5, end: RATE + RATE / 2 + RATE / 10 });
    for chunk_size in [1, 17, 319, 320, 701, 4096] {
        let mut chunked = Detector::new(300);
        let consumed: usize = input.chunks(chunk_size).map(|c| chunked.push(c)).sum();
        assert_eq!(consumed, accepted); assert_eq!(chunked.segment(), Some(segment));
    }
    let mut utterance = Utterance::new(300);
    assert_eq!(utterance.speech(), None);
    utterance.push(&input);
    assert_eq!(utterance.state(), VadState::Complete);
    assert_eq!(utterance.speech().unwrap(), &input[segment.start..segment.end]);
    utterance.cancel(); assert_eq!(utterance.state(), VadState::Cancelled);
    assert_eq!(utterance.speech(), None); assert_eq!(utterance.push(&input), 0);
    utterance.clear(300); assert_eq!(utterance.state(), VadState::Waiting);
    utterance.push(&[0; MAX_SAMPLES]); assert_eq!(utterance.state(), VadState::NoSpeech);
    utterance.clear(300);
    for _ in 0..1000 { utterance.push(&[0; FRAME]); }
    assert_eq!(utterance.state(), VadState::Waiting);
    for _ in 0..100 { utterance.push(&[1000; FRAME]); }
    for _ in 0..40 { utterance.push(&[0; FRAME]); }
    assert_eq!(utterance.state(), VadState::Complete);
    assert_eq!(utterance.speech().unwrap().iter().filter(|&&s| s == 1000).count(), FRAME * 100);
    println!("Voice PCM segmentation: PASS");
}
