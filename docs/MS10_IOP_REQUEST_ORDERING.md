# Bounded remote request ordering

A host behavioral reproducer exposed an independent native IOP scheduling defect:
after completing request 1 in slot 0, request 3 reused that slot and was selected
before unsent request 2 in slot 1. The actual authenticated receiver then rejected
request 2 through its existing per-session high-water replay check.

The outgoing selector now admits only the oldest unsent request for each peer and
session reference. Eligible stream heads are considered round-robin, so a
backpressured peer does not prevent other peers from being attempted. Selection
does not mark a request sent; only successful secure transport admission does.
There remains at most one request handoff attempt per poll, with fixed tables,
unchanged deadlines, capability validation, and replay rejection.

Behavioral tests exercise real request allocation, owned completion, slot reuse,
authenticated receiver admission, duplicate rejection, and bounded scheduler
fairness under backpressure. This proves the source bug and correction, not that
it caused the installed 32 KiB transfer timeout. That timeout requires separate
installed diagnostics and fresh-source acceptance; request retransmission has not
been introduced by this correction.
