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
| MEM-05 | PASS | Previous installed ARM guest saved, updated, rebooted, forgot and rebooted again | aikit-memory-installed-test.py | Verify latest browser build separately |
| MEM-06 | PASS | Independent cold ObjectStore remount reconstructs search from persisted records | aikit_memory.rs | — |
| MEM-07 | PASS | Every truncated checkpoint and CRC mutation is rejected | persistent_memory.rs | — |
| MEM-08 | PASS | Owner mismatch and revoked session are denied even after a cache hit | memory_api.rs | — |
| MEM-09 | PASS | Native record encode/decode and update preserve original content, metadata and stable identity | persistent_memory.rs | — |
| MEM-10 | PASS | Injected backend failure returns failure and invalidates cache | memory_api.rs | — |
| VEC-01 | NOT TESTED | Real pretrained provider executed on host, installed run pending | memory_embedding.rs | Verify guest |
| VEC-02 | NOT TESTED | Three actual model vectors stored in host engine | memory_api.rs | Verify guest |
| VEC-03 | NOT TESTED | Native WordPiece and learned query row pooling rank paraphrase first on host | memory_embedding.rs | Verify guest |
| VEC-04 | PASS | Invalid dimensions, nonfinite and zero vectors rejected | persistent_memory.rs | — |
| VEC-05 | PASS | Cosine ordering checked independently and real pretrained model ranked a paraphrase on host | memory_search.rs | — |
| VEC-06 | PASS | Top-K and threshold checked with numerical fixtures | memory_search.rs | — |
| VEC-07 | PASS | Updated record replaces original embedding and deleted ID excluded | persistent_memory.rs | — |
| VEC-08 | PASS | Mismatched model or dimensions excluded and counted | memory_search.rs | — |
| VEC-09 | PASS | Unavailable provider and Unicode case-insensitive lexical search exercised | memory_search.rs | — |
| VEC-10 | PASS | Host cache retrieval and first-token times measured in final-boundary.log | memory_api.rs | Profile on target hardware |
| AI-01 | NOT TESTED | Normal typed and conversational requests call owner-scoped retrieval in production | text_request.rs, ai/mod.rs | Observe installed request |
| AI-02 | PASS | Synthetic fact present in decoded final token buffer for authorized request only | qwen/service.rs | — |
| AI-03 | PASS | Real host Hermes returned ORBIT-7429 with recalled context in a fresh conversation | aikit-memory-test.rs | — |
| AI-04 | PASS | Orchard memory excluded from diagnostic question even though both contain “the” | memory_search.rs | — |
| AI-05 | PASS | Empty context produced a normal real Hermes response | qwen/service.rs | — |
| AI-06 | PASS | Unauthorized owner denied and cold model answer omits the synthetic fact | memory_api.rs | — |
| AI-07 | PASS | Complete recall records and framing fit the byte-derived upper bound | memory_search.rs | — |
| AI-08 | NOT TESTED | Storage is independent of the selected conversation model; engine reload exercised | aikit_memory.rs | Switch real models in a guest |
| AI-09 | PASS | Production Hermes polled incrementally through first token and completion on host | qwen/service.rs | Physical voice test outstanding |
| AI-10 | NOT TESTED | Recalled data is escaped and excluded from trusted tool prefix; raw model and application origins denied by broker tests | ai/mod.rs, memory_search.rs, kit.rs | Adversarial installed interaction |

All source paths in the matrix refer to the existing kernel runtime, native storage or tools directories. Runtime tests own their synthetic records and disposable disks. Physical voice and immunity to arbitrary prompt injection are unverified.
