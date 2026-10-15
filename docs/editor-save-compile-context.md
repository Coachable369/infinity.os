# Current-document save and compile

The Text Editor's attached assistant now recognizes explicit requests such as
`save and compile my code`, `save then compile this file`, and `compile my code`.
The same bounded intent parser is used by desktop text/voice command routing.
These requests operate on the owning editor's current document, not an LLM
response, earlier chat message, selection, or guessed filesystem path.

Save-and-compile first uses the editor's normal storage workflow. An unnamed
document opens the existing Save As sheet; successful saving resumes compilation
once. Cancellation, a changed revision, or a conflicting pending document action
discards the continuation. A failed save never starts compilation. Native Clang
receives the complete current source snapshot through the existing isolated
compiler job. Compiled executables are not automatically launched.

Explicit requests remain subject to app ownership, authenticated session, AI
policy, document revision, and compiler resource/capability checks. Quoted,
negated, explanatory, and compound run requests are not interpreted as this tool.

## Verification status

Behavioral cases in `tools/editor-assistant-test.rs` and `tools/ai-test.rs`
passed through build manifest `20261015T074020297018Z-64611.json`.
Save As now validates the original source fingerprint before changing its path,
then binds compilation to the new saved-path fingerprint. This avoids rejecting
an unchanged document merely because it acquired a filename.

The disposable installed harness has a `--save-compile` mode covering Save As
cancellation/retry, named-document saving, and compiler success/error outcomes.
ARM installed compiler-workflow verification passed, including detached-media
cold boot, Save As cancellation/retry, valid source publication, named-source
saving, invalid-source rejection, and explicit launch returning exit code 54.
This was QEMU/HVF with QEMU-addressed ARM test media, not a VirtualBox installation.
The focused test intentionally did not regenerate source with the LLM; its broader
`installed_acceptance` field remains false while `compiler_workflow_passed` is true.

Evidence:

- Host tests: `builds/manifests/20261015T074020297018Z-64611.json`
- Clean release: `builds/manifests/20261015T074107335436Z-64896.json` (passed)
- Independent checksums: `builds/manifests/20261015T090353168481Z-66818.json` (passed)
- Installed ARM: `builds/manifests/20261015T090535893218Z-66847.json` (passed)
- Structured guest results: `builds/validation/editor-save-compile-arm.json`
- Rendered success: `builds/validation/editor-save-compile-installed.png`

The additional x86_64 test booted the canonical release ISO under QEMU TCG on
the ARM host, but exceeded the 900-second provisioning deadline at installer
step 6 before reaching the editor. Its manifest is
`builds/manifests/20261015T090947147760Z-66920.json` (failed), with structured
evidence at `builds/validation/editor-save-compile-x86.json`. This does not
establish an editor regression or an x86 installed-workflow pass. Do not claim
x86 installed-runtime acceptance from the successful release build. Both
disposable guests were stopped; the user's VM was not reinstalled.

Release outputs (both include runtime commit `f99e228`):

| ISO | SHA-256 |
| --- | --- |
| `builds/InfinityOS-aarch64.iso` | `cfe01ac8160e1813f8daa505cefefe50d3481310620d786437611edb015a1dbc` |
| `builds/InfinityOS-x86_64.iso` | `93fff7c8179edae342a026f6fc6aa2282d9df43046b9ee692435558e05771263` |

Release and focused verification commands:

```sh
./build-kit run make editor-assistant-test ai-test
./build-kit full
```

Remaining runtime checks: x86_64 installed workflow on a practical execution
target and an injected storage-write failure. The passing ARM workflow covers
Save As cancellation, valid/invalid C, and subsequent named-document save. It
explicitly launches the published executable only after compilation is finished;
compilation itself does not auto-run the executable.
