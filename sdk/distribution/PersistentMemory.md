# AIKit persistent memory

## Quick start (Command Window or authenticated SSH)

Run each command separately. IDs are returned by `remember` and `search`; substitute
those actual IDs in later commands.

```text
ai memory status
ai memory remember Ollie likes morning walks
ai memory search Ollie
ai memory inspect 1
ai memory update 1 to Ollie likes evening walks
ai memory forget 1
ai memory search Ollie
```

`infinity memory` is an alias. `memory status` remains the RAM diagnostic.
`help` includes **ai memory** with permissions, examples and expected results.
Saving returns an ID only after durable commit. Forgetting removes the record from
both the checkpoint and active search index. The final search returns no matching
record. Ambiguous query deletion is refused; select an ID explicitly.

The desktop assistant also accepts “Infinity, remember that Ollie likes walks”,
“what do you remember about Ollie”, “update memory 1 to ...”, and “forget memory 1”.
Other phrasings can be proposed by the existing LLM intent engine using
`OS_MEMORY:remember TEXT`, `search QUERY`, `inspect ID`, `update ID to TEXT`, or
`forget ID`. Native parsing and the existing authorized dispatch execute them.
The model must search first to obtain IDs; it must not invent IDs or claim success
before the native receipt. Recalled procedures never authorize execution.

## Native API

This is an internal, freestanding Rust AIKit API, not a separate service or database.
All components use the same `AiRuntime` catalog and IdentitySystem session:

```rust
// Inside an existing authorized native kit handler, with current identity/session:
let receipt = ai.memory_command(owner, session,
    memory_request::Command::Remember(b"Ollie likes walks"), &mut output);
// Check receipt.outcome; Failed is never success. receipt.length bounds output.
let n = ai.recall_memory(owner, session, b"Ollie", 256, &mut context)?;
// Append context[..n] once as untrusted data, not system instructions.
```

Lower-level `memory_api::Engine` supports `execute`, `search`, `inspect`, and
`context` with a `Backend` enforcing live session authorization on every call.
`Operation` covers status, search, inspect, store, update and delete. The production
`Native` backend delegates to the existing ObjectStore and IdentitySystem. Host
tests use an explicit fixture backend; host production access is unavailable.

IntelligenceKit and DevKit use the existing AIKit prompt builder. Application
source/advice, application decisions, desktop chat and SSH advice add a single
bounded recall block; unavailable retrieval leaves normal inference intact.
Python/C applications do **not** have a new private-memory syscall. They may not
bypass owner policy via generated application actions. Do not invent Python/C
bindings or read the checkpoint as a normal file. Existing application APIs remain
unchanged. The Rust behavioral example is in `Examples/NativeMemoryHarness.rs`.

## Records and storage format

Each record has a stable monotonically increasing ID, immutable owner identity,
category (Episodic=1, Semantic=2, Procedural=3), original UTF-8 content, source,
creation/update UTC seconds, and optional embedding model ID/vector. A zero
timestamp explicitly means the clock was unavailable. Updates preserve ID and
creation time and discard stale embeddings. Exact case-insensitive ASCII duplicate
content within a category retains its original record; no autonomous merging.

Version 1 checkpoint is at `/system/ai-memory/<32 lowercase hex owner bytes>`.
It is protected native metadata named `@aikit-memory`, not a public document.
The binary layout uses little-endian integers:

- 8-byte header: `IMEM`, u16 version=1, two reserved zero bytes.
- 16-byte owner, u64 next ID, u16 record count.
- Per record: u64 ID, u8 category, u64 created, u64 updated; u16 content length,
  u8 source length, u8 model length, u16 vector dimensions; then content, source,
  model bytes and f32 vector components.
- CRC32 over all preceding bytes. This detects corruption; it is not encryption
  or a cryptographic authenticity claim. IdentitySystem enforces access.

Limits: 16 records per owner, 512 content bytes, 64 source bytes, 32 model-ID bytes,
256 vector dimensions and a 16 KiB total serialized checkpoint. Full catalogs
return capacity errors; they never silently evict. Embeddings consume checkpoint
space, so fewer maximum-sized embedded records may fit. Unknown versions,
truncation, invalid UTF-8, duplicate IDs and invalid vectors are rejected. A corrupt
checkpoint is not overwritten with an empty one.

The native store commits copy-on-write content and alternating metadata roots with
flushes. AIKit publishes its cache only after commit success. Failed or uncertain
commits invalidate the cache; subsequent reads remount/reload through existing
store recovery. Search indexes are rebuilt from authoritative records, including
deletions; no independent vector database or scheduler exists. Old personal-profile
fields remain an active compatibility contract for assistant identity/settings;
this feature does not migrate or alter protected voice settings.

## Search and embeddings

**Live semantic embedding generation is unavailable:** no validated native
embedding model is installed for this feature. Status reports this explicitly.
Lexical search independently counts distinct matching alphanumeric query terms,
case-insensitively (Unicode lowercase), sorting by count then stable ID. It examines
the first 512 UTF-8-safe query bytes and returns at most the requested limit/16.

`memory_search::EmbeddingProvider` exposes `model()` and `embed(text, out)`.
`embed_record` validates returned dimensions and finite nonzero vectors.
`semantic` performs exact cosine ranking with configurable threshold [-1,1],
owner isolation, model/dimension compatibility, and top-K. Incompatible records
are counted and skipped. Tests use mathematical vectors only, not fake semantic
embeddings. A real model must be validated in InfinityOS before advertising vector
memory. Changing embedding models requires re-embedding; switching conversational
models does not alter records. No model download or silent fallback to random
vectors occurs.

## Capture, privacy and latency

Explicit remember/update requires the active owning session. Common credential
markers are rejected, but this conservative filter is not a universal secret
classifier: do not submit secrets. There is no cross-user administrator override.
Raw model-origin and application-origin memory actions are denied. User-initiated
model proposals retain the initiating owner's existing policy boundary.

Automatic capture honors the existing `learning_enabled` preference. Initial
policy accepts literal “I prefer ...” / “I always prefer ...” declarations, and
native DevKit receipts proving source saved, compiled, launched and controls
published. The latter is labeled as that milestone, **not** proof that the whole
application is correct. Arbitrary generated candidates, failed executions and
sensitive content are refused. At most one candidate is held; it is committed
before a later idle inference request, never in the token loop. An owner/consent
change discards it. Explicit memory commands clear pending candidates so forgotten
information cannot be silently recreated by that queue. Uncommitted candidates
are intentionally not durable.

One owner cache avoids disk reads on repeated retrieval. Every hit still checks the
live session. Recall uses up to three records and a conservative 256-byte upper
bound on token budget, including its untrusted-data framing. Whole records that do
not fit are omitted; no instruction or permission is derived from their content.
Quotes, line breaks and prompt delimiter characters are escaped. Context failure
continues normal inference. Large app prompts with no room retain their original
input. Cold load/explicit writes remain bounded synchronous ObjectStore operations;
no new worker or scheduler was introduced.

`status`/`stats` expose record/embedding counts, serialized bytes, last/max retrieval
nanoseconds and load/commit errors. A missing performance clock yields zero, not a
fabricated timing. Compare cold and hot readings on the installed machine.

## Validation and limits

Run the production-code behavioral harness through the repository build kit:

```text
./build-kit run env CARGO_TARGET_DIR=build/behavior-harness cargo run --release --manifest-path tools/behavior-harness/Cargo.toml --bin aikit-memory-test
```

It checks durable disk cold remount, updates/deletes, corruption, owner isolation,
cache reuse/revocation, failed commit receipts, exact cosine ranking, fallback,
context bounds, deterministic commands and conservative candidate policy. Fixture
providers are not a real semantic model. Cold remount is not an OS reboot test.
For installed acceptance, save a distinctive fact, reboot the installed system
with the ISO detached, inspect/search the same ID, delete it, reboot again and
verify absence. Also compare a no-memory response and another conversational model.
Do not claim reboot, physical voice or model acceptance from host fixtures alone.

Forgetting removes authoritative records and future retrieval, not text already in
the current conversation history. Start a fresh conversation when checking that
previously injected facts are no longer available to the conversational model.
The installed acceptance runner is `tools/aikit-memory-installed-test.py`, invoked
through `build-kit run` with `--arch` and `--iso`. It uses real keyboard commands
and content-free structured receipts, then verifies records after cold boots.
