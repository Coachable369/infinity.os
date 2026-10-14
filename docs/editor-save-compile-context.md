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

Behavioral cases were added to `tools/editor-assistant-test.rs` and
`tools/ai-test.rs`. Execution is pending: the build kit rejected the focused test
because release build `20261014T004941694309Z-63659` already owns the lock.
No installed-system acceptance or updated-ISO claim is made for this change.
After that build releases its lock, run:

```sh
./build-kit run make editor-assistant-test ai-test
./build-kit full
```

Installed acceptance must exercise a named modified C document and an unnamed C
document, Save As cancellation/failure, and successful executable publication
without launching it. Check the release manifest and both ISO hashes before
claiming that installer media contains the change.
