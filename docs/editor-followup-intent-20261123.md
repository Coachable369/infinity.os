# Text Editor follow-up intent correction

The supplied transcripts use sentence punctuation after the application name and
verb, and one transcribes “write” as “Right”. The scoped command parser previously
required a space after the application name and did not recognize `input`.

Application-intent parsing now accepts clause separators at these syntax
boundaries. Explicit Text Editor requests accept write, type, insert, input,
enter, put and add; the homophone right is restricted to that explicit scope.
Literal content keeps its case and internal/trailing punctuation. Quoted content
is preserved. Negations, explanatory requests, empty values and an unscoped
right command remain rejected. No direct audio code or assets were changed.

Behavioral verification covers the exact screenshot transcripts, typed and voice
request envelopes, opening Text Editor followed by repeated requests through the
runtime execution broker, exactly-once dispatch, completed state, real document
insertion and undo. This is host behavioral verification, not a live microphone
or installed-VM interaction claim.

- Initial AI suite passed: `20261123T212023172657Z-22682.json`.
- Final focused contract passed: `20261123T212502643272Z-22940.json`.
- An intervening broader native model run was intentionally stopped after the
  contract phase; it is not reported as a completed test run.

Build manifests are under `builds/manifests/`.

## ARM publication

`builds/InfinityOS-aarch64.iso` passed kernel freshness, live/installed payload
parity and checksum verification. Release manifest:
`20261123T212535769393Z-23064.json`.
SHA-256: `c0ac8693cd381208fd9ff27de581593662d7959ae3b7527e35ff7be5b60ba2cd`.
The previously approved dependency-reuse path was used; the broad release
regression suite was deferred. The installed VM still requires user testing.

## x86 publication

`builds/InfinityOS-x86_64.iso` passed the same packaging and checksum gates,
after ARM publication. Release manifest: `20261123T213746966980Z-27225.json`.
SHA-256: `82958c1710427069a93e7691b0c66fe4a3258ec32b75ad2d115863b48c4b36a4`.
