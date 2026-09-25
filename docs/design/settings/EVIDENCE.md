# Settings redesign verification — 2026-09-24

## Follow-up: shared two-line cards

The follow-up adds native icon wells, distinct title/description lines and a separate value column to the shared disclosure-card renderer. This is a visible composition change, not a complete implementation of every concept board.

- `sh tools/settings-overflow-test.sh` passes containment and non-overlap assertions for card columns at multiple widths/scales.
- A new disposable fresh installation passed all eleven category transitions, disclosure open/close, moved-row targeting and lower-row scrolling with the ISO detached. See [structured result](evidence/cards-fresh-install.json), [General expanded](evidence/cards-general-expanded.png), and [Nodes](evidence/cards-nodes.png).
- This fixture used the intermediate card build, before the shared AI kernel follow-up. Its archived ELF is not byte-identical to the final release ELF; do not describe it as final-image installed proof.
- `make x86_64 aarch64` subsequently completed successfully. Release ISO timestamps: x86_64 2026-09-24 19:17:30 CDT; AArch64 19:28:14 CDT. The user's existing VirtualBox installation was not upgraded by producing these files.
- Remaining visual limitations include repeated category icons in row wells, tight trailing disclosure spacing, background content showing through glass, and dashboard composition that is not yet identical to the kit. The earlier outstanding acceptance list still applies.

## What changed

The screenshot's overlapping Nodes & Mesh controls came from independently scaling fixed authored rectangles while keeping native text readable. Settings now uses responsive control geometry, with authored materials and artwork retained. Shared row paint and hit geometry agree. Short windows scroll instead of compressing text into undersized controls.

- Eleven category concept boards and one shared IDesign board were generated before implementation.
- Each category's real controls, inspection-only states, and interaction sequence are specified in [the kit](README.md).
- Sidebar selection uses a full padded row rather than a narrow decorative band; current theme icons remain supported.
- Dashboard tabs wrap, labels are bounded, summary/control spacing is explicit, and supporting panels stack when narrow.
- Expanded details have intrinsic heights; hidden detail regions no longer expose invisible generic actions.
- Window-button hit targets align with their painted centers.
- The color picker respects viewport clipping; expanded-detail scrolling rounds upward at high display scale.
- Nodes artwork preserves aspect ratio. Network detail text wraps. Storage artwork no longer paints behind interactive rows.
- Input labels come from the preference operation's own definitions, avoiding mismatches with stale template copy.
- Expanded-action buttons reserve a consistent width for their labels and affordances.

## Evidence levels

| Check | Evidence and limit |
| --- | --- |
| Production layout behavior | `sh tools/settings-overflow-test.sh` passed: category geometry, expansion flow, scrolling, dashboard control separation, and window-button hit centers. Tested dimensions include 1024×768, 1920×1080, and 2560×1440. |
| Color input | `make settings-color-test` passed with visible picker interaction and resulting appearance values. |
| Timeout | `make settings-timeout-test` passed: persistent user isolation, selected idle deadline, and slider range. |
| Initial fresh installation | `tools/settings-installed-test.py` installed a disposable x86 guest, detached the ISO, authenticated, and exercised General, Themes, and Privacy. This was before the subsequent visual corrections. |
| Installed corrected layout | The stopped disposable disk was updated with the corrected installed kernel using `tools/update-installed-clone.py`; boot records and unchanged disk extents were verified. Category review then used real pointer events and read-only structured state, not injected UI state. |
| Category coverage | All eleven categories were navigated; first-row disclosure open/close was exercised on eight disclosure-based categories. Input is immediate-action; Network and Nodes use dashboards. This does not prove every dashboard action or every expander. |
| Actual display | Guest firmware selected 2048×2048 despite requesting 1920×1080. Screenshots are native 2048×2048; they must not be described as 1080p visual evidence. |
| Fresh-install packaging | Live and installed kernels on x86_64 and AArch64 are checked for the exact packaged Settings template and diagnostic artifact. No new runtime art dependency was added; concept boards remain documentation only. |
| Release build | Final `make x86_64 aarch64` completed with exit status 0. Updated installer files are `builds/InfinityOS-x86_64.iso` and `builds/InfinityOS-aarch64.iso`. Existing compiler warnings remain; this is not a warning-free-build claim. |
| Final installed code | The tested fixture's archived installed ELF compared byte-for-byte equal to `build/x86_64/installed-kernel.elf`. The final installed review returned exit status 0. |

## Reproduction

Retained review artifacts:

- [Initial fresh-install structured result](evidence/initial-fresh-install.json)
- [Installed category-navigation result](evidence/category-review.json)
- [General expanded](evidence/general-expanded.png)
- [Nodes & Mesh](evidence/nodes.png)
- [Input](evidence/input.png)

```sh
sh tools/settings-overflow-test.sh
make settings-color-test settings-timeout-test
make x86_64 aarch64
python3 tools/settings-installed-test.py /tmp/infinity-settings-new-acceptance
```

For an already provisioned disposable fixture, `--review-existing` boots its retained installed disk without the ISO and captures all category screens. The fixture's archived installed ELF must match its installed kernel for diagnostic symbol lookup.

## Not accepted yet

This is not a claim of 100% image fidelity or a complete Settings product redesign. Outstanding acceptance work includes all expander variants, dashboard subtabs and validation/error states, keyboard workflows, and supported backend mutations. Generated concept boards contain illustrative controls that the native product does not implement; those are not functioning features. The saved provider policy also needs truthful summary binding rather than the existing fixed label. Full visual parity must be reviewed against those facts, not inferred from passing geometry or compilation.

The user's existing VirtualBox VM was not reinstalled or altered by this verification. All installed mutations were confined to the disposable QEMU fixture.
