# Window Server

The Window Server owns window IDs, retained surface references, present and
previous bounds, restore bounds, minimum size, window state, focus, pointer
capture, modal relationships, and policy z-order classes: Desktop, Normal,
Floating, Menu, Modal, Trusted, and Cursor. Each window and surface has exactly
one Execution Context owner. A caller cannot mutate, close, move, resize, or
capture another context's window. Context failure removes its windows and
surfaces and repairs focus/capture without terminating unrelated services.

Ownership denial, states, old/new damage, trusted z-order, modal hit testing,
pointer capture denial, typed lifecycle events, and crashed-owner cleanup are
**TESTED**. A bounded checkpoint preserves ordinary window IDs, surface
references, geometry, and eligible focus across Window Server reconstruction.
Transient pointer capture and event queues are reset, and Trusted/Cursor layers
must be recreated through current secure authority; this restart policy is
**TESTED**. Hardware-protected shared surface pages are **PLANNED**, not
presented as MMU-enforced isolation.
