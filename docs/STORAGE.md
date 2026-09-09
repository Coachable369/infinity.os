# Infinity Storage

InfinityOS presents storage as purpose, policy, and shared capacity. GPT and FAT
are firmware compatibility mechanisms and are not the user-facing model.

## Hierarchy

```text
Physical Device
      -> Infinity Container
      -> Infinity Pool
      -> Storage Spaces
      -> Infinity Objects (future)
```

- **Device** is physical storage hardware exposed through a typed block-device
  capability. Milestone 3A obtains real ATA identify data and capacity.
- **Container** is the physical region assigned to InfinityOS. It has an explicit
  version, UUID, state marker, geometry, metadata locations, and checksum.
- **Pool** owns shared capacity. The initial pool has one member, but its metadata
  separately identifies the pool and member so another container can be added by
  a later format version.
- **Space** describes logical purpose and policy. System, Personal, Applications,
  and Recovery are records in pool metadata, not fixed physical partitions.
- **Object** is the future persistent data entity. Milestone 3A does not implement
  InfinityFS or an object store.

Unused capacity remains in the pool. All four Spaces use the same shared-dynamic
allocation policy; the installer does not ask users to choose a fictional
priority profile. Milestone 3A assigns no fixed physical ranges, quotas, or
mount-point semantics to them. The System Space record owns the boot kernel
reference used by the loader.

APFS containers, ZFS and Btrfs pools, LVM, and Storage Spaces solve related
problems. InfinityOS's distinction is making the pooled, policy-oriented model a
consistent native OS abstraction, not claiming those systems lack pooling.

## Current capability boundary

### Explicit resource publication

`pool advertise peer=node:<full-id> grant=<peer-issued-resource-advertise-grant>`
starts a bounded, session-owned publisher over the existing secure IOP path.
It samples native `ResourceInspect` through the shared capability broker, keeps
the real device/resource identities and reboot-monotonic sequence, and publishes
a 60-second lease. After an authenticated completion it waits 20 seconds before
sampling again. Four subscriptions are supported, advancing at most one per
runtime poll; there is no synchronous network wait or automatic trust grant.

Logout, locked sessions, failed authority, transport failure or unavailable
storage stop the subscription. A failed publication is not retried indefinitely.
The receiving directory retains offline resource identity and rejects older
observations. Subscriptions themselves are not persistent deployment policy and
must be explicitly restarted after boot or failure. This publisher does not
implement automatic replica placement or healing. Host behavioral checks are
separate from installed wire acceptance.

`StorageManager` routes discovery, planning, provisioning, and verification to a
`BlockDevice`. The first driver is 28-bit ATA PIO for a disposable QEMU disk.
The installer never performs port I/O or arbitrary sector writes directly.

There are no user-facing drive letters, mount points, POSIX roots, home
partitions, swap partitions, fstab entries, or inode assumptions.
