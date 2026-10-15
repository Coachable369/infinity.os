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
Installed-system acceptance and refreshed release evidence remain pending.
Release and focused verification commands:

```sh
./build-kit run make editor-assistant-test ai-test
./build-kit full
```

Installed acceptance must exercise a named modified C document and an unnamed C
document, Save As cancellation/failure, and successful executable publication
without launching it. Check the release manifest and both ISO hashes before
claiming that installer media contains the change.
