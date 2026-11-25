"""Fresh installed Fabric Manager launch and rendering evidence, without audio devices."""
import argparse
import functools
import importlib.util
import json
from pathlib import Path
import struct
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
spec = importlib.util.spec_from_file_location('native_editor', ROOT / 'tools/native-editor-installed-test.py')
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)
native.base.symbol = functools.lru_cache(maxsize=64)(native.base.symbol)

# ------------------------=
# FUNC: read
# DESC: Reads coherent numeric native lifecycle state from the installed kernel's own metadata.
# ------------------=
def read(guest, name, count):
    address, length = native.base.symbol(guest.work.parent / 'artifacts/installed-kernel.elf', name)
    assert length == count * 8
    return struct.unpack(f'<{count}Q', guest.memory(address, length))

# ------------------------=
# FUNC: main
# DESC: Installs the actual packaged generation and captures Fabric Manager after disk-only authentication.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--reuse-installed', action='store_true')
    parser.add_argument('--update-kernel', type=Path)
    options = parser.parse_args()
    work = ROOT / 'build/validation/fabric-manager-installed'
    artifacts = work / 'artifacts'
    work.mkdir(parents=True, exist_ok=options.reuse_installed)
    artifacts.mkdir(exist_ok=options.reuse_installed)
    media = ROOT / 'build/test-media/InfinityOS-aarch64-qemu-models.bootmedia'
    if not options.reuse_installed:
        native.desktop.copy_artifact(media, artifacts / 'installer.iso')
    receipt = {'media': str(media), 'audio_attached': False, 'complete': False}
    if not options.reuse_installed:
        receipt['artifacts'] = native.desktop.extract_media_kernels(artifacts / 'installer.iso', artifacts, 'aarch64')
    guest = native.NativeGuest(work, 1, '/opt/homebrew/share/qemu/edk2-aarch64-code.fd',
                               reuse=options.reuse_installed, width=1440, height=1000, memory_mb=16384,
                               arch='aarch64', acceleration='hvf')
    guest.boot_timeout_seconds = 1200
    guest.install_timeout_seconds = 1800
    # Keep the installed media payload unchanged; the generic desktop harness
    # otherwise supports an optional offline replacement kernel.
    guest.patched = True
    if options.update_kernel:
        assert options.reuse_installed, 'Kernel diagnostics require an existing disposable installation'
        kernel = options.update_kernel.resolve()
        kernel.relative_to(ROOT)
        with kernel.open('rb') as image:
            header = image.read(64)
        assert header[:6] == b'\x7fELF\x02\x01'
        assert struct.unpack_from('<H', header, 18)[0] == 183
        assert struct.unpack_from('<Q', header, 24)[0] >= 0x40000000, 'Use installed-kernel-qemu.elf for the QEMU RAM map'
        subprocess.run([sys.executable, str(ROOT / 'tools/update-installed-clone.py'),
                        str(guest.work / 'installed.raw'), str(kernel), '--apply'], check=True)
        shutil.copyfile(kernel, artifacts / 'installed-kernel.elf')
        receipt['diagnostic_kernel'] = str(kernel)
        receipt['canonical_release_media'] = False
    width = height = dock_width = scale = None
    try:
        if not options.reuse_installed:
            guest.install()
            guest.stop()
        receipt['installed'] = guest.onboard()
        state = guest.state()
        width, height = state[11:13]
        scale = 2 if width >= 2560 and height >= 1440 else 1
        dock_width = width * 54 // 100
        native.adaptive_click(guest, (width-dock_width)//2 + (dock_width//8)//2, height-46*scale)
        guest.wait(lambda state: state[4] == 6, 'launcher', timeout=30)
        guest.text('Infinity Fabric')
        started = time.monotonic()
        guest.key('ret')
        deadline = started + 180
        while time.monotonic() < deadline:
            gui = read(guest, 'INFINITY_NATIVE_GUI_SNAPSHOT', 80)
            python = read(guest, 'INFINITY_PYTHON_SNAPSHOT', 10)
            if gui[2] == gui[79] and not gui[2] & 1 and gui[3] and gui[14]:
                receipt['launch_seconds'] = time.monotonic() - started
                break
            assert python[3] not in (2, 3, 4, 5), {'python': python}
            time.sleep(.2)
        else:
            raise AssertionError({'launch_timeout': python, 'gui': gui})
        time.sleep(3)
        guest.screenshot('fabric-manager-normal')
        receipt['native_gui'] = gui
        receipt['python'] = python
        original_geometry = gui[9:13]
        receipt['window_states'] = []
        for expected, label in ((2, 'maximized'), (0, 'restored')):
            gui = read(guest, 'INFINITY_NATIVE_GUI_SNAPSHOT', 80)
            native.adaptive_click(guest, gui[9]+gui[11]-59*scale, gui[10]-22*scale)
            until = time.monotonic()+30
            while time.monotonic() < until:
                gui = read(guest, 'INFINITY_NATIVE_GUI_SNAPSHOT', 80)
                if gui[2] == gui[79] and not gui[2]&1 and gui[13] == expected:
                    break
                time.sleep(.2)
            else:
                raise AssertionError({'window_state': gui, 'expected': expected})
            time.sleep(2)
            guest.screenshot('fabric-manager-'+label)
            receipt['window_states'].append({'state': gui[13], 'geometry': gui[9:13]})
        assert gui[9:13] == original_geometry
        receipt['complete'] = True
    except BaseException as error:
        receipt['failure'] = repr(error)
        if guest.process is not None and guest.process.poll() is None:
            guest.screenshot('fabric-manager-failure')
            if width is not None:
                guest.key('esc')
                native.adaptive_click(guest, (width-dock_width)//2 + (dock_width//8)//2, height-46*scale)
                guest.wait(lambda state: state[4] == 6, 'diagnostic launcher', timeout=30)
                guest.text('Command Window')
                guest.key('ret')
                time.sleep(2)
                guest.screenshot('fabric-manager-console')
        raise
    finally:
        guest.stop()
        (work / 'evidence.json').write_text(json.dumps(receipt, indent=2) + '\n')

if __name__ == '__main__':
    main()
