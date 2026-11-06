"""Reproducible, original six-second demonstration media; no external downloads."""
from pathlib import Path
import math
import os
import struct
import subprocess
import wave

# ------------------------=
# FUNC: main
# DESC: Authors a soft arpeggio and a branded video with codecs the native player ships.
# ------------------=
def main():
    assert os.environ.get('INFINITY_BUILD_KIT_ACTIVE')
    root = Path(__file__).resolve().parents[1]
    out = root / 'sdk/system'
    wav = out / 'Infinity-Chime.wav'
    rate = 24000
    notes = [261.63, 329.63, 392.00, 523.25, 392.00, 329.63]
    with wave.open(str(wav), 'wb') as f:
        f.setparams((1, 2, rate, 0, 'NONE', 'not compressed'))
        data = bytearray()
        for i in range(rate * 6):
            t = i / rate
            local = t % 1
            envelope = min(local / .025, 1) * min((1-local) / .18, 1)
            tone = math.sin(2*math.pi*notes[int(t)]*t)
            data.extend(struct.pack('<h', int(6000*envelope*tone)))
        f.writeframes(data)
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(wav), '-c:a', 'libmp3lame', '-b:a', '96k', str(out/'Infinity-Melody.mp3')], check=True)
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-loop', '1', '-i', str(out/'media-cover.png'), '-i', str(wav),
        '-vf', 'scale=512:288:force_original_aspect_ratio=decrease,pad=512:288:(ow-iw)/2:(oh-ih)/2:color=0x081426,fade=t=in:st=0:d=1,fade=t=out:st=5:d=1',
        '-t', '6', '-r', '24', '-c:v', 'libx264', '-profile:v', 'baseline', '-pix_fmt', 'yuv420p', '-crf', '26',
        '-c:a', 'aac', '-b:a', '96k', '-movflags', '+faststart', str(out/'Infinity-Motion.mp4')], check=True)

if __name__ == '__main__':
    main()
