# Native x86-64 speech integration

One shared speech implementation serves ARM64 and x86-64. Native x86 adapters
provide AP startup and a calibrated TSC clock. Firmware enumerates CPUs and
reserves memory before ExitBootServices; APIC INIT/SIPI starts workers afterward.
Inference does not execute on the desktop thread or use a host speech service.

Both x86 kernels link Kokoro, its immutable model/phonemizer resources, private
C/C++ runtimes and PocketSphinx. The shared controller handles phrase boundaries,
microphone interruption, cancellation and off/on state. At least two CPUs and
sufficient memory for the kernel plus the bounded 1 GiB synthesis arena are needed.
Use at least 16 GiB RAM for the model-inclusive installer test VM. Before firmware
exit the x86 loader reserves bounded immutable installer shards in RAM; its
runtime payload callback never calls firmware. Installed boot loads Hermes and
optional Ministral from the installed ESP before firmware exit, using the same
model verification/runtime as ARM64. Kernels above 256 MiB use the bounded 513 MiB object-store
offset; kernels remain capped at 512 MiB. Existing 257 MiB and legacy 128 MiB
store locations remain mountable without migration.

## Verification and remaining gates

- Native guest: three independent AP callbacks after firmware exit, floating
  point, separate stacks and repeat-start rejection passed.
- Native guest: 205 float16/float32 dot-product and rejection cases across
  unaligned rows, SIMD boundaries and bit-identical four-row tiles passed.
- Shared tests: five output and five conversation-toggle/interruption tests passed.
- Both linked kernels contain all 366 Kokoro resources and three constructors.
- Focused tests cover the larger kernel layout, object persistence/remount at the
  new offset, and continued access to legacy stores without overwriting them.
- Wall-clock x86 emulation on the ARM host reached inference but cancelled at
  the unchanged 90-second production deadline; no allocation failure occurred,
  and the BSP continued its heartbeat. This is not a native x86 latency result.
- Profiling attributed over 98% of measured work to matrix multiplication.
  Shared four-row tiling advanced the instruction-timed guest from 166 to 192
  completed matrix operations before cancellation. A normal wall-clock retry
  still returned cancellation and zero frames; the optimization is not proof
  of complete synthesis or usable native latency.
- Complete x86 PCM synthesis, audible HDA playback/capture, and detached-media
  installed acceptance remain unverified. Deterministic instruction-time probes
  are correctness checks, not hardware latency claims.
- Model-payload loading and packaging now share the architecture-selected build.
  Cache bounds and reads after firmware exit are guest-tested; actual model
  inference after detached-media installed boot remains a separate acceptance gate.
- The model-inclusive x86 ISO build passed extracted Hermes/Ministral hashes,
  installed-kernel and bootloader byte comparisons, native speech-resource parity,
  and cursor-payload parity. This is artifact proof, not installed execution.

## Reproduction

All commands run through the build kit:

```sh
./build-kit run python3 tools/voice-kokoro/probe/x86-workers.py
./build-kit run python3 tools/voice-kokoro/probe/run-x86.py --dots-only
./build-kit run python3 tools/voice-kokoro/probe/run-x86.py
./build-kit run python3 tools/voice-kokoro/probe/run-x86.py --clock realtime
./build-kit run make voice-output-test
./build-kit run make payload-cache-test
./build-kit x86_64-models
./build-kit x86_64
```

On a disposable installed x86 machine with native HDA, `voice devices`,
`voice say Hi.`, `voice status`, and `voice stop` exercise standalone speech.
Only test the conversation loop after native model readiness is established.
Do not reset existing user accounts or reuse an unrelated installed test disk.
