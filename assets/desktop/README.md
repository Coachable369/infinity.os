# InfinityOS Session Artwork

`infinity-desktop-wallpaper-v1-source.png` is the preserved ImageGen result.
The 1920x1080 PNG is the review master and the top-down 24-bit BMP is the
deterministic framebuffer asset. No controls or text are baked into it.

The same scene underpins account selection, authentication, lock, and desktop
surfaces so the session transition is visually continuous. Native UI overlays
remain interactive, resolution-aware, and independently damage tracked.

`infinity-onboarding-wallpaper-v1.png` is the 2048x1152 first-boot master. It
keeps low-detail negative space behind the onboarding card and moves the branded
infinity light sculpture to the right side of the composition. Its preserved
ImageGen source is in `../source-artwork/`; the 24-bit BMP is the deterministic
framebuffer form embedded in both live and freshly installed kernels.

`infinity-topbar-icon-v1.png` is the generated transparent master for the
compact desktop brand mark. `infinity-topbar-icon-v1.bmp` preserves the alpha
channel in a deterministic 32-bit framebuffer asset so the illuminated ribbon
composites cleanly over every top-bar state without a baked backdrop.

`infinity-topbar-icon-v2.png` is the slimmer cyan-to-pearl desktop navigation
mark matched to the compact glass-rail reference. Its 32-bit BMP derivative is
the live framebuffer asset; the wordmark and every control remain native.
