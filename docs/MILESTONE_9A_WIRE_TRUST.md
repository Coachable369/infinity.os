# Milestone 9-A — Wire Trust Establishment

Milestone 9-A — Wire Trust Establishment: TESTED.
Overall Milestone 9: PARTIAL.

Evidence boundary: PRODUCTION WIRE / ENGINEERING GUEST and HOST, not final
INSTALLED SYSTEM acceptance.

## Contract and authority

The production path is NodeTransport → connected native UDP → IPv4/ARP/Ethernet
→ Intel 82540EM → QEMU virtual Ethernet → the other guest, in both directions.
Static IPv4 is the reference deployment. `NodeTransport::provision` binds the
configured native interface and exact remote endpoint under existing capability
and firewall policy. Inspection exposes endpoint/connection/peer identifiers,
never private keys. Discovery alone grants no trust, session, membership or IOP
authority. The trusted local service caller must explicitly begin and confirm.

The engineering guest uses firmware entropy independently on each machine. Its
UART operator channel carries public diagnostic records and explicit commands;
it does not forward protocol frames or inject identity/handshake material. The
external controller compares both independently derived fingerprints and codes,
then explicitly submits two approvals. No guest auto-confirms. This is not final
installed GUI human-verification evidence.

## Wire schema

Discovery retains its 40-byte nonce challenge and 199-byte signed advertisement
(see NODE_TRANSPORT.md). Pair/session envelopes use magic `IN9A0001`, message
class, protocol version 1, zero reserved bytes, source/destination NodeIds and a
32-byte transaction identifier: 112-byte header. Integer encoding is explicit
little-endian except endpoint ports, which are network order. IPv4 addresses
use the same family-tagged 19-byte endpoint encoding as discovery.

| Class | Body and authentication |
| --- | --- |
| PairBegin / HandshakeInit | 155-byte offer + Ed25519 signature; 331 bytes total |
| PairResponse / HandshakeResponse | Offer + initiator-message digest + signature; 363 bytes |
| Pair proof / ACK / Confirm / Cancel / HandshakeFinish / ACK | Transcript digest + signature; 208 bytes |
| SessionData / SessionClose | Shared session reference, direction-local sequence, length, encrypted bytes, Poly1305 tag; at most 346 bytes |

An offer contains the long-term public identity, fresh X25519 public value,
fresh nonce, bound endpoint, requested scope and prior confirmed pairing digest
(zero for initial pairing). Canonical transcript hashing binds protocol/class,
transaction identifier, ordered NodeIds and both complete offers. Identity keys
must hash to the discovered NodeIds; endpoints must match the connected link.
Both identities sign/prove the same digest. Verification is unavailable before
mutual proof. Its six-digit code derives locally from that digest; full digest,
local/remote identities, local PairingId, scope, expiry and compatibility are
available for independent comparison.

Explicit PendingVerification, LocallyConfirmed, RemotelyConfirmed, Confirmed,
Cancelled, Expired and Failed states prevent implicit approval. Confirmation
rechecks identity, compatibility, expiry, current trust restrictions and code.
Cancellation is terminal locally and best-effort authenticated on wire. Retired
transaction IDs cannot reopen. Both consent states are required before trust
commit; pairing does not automatically create a session.

## Session boundary and recovery

A separate fresh signed handshake binds the confirmed pairing digest and fresh
ephemeral material. Only authenticated transitions call the existing Crypto and
NodeRuntime session APIs. X25519/HKDF-SHA256 derives separate low→high and
high→low keys, ordered by NodeId. ChaCha20-Poly1305 authenticates the complete
data header. The shared protocol session reference is NOT either machine's
runtime-local handle; handles never appear in nonce derivation or on wire.
Each direction maintains its own monotonically increasing sequence/replay state.

Close authenticates its notification before locally zeroizing keys. Timeout,
offline, trust loss and terminal negotiation clear secret/pending state. A
returning peer requires a fresh signed handshake and gets a new reference and
local handles. Old ciphertext cannot authenticate in the new session.

Confirmed wire pairing digests and retired transaction IDs are boot-scoped in
this adapter. Cold restart/persisted wire-trust recovery is not claimed. Native
return acceptance pauses/resumes the peer VM; it does not cold-boot an installed
disk. SessionData is bounded UDP control data, not reliable stream delivery.
Remote IOP, GUI/Console convergence, synchronized membership and full IEF
reconciliation remain M9-B work; installed lifecycle/performance is M9-C.

## Resource and failure contract

| Resource | Bound |
| --- | --- |
| Pairing + handshake + active wire slots | 4 total |
| Retired transaction IDs | 16, fail closed when full; no replay-tombstone eviction |
| Existing runtime session slots | 16; wire service uses at most 4 concurrently |
| Pending transport packets | 1 per transaction + 1 discovery response per link |
| Received protected payloads | 8 × 192 bytes |
| Maximum accepted trust envelope | 363 bytes; exact class-specific lengths |
| Retry | Every 2 seconds, at most 12 transmissions |
| Verification / handshake lease | 120 seconds |
| Session lifetime | 3600 seconds, then a fresh handshake is required |
| Close retry lifetime | 6 seconds |
| Replay window | Strict increasing sequence, one high-water value per direction |
| Work admission | One received transition and one send per link per second |

No peer wait, busy-spin, heap-backed queue, compositor mutation or per-packet
durable IEF publication is introduced. Existing fixed-size audit storage bounds
security records. Typed NodeError/NetworkError results cover malformed/version,
identity/signature, endpoint, verification, expiry, trust, queue, replay and AEAD
failures. Invalid in-flight identity proof terminates negotiation. Established
data authentication failure does not advance the receive sequence or expose
payload bytes. Backpressure is explicit, not unbounded buffering.

Structured audit classes: 0xda01 begin, 0xda02 cancel, 0xda03 verification denied,
0xda04 negotiation failure/expiry, 0xda05 rejected frame with typed error,
0xda06 close, plus existing 0xd001 paired and 0xd003 session established.
Records include public IDs/outcomes, not private, ephemeral or traffic secrets.

## Verification checklist and evidence

Final build: `./build.sh` exit 0 on 2026-09-07, including both architecture ISOs,
exact installed-kernel and loader parity, UI/input, retained-drag, resource-policy,
network-wire and node host behavior gates. `make milestone-9-test` also exited 0
independently after the last production change. `make native-nic-test` exited 0:
native paired guest outcomes [33, 33], absent-peer outcome [35]. Final
`make milestone-9-wire-trust-test` exited 0 after the finished ISO build and all
production changes. Tests are behavioral:

- `make milestone-9-test`: production frame encoding/decoding in HOST scope,
  independent confirmation, cancellation and terminal rejection, offline during
  verification/handshake, address loss, exact protected payloads and audit state;
  existing discovery, pairing, duplex, authority and resource regression cases.
- `make milestone-9-wire-trust-test`: two independent firmware-booted native
  engineering guests; actual E1000/UDP discovery, authenticated pairing,
  independently compared codes, explicit approvals, deliberately unequal local
  handles, duplex data, directional counters, replay both ways, reflection,
  tag/cipher/reference/sequence tamper, close, fresh reconnect, old-frame rejection,
  peer loss during PairBegin and active session, return without resurrection.
- `./build.sh`: architecture builds, installed binary/payload parity, existing
  UI/input, retained-drag, resource-policy and network-wire gates.
- `make native-nic-test`: existing independent native NIC regression.

UART results are binary typed records; exact bytes, counters, state transitions,
audit classes and exit statuses are assertions. Rendered text/log searches are
not acceptance oracles. Native proof is saved to
`build/milestone-9a-wire-proof.json`. The B fixture first abandons an unrelated
local-only pending transaction to prove handle inequality; real peer trust and
session material are subsequently negotiated exclusively on wire.

The new production module is compiled into the existing core node runtime in
both live and installed kernel payloads; no separate host/test component is
installed. No new default network grant is introduced. Binary installer parity
is packaging evidence, not INSTALLED SYSTEM behavioral acceptance.

Final native proof, 2026-09-07:

- Independent NodeIds:
  `75ebd612dc916f383252ecb1a50d0572bf3fba1486f089c1970be55b5cac98b5`
  and `8ef42844695a121a0b26a048852b0c25488c57322ec13e8c038167579ec68edc`.
- First negotiated local handles: A=3, B=4.
- Independently matched verification; two explicit external approvals.
- Three distinct authenticated session references:
  `0709ac3689283986e7183f1a31052478`,
  `fbe49c5c0fb6325b341ba148357876cc`,
  `2c7437b5e3a08f936acd3e767a1e83c7`.
- Native duplex payload/counter assertions, bidirectional replay, reflection,
  invalid tag/cipher/reference/sequence, old-frame rejection, close, offline
  PairBegin, offline session, and explicit fresh reconnect all passed.
- Return mechanism: QEMU stop/cont. Installed GUI acceptance: false.

## Phase acceptance matrix

W = PRODUCTION WIRE / ENGINEERING GUEST. H = HOST.

| Item | Status | Evidence |
| --- | --- | --- |
| Production Endpoint Provisioning | TESTED | W + H |
| Signed Wire Discovery | TESTED | W + H adversarial cases |
| Wire PairBegin | TESTED | W + H |
| Authenticated Pairing Transcript | TESTED | W + H |
| Mutual NodeIdentity Authentication | TESTED | W + H |
| Independent Verification Material | TESTED | W, independently generated and compared |
| Explicit PairConfirm | TESTED | W, external operator channel |
| Authenticated Wire Handshake | TESTED | W + H |
| Secure Session Establishment | TESTED | W + H |
| Independent Local Handles | TESTED | W, 3 versus 4 |
| Directional Keys | TESTED | W + H |
| A→B Encrypted Traffic | TESTED | W |
| B→A Encrypted Traffic | TESTED | W, including B-initiated second payload |
| A→B Replay Rejection | TESTED | W |
| B→A Replay Rejection | TESTED | W |
| Reflection Rejection | TESTED | W |
| Invalid Tag Rejection | TESTED | W |
| Session Close | TESTED | W + H |
| Reconnect / Fresh Session | TESTED | W |
| Old-Frame Rejection | TESTED | W |
| Offline During Pairing | TESTED | W PairBegin; H verification/handshake |
| Offline During Session | TESTED | W |
| Return / Fresh Reconnect | TESTED | W, pause/resume rather than cold reboot |
| Bounds | TESTED | H behavioral limits; W bounded timeout/no-delivery checks |
| Audit | TESTED | H typed failure records; W paired/open/close records |
| Build Regression | TESTED | H, full build + binary installed payload parity |
| Performance Regression | TESTED | H, existing UI/input and retained-drag gates |
| Resource-Policy Regression | TESTED | H, existing gates |
| Documentation | TESTED | H, reviewed against implementation and recorded outcomes |

UNSUPPORTED within this reference wire path: ARM64 native wire, other NIC
families, IPv6 wire, TCP/DHCP/wire DNS. Final installed GUI, persisted cold-restart
wire recovery and active-network desktop acceptance remain PARTIAL overall and
outside this phase; no completion or test claim is made for them.

Next dependency: M9-B must carry versioned remote IOP requests/results over these
authenticated sessions and revalidate execution-context capabilities/current
trust at dispatch. This increment deliberately does not implement that layer.

## Build artifacts and performance

2026-09-07 artifact hashes (SHA-256):

- `builds/InfinityOS-x86_64.iso`:
  `0d3683b4cf24855270227ec34ca6a0f5d421eb223b9ac5b4d8f5572984ed2b48`
- `builds/InfinityOS-aarch64.iso`:
  `9cdeef6bc2611594ee575a684fd0fca5991afd0c6917b46c782cb523feb4be8b`

HOST retained drag: 300 frames, average 34,303 ns, p95 48,750 ns, worst 95,375 ns.
No installed desktop under active node traffic was measured. ARM64 builds and
payload parity pass, but this phase does not add ARM64 native wire support.
The ISO build consumes the working tree, including preserved pre-existing user
UI/asset edits; those unrelated edits are not part of this phase's commit.
