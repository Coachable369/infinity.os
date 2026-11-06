# Media Player

Open **Media Player** in the launcher and choose a personal file in Library, or double-click that file in File Navigator to start playback. Space pauses or resumes. The position slider seeks, Previous and Next move through your library, and Volume controls the player's output.

The player decodes MP3, PCM WAV, and FLAC audio. MP4, M4A, and MOV containers support AAC, ALAC, MP3, H.264, MPEG-4 Part 2, and MJPEG streams. Each media object may contain up to 4 MiB, the current Object Store download limit. The library includes your personal media and shared Documents media readable by your account. Video is scaled proportionally to fit 512 × 288. Output is local to the guest.

The native Python process owns the decoder, file snapshot, and UI. The kernel checks the signed-in session and the immutable player entry point before granting file access or speaker output. The build compiles a decoder-only subset of [FFmpeg 7.1.3](https://ffmpeg.org/releases/ffmpeg-7.1.3.tar.xz). The exact configuration and checksum are in `tools/native-python/media_build.py`. The linked decoder libraries, individual Python object files, and link command are reproducible with `./build-kit run make builds/toolchains/aarch64/python.elf` or the equivalent x86_64 target. The packaged Python license file includes the FFmpeg license.

# Disk

Type `disk` in the local Command Window. The first view shows the mounted system disk, its GPT partitions, and proportional colored bars. Select a partition with the arrows and press Enter. The Infinity container shows space used and personal files ranked by size. FAT16/32 partitions provide a bounded read-only short-name listing. Other partition types show their size and starting sector.

Enter on a file shows the full name and exact byte size. **D**, then **Y**, moves the selected owned file to Trash. **U** restores a Trash entry. Pressing **D**, then **Y**, on a Trash entry deletes it and collects unreferenced storage blocks. **R** refreshes, **Left** returns to the previous view, and **Q** returns to the terminal.

## Bundled examples

`/System/Infinity-Chime.wav`, `/System/Infinity-Melody.mp3`, and
`/System/Infinity-Motion.mp4` are original six-second examples, included in both
live and installed system catalogs. Open Media Player using the existing Media
icon, then Library; or navigate File Navigator to `/System` and double-click any
example to play it. The MP4 contains H.264 video and AAC audio. Samples are
read-only shared resources; personal media permissions are unchanged.

Regenerate examples with `./build-kit run python3 tools/build-media-samples.py`.
The arpeggio is synthesized locally and the video reuses the packaged Media
artwork. No downloaded music or videos are required.

The disk overview positions partitions by their actual LBA extents. Its separate
pool bar measures allocated object-store blocks, not partition reservations or
file logical sizes. Non-pool filesystem usage is not inferred. In short windows
the diagram collapses so partition navigation remains available.
