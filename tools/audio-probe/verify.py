"""Verify hardware DMA via VM exit status and output PCM, not diagnostic prose."""
import array
import math
import subprocess
import sys
import struct
from pathlib import Path

backend = sys.argv[1]
assert backend in ("wav", "coreaudio")
audio = "wav,id=audio,out.frequency=48000,path=build/audio-probe/tone.wav" if backend == "wav" else "coreaudio,id=audio,out.frequency=48000"
subprocess.run([
    "qemu-system-aarch64", "-machine", "virt", "-accel", "hvf", "-cpu", "host", "-m", "256M",
    "-display", "none", "-serial", "none", "-monitor", "none",
    "-semihosting-config", "enable=on,target=native", "-kernel", "build/audio-probe/probe.elf",
    "-audiodev", audio, "-device", "intel-hda", "-device", "hda-duplex,audiodev=audio",
], timeout=30, check=True)
if backend == "wav":
    # QEMU 11's WAV sink leaves RIFF/data sizes zero on guest shutdown.
    # Validate its PCM header and analyze actual emitted bytes, never repair samples.
    raw = Path("build/audio-probe/tone.wav").read_bytes()
    assert raw[:4] == b"RIFF" and raw[8:16] == b"WAVEfmt " and raw[36:40] == b"data"
    encoding, channels, rate, byte_rate, align, bits = struct.unpack_from("<HHIIHH", raw, 20)
    assert encoding == 1 and bits == 16 and align == channels * 2
    assert byte_rate == rate * align and (len(raw) - 44) % align == 0
    pcm = array.array("h", raw[44:])
    if sys.byteorder != "little":
        pcm.byteswap()
    samples = pcm[::channels]
    active = [i for i, value in enumerate(samples) if abs(value) > 200]
    assert len(active) > rate * 1.8, "Missing full resident hardware PCM output"
    samples = samples[active[0]:active[-1] + 1]
    expected = array.array("h")
    for frame in range(95_999):
        period = 96 if frame < 24_000 else 64 if frame < 72_000 else 48
        phase = frame % period
        expected.append(-6000 + phase * 24000 // period if phase < period // 2
                        else 18000 - phase * 24000 // period)
    assert rate == 48_000 and channels == 2
    assert pcm[::2] == pcm[1::2], "Stereo channels diverged"
    assert len(samples) == len(expected), f"Lost or repeated PCM frames: {len(samples)} != {len(expected)}"
    assert samples == expected, "Resident PCM differs from the intended complete waveform"
    duration = len(samples) / rate
    assert abs(duration - 2.0) < 0.02, f"Truncated or repeated resident playback: {duration:.3f}s"
    assert max((sum(abs(value) < 200 for value in samples[start:start + rate // 100])
                for start in range(0, len(samples) - rate // 100, rate // 100)), default=0) < rate // 500
    markers = []
    for start, end, expected in [(0.0, 0.5, 500), (0.5, 1.5, 750), (1.5, duration, 1000)]:
        segment = samples[round(start * rate):round(end * rate)]
        crossings = sum(a <= 0 < b for a, b in zip(segment, segment[1:]))
        hz = crossings * rate / len(segment)
        assert abs(hz - expected) < 4, (expected, hz)
        markers.append(hz)
    rms = math.sqrt(sum(x * x for x in samples) / len(samples))
    assert 1000 < rms < 10000, rms
    print(f"HDA DMA playback PASS: {len(samples)} exact PCM frames, full start/body/end markers {markers}, {duration:.3f} s, RMS {rms:.1f}, {rate} Hz/{channels} channels")
