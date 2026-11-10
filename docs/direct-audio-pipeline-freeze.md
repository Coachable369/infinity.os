# Direct audio pipeline freeze

Effective 2026-11-10, by explicit user instruction. The user accepted the working
audio pipeline and requested no further direct audio work. This supersedes prior
authorization to improve or repair that pipeline. Only new explicit user
authorization reopening the affected audio scope permits changes.

This is a repository change policy, not a claim that files are technically
write-protected. The accepted working state is the baseline, including pending
changes present at acceptance. Do not substitute an older commit, reset pending
work, or treat a diagnostic ISO as the accepted release. This policy does not
claim additional testing or target acceptance.

## Protected implementation and assets

The following inventory identifies the direct audio boundary. It is not an
exhaustive filename allowlist: moving code or changing a shared dependency does
not bypass the freeze.

| Area | Protected paths and behavior |
| --- | --- |
| Device capture and playback | `kernel/drivers/audio.rs`, `kernel/drivers/hda.rs`, `kernel/drivers/speech_pcm.rs`, `kernel/drivers/speech_filter.rs`, `kernel/runtime/audio.rs`; device/DMA ownership, buffering, format conversion and timing |
| Conversation pipeline | `kernel/runtime/ai/voice*.rs`, `kernel/runtime/ai/wake_word.rs`, `kernel/runtime/ai/speech_chunk.rs`, `kernel/runtime/ai/pipeline_log.rs`; capture, wake, VAD, noise floor, recognition, echo filtering, duplex interruption, follow-up, synthesis, playback, cancellation and recovery |
| Packaged PCM | `kernel/runtime/ai/voice-pcm-44100.bin`, `kernel/runtime/ai/voice-pcm-48000.bin` |
| Native recognition and synthesis | Production inputs under `tools/voice-whisper/`, `tools/voice-kokoro/`, `tools/voice-flite/`, `tools/voice-native-runtime/`, and `tools/voice_target.py`; native bridges, allocators, worker adapters, compiler settings, source revisions and model pins |
| Dependencies and speech data | `third_party/patches/voice-kokoro/`, `third_party/patches/virtualbox/audio-capture-progress.patch`, `model-cache/kokoro-v1_0.gguf`, `model-cache/whisper-tiny.en.bin`; pinned engines, voice banks, phonemizer data and native speech dependencies, including fetched/cached inputs |
| Direct audio controls and artwork | `kernel/ui/voice_indicator.rs`, `assets/ui-design-kit/default/stop-audio-v1.*`, `assets/ui-design-kit/default/voice-interaction-v2*`, `tools/build-voice-artwork.py`; preserve existing voice controls, state feedback and their interaction contract |

## Protected portions of shared files

Shared files remain usable for unrelated work. Preserve their audio-specific
portions and the effects of changes on audio:

- `kernel/core/main.rs`, `kernel/core/console.rs`,
  `kernel/core/console_session_actions.rs`, and `kernel/drivers/input/{uefi,ps2}.rs`:
  audio initialization, login/logout, polling, microphone authority and cleanup.
- `kernel/core/bootstrap/desktop.rs`, `kernel/ui/ai_widget.rs`, and settings/
  identity code: audio controls, defaults, saved speech/wake preferences,
  reply visibility and the accepted listening/speaking state behavior.
- `kernel/runtime/ai/mod.rs`, `kernel/runtime/ai/qwen/workers.rs`, and adjacent
  generation/service code: voice request ownership (`submit_voice_chat`,
  `submit_chat_from`, `poll_qwen`, `cancel_chat`), text/stream handoff, scheduling,
  bounded queues, cancellation and speech worker interfaces. Do not blanket-freeze
  the LLM modules or `qwen/` tree.
- `boot/common/worker_bridge.h`, `boot/aarch64/handoff.S`, linker scripts,
  `Makefile`, `build.sh`, `build-kit.toml`, `tools/build-qwen.sh`, and installer
  packaging: audio worker resources, private speech memory, native link inputs,
  model/data selection and equivalent live/installed payloads for both targets.

Keep existing audio acceptance cases and evidence intact. Do not weaken checks
or update expected results to conceal a change to the frozen behavior.

## Permitted LLM work

Requested changes to LLM models, inference internals, prompts, grounding,
reasoning/weighted decisions, memory, intent routing, AI Kit, typed Command
Window requests, and application integrations may proceed. They must preserve
the direct audio contract above. Model selection may change the language model;
it does not authorize replacing speech recognition/synthesis models or voices.

Normal requested builds may compile and package unchanged protected inputs.
They do not authorize retuning, asset regeneration from new inputs, dependency
upgrades, altered speech defaults, or removal of the pipeline from an installer.
If permitted work exposes a need for a protected change, leave that portion
untouched, explain the specific dependency, and request explicit authorization.
