# VirtualBox ARM microphone progress without pointer activity

## Confirmed failure

The 1 ms HDA polling limit did not fix the installed failure. The retained
session showed continued capture-authority renewals while PCM stopped arriving,
and VirtualBox accumulated 542.84 seconds of device-timer debt.

A native, isolated eight-vCPU diagnostic reproduced the failure. It uses the
production HDA backend and boot bridge, records numeric counters rather than
speech, and never uses a host speech service. With all seven secondary CPUs
claimed as native workers, capture stopped after about two seconds. Removing or
throttling USB polling, periodic SEV, and bounded waits on the main CPU did not
restore capture. A diagnostic timed wait on CPU7 immediately restored progress.

VirtualBox's ARM execution loop polls device timers before entering its inner
guest-execution loop. Ordinary successful exits need not leave that loop; a
genuine halt does. This supports the observed starvation on the timer-owning
secondary CPU, rather than a wake-word or microphone-permission failure.
[Oracle source](https://github.com/VirtualBox/virtualbox/blob/main/src/VBox/VMM/VMMR3/target-armv8/NEMR3Native-darwin-armv8.cpp).

## Scoped correction

On checksum-validated VirtualBox ARM ACPI topology (`ORCLVB`), leave the highest
eligible secondary CPU under firmware control instead of claiming it for the
native compute pool. Both MP Services and PSCI launch paths apply the same
affinity filter. Other ARM platforms and x86 retain their existing policy.
There is no host-OS runtime dependency, injected pointer event, speech delay,
timer-register takeover, or change to the shared voice/AI implementation.

The normal eight-vCPU VM retains the main CPU and six native compute workers.
The policy must not remove the only compute worker on a two-vCPU system; that
configuration is not covered by this microphone acceptance.

## Native evidence

With the highest CPU unclaimed, a single uninterrupted 40.001-second test
delivered 1,764,000 frames at 44.1 kHz, with a maximum no-sample interval of
106 ms. It executed 99,570 firmware HID calls with zero mouse reports, without
host input or debugger intervention. Evidence is retained in
`build/arm-capture-timer-probe/reserved-ap-evidence.json`.

The actual patched production loader then passed the same test: it requested
seven workers, started six on CPUs 1–6, and delivered all 1,764,000 expected
frames over 40.000003 seconds. The maximum no-sample interval was 179 ms;
96,594 HID calls produced zero mouse reports. No timer-register writes, host
pointer input, or debugger intervention were used. Structured evidence is in
`build/arm-capture-timer-probe/production-policy-evidence.json`; the run manifest
is `builds/manifests/20261004T173604012296Z-12298.json`.

The mandatory voice regression gate passed, including executable topology and
actual MP-launcher tests for CPU eligibility, sparse/unsorted affinities,
corrupt ACPI, other platforms, and small systems. Its manifest is
`builds/manifests/20261004T173755180883Z-12988.json`.

This verifies native capture progress, not recognition accuracy or complete
conversational latency. The probe is a disposable native guest, not an installed
desktop acceptance run. Loader-policy verification and full-bundle ISO packaging
use the repository build kit; ISO production goes through
`./build.sh --target aarch64` with binary live/installed loader parity checks.
