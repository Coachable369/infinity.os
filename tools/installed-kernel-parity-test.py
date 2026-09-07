"""Behavioral artifact parity: each installer embeds the exact installed ELF."""
from pathlib import Path
import mmap

# ------------------------=
# FUNC: main
# DESC: Checks architecture and byte-identical installed kernel payloads inside both linked installers.
# ------------------=
def main():
    root = Path(__file__).resolve().parent.parent
    for architecture, machine in [('aarch64', 183), ('x86_64', 62)]:
        installed = (root / 'build' / architecture / 'installed-kernel.elf').read_bytes()
        assert installed[:6] == b'\x7fELF\x02\x01'
        assert int.from_bytes(installed[18:20], 'little') == machine
        with (root / 'build' / architecture / 'kernel.elf').open('rb') as stream:
            with mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as live:
                assert live.find(installed) >= 0, architecture
        print({'architecture': architecture, 'installed_bytes': len(installed), 'parity': True})

if __name__ == '__main__':
    main()
