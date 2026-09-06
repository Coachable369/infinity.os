# Settings Service

Appearance settings are typed and scoped to machine, user, or session. Skin,
scale, primary, secondary, and wallpaper changes route through typed operations. Skin
activation is transactional and retains Previous and LastKnownGood references;
it cannot grant capabilities or override trusted UI semantics. The graphical
Settings surface includes an Appearance section backed by the same service.

Configuration rows use a single-open inline accordion: activating the twiddle
reveals that row's real control or explanation directly beneath its summary,
and activating it again collapses the well. Content is clipped to an internal
viewport and mouse-wheel or scrollbar paging is used whenever the resized
window cannot show the complete accordion. No configuration control is painted
outside the Settings window.

Status: **TESTED** for typed persistent state, graphical color selection, live
semantic-theme propagation, and keyboard navigation of the graphical Settings
surface from an authenticated installed-disk session.

Settings is a typed service projection over authoritative machine, user,
AI-profile, and voice-profile objects. The GUI and Infinity Console do not keep
independent preference databases. Current scopes are System, Machine, User,
Session, and Application; Milestone 7 implements machine naming and user-scoped
appearance, local-AI policy, and opt-in voice state.

`Settings > Privacy & Security > No Activity Timeout` is a durable, user-scoped
slider from 1 through 120 minutes. The default is 5 minutes. Keyboard activation
cycles practical presets, while pointer input previews the exact minute and
commits once on release. Authenticated keyboard, pointer, and wheel activity
restart the deadline; expiry locks the session while preserving desktop layout.

## Installed theme colors

`Settings > Themes & Skins` exposes adjacent `Primary` and `Secondary` rows.
Each expands to its own inline HSV color picker. Pointer movement previews the
selected color immediately; release performs one durable commit. Primary is
machine-scoped and controls the frosted window, header, navigation, dock, and
widget surfaces. Secondary is user-scoped and controls outlines, focus,
selection, twiddles, and scrollbar details. The values remain independent and
are restored when the installed desktop starts.

The pickers change semantic theme tokens rather than recoloring individual
widgets. This keeps contrast and surface depth consistent across the desktop
while allowing both theme colors to remain recognizable.

The identity-state encoder owns persistence; the skin registry owns live color
resolution; and Settings owns input/preview. This separation keeps the setting
available to the installed System Generation without making the live ISO a
runtime dependency.

## Network

The Network section is a responsive seven-page editor: Overview, Interfaces,
IPv4, DNS, Routes, Profiles, and Policy. Mouse and keyboard users can select a
connection mode, enable or disable the discovered adapter, configure a static
IPv4 address/prefix/default gateway/metric, return to dynamic addressing, set
two typed DNS endpoints, remove a static default route, activate an operational
profile, and select the unmatched outbound policy. The adjacent inspector shows
only live observed counts and adapter state.

Each mutation passes through the Settings service's scoped network capability,
updates native typed runtime state, and commits the versioned network-state
object on the installed system. The GUI does not parse Console output or keep a
separate settings database. Responsive geometry, direct hit targeting, atomic
static-address replacement, invalid-prefix rollback, and binary configuration
round-trip are **HOST-TESTED**. Direct installed-VM visual acceptance remains
pending. Wire DHCP, Wi-Fi association, and DNS packet exchange remain dependent
on their respective device and protocol adapters and are not fabricated by the
Settings surface.

Unknown settings are rejected. Enabling voice changes preference only and does
not grant microphone authority. Remote AI remains denied unless the explicit
provider policy allows it.
