# Network Inspector and Topology

System Settings contains a Network surface with Overview, Profiles,
Interfaces & Topology, Application & Service Access, DNS/Resolution, Routes,
Connections, and Diagnostics. It reads the same typed Network Runtime used by
Infinity Console; there is no GUI-only state database and no shell parsing.

The Network settings surface uses a dedicated glass dashboard instead of a
generic list. Its status hero, scalable vector topology, observed-state panel,
and five operational-mode cards all read directly from Network Runtime. The
overview reports committed connectivity, active profile, interface, address,
route, resolver, policy, connection, discovery, and degraded state.
Connection ownership is returned only to its owner or an explicitly privileged
inspector. Topology contains the local machine, interfaces, routes, and leased
discovered services; discovery never grants authority.

Shared typed state, ownership redaction, responsive dashboard geometry, and
pointer hit targets are **HOST-TESTED**. Profile cards execute the same
capability-checked, transactional, durable activation path used by Console.
Rendering, dynamic event refresh, and installed-disk GUI acceptance are
**IMPLEMENTED BUT UNTESTED IN A VM**.
