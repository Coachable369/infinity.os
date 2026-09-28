# Browser settings

Reuse the default Infinity IDesign Kit and browser sapphire skin. No new raster
asset is needed: the right-hand control uses the native settings gear family.
Settings occupies the page area, never a floating dialog. Keep tabs and address
chrome intact, with a Back to page action and Escape dismissal.

Use 24px page gutters, 8px rhythm, 32px controls, 14px native Inter, dark navy
surfaces, soft blue borders and cyan selected states. Search provider choices
form one segmented row; appearance and data actions use balanced label/control
rows. A second click explicitly confirms clearing favorites. Persist changes
before showing success. Failed saves retain the last committed values.

Scope: Google/DuckDuckGo/Bing address-bar search, favorites-bar visibility,
clear favorites, restore defaults. TLS and download consent remain mandatory;
do not expose unsupported engine switches. Settings are private per-user native
object-store metadata, shared by installed/live and both CPU targets. Version-two
browser state embeds preferences in the existing favorites object; legacy records
retain defaults until explicitly changed. This requires no additional namespace
entry, including on fresh installs whose bounded namespace table is already full.

## Verification

- 38 browser-core behavior tests pass, including provider routing and hidden-rail
  hit geometry at scales 1–4.
- Native object-store regression fills the namespace table, upgrades legacy
  favorites in place, cold-remounts, and checks preferences plus original URLs.
- ARM installed acceptance exercises all provider choices, visibility, keyboard
  activation, clear cancellation/confirmation, defaults, and detached cold reboot.
  Receipt: `build/browser-installed-1790580476233745000/result.json`.
- The initial fresh installation passed payload parity but exposed namespace
  exhaustion. The corrected acceptance resumed on that disposable disk with a
  verified kernel replacement; it is not a second unmodified fresh-install run.
- `installed-settings-v1.png` is the real installed desktop capture. Browser
  spacing, control bounds, selected states and favorites placement were reviewed.
  The desktop top-bar overlap outside the browser is pre-existing and out of scope.
- x86 uses the same source but was not built or installed-tested for this change.
