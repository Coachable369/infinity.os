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
    route_tests();
    use runtime::{audio::*, capability::*, execution::SecurityIdentity, iop::*};
    assert_eq!(std::mem::align_of::<InfinityAudio<16, 4>>(), 128);
    let split = hda::resident_descriptor_lengths(0x1000, 15_004).unwrap();
    assert_eq!(split[0] + split[1], 15_004);
    assert_eq!(split[0] % 128, 0);
    assert_eq!((0x1000 + split[0]) % 128, 0);
    assert_eq!(split[1] % 4, 0);
    assert!(hda::resident_descriptor_lengths(0x1004, 15_004).is_none());
    assert!(hda::resident_descriptor_lengths(0x1000, 6).is_none());
    assert_eq!(hda::resident_duration_ns(96_000, 48_000), Some(1_000_000_000));
    assert_eq!(hda::resident_duration_ns(88_200, 44_100), Some(1_000_000_000));
    assert_eq!(hda::resident_duration_ns(0, 48_000), None);
    assert_eq!(hda::resident_duration_ns(3, 48_000), None);
    assert!(hda::capture_fault_is_recoverable(hda::Error::Dma));
    assert!(!hda::capture_fault_is_recoverable(hda::Error::Invalid));
    assert!(!hda::capture_fault_is_recoverable(hda::Error::Timeout));
    let mut buffer = AudioBuffer::<3>::new();
    buffer.push_stereo(&[100, 300, -32768, 32767]).unwrap();
    assert!(buffer.push_stereo(&[1, 1, 2, 2]).is_err());
    let mut output = [0; 2];
    assert_eq!(buffer.read(&mut output), 2); assert_eq!(output, [200, 0]);
    buffer.push_stereo(&[4, 6, 8, 10, 12, 14]).unwrap();
    assert_eq!(buffer.read(&mut output), 2); assert_eq!(output, [5, 9]);
    buffer.clear(); assert_eq!(buffer.read(&mut output), 0);
    assert!(buffer.push_stereo(&[1]).is_err());
    let mut zero = AudioBuffer::<0>::new();
    assert!(zero.push_stereo(&[0, 0]).is_err()); assert_eq!(zero.read(&mut output), 0);
    let mut stream = InfinityAudio::<16, 4>::new();
    stream.reset(9);
    stream.append(9, &[10, 30, 20, 40], 0, 4).unwrap();
    stream.append(9, &[30, 50, 40, 60], 4, 8).unwrap();
    assert_eq!(stream.status(1).content_position, 2);
    assert_eq!(stream.status(3).content_position, 6);
    assert!(stream.append(8, &[1, 1], 8, 9).is_err());
    stream.seal(9).unwrap();
    assert!(stream.sealed_resident(9).is_some());
    let mut period = [7i16; 10];
    assert_eq!(stream.read(9, &mut period).unwrap(), 8);
    assert_eq!(&period[..8], &[10, 30, 20, 40, 30, 50, 40, 60]);
    assert_eq!(&period[8..], &[0, 0]);
    let mut reference = [0i16; 4];
    assert!(stream.reference_mono(4, 4, 4, &mut reference));
    assert_eq!(reference, [20, 30, 40, 50]);
    assert_eq!(stream.status(99).content_position, 8);
    assert!(stream.sealed_resident(9).is_none());
    assert_eq!(stream.erase_pcm(), 8);
    assert_eq!(stream.erase_pcm(), 0);
    stream.reset(10);
    assert!(stream.sealed_resident(10).is_none());
    assert!(stream.read(9, &mut period).is_err());
    // Repeated short reply generations erase only written PCM instead of the
    // fixed 32-second resident capacity that exists in the installed runtime.
    for generation in 11..=18 {
        stream.reset(generation);
        stream.append(generation, &[generation as i16; 4], 0, 2).unwrap();
        stream.seal(generation).unwrap();
        assert_eq!(stream.erase_pcm(), 4);
        assert_eq!(stream.erase_pcm(), 0);
    }
    let mut wrapped = InfinityAudio::<8, 4>::new();
    wrapped.reset(11);
    wrapped.append(11, &[1, 11, 2, 12, 3, 13, 4, 14], 0, 4).unwrap();
    let mut consumed = [0i16; 4];
    assert_eq!(wrapped.read(11, &mut consumed).unwrap(), 4);
    wrapped.append(11, &[5, 15, 6, 16], 4, 6).unwrap();
    assert!(!wrapped.can_append(11, 2, 3, 7));
    let mut wrapped_reference = [0i16; 4];
    assert!(wrapped.reference_mono(6, 4, 4, &mut wrapped_reference));
    assert_eq!(wrapped_reference, [8, 9, 10, 11]);
    assert_eq!(wrapped.erase_pcm(), 8);
    wrapped_reference.fill(7);
    assert!(wrapped.reference_mono(6, 4, 4, &mut wrapped_reference));
    assert_eq!(wrapped_reference, [0; 4]);
    assert!(wrapped.append(11, &[7, 17], 3, 7).is_err());
    let mut malformed = InfinityAudio::<3, 1>::new();
    malformed.reset(12);
    assert!(malformed.append(12, &[1, 2], 0, 1).is_err());
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
    let capture_lease = AudioStream::authorize(&capture, owner, &caps, 10).unwrap();
    let same_authority = IopMessage::request(OperationId::AudioCaptureStart, 3, owner, input, 17, 3, &[]).unwrap();
    let same_renewed = capture_lease.renew(&same_authority, &caps, 12).unwrap();
    assert_eq!(same_renewed.capability, input); assert_eq!(same_renewed.deadline, 17);
    let replacement = caps.grant(CapabilityType::AudioInput, 0, 1, 0, owner, owner, Some(30), 0).unwrap();
    let mut renewal = IopMessage::request(OperationId::AudioCaptureStart, 4, owner, replacement, 17, 4, &[]).unwrap();
    let renewed = capture_lease.renew(&renewal, &caps, 12).unwrap();
    assert_eq!(renewed.capability, replacement); assert_eq!(renewed.deadline, 17);
    assert!(capture_lease.renew(&renewal, &caps, 14).is_err());
    renewal.header.caller_identity = other;
    assert!(capture_lease.renew(&renewal, &caps, 12).is_err());
    renewal.header.caller_identity = owner;
    renewal.header.operation_type_id = OperationId::AudioTone as u32;
    renewal.header.capability_ref = output;
    assert!(capture_lease.renew(&renewal, &caps, 12).is_err());
    renewal.header.operation_type_id = OperationId::AudioCaptureStart as u32;
    renewal.header.capability_ref = replacement;
    caps.revoke(replacement).unwrap();
    assert!(capture_lease.renew(&renewal, &caps, 12).is_err());
    capture.header.payload_length = 1;
    assert!(AudioStream::authorize(&capture, owner, &caps, 10).is_err());
    caps.revoke(output).unwrap(); assert!(!lease.valid(&caps, 11));
    caps.retire_leaf(output, owner).unwrap();
    let speech_cap = caps.grant(CapabilityType::AudioOutput, 0, 1, 0, owner, owner, Some(60), 0).unwrap();
    let mut speech = IopMessage::request(OperationId::AudioPlaybackStart, 3, owner, speech_cap, 45, 3, &[]).unwrap();
    let speech_lease = AudioStream::authorize(&speech, owner, &caps, 10).unwrap();
    assert_eq!(speech_lease.route, AudioRoute::Playback);
    assert!(speech_lease.valid(&caps, 44));
    assert!(!speech_lease.valid(&caps, 45));
    speech.header.deadline = 46;
    assert!(matches!(AudioStream::authorize(&speech, owner, &caps, 10), Err(AudioError::Expired)));
    speech.header.deadline = 45; speech.header.capability_ref = input;
    assert!(matches!(AudioStream::authorize(&speech, owner, &caps, 10), Err(AudioError::Denied)));
    caps.revoke(speech_cap).unwrap();
    assert!(!speech_lease.valid(&caps, 11));
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
// ------------------------=
// FUNC: route_tests
// DESC: Exercises direct, VirtualBox selector/amplifier, cyclic, malformed and missing input topologies.
// ------------------=
fn route_tests() {
    let mut kinds = [0u32; 32];
    let mut edges = [0u32; 32];
    let mut counts = [0u32; 32];
    kinds[6] = 1; kinds[23] = 3; kinds[18] = 3; kinds[14] = 4;
    edges[6] = 23; edges[23] = 18; edges[18] = 14;
    counts[6] = 1; counts[23] = 1; counts[18] = 1;
    let read = |n: u32, c: u32| -> Result<u32, hda::Error> {
        assert!((n as usize) < kinds.len());
        Ok(match c { 0xf0009 => kinds[n as usize] << 20, 0xf000c => 0x20,
            0xf000e => counts[n as usize], 0xf0200 => edges[n as usize], _ => panic!("unexpected codec write") })
    };
    let route = hda::capture_route(6, 2, 28, &mut read.clone()).unwrap().unwrap();
    assert_eq!(route.length, 4); assert_eq!(&route.nodes[..4], &[6, 23, 18, 14]);
    edges[6] = 14;
    let mut calls = 0;
    let mut read = |n: u32, c: u32| -> Result<u32, hda::Error> {
        calls += 1;
        Ok(match c { 0xf0009 => kinds[n as usize] << 20, 0xf000c => 0x20,
            0xf000e => counts[n as usize], 0xf0200 => edges[n as usize], _ => panic!("unexpected codec write") })
    };
    assert_eq!(hda::capture_route(6, 2, 28, &mut read).unwrap().unwrap().length, 2);
    assert!(calls < 10);
    edges[6] = 23; edges[18] = 6;
    let mut read = |n: u32, c: u32| -> Result<u32, hda::Error> {
        Ok(match c { 0xf0009 => kinds[n as usize] << 20, 0xf000c => 0x20,
            0xf000e => counts[n as usize], 0xf0200 => edges[n as usize], _ => panic!("unexpected codec write") })
    };
    assert!(hda::capture_route(6, 2, 28, &mut read).unwrap().is_none());
    assert_eq!(hda::capture_route(128, 2, 129, &mut read), Err(hda::Error::Invalid));
    assert!(hda::capture_route(2, 2, 28, &mut read).unwrap().is_none());
    assert_eq!(hda::capture_route(6, 2, 28, &mut |_, _| Err(hda::Error::Timeout)), Err(hda::Error::Timeout));
}
