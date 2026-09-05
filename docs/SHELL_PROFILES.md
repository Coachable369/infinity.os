# Shell Profiles

Shell Profiles are versioned declarative Infinity Objects stored under `/system/settings/shell/profiles` for built-ins and the user's shell settings projection for user-owned profiles. They map a familiar first command token to a validated Infinity Native command template; they are not scripts, dotfiles, startup programs, or alternate system semantics.

Stable built-ins are `infinity.native`, `compat.linux`, and `compat.unix`. Native commands always win. Enabled user profiles then resolve by explicit priority and stable ProfileId, followed by compatibility profiles. `shell alias resolve` exposes the selected and shadowed mappings. Native names such as `cd`, `idir`, `path`, and `list` cannot be shadowed.

Commands cover profile list, inspect, create, clone, enable, disable, set-default, and delete, plus alias list, add, delete, and resolve. Built-ins are immutable. Alias names accept a narrow ASCII identifier set; control characters, Unicode confusables, zero-width text, operators, unknown commands, and unbounded expansion are rejected. Capability checks still occur after translation.
