# Milestone 10 execution checklist

Status: IN PROGRESS, not complete. Installer click/progress prerequisite passed; see INSTALLER_CLICK_PROGRESS_ACCEPTANCE.md.

Authoritative request: user-supplied Resource Fabric & Infinity Pool Storage milestone.

1. Authenticated, versioned resource advertisements with explicit owner, capacity, health and node/device failure domain.
2. Stable ObjectId independent of namespace, authority and placement; reuse existing independent-copy/COW semantics.
3. Explicit Temporary/Protected/Critical policy, independently placed verified replicas, capacity reservations and bounded transfer state.
4. Coherent durable manifest/version commits; incomplete or corrupt replicas never become available.
5. Integrity-checked reads survive a lost storage node; preserve known offline object identity and namespace.
6. Single authoritative bounded healing claims, resumable transfer, verified completion and safe returning-node reconciliation.
7. Native capability-governed IOP/IEF integration, common Console/GUI inspection, System Generation packaging.
8. Behavioral local/placement/integrity/failure/healing/reconciliation tests.
9. At least three independently installed nodes: write/protect/distribute, remove replica host, read/degraded/heal/healthy, return stale node, cold reboot and verify persistence.
10. Preserve MS9/security, desktop performance and installer regressions. Do not substitute host fixtures for installed acceptance.

Dependency still open: the last installed MS9 run lost its secure session before remote inspection admission. Rapid installed input also accepted six of ten keys. Neither is marked resolved by this milestone or by the installer fixes.
