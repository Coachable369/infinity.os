# Milestone 6 conformance record

## Architecture delivered

InfinityOS now has modular native Local ML, Infinity AI, Voice, and Agent
service contracts above the Runtime Core. Model Registry, Provider Router,
Context Broker, Tool Broker, intent planning, voice sessions, and agent lifecycle
are separate typed modules. The kernel remains limited to execution, scheduling,
IPC, capability, memory, interrupt, and accounting mechanisms.

The local development model is a versioned native System object containing
serialized quantized parameters. CPU inference evaluates those parameters over
tokenized arbitrary input and returns a typed class/result; it is not a response
lookup table. Installed systems bind Model Registry metadata to the persisted
Object ID and validate both Object Store CRC and model-parameter checksum.

No filesystem, process, shell, JSON wire format, host-network bridge, or vendor
SDK was introduced. Exact commands bypass AI. Model proposals contain only
closed IOP operation IDs and remain subject to OS consequence and capability
policy.

## Status matrix

| Area | x86 | x86_64 | AArch64 |
|---|---|---|---|
| Compile and boot ISO | TESTED | TESTED | TESTED in QEMU; VirtualBox ISO built |
| Local CPU intent inference | TESTED in QEMU | TESTED in QEMU | TESTED in QEMU |
| Model/provider/context/tool host suite | TESTED | TESTED | Architecture-neutral code TESTED on host |
| Clean install and detached-media boot | UNSUPPORTED | TESTED | IMPLEMENTED BUT UNTESTED in this milestone |
| Native model Object persistence | UNSUPPORTED | TESTED on raw artifact and detached boot | IMPLEMENTED BUT UNTESTED |
| GPU/NPU inference | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Native audio and speech model | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Remote provider | SCAFFOLDED | SCAFFOLDED | SCAFFOLDED |
| MMU-separated service contexts | SCAFFOLDED | SCAFFOLDED | SCAFFOLDED |

## Verification commands

```sh
make milestone-6-test
./build.sh
make test-console
make install-test
make installed-object-test
make installed-console-test
make system-generation-test
```

The host suite covers model corruption, real inference, resource rejection,
deadline/cancellation/backpressure, provider locality/privacy/approval, least
context, Tool Broker revocation, prompt-injection text, OS-owned destructive
confirmation, live microphone revocation, honest speech unavailability, agent
tool and queue bounds, AI service crash/restart identity replacement, and
correlated capability-filtered inference events.

The VM suite proves the local model handles a previously unresolved natural
request on all three architectures. The x86_64 flow then installs to a blank raw
disk, verifies the native model and service objects, performs firmware reboot,
boots with no ISO device, inspects the installed model binding, executes local
inference, and rejects incomplete or corrupt generations.

Performance values printed by `ai-test` and `runtime-test` are host bring-up
measurements, not production benchmarks. They include local inference average,
IOP round-trip, event delivery, and service startup; results must be reported
from the current run rather than copied as constants.

## Known limitations and temporary compromises

- Services are distinct logical Execution Contexts but remain statically linked
  and colocated; page-table isolation and preemptive scheduling are not complete.
- The native CPU model proves classification/intent inference only. General
  reasoning, embeddings, coding, vision, STT, and TTS require future model
  adapters and assets.
- The voice capability/session boundary is tested, but no hardware capture
  driver or speech model is claimed.
- Agent identities, typed tasks, tool bounds, cancellation, and result envelopes
  are tested on the host. Autonomous service execution is not enabled.
- A general durable AI audit stream is SCAFFOLDED; inference StateChange events
  and existing security Record infrastructure are implemented.
- Format v3 bootstrap tables remain bounded. Persistent tree indexes and modern
  cryptographic content hashes remain future storage work.

## Next recommended milestone

Complete MMU-backed service separation and a native audio-service/driver path,
then integrate a small redistributable offline STT model through the existing
adapter, capability, ObjectRef, IOP, event, and resource contracts. Networking
and remote providers should remain later work until native network isolation and
privacy approval are available.
