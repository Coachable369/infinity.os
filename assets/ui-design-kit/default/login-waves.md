# Login wave ribbons

Extends Eclipse Precision without changing panels, fonts, controls or focus states.
Reference: the authored Glossy Blue Infinity Wave Wallpaper in the login document.
Reuse that supplied artwork, not a replacement wallpaper or a new icon family.
Kit gap: no continuous audio-reactive ribbon component existed.

- Eight fine, phase-separated sine ribbons follow the wallpaper's horizontal flow.
  Cobalt glass bodies, cyan edges and thin ice-white highlights retain the
  reference's hierarchy; antialiased edges and restrained glow provide depth, not equalizer bars.
- Idle: continuous 12-second phase drift, no flashing, snapping or stopping at silence.
- Audio: authorized login-listener amplitude increases excursion and light visibly;
  approximately 40 ms attack and 300 ms release avoid twitching between syllables.
  No new microphone lease, recording, recognition path or permission is introduced.
- Paint behind the existing authored foreground. Animation damage excludes every
  visible foreground layer, including text, password controls, panels and branding.
  Restore the authored background before drawing each frame; never accumulate trails.
- Existing orb, splash and ripple choreography retains exclusive ownership of its
  damage area and timeline. Wave-only updates never repaint those pixels. The
  original choreography composites the current background waves beneath itself.
  Both hero and wave damage restore the cached static wallpaper rather than
  decoding it on each animation frame. Eight single-pass filaments replace
  twelve broad trail passes to bound per-frame blend work.
  Authored login controls settle independently of the unused legacy panel
  breathing timer; invisible legacy effects must never force whole-scene redraws.
  Full repaint requests also reuse the same wallpaper snapshot.
  Audio peaks are observed independently from the existing desktop visualizer.
- Reduced motion: fixed, quiet ribbons. Revoked/missing audio: immediate return to
  idle amplitude; no stale peak persists between accounts or capture sessions.
- Shared fixed-point native geometry on ARM64 and x86_64. No allocations, image
  decoding, host service or new framework in the animation loop.

Acceptance: production framebuffer captures at compact and wide sizes, idle and
speaking frames, bounded paint cost, protected-foreground pixel equality, envelope
attack/release, permission loss, reduced motion and live/installed packaging parity.
