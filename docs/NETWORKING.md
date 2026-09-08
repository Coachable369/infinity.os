# InfinityOS Native Networking

Node services consume typed Network Service boundaries and do not identify peers
by address. Phase 9-A carries signed discovery, explicit pairing and encrypted
duplex sessions over the native static IPv4/UDP reference path. See
[wire trust](MILESTONE_9A_WIRE_TRUST.md); remote IOP remains outside this phase.

The x86_64 QEMU reference NIC/ARP/IPv4/UDP path now has two-guest engineering
evidence through the production typed datagram manager. This is **TESTED in the
native engineering fixture**, not detached-media installed acceptance. See
[the native path audit](NATIVE_NETWORK_PATH_AUDIT.md) for exact evidence, bounds,
integration status and remaining prerequisites. Milestones 8 and 9 are **PARTIAL**.

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
  ACPI MCFG fallback discovery for VirtualBox ARM's E1000 PCI function;
  responsive Settings page/control hit geometry; static IPv4, prefix, gateway,
  and metric replacement with rollback; resolver server configuration;
  interface and default-policy configuration; and versioned binary round-trip
  of those settings.
- **IMPLEMENTED BUT UNTESTED IN A VM:** settings-side Network Inspector,
  mouse/keyboard Network editor, transactional built-in profile selection,
  native profile and network-configuration persistence,
  the graphical post-install Network step, and System Generation bootstrap
  objects.
- **SCAFFOLDED:** firmware-backed packet transport, secure-connection identity boundary, future NodeIdentity on
  connections/discovery, dynamic address sources, listener operation IDs,
  wire resolver adapter, and physical adapter registration boundary.
- **UNSUPPORTED:** native PCI NIC interrupts, virtio-net packet engines,
  DHCP/SLAAC exchanges, DNS wire queries, TCP and IPv6 wire
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

## Native reference datagrams

`NetworkRuntime::send_datagram` rechecks the send and original connection
capability, deadline, active profile, current outbound policy and selected IPv4
route before queuing a frame. RX delivery matches exact connected endpoints and
rechecks current inbound policy and connection authority. An unresolved neighbor
returns `AddressUnavailable` while a bounded ARP request is pending; callers must
retry under their deadline. Opening a datagram object does not prove reachability.
The existing local-only `ConnectionManager::send` cannot accidentally echo a
remote datagram into its own receive queue.

The runtime pump uses restored static configuration only: it does not install
test IPs, open public endpoints or grant test capabilities. The explicit test
configuration in `tools/nic-probe/fixture.rs` belongs solely to that engineering
fixture. Console/IOP integration for native datagram operations and installed
operator acceptance remain unfinished.

The reference descriptor implementation follows the
[Intel 8254x software developer manual](https://www.intel.com/content/dam/doc/manual/pci-pci-x-family-gbe-controllers-software-dev-manual.pdf).
