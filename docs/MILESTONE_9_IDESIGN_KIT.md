# InfinityOS Milestone 9 IDesign Kit

9-B introduces no visual redesign. Existing node frontends still require shared
typed routing, selected-resource detail and stale/reconciling states. The remote
engineering fixture is not visual acceptance. See [9-B status](MILESTONE_9B_CONTROL_PLANE.md).

## Phase 9-A evidence boundary

This increment changes no visual controls. Its trusted engineering operator
channel exposes local/remote NodeIds, transcript fingerprint, short code,
PairingId, requested scope, expiry, compatibility and explicit confirmation state.
Future GUI wiring must display locally computed values and require external
approval; it must not fetch and auto-submit an expected code. Final installed
two-screen human verification remains outside 9-A. See
[wire trust](MILESTONE_9A_WIRE_TRUST.md).

Status: IMPLEMENTED DESIGN CONTRACT

## Visual foundation

The node-trust surfaces extend `infinity.default.dark`: translucent midnight glass,
one-pixel accent outlines, soft cyan focus bloom, 12 px corners, and the existing
InfinityUI Roboto-like text atlas. No text is baked into imagery. The generated
topology hero is decorative; all node, trust, health, policy, and audit data is
rendered from typed runtime state.

## Layout

`Nodes & Mesh` is a first-class Settings destination with five stable pages:

1. Trusted Nodes — inventory, reachability, trust state, fingerprint summary.
2. Pair Node — secure pairing state, verification code, confirm/cancel actions.
3. Mesh Health — membership, role, heartbeat age, online/degraded/offline state.
4. Access Policy — twelve independently scoped authority categories, deny first.
5. Security Audit — bounded structured records with correlation and result state.

Each page uses the same geometry: seven-line-height summary card, 68/32 content
split, six large keyboard/mouse targets, and a right-side visual/status panel.
Nothing is positioned outside the content viewport.

## Typography and spacing

- Window title: strong 2x InfinityUI atlas.
- Page tabs and card headings: strong 1x, uppercase, 14 px minimum visual height.
- Body/value text: regular 1x with 24 px baseline spacing.
- Minimum interior gutter: 15 scaled pixels.
- Minimum interactive height: 40 scaled pixels.

## Interaction states

All controls expose idle, hover/focus, pressed, disabled, success, warning, and
denied states using the shared semantic button recipe. Keyboard order is tabs,
page controls, then trusted confirmation actions. Pair confirmation is permitted
only while secure input is leased to the Trusted UI owner. Closing or expiry
cancels the transaction without creating trust.

## Icon family

The existing vector semantic roles are reused for node, pairing, mesh, policy,
audit, shield, online, degraded, blocked, and revoked states. The topology art at
`assets/mesh/infinity-node-trust-topology-v1.png` supplies the atmospheric hero,
while interactive icons remain theme-tintable vector/runtime assets.

## Behavior contract

- Discovery never grants trust, membership, sessions, or capabilities.
- Pairing displays both a fingerprint and short verification code.
- Trust changes persist before IEF notification.
- Revocation immediately closes sessions and remote grants.
- Mesh membership is explicit and remains independent from trust.
- Audit rows contain structured IDs and state, not parsed log text.
- Empty and degraded states remain usable and explain the next safe action.
