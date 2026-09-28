# Native charcoal welcome page

Target: `welcome-charcoal-v1.png`. Preserve its two-column hero, cyan heading,
neutral charcoal surface, two clear actions, three guidance cards and quiet AI footer.
Use existing Inter typography, native button states and browser/AI tab sprites.
No invented favorites folders, fake lock/privacy indicators or remote requests.

Checklist:
- Open the local welcome page when launching a new browser window from the catalog.
  Refocusing an existing browser preserves its tabs. Explicit URL launches navigate normally.
- Start browsing focuses the URL/search field; Browser settings opens real persisted settings.
- Keyboard focus, scroll and hit geometry remain usable at minimum window dimensions.
- Shared renderer and packaged hero are identical on ARM and x86, live and installed.
- Verify typed layout tests, installed interactions/screenshots, payload bytes, both ISO builds.

The welcome page is the default local start page, not a one-time dismissed onboarding.
It reappears on a new browser session and needs no extra per-user storage object.

Kit gap: no welcome hero existed. Built-in imagegen extracted only the smooth blue/cyan
glass infinity loop and its reflection from the approved mockup onto transparency,
removing text, chrome, cards, stars and backdrop. Master:
`assets/ui-design-kit/default/browser-welcome-hero-v1.png`.
`tools/build-browser-welcome-artwork.py` generates the bounded native alpha BMP.

## Verification

- Browser core: 40 behavioral tests passed, including nonoverlapping action/artwork/card
  geometry at 520–1900 logical pixels and 1–4x scale.
- Installed ARM, ISO detached: local catalog launch leaves engine state and completed
  network requests at zero; real Settings opens/closes; Tab/Enter focuses the omnibox;
  entering an HTTPS URL replaces the welcome with a successfully rendered page (HTTP 200).
- Final runtime receipt: `builds/manifests/20260928T132728108477Z-60054.json`.
- Pixel review: `installed-welcome-v1.png`. Corrected the clipped card description;
  preserved the charcoal background, balanced two-column hierarchy, alpha reflection,
  soft card borders and consistent gutters. Existing browser chrome remains shared.
- This runtime proof uses an updated disposable installed disk, not a fresh install
  from the final ISOs. x86 runtime acceptance has not been performed in this change.
