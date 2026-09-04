# InfinityOS testing

## Milestone 7.x

Run `make milestone-7x-test` for skin compiler safety, native package integrity,
scale/layout checks, focus and pointer behavior, retained damage, alternate
activation/rollback/SafeSkin, window ownership denial, and install-parity source
checks. Run `./build.sh` for all architecture images and `make install-test` for
fresh x86_64 installation followed by detached-media boot.

## Milestone 6 native AI host acceptance

```sh
make ai-test
make milestone-6-test
make milestone-6-5-test
```

`ai-test` validates model-object integrity/corruption, actual quantized CPU
classification, provider privacy selection, inference deadline/cancellation and
queue saturation, context minimization, Tool Broker capability/revocation,
destructive-plan rejection, microphone lease enforcement, honest unavailable
speech behavior, independent agent tools/budgets, service discovery, and
capability-filtered correlated AI events. It also rejects traditional
filesystem/process/JSON shortcuts in the AI runtime source.

`milestone-6-test` combines that suite with native object-store and Runtime/IOP/
IEF acceptance. VM boot and detached-media installation remain separate tests;
host success must not be described as VM proof.

## Milestone 5.x native installed boot

```sh
make install-boot-test
```

This resets only `build/infinity-test-disk.raw`, drives the real recovery UI, checks generation lifecycle and component verification markers, terminates that VM, and starts a second QEMU command containing only OVMF plus the target disk. It requires native-boot, valid-manifest, valid-kernel, runtime, storage, and Infinity Console markers and fails if the Recovery Node marker appears.

Two follow-up disk copies verify that INSTALLING state and kernel corruption both produce `No valid ACTIVE system generation found` and never reach `Kernel online.`. Individual reruns are `make boot-installed` and `make system-generation-test` after a successful install.
# Milestone 7

Run `make milestone-7-test` for stable-ID, password-verifier, rate-limit,
revocation, cross-user isolation, Personal Space, lock/unlock/logout,
profile-persistence, corruption, interrupted-onboarding, Console-schema, and
service-registry coverage. The test also audits the new native modules for
filesystem, process, socket, JSON/YAML, and credential-exposure shortcuts.
