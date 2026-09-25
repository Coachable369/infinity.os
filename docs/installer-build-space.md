# Installer build storage

The streamed ARM64 release stages under `build/hermes/payload.*`, not the system
temporary directory. Compiler outputs are in repository build directories; the
live build's private Cargo tree is removed after linking. System temporary space
is used for small verification extracts and optional logs, not the model payload.

The previous pipeline allocated fixed 7 GiB installed and 8 GiB live FAT images.
It retained each complete input tree until its image finished copying. Those
overlapping payload copies, compiler products and final ISO could exhaust a
volume with approximately 15 GiB initially free.

The release builder now:

- Sizes FAT images from actual files, with metadata/cluster headroom and alignment.
- Deletes each private staging file only after mcopy succeeds, bounding transient
  duplication to an individual shard rather than an entire payload tree.
- Checks free bytes before build staging, payload sharding, FAT population and
  final ISO creation. These checks are conservative guards, not a reservation;
  unrelated processes can still consume the remaining disk space.
- Rejects simultaneous ARM64 release builds with a build lock.
- Cleans its private tree and partial output on normal/error exit and handled
  signals. An uncatchable kill can leave a lock/staging tree requiring inspection.
- Preserves the old `builds/InfinityOS-aarch64.iso` until the new artifact passes
  binary audio/kernel/loader parity, then replaces it atomically.

No VM disk, model cache or previously published ISO is deleted to obtain space.
Changing `TMPDIR` alone cannot fix this pipeline's storage peaks.

`python3 tools/iso-staging-test.py` exercises actual FAT creation and extraction,
failed-copy source preservation, and structured free-space rejection. It is also
run by `build.sh` before expensive builds. A second real build invocation was
rejected while the first retained the lock.

## Verified September 25

`sh tools/build-hermes.sh` completed successfully on the same constrained volume.
The installed FAT image was about 4.4 GiB rather than 7 GiB. The published ISO
is 5,503,328,256 bytes (about 5.1 GiB), versus the previous 8,590,336,000 bytes.
Binary parity verified the 252,425,992-byte installed kernel, 24,576-byte loader,
and 33,722,129 immutable speech-model bytes. `build/hermes` was empty afterward.
This is artifact proof, not a new physical-VM boot/install test.
