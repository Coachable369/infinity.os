# Connect installed InfinityOS nodes

## UI entry points

- **Infinity Fabric → Connect nodes…** (toolbar), or **Help → Connect nodes…**.
- Pairing and trust controls are available directly in the Fabric connection panel and in **System Settings → Nodes & Mesh**.
- Contribution limits and capacity: **Infinity Fabric** from the app launcher.

## Connect two nodes

1. Sign in as an administrator on both installed computers. Put them on the same reachable LAN. For VirtualBox guests, bridged adapters are the simplest arrangement; independent NAT networks do not provide peer discovery.
2. Open **Infinity Fabric → Connect nodes…** on both computers. Nearby nodes appear automatically on the same subnet when the network profile allows discovery (Standard does). Discovery uses UDP ports 47777 and 47778; isolated NAT networks will not discover one another.
3. Select the peer and choose **Verify Identity** on one computer. Wait for the comparison code and fingerprint to appear on both. Enter the other computer's six-digit code and select **Confirm matching identity** on each. Never confirm a mismatch.
4. Select **Allow resource sharing** on both computers. This explicitly allows membership, compute and mesh for that peer; discovery alone grants no access.
5. Select **Open Session** if needed, then **Join Mesh** on both computers. The list updates automatically as trust and connection state change. Settings → Nodes & Mesh remains an alternative path.
6. Launch **Infinity Fabric** on each computer. Enable **Contribute resources**, choose CPU units, memory allowance and concurrent-job limits, and select **Apply contribution**.
7. Select **Refresh**. Inspect the peer's connection and trust, per-node available capacity, reservations, and combined totals. Select a node to inspect its details.

## Disconnect or stop sharing

Disable **Contribute resources** and apply to stop offering new work. **Mesh Health → Leave** withdraws membership; **Trusted Nodes → Revoke Trust** removes the trusted relationship. Policy changes are explicit and per peer.

## Scope and verification

CPU values are scheduling slots. Memory shown here is a provider execution allowance, not transparent shared addressable RAM. GPU execution requires a compatible provider. This UI does not establish arbitrary-workload CPU/RAM/GPU pooling as complete.

The connection instructions follow the native typed Settings actions and authority checks. Host interaction checks cover peer arrival/removal, manual confirmation, trust-gated sharing, preserved input, contribution controls, selection, and zoom. Native network checks cover LAN broadcast delivery, discovery limits, subnet isolation, profile opt-out, and authenticated pairing reconciliation. This delivery does not claim a new two-node installed pairing acceptance run or large-display visual parity.
