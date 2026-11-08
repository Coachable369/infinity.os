"""Decode every bundled sample and inspect actual stream properties."""
import json
import os
from pathlib import Path
import subprocess

# ------------------------=
# FUNC: main
# DESC: Checks format, duration, bounded size, full decode, and non-silent audio of installed samples.
# ------------------=
def main():
    assert os.environ.get('INFINITY_BUILD_KIT_ACTIVE')
    root = Path(__file__).resolve().parents[1]
    for name, audio, video in [('Infinity-Chime.wav', 'pcm_s16le', None),
                                ('Infinity-Melody.mp3', 'mp3', None),
                                ('Infinity-Motion.mp4', 'aac', 'h264')]:
        path = root / 'runtime/install/System' / name
        assert 0 < path.stat().st_size < 4*1024*1024
        info = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-show_streams', '-show_format', '-of', 'json', str(path)]))
        assert 5.9 <= float(info['format']['duration']) <= 6.2
        streams = {s['codec_type']: s for s in info['streams']}
        assert streams['audio']['codec_name'] == audio
        if video:
            assert streams['video']['codec_name'] == video
            assert (streams['video']['width'], streams['video']['height']) == (512, 288)
        pcm = subprocess.check_output(['ffmpeg', '-v', 'error', '-xerror', '-i', str(path), '-map', '0:a:0', '-f', 's16le', '-ar', '24000', '-ac', '1', '-'])
        assert len(pcm) >= 24000*2*5.9 and any(pcm)
        subprocess.run(['ffmpeg', '-v', 'error', '-xerror', '-i', str(path), '-f', 'null', '-'], check=True)
        print(name, 'decoded completely')

if __name__ == '__main__':
    main()
