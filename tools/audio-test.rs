#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "../kernel/ui/mod.rs"] mod ui;
#[path = "../kernel/drivers/hda.rs"] mod hda;
// ------------------------=
// FUNC: output_text
// DESC: Supplies the host harness diagnostic sink.
// ------------------=
fn output_text(_: &[u8]) {}
// ------------------------=
// FUNC: main
// DESC: Runs behavioral authority, deadline, route-isolation and PCM checks.
// ------------------=
fn main() {
    use runtime::{audio::*, capability::*, execution::SecurityIdentity, iop::*};
    let owner = SecurityIdentity([7; 16]);
    let other = SecurityIdentity([8; 16]);
    let mut caps = CapabilityManager::new();
    let output = caps.grant(CapabilityType::AudioOutput, 0, 1, 0, owner, owner, Some(20), 0).unwrap();
    let input = caps.grant(CapabilityType::AudioInput, 0, 1, 0, owner, owner, Some(20), 0).unwrap();
    let request = IopMessage::request(OperationId::AudioTone, 1, owner, output, 14, 1, &[]).unwrap();
    let lease = AudioStream::authorize(&request, owner, &caps, 10).unwrap();
    assert_eq!(lease.route, AudioRoute::Playback);
    assert!(lease.valid(&caps, 13)); assert!(!lease.valid(&caps, 14));
    assert!(AudioStream::authorize(&request, other, &caps, 10).is_err());
    let mut capture = IopMessage::request(OperationId::AudioCaptureStart, 2, owner, output, 14, 2, &[]).unwrap();
    assert!(AudioStream::authorize(&capture, owner, &caps, 10).is_err());
    capture.header.capability_ref = input;
    assert_eq!(AudioStream::authorize(&capture, owner, &caps, 10).unwrap().route, AudioRoute::Capture);
    capture.header.payload_length = 1;
    assert!(AudioStream::authorize(&capture, owner, &caps, 10).is_err());
    caps.revoke(output).unwrap(); assert!(!lease.valid(&caps, 11));
    caps.retire_leaf(output, owner).unwrap();
    let mut pcm = [0; hda::SAMPLES]; hda::tone(&mut pcm);
    let mut crossings = 0;
    for frame in 0..hda::FRAMES {
        assert_eq!(pcm[frame * 2], pcm[frame * 2 + 1]);
        assert!(pcm[frame * 2].abs() <= 8191);
        if frame > 0 && pcm[(frame - 1) * 2] <= 0 && pcm[frame * 2] > 0 { crossings += 1; }
    }
    assert_eq!(crossings, 44);
    let mut fallback = [0i16; 8820];
    hda::tone_at_rate(&mut fallback, 44100).unwrap();
    assert_eq!(fallback.chunks_exact(2).map(|s| s[0]).collect::<Vec<_>>()
        .windows(2).filter(|s| s[0] <= 0 && s[1] > 0).count(), 44);
    assert!(hda::tone_at_rate(&mut fallback, 48000).is_err());
    assert_eq!(PCM.sample_rate, 48000);
    println!("Audio authority and deterministic PCM: PASS");
}
