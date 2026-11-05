# Examples for intent-driven development

`catalog.json` is the machine-readable selection index. Paths are relative to
`/InfinitySDK`. Read root and language `AGENTS.md` instructions before adapting
an example. Copy files into your workspace; never modify the installed SDK.

| Request or task | Starting point | Adaptation and acceptance |
| --- | --- | --- |
| Create a native app controlled by voice or clicks | Python/counter.py or C/counter.c | Replace domain logic; verify click and AI each change state exactly once |
| Create a calculator | C/calculator.c | Preserve validated operations and error handling; check arithmetic results |
| Create a terminal program | Python/hello.py or C/hello.c | Replace output with requested behavior; check exit status and output |
| Add diagnostic logging | Python/logging_demo.py or C/logging_demo.c | Use bounded nonsecret fields; decode JSON records |
| Connect to an HTTPS service | Python/network_head.py or C/network_head.c | Request permission; validate host, verified TLS and cleanup; HEAD is not a full HTTP parser |
| Generate an audio asset | Python/audio_tone.py | Check WAV metadata and samples; playback is a separate capability |
| Capture a camera frame | C/camera.c | Obtain permission, complete paged capture, close resources; verify on a real camera |

Example prompts: “Create a Python counter with Reset and AI controls”; “Add
structured error logging to this app”; “Update my app to fetch headers from a
user-specified HTTPS host, asking before network access.” These are adaptation
tasks, not a phrase-to-command lookup table.

For repair, retrieve the user's current source and diagnostics rather than
overwriting it with an example. Keep a small acceptance checklist for the
requested behavior. Repository checks cover selected examples and contracts;
consult verification results rather than assuming every device workflow has
been exercised on the user's machine.
