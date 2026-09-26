# Voice Kokoro native-port patches

These patches preserve unfinished native-port work independently of disposable
build trees. They are not part of the current installed voice backend.

Pinned upstream bases:

- LLVM project: `ea7d852a70e8bdfaf601d6626a760f9771b2c4b4`
- eSpeak NG: `4870adfa25b1a32b4361592f1be8a40337c58d6c`
- GGML: `36da57138425487184aa1da2eee2cde155909c6f`

Apply a patch from the corresponding clean upstream checkout with
`git apply <patch>`. The patch files include new GGML OpenCL kernels and are the
durable source of truth; generated clones and objects belong under `build/`.
