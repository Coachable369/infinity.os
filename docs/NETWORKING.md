# InfinityOS Native Networking

## Architecture

Milestone 8 introduces an architecture-neutral Network Runtime above typed
`NetworkDevice` capabilities. Its public model is interfaces, addresses,
routes, resolver results, connections, policies, profiles, discovery records,
and diagnostics. It does not expose POSIX sockets, descriptors, `errno`,
device files, or ioctl control.

The current service graph is:

`Device -> Network -> Network Policy -> Network Transport -> Network Discovery`

Each service has a versioned manifest, bounded resources, explicit readiness,
and a bounded restart policy. Network failure is Important rather than kernel
critical, so local storage, authentication, UI, Console, and local AI continue.

## Implemented state

- **TESTED:** native IPv4 and IPv6 address types; host-local IPv4/IPv6;
  deterministic routing; fixed-capacity state; Offline profile behavior;
  scoped policy; capability revocation and leases; typed loopback connection
  queues; resolver deadlines/cache expiry; discovery leases; Network-domain
  IEF records; Console schema mapping; service restart; first-boot wired,
  already-associated wireless, and offline selection behavior; setup-mode
  persistence; resolution-aware network-step pointer targets; UEFI Simple
  Network Protocol enumeration; and typed registration of a NAT-backed virtual
  wired adapter with observed MAC, MTU, link state, and protocol capabilities;
  ACPI MCFG fallback discovery for VirtualBox ARM's E1000 PCI function.
- **IMPLEMENTED BUT UNTESTED IN A VM:** settings-side Network Inspector,
  transactional built-in profile selection, native profile persistence,
  the graphical post-install Network step, and System Generation bootstrap
  objects.
- **SCAFFOLDED:** firmware-backed packet transport, secure-connection identity boundary, future NodeIdentity on
  connections/discovery, dynamic address sources, listener operation IDs,
  wire resolver adapter, and physical adapter registration boundary.
- **UNSUPPORTED:** native PCI NIC interrupts/DMA, native virtio-net/e1000 packet engines,
  DHCP/SLAAC exchanges, routed packet IO, DNS wire queries, TCP/UDP wire
  engines, Wi-Fi scanning/association, and production TLS/certificate
  validation. When those devices or links are unavailable, onboarding reports
  that state and offers offline setup; it never invents an SSID or connection.

`InternetReachable` is never inferred from an address or default route. The
implemented connectivity states are Offline, LinkOnly, LocalNetwork, Routed,
LimitedConnectivity, and Degraded; Internet reachability remains optional
future observed state.

## Resource bounds

Interfaces, addresses, routes, policies, profiles, connections, per-connection
messages, resolver entries, discovery advertisements, IOP queues, and IEF
queues all have compile-time limits. Saturation returns typed resource or queue
errors. No networking queue grows dynamically.
