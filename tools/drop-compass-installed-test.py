"""Real desktop hold/navigation proof after installation and ISO-detached boot."""
import argparse
import importlib.util
import json
from pathlib import Path
import shutil
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('native_editor', ROOT / 'tools/native-editor-installed-test.py')
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)

# ------------------------=
# FUNC: verify
# DESC: Holds the real desktop, compares visible card pixels, cancels, then opens Projects through keyboard selection.
# ------------------=
def verify(guest):
    state = guest.state()
    guest.return_to_desktop(state)
    state = guest.wait(lambda s: s[4] == 5, 'desktop')
    guest.width, guest.height = state[11:13]
    x, y = guest.width // 3, guest.height // 2
    guest.click(x, y, press=False)
    before = native.desktop.read_pixels(guest.screenshot('compass-before'))
    guest.qmp('input-send-event', {'events': [{'type': 'btn', 'data': {'button': 'left', 'down': True}}]})
    guest.wait(lambda s: bool(s[15] & 1), 'hold received')
    time.sleep(1)
    guest.qmp('input-send-event', {'events': [{'type': 'btn', 'data': {'button': 'left', 'down': False}}]})
    guest.wait(lambda s: not s[15] & 1, 'hold released')
    shown = native.desktop.read_pixels(guest.screenshot('compass-open'))
    assert shown[:2] == before[:2]
    # The top card is outside the desktop widget column and all changing clock pixels.
    cx = max(min(380, guest.width//2), min(x, max(guest.width-380, guest.width//2)))
    cy = max(min(270, guest.height//2), min(y, max(guest.height-270, guest.height//2)))
    changed = sum(before[2][(row*guest.width+col)*3:(row*guest.width+col)*3+3] != shown[2][(row*guest.width+col)*3:(row*guest.width+col)*3+3]
                  for row in range(cy-250, cy-190) for col in range(cx-110, cx+110))
    assert changed > 5000, {'card_pixels_changed': changed}
    guest.key('esc')
    guest.screenshot('compass-dismissed')
    guest.click(x, y, press=False)
    guest.qmp('input-send-event', {'events': [{'type': 'btn', 'data': {'button': 'left', 'down': True}}]})
    time.sleep(1)
    guest.qmp('input-send-event', {'events': [{'type': 'btn', 'data': {'button': 'left', 'down': False}}]})
    time.sleep(.2)
    guest.key('up')
    guest.key('ret')
    deadline = time.monotonic()+30
    while time.monotonic()<deadline:
        windows = native.read(guest, 'INFINITY_WINDOW_CONTROL_SNAPSHOT', 81)
        if windows[8] == 1 and windows[4] == 0:
            guest.screenshot('compass-projects')
            return {'hold_open': True, 'card_pixels_changed': changed, 'keyboard_opened_file_navigator': True, 'iso_detached': True}
        time.sleep(.2)
    raise AssertionError({'windows': windows})

# ------------------------=
# FUNC: main
# DESC: Uses an isolated installation and records actual guest interaction evidence without changing user VMs.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--arch', choices=('aarch64','x86_64'), required=True)
    parser.add_argument('--iso', type=Path, required=True)
    parser.add_argument('work', type=Path)
    args = parser.parse_args()
    work = args.work.resolve(); work.relative_to(ROOT/'build'); work.mkdir()
    artifacts = work/'artifacts'; artifacts.mkdir()
    shutil.copyfile(args.iso, artifacts/'installer.iso')
    native.desktop.extract_media_kernels(artifacts/'installer.iso', artifacts, args.arch)
    guest = native.NativeGuest(work, 1, f'/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd', width=1024, height=768, memory_mb=12288)
    guest.arch=args.arch; guest.acceleration='hvf' if args.arch=='aarch64' else 'tcg'
    guest.patched=True; guest.boot_timeout_seconds=600
    report={'accepted':False,'arch':args.arch}
    try:
        guest.install(); guest.stop(); guest.onboard()
        report.update(verify(guest)); report['accepted']=True
    finally:
        guest.stop(); (work/'evidence.json').write_text(json.dumps(report,indent=2))
    print(json.dumps(report,indent=2))

if __name__ == '__main__':
    main()
