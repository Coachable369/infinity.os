# Native browser fonts and image follow-up

## Findings and changes

Coachable requests common platform font families without always supplying a
generic family. The native catalog previously exposed only three families.
The browser now embeds the existing licensed Inter, Arimo and Roboto assets.
Segoe UI and Apple system-family names explicitly substitute Inter; Arial and
Helvetica names substitute Arimo. These are substitutions, not redistributed
Microsoft or Apple fonts. Live and installed kernels link the same component.

Font Awesome's solid WOFF2 downloaded successfully but failed sanitization.
Its header estimates 399,937 SFNT bytes; the native decoder reconstructs
400,116 bytes. OTS incorrectly constrained reconstruction to the estimate.
The staged patch permits alignment differences within both the existing OTS
limit and destination limit, and retains subsequent SFNT table validation.
It does not bypass sanitization or remove decompression limits.

Software-renderer atlases and picture tiles use smaller allocation units to
reduce sparse backing allocation. Image resolution and eviction semantics are
unchanged; this is not a measured page-speed claim.

## Behavioral evidence

- `builds/evidence/browser-font-catalog/result.json`: packaged local families
  load through native FontFace APIs, together with the lazy-image fixture.
- `builds/evidence/browser-fontawesome-direct/result.json`: actual downloaded
  Font Awesome WOFF2 fails before the sanitizer correction.
- `builds/evidence/browser-fontawesome-fixed/result.json`: the same font loads
  after correction; native modern-web checks pass with no rejected allocation.
- `tools/servo-platform-probe/coachable-images.js` exercises the site's actual
  gallery buttons and scrolls lazy images into view without replacing page DOM.
  Large gallery PNGs download and decode, so a blanket large-image cutoff was
  not the observed cause. The test host allowlist now includes the external
  badge host; its earlier rejection was a fixture limitation.

Native guest evidence is distinct from installed-OS verification. Full-page
gallery timing and final publication evidence remain separate gates.

## Button interaction

The shared application button renderer now consumes the compositor's pointer
state. Text Editor terminal actions publish hover/press hit regions and use that
renderer; disabled actions stay disabled. The native painter test checks that
idle, hover and pressed pixels differ and that repeated navigation repaints
match a clean scene without accumulated alpha.

No direct audio implementation, assets or configuration were changed.

## Transfer diagnosis

The featured Active Storage PNG is 2,222,910 bytes and redirects to a signed
same-origin disk URL. A native fixture run rejected that transfer at its own
30-second deadline. Production browser streaming already uses a 120-second
hard deadline. The fixture now uses that same deadline; no global HTTP timeout
or audio service behavior was changed. CPU rendering can delay network pumping,
so transfer success and full-page timing are checked separately.

## Current-page native acceptance

`builds/evidence/coachable-final-images/result.json` passes with the current
live homepage: 80/80 images decoded, zero unresolved images, load completion,
55,468,878 received bytes and zero failed allocations. Peak heap was
880,316,224 bytes. The test exercised every image, including galleries revealed
through the site's controls, over 340 seconds; this is not initial viewport
latency. The live site changed during this investigation, so it is not a valid
before/after speed comparison. Maximum paint work remains 2.44 seconds and is
not claimed to meet an interactive performance target.

The captured `page.png` shows the correct sans-serif families and icon glyphs.
The earlier screenshot's featured book URL requires a separate transfer check.
The ARM kernel check and installed-font byte parity are separate build gates.

## Revalidation deadlock

The native cache integration retained an existing entry's write guard across
HTTP revalidation, then attempted to reacquire the same guard. A 304 response
therefore stalled delivery indefinitely. Explicitly releasing existing entries
before revalidation preserves the first-request placeholder while allowing
conditional cache refresh. The earlier completion-notification experiment did
not resolve the failure and was removed. A deterministic native fixture now
fetches a no-cache resource twice, requiring a 200/304 round trip, identical
body delivery, and exactly two transport requests.

The corrected lock path passes `browser-font-cache-final` (manifest
`20261123T191514749871Z-11368.json`). The previously stalled 2,222,910-byte
image now completes and renders in `coachable-featured-image-lock-fixed`
(manifest `20261123T191552337329Z-11397.json`); its captured pixels were reviewed.

## Final native run and ARM publication

The final current-code homepage run, `coachable-release-images`, passed:
80/80 images decoded, no unresolved images, completed page load and no failed
allocation. Its manifest is `20261123T191648249191Z-11463.json`.
The complete gallery exercise took 346.6 seconds, with peak heap 890,647,552
bytes and maximum paint work 2.11 seconds. These results establish image
completion, not satisfactory interactive performance or installed-VM acceptance.

ARM installer: `builds/InfinityOS-aarch64.iso`.
Release manifest: `builds/manifests/20261123T192337485802Z-11976.json`.
SHA-256: `b962df94bd85bf9bb8210f5ff5690c80816b12171b1de66cb48d95db412f7454`.
Packaging, checksum and live/installed parity passed. Additional browser font
and artwork parity passed in manifest `20261123T193605774717Z-15406.json`.
Dependencies were reused under the user's incremental-release authorization.

## x86 publication

x86 installer: `builds/InfinityOS-x86_64.iso`.
Release manifest: `builds/manifests/20261123T194237781355Z-17607.json`.
SHA-256: `ec23bc602d6b635d2b4f72661f3dae1e0e6282cf343939c1ec4a8dc8e6bc5496`.
Packaging, checksum and live/installed parity passed. Additional browser font
and artwork parity passed in manifest `20261123T195418786988Z-21011.json`.
ARM publication completed before x86 packaging began. The broad regression
suite was deferred; neither package result establishes installed-VM acceptance.
