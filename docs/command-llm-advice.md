# Local LLM commands and decision advice

Command Window accepts:

```
ai ask Explain the difference between a daemon and a scheduled task
llm What information would help diagnose an intermittently failing script?
ai decide A runs every minute at 5% CPU; B runs hourly at 20% CPU. Freshness matters more than latency. Compare them.
ai decide weights=80,20 scores=9,4/4,9
ai response
ai cancel
ai new
```

`llm ask`, `llm decide`, `llm response`, `llm cancel`, and `llm new` are aliases.
Command words accept spaces or tabs; prompt content keeps its internal spacing.
`ai help` and `llm help` work even when chat is disabled. Controls reject extra
arguments; use `ai ask PROMPT` to send text that starts with a control word.
Command Window has a 512-byte input line, including the command words. The AI
widget accepts up to 4096 bytes; use it for longer questions.

The selected Hermes or Ministral model produces the answer locally. A missing
model or an active generation returns an admission error. Command Window remains
usable while inference runs; the completed response is delivered to the initiating
terminal once. `ai response` displays that command session's current reply and
`ai cancel` cancels only its still-current generation. A newer widget or voice turn
cannot be mistaken for the terminal request. Lock, logout, identity changes, and
AI permission revocation invalidate pending terminal delivery.

`ai new` clears model context and transcript only for this session's still-current,
idle typed advice turn. It preserves saved memory, selected model, preferences,
and the unsent widget draft. Active inference, voice generation/playback,
application work, another session, or a newer non-advisory turn prevents the
reset. An exhausted typed context can then accept a fresh request. Use `ai cancel`
first if your command request is still running.

The AI widget uses the same advisory endpoint for `decide ...`, `help me decide
...`, `should I ...`, `which is better ...`, `compare ...`, and other explicit
comparative questions. Ordinary widget actions retain the existing authorized
intent dispatcher. Both `ai ask` and `llm` also recognize these decision questions.
Admission failures appear as a separate "Request not sent" notice with a specific
recovery instruction; they preserve the draft, previous answer, and any active
generation. The notice clears when a request succeeds or a newer turn starts.

Decision context combines the user's question and stated constraints with selected
personal memory and a labelled native telemetry snapshot. An unavailable RAM/CPU
sample is marked unavailable. The guidance asks the model to compare options,
respect hard constraints, calculate supplied weighted scores, distinguish facts
from assumptions, identify uncertainty, and explain what could change its choice.
For exact numeric comparisons, supply `weights=80,20 scores=9,4/4,9`:
weights total 100, rows are options, columns are criteria, and higher scores are
better. Two to eight options and one to eight criteria are supported; weights
are whole numbers from 0 to 100, and scores are whole numbers from 0 to 1000. The native decision
helper calculates exact weighted sums and supplies them as verified evidence to
the model. `eligible=0,1` excludes option 1 under a supplied hard constraint. Ties
remain ties. Missing or invalid weights, scores, eligibility, and zero eligible
options return distinct errors. Explicit natural-language matrices are supported too, for example `Cost weight
80%, reliability weight 20%. Option 1: cost 9, reliability 4. Option 2: cost 4,
reliability 9.` Criteria use single-word names, weights total 100, and scores are
nonnegative whole numbers with higher values preferred. Numbered definitions use
`Option 1:` or `Option 1 cost 9, ...`; later references such as "explain why option
1 is better" are not duplicate definitions. Unstructured arithmetic
outside these supported forms remains model-dependent; unsupported or incomplete
matrices are not filled in with invented values.

It does not invent external evidence or perform web retrieval. Advice is not an
execution grant: even an action-shaped prompt cannot admit an OS action through
this endpoint. The direct audio pipeline and its generation handoff are unchanged.

The shared kernel contains these modules, so existing installed-kernel byte parity
covers packaging. Host contracts exercise typed request parsing/admission and
session/turn ownership. The optional real-Hermes acceptance runs weighted choices
with inverted priorities and a hard budget constraint, asserts a strict `CHOICE:N`
protocol, and verifies no OS control is staged. Those results demonstrate
bounded cases, not general reasoning correctness or installed UI behavior.

Run with the repository build kit:

```
./build-kit run env CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin llm-advice-test
./build-kit run env CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin llm-advice-test -- --native
```
