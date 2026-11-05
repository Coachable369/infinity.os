"""Build a decoder-only native FFmpeg subset for the InfinityOS media player."""
import hashlib
import pathlib
import subprocess
import tarfile
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]
VERSION = "7.1.3"
DIGEST = "f0bf043299db9e3caacb435a712fc541fbb07df613c4b893e8b77e67baf3adbe"

# ------------------------=
# FUNC: codec_libraries
# DESC: Compiles bounded local-file codecs for the target ABI with network protocols and host devices disabled.
# ------------------=
def codec_libraries(arch, llvm, newlib):
    cache = ROOT / "build/media-source"
    cache.mkdir(exist_ok=True, parents=True)
    archive = cache / f"ffmpeg-{VERSION}.tar.xz"
    if not archive.exists():
        with urllib.request.urlopen(f"https://ffmpeg.org/releases/{archive.name}", timeout=120) as response:
            archive.write_bytes(response.read())
    if hashlib.sha256(archive.read_bytes()).hexdigest() != DIGEST:
        raise ValueError("FFmpeg source checksum mismatch")
    source = cache / f"ffmpeg-{VERSION}"
    if not source.exists():
        with tarfile.open(archive) as bundle:
            bundle.extractall(cache, filter="data")
    out = cache / arch
    out.mkdir(exist_ok=True)
    libraries = ("avformat", "avcodec", "swresample", "swscale", "avutil")
    archives = [out / f"lib{name}/lib{name}.a" for name in libraries]
    flags = [f"--target={arch}-none-elf", "-fPIC", "-ffunction-sections", "-fdata-sections",
             f"-I{newlib}/targ-include", f"-I{newlib}",
             f"-I{ROOT}/build/newlib-4.6.0.20260123/newlib/libc/include"]
    flags += ["-mno-red-zone"] if arch == "x86_64" else ["-mno-outline-atomics", "-mstrict-align"]
    # Configure only link-probes newlib. The final application links the real
    # InfinityOS implementations from sdk/compiler, never these probe addresses.
    probes = " ".join("-Wl,--defsym=" + name + "=0x1000" for name in
                      "_sbrk _read _write _close _lseek _fstat _isatty _exit _kill _getpid".split())
    with (out / "codec-build.log").open("w") as log:
        if not (out / "config.h").is_file() or (out / "config.h").stat().st_mtime < pathlib.Path(__file__).stat().st_mtime:
            args = [str(source/"configure"), f"--arch={arch}", "--target-os=none", "--enable-cross-compile",
                    f"--cc={llvm}/clang", f"--ar={llvm}/llvm-ar", f"--ranlib={llvm}/llvm-ranlib",
                    f"--nm={llvm}/llvm-nm", "--extra-cflags="+" ".join(flags),
                    f"--extra-ldflags=--target={arch}-none-elf -fuse-ld=lld -nostdlib -Wl,-e,main -L{newlib} {probes}", "--extra-libs=-lm -lc",
                    "--disable-autodetect", "--disable-everything", "--disable-programs", "--disable-doc",
                    "--disable-network", "--disable-asm", "--disable-pthreads", "--disable-w32threads",
                    "--disable-os2threads", "--disable-avdevice", "--disable-avfilter", "--disable-postproc",
                    "--disable-iconv", "--enable-pic", "--enable-small", "--enable-swscale", "--enable-swresample",
                    "--enable-demuxer=wav,mp3,mov,flac", "--enable-parser=aac,h264,mpegaudio,mpeg4video,flac",
                    "--enable-decoder=mp3float,aac,h264,mpeg4,mjpeg,alac,flac,pcm_u8,pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le"]
            subprocess.run(args, cwd=out, stdout=log, stderr=subprocess.STDOUT, check=True)
        subprocess.run(["make", "-j6", *(str(p.relative_to(out)) for p in archives)],
                       cwd=out, stdout=log, stderr=subprocess.STDOUT, check=True)
    return source, archives
