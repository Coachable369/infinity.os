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

## Users And Groups

Settings uses Users/Groups selection, inline forms, masked password confirmation,
membership controls and explicit delete confirmation. Console exposes the same
changes through `runtime::manage_accounts`; success is reported after the native
storage transaction commits. User and group references accept their stable
numeric ID or handle/name.

```text
user list
user create handle=alex display-name="Alex Chen"
user read user:alex
user update user:alex display-name="Alex C."
user password user:alex
user disable user:alex
user enable user:alex
group create name=engineering display-name="Engineering"
group add group:engineering user=user:alex
group members group:engineering
user groups user:alex
group update group:engineering display-name="Product Engineering"
group remove group:engineering user=user:alex
group delete group:engineering
user delete user:alex
```

`user create` and `user password` start private password entry. Escape cancels
without creating a partial account. To grant administrator privileges, add an
existing user to `group:administrators`; the shared service protects the last
active administrator regardless of interface.
