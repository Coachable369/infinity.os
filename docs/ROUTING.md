# Native Routing

Routes carry stable identity, IPv4/IPv6 destination and prefix, optional next
hop, interface, metric, source, state, and optional policy scope. Mutation
validates family, prefix, interface, duplicate/conflict, and fixed capacity.

Selection is deterministic: longest matching prefix, lowest metric, then
lowest route identity. Disabled/link-down interfaces are excluded. Default,
specific, removal, no-route, and IPv6-safe behavior are **TESTED**. Static
state exists in memory; persisted route objects and dynamic gateway acquisition
are **SCAFFOLDED**.
