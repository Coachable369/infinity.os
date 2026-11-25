# Infinity Fabric Manager design specification

Reference: `fabric-manager-reference.png`. Reuse Sapphire UIKit and compositor-owned Eclipse Precision frames. Reference values are illustrative only; production must use authenticated management snapshots.

## Geometry and states

Fixed compositor title; 36 px File/View/Help menu; 48 px action toolbar. Minimum 1024x700. Left navigation 210 px, right contribution inspector 260 px; center reflows, never stretches controls. Inter 18 px body, 20 px card headings, 32 px capacity totals. Eight-pixel spacing grid; 12 px control inset, 40 px control height. Sapphire canvas #091522, surfaces #0E1C36, primary #245AD4, foreground #F2F6FC, secondary #AEC3DA, steel #365774. Reuse existing filled toggles, slider states, menus, buttons, and tab images.

Overview places CPU, memory, and accelerator cards above node and job tables. Selecting a node displays its identity, trust, reachability, freshness, resource limits and allocations. Only authorized mutations are enabled. Contribution updates use expected revisions and explicit Apply; failed persistence leaves values unchanged. No inaccessible remote policy controls should imply authority.

## Data fidelity

Compute units are slots, never label them cores without a core-specific resource contract. Memory advertisements are not proof of remotely addressable RAM. Show advertised capacity separately from executable support; unsupported accelerators show unavailable. Stale/offline/withdrawn resource capacity is excluded from usable totals. Node names must come from authoritative identity; otherwise display abbreviated ID with full ID available.

## Acceptance still required

Native normal/maximized/restored screenshots, keyboard and pointer selection, refresh without selection loss, authorization failure, revision conflict, offline node, no nodes, and disabled contribution. Compare hierarchy, spacing, controls and surface rendering against reference. Production rendering and installed-node acceptance remain open.
