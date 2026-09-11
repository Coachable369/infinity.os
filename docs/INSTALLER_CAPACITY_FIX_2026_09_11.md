# Installer kernel reservation correction

The September 11 ARM64 installed ELF was approximately 174 MiB. The disk
planner reserved only 128 MiB before object-store metadata, with the kernel
starting 1 MiB into that reservation. It therefore rejected even the default
16 GiB virtual disk. Increasing the virtual disk size would not fix this.

Fresh stores now start at relative sector 526336 (257 MiB), leaving the full
256 MiB supported by the UEFI loader for the installed ELF. Kernel and object
storage bounds remain checked before formatting. The provisioning default
remains 16 GiB.

ObjectStore retains its resolved physical store base. Mount probes the new
location and the historical sector 262144 location, validates roots and banks,
and keeps subsequent reads and writes at that location. Existing stores are
not migrated. Older kernels must not be used to open newly formatted stores.

Verification covers real payload geometry for both architectures, the 256 MiB
kernel boundary and one-sector overflow, old-location mount/mutation/remount,
and the existing object-store persistence and corruption suite. `build.sh`
now invokes the artifact capacity test before publishing its output ISOs.

No running VM disk was modified. A fresh VM installation using the corrected
ISO still needs end-to-end verification; source/harness checks are not VM proof.
