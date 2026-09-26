# Native x86-64 speech integration

One shared speech implementation serves ARM64 and x86-64. Native x86 adapters
provide AP startup and a calibrated TSC clock. Firmware enumerates CPUs and
reserves memory before ExitBootServices; APIC INIT/SIPI starts workers afterward.
Inference does not execute on the desktop thread or use a host speech service.

Both x86 kernels link Kokoro, its immutable model/phonemizer resources, private
C/C++ runtimes and PocketSphinx. The shared controller handles phrase boundaries,
microphone interruption, cancellation and off/on state. At least two CPUs and
sufficient memory for the kernel plus the bounded 1 GiB synthesis arena are needed.
Use at least 8 GiB RAM for the installer test VM, which also holds embedded
installation payloads. Kernels above 256 MiB use the bounded 513 MiB object-store
offset; kernels remain capped at 512 MiB. Existing 257 MiB and legacy 128 MiB
store locations remain mountable without migration.

## Verification and remaining gates

- Native guest: three independent AP callbacks after firmware exit, floating
  point, separate stacks and repeat-start rejection passed.
- Native guest: 160 float16/float32 dot-product cases across unaligned rows and
  SIMD boundaries passed.
- Shared tests: five output and five conversation-toggle/interruption tests passed.
- Both linked kernels contain all 366 Kokoro resources and three constructors.
- Focused tests cover the larger kernel layout, object persistence/remount at the
  new offset, and continued access to legacy stores without overwriting them.
- Wall-clock x86 emulation on the ARM host reached inference but cancelled at
  the unchanged 90-second production deadline; no allocation failure occurred,
  and the BSP continued its heartbeat. This is not a native x86 latency result.
- Complete x86 PCM synthesis, audible HDA playback/capture, and detached-media
  installed acceptance remain unverified. Deterministic instruction-time probes
  are correctness checks, not hardware latency claims.
- Conversation start also requires a loaded native LLM. The existing ARM-only
  model-payload loading/packaging path has not been ported by this speech change;
  ordinary x86 ISO conversation must not be claimed functional without that gate.

## Reproduction

All commands run through the build kit:

```sh
./build-kit run python3 tools/voice-kokoro/probe/x86-workers.py
./build-kit run python3 tools/voice-kokoro/probe/run-x86.py --dots-only
./build-kit run python3 tools/voice-kokoro/probe/run-x86.py
./build-kit run make voice-output-test
./build-kit x86_64
```

On a disposable installed x86 machine with native HDA, `voice devices`,
`voice say Hi.`, `voice status`, and `voice stop` exercise standalone speech.
Only test the conversation loop after native model readiness is established.
Do not reset existing user accounts or reuse an unrelated installed test disk.
