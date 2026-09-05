# Settings Service

Appearance settings are typed and scoped to machine, user, or session. Skin,
scale, accent, and wallpaper changes route through typed operations. Skin
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

## Installed theme color

`Settings > Themes & Skins > Accent` expands to an inline HSV color picker. Pointer
movement previews the selected color immediately; release commits it once to
the authenticated user's profile. The selected RGB value is restored when that
user next enters the installed desktop. Older profiles with no stored color
resolve to Infinity Blue (`#20BFFF`).

The picker changes one semantic accent token rather than recoloring individual
widgets. Derived roles cover window outlines, title/header surfaces, the system
top bar, dock glass, widgets, focus indicators, and selection backgrounds. This
keeps contrast and surface depth consistent across the desktop while allowing
the user's color to remain recognizable.

The identity-state encoder owns persistence; the skin registry owns live color
resolution; and Settings owns input/preview. This separation keeps the setting
available to the installed System Generation without making the live ISO a
runtime dependency.

Unknown settings are rejected. Enabling voice changes preference only and does
not grant microphone authority. Remote AI remains denied unless the explicit
provider policy allows it.
