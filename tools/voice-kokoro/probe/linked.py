"""Build a diagnostic entry around an unchanged linked ARM installed image."""
from pathlib import Path
import struct
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]

# ------------------------=
# FUNC: main
# DESC: Resolves production symbols and appends one isolated diagnostic segment without changing existing loadable bytes.
# ------------------=
def main():
    out = ROOT / "build/voice-kokoro/vbox"
    source = ROOT / "build/aarch64/installed-kernel.elf"
    names = {"SPEECH_WORKER": "voice_output6worker", "WORKER_ENTRY": "workers12worker_entry",
             "BACKGROUND": "workers10background"}
    for name in ("STATE", "TEXT", "LENGTH", "RATE", "DEADLINE", "FRAMES", "PCM", "PEAK", "ERROR"):
        names[name] = "voice_output" + str(len(name)) + name
    symbols = subprocess.check_output(["/opt/homebrew/opt/llvm/bin/llvm-nm", str(source)], text=True).splitlines()
    definitions = []
    resolved = {}
    for name, suffix in names.items():
        matches = [int(row.split()[0], 16) for row in symbols if row.endswith(suffix) or row.endswith(suffix + ".0")]
        assert len(matches) == 1, (name, matches)
        definitions.append(f"#define {name} 0x{matches[0]:x}ULL")
        resolved[name] = matches[0]
    original = source.read_bytes()
    capture = "--capture" in sys.argv
    definitions.append(f"#define KERNEL_ENTRY 0x{struct.unpack_from('<Q',original,24)[0]:x}ULL")
    (out / "linked-symbols.h").write_text("\n".join(definitions) + "\n")
    subprocess.run(["/opt/homebrew/opt/llvm/bin/clang", "--target=aarch64-none-elf", "-O2", "-ffreestanding",
        "-fno-stack-protector", "-fno-builtin", "-mgeneral-regs-only", *(["-DCAPTURE_NATIVE"] if capture else []), "-I", str(out), "-c",
        str(ROOT / "tools/voice-kokoro/probe/linked.c"), "-o", str(out / "linked.o")], check=True)
    objects = [str(out / "linked.o")]
    return_sites = []
    if capture:
        # Diagnostic-only return guards preserve x0-x15 and all callee-saved
        # registers; x16/x17 and flags are caller-clobbered under AAPCS64.
        for offset in range(0x10000, 0x10000 + 0x130000, 4):
            if struct.unpack_from('<I', original, offset)[0] == 0xd65f03c0:
                return_sites.append((offset, 0x10000000 + offset - 0x10000))
        guards = ['.section .ret_probes,"ax"', '.balign 4']
        for _, site in return_sites:
            guards += [f'movz x16,#{site & 65535}', f'movk x16,#{site >> 16},lsl #16',
                       'lsr x17,x30,#28', 'cmp x17,#1', 'b.eq 1f', 'brk #0x513', '1: ret']
        (out/'returns.S').write_text('\n'.join(guards)+'\n')
        subprocess.run(['/opt/homebrew/opt/llvm/bin/clang','--target=aarch64-none-elf','-c',str(out/'returns.S'),'-o',str(out/'returns.o')],check=True)
        objects.append(str(out/'returns.o'))
    if capture:
        patch_address = resolved["WORKER_ENTRY"] + 12
        phoff = struct.unpack_from("<Q", original, 32)[0]
        phsize, count = struct.unpack_from("<HH", original, 54)
        for i in range(count):
            ph = struct.unpack_from("<II6Q", original, phoff+i*phsize)
            if ph[0] == 1 and ph[3] <= patch_address < ph[3]+ph[5]:
                patch_offset = ph[2]+patch_address-ph[3]
                break
        instructions = struct.unpack_from("<4I", original, patch_offset)
        assert instructions == (0xa9ba7bfd,0xa9016ffc,0xa90267fa,0xa9035ff8), instructions
        assembly = '.text\n.globl capture_worker\ncapture_worker:\nmsr daifset,#15\nldr x16,=linked_vectors\nmsr vbar_el1,x16\nisb\n'
        assembly += '\n'.join(f'.inst 0x{value:x}' for value in instructions)
        assembly += f'\nldr x16,={patch_address+16}\nbr x16\n'
        (out / 'capture.S').write_text(assembly)
        subprocess.run(['/opt/homebrew/opt/llvm/bin/clang','--target=aarch64-none-elf','-c',str(out/'capture.S'),'-o',str(out/'capture.o')],check=True)
        objects.append(str(out/'capture.o'))
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "-Ttext=0xc0000000", "--section-start=.ret_probes=0x0f000000", "-e", "linked_entry",
        *objects, "-o", str(out / "linked.elf")], check=True)
    shim = (out / "linked.elf").read_bytes()
    image = bytearray(source.read_bytes())
    if capture:
        for index,(offset,site) in enumerate(return_sites):
            destination=0x0f000000+index*28
            delta=destination-site
            assert -(1<<27)<=delta<(1<<27) and delta%4==0
            struct.pack_into('<I',image,offset,0x14000000|((delta//4)&0x03ffffff))
        shim_symbols = subprocess.check_output(['/opt/homebrew/opt/llvm/bin/llvm-nm',str(out/'linked.elf')],text=True)
        target = [int(row.split()[0],16) for row in shim_symbols.splitlines() if row.endswith(' capture_worker')]
        assert len(target) == 1
        struct.pack_into('<IIQ',image,patch_offset,0x58000050,0xd61f0200,target[0])
    phoff = struct.unpack_from("<Q", image, 32)[0]
    phsize, count = struct.unpack_from("<HH", image, 54)
    shim_phoff = struct.unpack_from("<Q", shim, 32)[0]
    shim_phsize, shim_count = struct.unpack_from("<HH", shim, 54)
    for i in range(shim_count):
        ph = list(struct.unpack_from("<II6Q", shim, shim_phoff + i*shim_phsize))
        if ph[0] != 1 or (ph[3] < 0xc0000000 and ph[3] != 0x0f000000):
            continue
        image.extend(bytes((-len(image)) % 65536))
        offset = len(image)
        image.extend(shim[ph[2]:ph[2]+ph[5]])
        ph[2] = offset
        assert phoff+(count+1)*phsize < 65536
        struct.pack_into("<II6Q", image, phoff+count*phsize, *ph)
        count += 1
    struct.pack_into("<H", image, 56, count)
    struct.pack_into("<Q", image, 24, struct.unpack_from("<Q", shim, 24)[0])
    (out / "linked-kernel.elf").write_bytes(image)

if __name__ == "__main__":
    main()
