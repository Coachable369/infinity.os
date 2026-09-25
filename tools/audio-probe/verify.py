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
    assert len(active) > rate, "No sustained hardware PCM output"
    samples = samples[active[0]:active[-1] + 1]
    crossings = sum(a <= 0 < b for a, b in zip(samples, samples[1:]))
    hz = crossings * rate / len(samples)
    rms = math.sqrt(sum(x * x for x in samples) / len(samples))
    assert abs(hz - 440) < 2, hz
    assert 1000 < rms < 10000, rms
    print(f"HDA DMA playback PASS: {hz:.2f} Hz, {len(samples)/rate:.3f} s, RMS {rms:.1f}, {rate} Hz/{channels} channels")
