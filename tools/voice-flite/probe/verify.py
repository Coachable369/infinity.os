"""Native guest synthesis and real HDA PCM proof; host only verifies resulting artifacts."""
import array
import argparse
import json
from pathlib import Path
import struct
import subprocess
import sys
import wave

# ------------------------=
# FUNC: verify_waveform
# DESC: Compares every played speech sample with independently synthesized PCM after exact rate conversion; missing or corrupted samples fail.
# ------------------=
def verify_waveform(mono, reference, rate):
    expected = array.array("h")
    for index in range((len(reference) * rate + 15999) // 16000):
        source, fraction = divmod(index * 16000, rate)
        a = reference[source]
        b = reference[source + 1] if source + 1 < len(reference) else 0
        delta = (b - a) * fraction
        # C/Rust integer division truncates toward zero, not negative infinity.
        expected.append(a + (abs(delta) // rate if delta >= 0 else -(abs(delta) // rate)))
    assert len(mono) >= len(expected), "Playback lost the end of the phrase"
    assert mono[:len(expected)] == expected, "Playback differs from the complete synthesized waveform"
    assert all(sample == 0 for sample in mono[len(expected):]), "Non-silent data after the phrase"
    return len(expected)

# ------------------------=
# FUNC: main
# DESC: Boots a freestanding ARM64 synthesis harness and checks its binary metrics and emitted HDA audio.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--accel", choices=("hvf", "tcg"), default="hvf")
    args = parser.parse_args()
    output = Path("build/voice-flite/aarch64")
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", args.accel, "-cpu", "host" if args.accel == "hvf" else "max", "-m", "256M",
                    "-display", "none", "-serial", f"file:{output / 'guest-metrics.bin'}", "-monitor", "none",
                    "-semihosting-config", "enable=on,target=native", "-kernel", str(output / "probe.elf"),
                    "-audiodev", f"wav,id=audio,out.frequency=48000,path={output / 'speech.wav'}",
                    "-device", "intel-hda", "-device", "hda-duplex,audiodev=audio"], check=True, timeout=30)
    version, frames, peak, ticks, frequency, refills = struct.unpack("<6Q", (output / "guest-metrics.bin").read_bytes())
    assert version == 1, (version, hex(frames), hex(peak), hex(ticks), frequency, refills)
    assert 16000 < frames < 480000 and 0 < peak <= 8 * 1024 * 1024
    assert frequency > 0 and ticks > 0 and refills > 20
    raw = (output / "speech.wav").read_bytes()
    assert raw[:4] == b"RIFF" and raw[8:16] == b"WAVEfmt " and raw[36:40] == b"data"
    encoding, channels, rate, byte_rate, align, bits = struct.unpack_from("<HHIIHH", raw, 20)
    assert (encoding, channels, rate, byte_rate, align, bits) == (1, 2, 48000, 192000, 4, 16)
    pcm = array.array("h", raw[44:])
    if sys.byteorder != "little":
        pcm.byteswap()
    assert len(pcm) % 2 == 0 and all(a == b for a, b in zip(pcm[::2], pcm[1::2]))
    mono = pcm[::2]
    assert sum(abs(s) > 100 for s in mono) > rate
    assert abs(len(mono) / rate - frames / 16000) < 0.3
    with wave.open("build/voice-flite/host/native-voice.wav", "rb") as reference_file:
        assert (reference_file.getnchannels(), reference_file.getsampwidth(), reference_file.getframerate()) == (1, 2, 16000)
        reference = array.array("h", reference_file.readframes(reference_file.getnframes()))
        if sys.byteorder != "little":
            reference.byteswap()
    assert len(reference) == frames
    matched_frames = verify_waveform(mono, reference, rate)
    # QEMU may omit RIFF lengths at poweroff; repair container metadata only, never PCM.
    fixed = bytearray(raw)
    struct.pack_into("<I", fixed, 4, len(raw) - 8)
    struct.pack_into("<I", fixed, 40, len(raw) - 44)
    (output / "speech-playable.wav").write_bytes(fixed)
    metrics = {"environment": f"freestanding ARM64 QEMU {args.accel}, native HDA", "speech_seconds": frames / 16000,
               "synthesis_seconds": ticks / frequency, "synthesis_rtf": ticks / frequency / (frames / 16000),
               "arena_peak_bytes": peak, "playback_polls": refills, "poll_interval_ms": 120,
               "playback_mode": "resident phrase DMA with silence tail", "stt_verified": False,
               "exact_reference_frames": matched_frames,
               "installed_system_verified": False}
    (output / "metrics.json").write_text(json.dumps(metrics, indent=2) + "\n")
    print(json.dumps(metrics, indent=2))

if __name__ == "__main__":
    main()
