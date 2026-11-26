# Connect installed InfinityOS nodes

## UI entry points

- **Infinity Fabric → Connect nodes…** (toolbar), or **Help → Connect nodes…**.
- Pairing and trust controls: **System Settings → Nodes & Mesh**.
- Contribution limits and capacity: **Infinity Fabric** from the app launcher.

## Connect two nodes

1. Sign in as an administrator on both installed computers. Put them on the same reachable LAN. For VirtualBox guests, bridged adapters are the simplest arrangement; independent NAT networks do not provide peer discovery.
2. On each computer open **System Settings → Nodes & Mesh → Pair Node**. Discovery runs automatically. **Select Node** cycles through discovered peers. On one computer choose **Verify Identity** to begin pairing.
3. Compare the displayed fingerprint and six-digit confirmation code on both computers. On each computer choose **Confirm**, enter the matching code, and press **Enter**. Do not confirm a mismatch. Discovery alone does not authorize resource access.
4. In **Access Policy**, enable **NAMESPACE** for membership control and **COMPUTE** for compute sharing. Use **Next categories** to reach **MESH** and allow it for the intended peer. These category controls cycle their policy and apply immediately. Do this on both nodes.
5. In **Mesh Health**, select the peer and choose **Join Mesh** on both computers. If trust exists but the authenticated session is closed, use **Trusted Nodes → Open Session**.
6. Launch **Infinity Fabric** on each computer. Enable **Contribute resources**, choose CPU units, memory allowance and concurrent-job limits, and select **Apply contribution**.
7. Select **Refresh**. Inspect the peer's connection and trust, per-node available capacity, reservations, and combined totals. Select a node to inspect its details.

## Disconnect or stop sharing

Disable **Contribute resources** and apply to stop offering new work. **Mesh Health → Leave** withdraws membership; **Trusted Nodes → Revoke Trust** removes the trusted relationship. Policy changes are explicit and per peer.

## Scope and verification

CPU values are scheduling slots. Memory shown here is a provider execution allowance, not transparent shared addressable RAM. GPU execution requires a compatible provider. This UI does not establish arbitrary-workload CPU/RAM/GPU pooling as complete.

The connection instructions follow the native typed Settings actions and authority checks. Host UI tests cover guide lifecycle, contribution controls, selection, and zoom. This delivery does not claim a new two-node installed pairing acceptance run or large-display visual parity.
