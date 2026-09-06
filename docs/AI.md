# Native AI, local ML, voice, and agents

## Architectural rule

AI is a capability-constrained InfinityOS service above the kernel. It proposes
typed plans; deterministic InfinityOS policy validates and executes them. A
model is never an authority, does not receive an arbitrary shell, and cannot
invent operation identifiers.

```text
human request
    -> deterministic command resolver (first)
    -> Context Broker (least required typed context)
    -> Provider Router
    -> local model inference
    -> typed IntentPlan proposal
    -> deterministic consequence/capability policy
    -> existing typed operation dispatcher
```

The implementation is split into `model`, `provider`, `broker`, `intent`,
`voice`, and `agent` modules. `AiRuntime` composes those interfaces for initial
bring-up; inference, context selection, tool authority, and voice state do not
share one untyped state bag.

## Status

| Facility | Status | Evidence or boundary |
|---|---|---|
| Native model object and integrity check | TESTED | Versioned `INFMLM1` binary object, parameter count and CRC-32 validated. |
| CPU intent inference | TESTED | Quantized multinomial model tokenizes arbitrary text and computes class scores locally. |
| Native dialogue generation | TESTED (host) | Two bounded prompt-conditioned compositional models generate distinct local responses for novel prompts. They are not transformer language models. |
| Model registry/install/upgrade/remove | TESTED (host) | Package installation and upgrade are atomic, validate identity/integrity/resource requirements, preserve prior state on failure, and protect CORE models. |
| Model registry/load/capability metadata | TESTED | Bounded registry exposes typed descriptors; all three bootstrap models load without network access. |
| Provider routing/privacy | TESTED | Local is selected for local-only/private requests even when a remote test provider advertises higher quality. |
| Remote provider execution | SCAFFOLDED | Provider-neutral descriptor and policy exist; production registers no remote provider, credentials, or networking. |
| Context Broker | TESTED | Requested context classes must be a subset of the caller's explicitly authorized classes. |
| Tool Broker | TESTED | Closed operation-to-capability mapping, validation at use, and immediate revocation. |
| Intent planning | TESTED | Model output is a typed proposal; unknown/destructive/unconfirmed plans are rejected by OS policy. |
| Inference deadlines/cancellation/backpressure | TESTED | Bounded queue, expired request, cancellation, and saturation cases. |
| Voice session boundary | TESTED (host) | Leased push-to-talk session requires `AudioInput`; state changes are explicit. |
| Speech recognition/synthesis | UNSUPPORTED | Provider interfaces exist; the local provider returns `ProviderUnavailable` and never fabricates a transcript. |
| Agent identity/tools/budget | TESTED (host) | Independent identity, explicit tool set, bounded task queue, and denial outside the set. |
| GPU/NPU inference | UNSUPPORTED | Backend enumeration reports the truth; no accelerator path is claimed. |
| MMU isolation of AI services | SCAFFOLDED | Logical Execution Context and memory ownership exist; hardware page-table isolation is not implemented yet. |

## Model objects and registry

The default System Generation contains three versioned model entries:
`local-intent-v1`, `infinity-dialogue-v1`, and `infinity-creative-v1`.
`local-intent-v1` has its own native System object. The two conversational model
payloads are packed into the native `/system/ai/bootstrap` System object to
preserve the bounded bootstrap object table. They are not host files,
pathname-backed models, JSON documents, or subprocesses. Boot validation checks
every embedded model identity and checksum and rejects corruption. Installed
runtime registration binds each descriptor to its resolved 128-bit Object ID.

The two dialogue engines construct bounded responses from the prompt topic and
independently selected response components. This proves the local conversation,
model selection, packaging, and lifecycle contracts without pretending a large
neural model or GGUF transformer backend exists. Neural GGUF/ONNX execution,
tokenizers, tensor storage, and accelerator execution remain UNSUPPORTED.

`ModelDescriptor` records a stable numeric model ID, native Object ID,
capabilities, runtime adapter, backend class, integrity checksum, memory budget,
and trust/install state. Current model operations have stable IOP identifiers:
`Model.List`, `Model.Inspect`, `Model.Load`, `Model.Unload`,
`Model.Capabilities`, and `Model.Infer`.

The registry's typed package lifecycle supports atomic install, upgrade, and
optional-model removal. A failed memory admission or invalid package leaves the
previous registry state unchanged. Download/catalog UX and cryptographic
third-party package signatures remain PLANNED; an untrusted package cannot be
installed through the current API.

The CPU backend has a bounded request queue. Each request carries workload,
locality/privacy requirements, memory budget, deadline, correlation and
cancellation identifiers. Large future tensors must travel by ObjectRef or
shared-buffer capability rather than oversized IOP payloads.

## Provider and privacy policy

Provider selection filters by model capability, availability, data locality,
privacy policy, and backend support before applying quality preference.
`RequireLocal` and private-local context cannot fall through to a remote
provider. A remote provider remains optional and cannot be a boot dependency.
InfinityOS remains usable with no model service because exact commands and typed
operations bypass inference.

## Context and tool brokers

The Context Broker issues a bounded typed context view containing only requested
classes such as system summary or service state. Raw Object Store, namespace,
microphone, personal data, and unrestricted history are not ambient inputs.

The Tool Broker accepts only closed `OperationId` values and maps each to its
required capability. Tool execution still crosses the ordinary capability and
dispatcher boundary. Model confidence never grants authority.

## Voice

Voice is layered as audio capability, explicit capture session, speech provider,
intent resolution, confirmation, and typed execution. Capture is push-to-talk,
lease bounded, and observable as `Idle`, `Listening`, `Processing`, or `Failed`.
The current release does not have native audio capture or a speech model, so it
reports those facilities as unsupported instead of simulating recognition.

## Agents

An agent is an Execution Context participant with its own stable identity,
allowed tool IDs, memory and CPU budgets, deadline, and bounded task queue. It
does not inherit the human's authority. Agent-to-service activity is designed to
use IOP and IEF, not console automation.

## Events and observability

The schema reserves typed AI provider/model/inference, voice, and agent event
IDs. Inference start, completion, and failure StateChange events are implemented
with correlation/causation and capability-filtered publication. Voice/agent
event publication and a persistent general audit stream are SCAFFOLDED.
Counters expose accepted/completed/failed inference requests and last data
locality. No performance number is documented unless produced by a test run.

## Installed and live profiles

Both profiles use the same runtime modules. The native service registry contains
Local ML, Infinity AI, Voice, and Agent services. A clean installation writes
and verifies the three model entries across two native System objects, their AI
bootstrap references, the voice framework object, agent policy object, and
updated service registry. No network download is required. Installed-disk boot
must resolve these objects from System Space with the ISO detached.
