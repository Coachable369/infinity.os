# Settings Service

Appearance settings are typed and scoped to machine, user, or session. Skin,
scale, accent, and wallpaper changes route through typed operations. Skin
activation is transactional and retains Previous and LastKnownGood references;
it cannot grant capabilities or override trusted UI semantics. The graphical
Settings surface includes an Appearance section backed by the same service.

Status: **TESTED** for typed persistent state and keyboard navigation of the
graphical Settings surface from an authenticated installed-disk session.

Settings is a typed service projection over authoritative machine, user,
AI-profile, and voice-profile objects. The GUI and Infinity Console do not keep
independent preference databases. Current scopes are System, Machine, User,
Session, and Application; Milestone 7 implements machine naming and user-scoped
appearance, local-AI policy, and opt-in voice state.

Unknown settings are rejected. Enabling voice changes preference only and does
not grant microphone authority. Remote AI remains denied unless the explicit
provider policy allows it.
