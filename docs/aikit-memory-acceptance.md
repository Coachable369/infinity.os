# AIKit memory acceptance

The existing ObjectStore checkpoint, IdentitySystem authorization, memory CLI, IntelligenceKit goal context and DevKit prompt builder formed the baseline. The earlier ARM installed run proved update and deletion across cold boots. Source inspection found that production recall called only lexical search, that the 1,536-byte inference context buffer was smaller than the 1,792 bytes its caller could submit, and that the launcher had no memory browser.

The new native provider uses the packaged Potion Base 2M table. The native Infinity Memory program runs through the installed Python/Tk application and window services. Its buttons use the active UIKit primary and secondary surface artwork. The Eclipse Precision design reference supplies the dark background, panel and typography roles. The dedicated memory icon fills the launcher artwork gap.

## Evidence

The production-code test entry point is `./build-kit run env CARGO_TARGET_DIR=build/behavior-harness cargo run --release --manifest-path tools/behavior-harness/Cargo.toml --bin aikit-memory-test -- --inference`. It uses the repository’s Hermes GGUF and checks the synthetic fact against the final input token sequence and real generated answer. The native guest runner is `tools/memory-browser-installed.py`. Host and guest results have different scopes.

| ID | Status | Evidence | Implementation | Remaining work |
|---|---|---|---|---|
| MEM-01 | PASS | Store returns stable ID, duplicate saves preserve it | persistent_memory.rs | — |
| MEM-02 | PASS | Inspect resolves ID in the authorized catalog | memory_api.rs | — |
| MEM-03 | PASS | Update keeps the ID and count | persistent_memory.rs | — |
| MEM-04 | PASS | Delete and cold remount exclude the record | aikit_memory.rs | — |
| MEM-05 | PASS | GUI-edited record 2 retained its ID across a cold boot. After CLI deletion a second boot found no record. See arm-focus-and-reboot.json | memory-browser-installed.py | — |
| MEM-06 | PASS | Independent cold ObjectStore remount reconstructs search from persisted records | aikit_memory.rs | — |
| MEM-07 | PASS | Every truncated checkpoint and CRC mutation is rejected | persistent_memory.rs | — |
| MEM-08 | PASS | Owner mismatch and revoked session are denied even after a cache hit | memory_api.rs | — |
| MEM-09 | PASS | Native record encode/decode and update preserve original content, metadata and stable identity | persistent_memory.rs | — |
| MEM-10 | PASS | Injected backend failure returns failure and invalidates cache | memory_api.rs | — |
| VEC-01 | PASS | Installed ARM record detail shows the packaged Potion model and 64 dimensions | memory_embedding.rs | — |
| VEC-02 | PASS | Both GUI-created and conversational records report embeddings in installed ARM | memory_api.rs | — |
| VEC-03 | PASS | arm-semantic-result.png shows the complete paraphrase “Compilation failed because a library was absent” finding the stored compiler dependency repair on ARM | memory_embedding.rs | — |
| VEC-04 | PASS | Invalid dimensions, nonfinite and zero vectors rejected | persistent_memory.rs | — |
| VEC-05 | PASS | Cosine ordering checked independently and real pretrained model ranked a paraphrase on host | memory_search.rs | — |
| VEC-06 | PASS | Top-K and threshold checked with numerical fixtures | memory_search.rs | — |
| VEC-07 | PASS | Updated record replaces original embedding and deleted ID excluded | persistent_memory.rs | — |
| VEC-08 | PASS | Mismatched model or dimensions excluded and counted | memory_search.rs | — |
| VEC-09 | PASS | Unavailable provider and Unicode case-insensitive lexical search exercised | memory_search.rs | — |
| VEC-10 | PASS | Host production semantic retrieval and context encoding over 3 records, 256 samples, p50 8.54 µs and p95 10.5 µs (integration-final.log) | memory_api.rs | Profile larger catalogs on target hardware |
| AI-01 | PASS | Installed ARM Command Window answered ORBIT-7429 from a separate GUI-created record in a fresh conversation | text_request.rs, ai/mod.rs | Test physical voice separately |
| AI-02 | PASS | Synthetic fact present in decoded final token buffer for authorized request only | qwen/service.rs | — |
| AI-03 | PASS | Real host Hermes returned ORBIT-7429 with recalled context in a fresh conversation | aikit-memory-test.rs | — |
| AI-04 | PASS | Orchard memory excluded from diagnostic question even though both contain “the” | memory_search.rs | — |
| AI-05 | PASS | Empty context produced a normal real Hermes response | qwen/service.rs | — |
| AI-06 | PASS | Unauthorized owner denied and cold model answer omits the synthetic fact | memory_api.rs | — |
| AI-07 | PASS | Complete recall records and framing fit the byte-derived upper bound | memory_search.rs | — |
| AI-08 | PASS | Installed console selected Ministral (1095307268), found the original stable ID, and restored Hermes (1095307269) | console.rs, memory_api.rs | — |
| AI-09 | PASS | Production Hermes polled incrementally through first token and completion on host | qwen/service.rs | Physical voice test outstanding |
| AI-10 | NOT TESTED | Recalled data is escaped and excluded from trusted tool prefix; raw model and application origins denied by broker tests | ai/mod.rs, memory_search.rs, kit.rs | Adversarial installed interaction |

All source paths in the matrix refer to the existing kernel runtime, native storage or tools directories. Runtime tests own their synthetic records and disposable disks. Physical voice and immunity to arbitrary prompt injection are unverified.


## Installed application and integration

The November 12 ARM evidence below uses a private installed disk with an updated kernel, with its installation medium detached. Screenshots and machine-readable receipts are under `builds/validation/aikit-memory/`.

| ID | Status | Evidence | Implementation | Remaining work |
|---|---|---|---|---|
| GUI-01 | PASS | Launcher opens Infinity Memory; native window service reports a visible 840 × 550 surface | infinity-memory.py, launcher registry | — |
| GUI-02 | PASS | ARM shows the compiler record written through Command Window and its embedding count | console_memory_browser.rs | — |
| GUI-03 | PASS | Graphical create and edit saved record 2; an independent CLI search returned the corrected ORBIT-7429 | infinity-memory.py | Input ordering tracked below |
| GUI-04 | FAIL | Rapid search followed by editing transferred four trailing characters into the new record, visible in focus-record.png | Tk and native input dispatch | Isolate and verify event ordering |
| GUI-05 | NOT TESTED | Confirmed-delete and cancellation in the graphical interface have not been driven on target | infinity-memory.py | Verify both dialog choices |
| GUI-06 | PASS | Close destroyed the native application and reclaimed its allocated process pages | native_gui.rs, memory-browser-installed.py | — |
| GUI-07 | NOT TESTED | Six-row pagination, keyword and category filtering exercised through the browser service on host | memory_browser.rs | Drive each widget on target |
| INT-01 | PASS | Separate real-model ARM prompt returned the GUI-edited code (arm-model-answer.png). No fact was given to the conversation | text_request.rs, qwen/service.rs | — |
| PERF-01 | PASS | Single host cold runs took 601 ms without recall and 1,733 ms with recall to first token. Cached keyword retrieval p50 1.00 µs and p95 1.04 µs | aikit-memory-test.rs | Repeat with interleaved requests on target |
| PERF-02 | NOT TESTED | No physical microphone-to-speaker regression or GUI write latency distribution was measured | normal AIKit voice request and native app | Measure through the installed OS |
| ARCH-01 | NOT TESTED | Latest app has been exercised on ARM; x86_64 guest evidence for this milestone is outstanding | shared native runtime | Verify on installed x86_64 |
| ICON-01 | PASS | The production launcher interaction test resolved the registered app, and its installed launcher entry successfully started the native surface | app_launcher.rs, launcher_icons.rs | Inspect icon at additional scales |
| TRACE-01 | PASS | Host tests verified selected IDs and excluded irrelevant and unauthorized records at the actual inference boundary; native guest recorded input admission, 11 generated tokens and final delivery | memory_api.rs, qwen/service.rs | Verify operator-facing diagnostic commands on x86_64 |
