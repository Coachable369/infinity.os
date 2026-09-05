# Network Devices

`NetworkDevice` reports a stable device and driver identity, link type,
optional hardware address, link/operational state, real MTU, receive/transmit
capability, reported offloads, counters, and an error code. Unknown information
is absent or zero; it is never invented.

The native host-local loopback device is **TESTED** with IPv4 `127.0.0.1` and
IPv6 `::1`. It is a System device rather than `/dev` state.

Physical and virtual bus discovery is **UNSUPPORTED** in the current kernel:
there is no PCI/virtio-net enumerator, DMA queue, interrupt path, or firmware
network handoff. Consequently link speed, duplex, Wi-Fi quality, encryption,
and Internet reachability are not reported. A future adapter service registers
discovered hardware through the same typed interface manager without changing
callers.
