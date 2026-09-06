# Network Devices

`NetworkDevice` reports a stable device and driver identity, link type,
optional hardware address, link/operational state, real MTU, receive/transmit
capability, reported offloads, counters, and an error code. Unknown information
is absent or zero; it is never invented.

The native host-local loopback device is **TESTED** with IPv4 `127.0.0.1` and
IPv6 `::1`. It is a System device rather than `/dev` state.

UEFI Simple Network Protocol discovery and typed adapter registration are
**TESTED** with an AArch64 UEFI virtual NAT adapter. The loader preserves the
observed MAC address, MTU, media state, and RX/TX protocol availability; the
runtime exposes that device as a wired interface before first-boot onboarding.
The same loader and installed System Generation path are used by VirtualBox.
When VirtualBox ARM omits that protocol, the loader's **TESTED** ACPI MCFG
fallback discovers its E1000-class PCI function while leaving unavailable MAC,
MTU, link, and transport fields explicitly unknown. First-boot setup accepts
that adapter as present and reports its link as unverified rather than claiming
that hardware is absent or that routed connectivity already exists.

Native PCI/virtio/e1000 DMA and interrupt drivers remain **UNSUPPORTED**. The
firmware handoff therefore establishes honest discovery and a future packet-I/O
boundary, but it does not yet claim DHCP, routed transport, link speed, duplex,
Wi-Fi quality, encryption, or Internet reachability. A future adapter service
can replace the firmware transport without changing typed callers.
