# AI window tab — UIKIT correction

Target: the supplied dark-window reference with a luminous side fin, not the
older rectangular editor assistant kit. Retain the existing assistant behavior.

Shared recipe:
- 48 × 104 logical-pixel attached tab, unchanged when expanded.
- Straight root attached to the window edge; outward-sloping shoulders joined
  smoothly to the outer edge. No closed octagonal button outline.
- Antialiased navy glass, blue-to-violet rim, bounded four-pixel exterior glow.
- Centered existing four-point AI mark; no extra text or replacement icon.
- Mirror for left-edge attachment; keep the control visible when maximized.

`kernel/ui/assistant_tab.rs` is the production pixel recipe used by every caller
of the shared window assistant painter. The proof harness renders these same
pixels (without the separately painted star glyph) and verifies left/right
symmetry, attachment coverage, and transparent outer corners at three scales.

Visual component proof: `build/behavior-tests/assistant-tab-proof.png`.
This is component-render evidence, not a screenshot of a cold-installed desktop.
The generated recipe is compiled into shared live and installed kernels, with
no new packaged image dependency. ISO rebuild and installed-window visual
verification remain required before claiming full UIKIT acceptance.
