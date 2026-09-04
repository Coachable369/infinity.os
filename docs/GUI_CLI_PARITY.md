# GUI and CLI Parity

Status: **TESTED** at the operation registry/state layer.

The first graphical shell and Infinity Console are two renderers/controllers
over the same Identity, Authentication, Session, Settings, AI, Voice, Object,
and Storage services. GUI mutations call those native service operations and
persist the same object that Console operations read. There is no shadow GUI
database and the system never parses rendered terminal output.

Milestone 7 registers typed CRUD-style human commands for `user`, `identity`,
`machine`, `credential`, `session`, `personal-space`, `ai-profile`,
`voice-profile`, and `settings`. Password creation remains a specialized masked
workflow because accepting a secret as an ordinary command argument would leak
it into console history. Destructive or security-sensitive operations remain
subject to capability and policy checks regardless of interface.
