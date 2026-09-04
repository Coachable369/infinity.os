# Window Server

The Window Server owns window IDs, surface references, bounds, focus, pointer
capture, modal relationships, and z-order classes: Desktop, Normal, Floating,
Menu, Modal, Trusted, and Cursor. Each window and surface has exactly one
Execution Context owner. A caller cannot mutate or capture another context's
window. Context failure removes its surfaces and repairs focus/capture without
terminating unrelated services.

Ownership denial, z-order hit testing, pointer capture denial, and crashed-owner
cleanup are TESTED. Hardware-protected shared surfaces and a complete compositor
protocol are SCAFFOLDED, not presented as MMU-enforced isolation.
