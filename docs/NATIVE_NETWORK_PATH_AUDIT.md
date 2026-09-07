# Native network prerequisite audit

Scope: revised Milestone 9, x86_64 UEFI/QEMU reference target. No M10 work.

## Before implementation

| Layer | Status | Evidence and boundary |
| --- | --- | --- |
| Adapter discovery | PARTIAL | UEFI SNP/PCI metadata copied into BootInfo and registered as NetworkDevice; not a runtime driver |
| NIC RX / TX | UNSUPPORTED | No native descriptor engine feeding NetworkRuntime |
| Interrupt / polling | UNSUPPORTED | No NIC service work dispatched by the runtime |
| Ethernet parsing | UNSUPPORTED | No Ethernet ingress dispatch |
| IPv4 / IPv6 | PARTIAL | Address/routing types exist, no wire packet path |
| ARP / NDP | UNSUPPORTED | No peer address-resolution exchange |
| Routing | PARTIAL | Local state and route selection; no packet forwarding |
| UDP / TCP | UNSUPPORTED | ConnectionManager rejects non-host-local destinations |
| DNS | PARTIAL | Resolver cache/deadline logic; no wire adapter |
| Service discovery | SCAFFOLDED | Leased local records, no network exchange |
| Non-loopback connection | UNSUPPORTED | Explicit UnsupportedOperation guard |
| Installed-system packet exchange | UNSUPPORTED | No detached-media two-node evidence |

The pre-change path ended at firmware adapter metadata. Network.Send pushed into
the same runtime's local receive queue. This is not packet transmission and must
not be described as routed or remote networking. Milestone 8 is PARTIAL.

## Implementation checklist

1. One reference NIC backend with bounded DMA ownership and polling.
2. Checked Ethernet/ARP/IPv4/UDP ingress and egress beneath existing services.
3. Independent installed-node packet exchange before node acceptance.
4. Signed discovery, authenticated duplex sessions and explicit human pairing.
5. Shared authorized IOP mutation/inspection and post-commit IEF reconciliation.
6. Installed GUI/Console lifecycle, revocation/reconnect/reboot and performance gates.

No protocol parser, type registration or host simulation can change the
installed-system status to TESTED. Existing loopback and security gates remain.

## Implemented reference path and evidence

**TESTED — native QEMU engineering fixture, not installed acceptance:** two
independent x86_64 UEFI guests use the production 82540EM backend, production
Ethernet/ARP/IPv4/UDP parser and production NetworkRuntime connection, route,
capability and policy managers. Each resolves the other MAC, sends requests,
receives requests through its own RX descriptors and sends/receives replies.
Both exit through a binary diagnostic device with status 33. With no peer, the
guest exits with bounded timeout status 35. `make native-nic-test` requires both
outcomes; guest log text is diagnostic only. The QEMU socket backend is a virtual
Ethernet link, not a guest socket API or protocol proxy.

**TESTED — host behavioral gates:** malformed/truncated IP/UDP, ARP expiry,
bounded queues, payload bounds, current inbound/outbound policy, capability
revocation, lease expiry and invalidation after address/link loss. Run
`make network-wire-test` (includes the typed datagram test).

**IMPLEMENTED BUT UNTESTED — installed runtime integration:** the common kernel
initializes the backend after restoring network configuration. Input loops
service at most four RX and four TX descriptors per millisecond. No renderer
calls, compositor modifications or per-packet durable events are introduced.
The installed/live builds share this implementation. Actual installed boot,
operator configuration and two-node detached-media acceptance remain unproven.

Reference limitations: one root-bus Intel 82540EM (`8086:100e`), below-4GiB
identity-mapped MMIO, fixed resident DMA buffers, x86_64 QEMU PC HPET fallback
when a calibrated TSC is unavailable, static IPv4 and connected UDP only. The
reference driver is not certified for physical hardware or ARM64. Interrupts,
device-reset recovery, DHCP, NDP, IPv6 wire IO, TCP, wire DNS and native node
protocol carriage remain unsupported or unfinished. No trust claim follows
from receiving a datagram.

Bounds: 32 RX and 32 TX descriptors, 2048-byte DMA buffers, 1514-byte Ethernet
frames, 512-byte UDP payloads, eight queued ingress and egress frames/datagrams,
16 ARP entries, 60-second neighbor expiry and two-second ARP retry suppression.
Offline/link/address loss clears native wire queues and invalidates remote
connections; recovery does not resurrect prior connection authority.

Milestone 8 and Milestone 9 remain **PARTIAL**. Checklist items 3–6 above have
not passed. In particular, the engineering fixture is deliberately not a
replacement for two ordinary installed System Generations.

## Release verification — 2026-09-07

- `./build.sh`: exit 0, both supported architecture ISO payloads built; binary
  installed-kernel and packaged-asset parity, existing UI/input, network,
  Milestone 9 security, performance and resource-policy gates passed.
- `make native-nic-test`: exit 0 with native guest exit values `[33, 33]` and
  absent-peer `[35]`. The final fixture also checks observed RX/TX completions
  and uses the policy-revalidating native receive operation.
- Observed host retained-drag fixture: 300 frames, average 31,227 ns,
  p95 44,791 ns, worst 69,042 ns; 23,159,280 damaged pixels in aggregate.
  These are host fixture measurements, **not installed desktop timings**.
- Full build initially caught a stale clock-target test expectation and an
  installer asset edited between architecture builds. The clock target assertion
  was updated to its existing control ID; a clean rebuild incorporated the
  latest authored assets without changing their design or weakening parity.
- x86_64 ISO SHA-256:
  `9979c10461b11f06549293645a612d8b92d65c7cc13ddc796c0ec374025468f5`
- ARM64 ISO SHA-256:
  `08cf04077ccaffe067af5a6c5f1e57363edc5b2660e73cf1fc520d0c47774041`

Not verified: ordinary two-node installed boots with media detached, native
signed discovery, negotiated secure duplex sessions, human pairing over that
transport, IOP/GUI/Console lifecycle convergence, IEF gap recovery and native
desktop performance under networking load. The release artifacts are a tested
foundation increment, not a Milestone 9 completion release.
