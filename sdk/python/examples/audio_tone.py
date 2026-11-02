"""Create bounded PCM WAV data; this example does not claim speaker playback."""
import math
import struct
import wave


# ------------------------=
# FUNC: write_tone
# DESC: Writes a bounded mono PCM tone with quiet attack and release edges.
# ------------------=
def write_tone(path, frequency=440.0, duration=0.5, rate=16000):
    if rate not in (8000, 16000, 22050, 44100, 48000):
        raise ValueError("Unsupported sample rate")
    if not 0 < frequency < rate / 2 or not 0 < duration <= 10:
        raise ValueError("Frequency or duration outside supported range")
    frames = int(rate * duration)
    if frames < 2:
        raise ValueError("At least two frames required")
    ramp = max(1, min(int(rate * 0.01), frames // 2))
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(rate)
        for index in range(frames):
            gain = min(1.0, index / ramp, (frames - 1 - index) / ramp)
            sample = int(6553 * gain * math.sin(2 * math.pi * frequency * index / rate))
            output.writeframesraw(struct.pack("<h", sample))
    return frames


if __name__ == "__main__":
    destination = "infinity-tone.wav"
    write_tone(destination)
    print("Created", destination)
