# InfinityOS Session Artwork

`infinity-default-dark-wallpaper-v2-source.png` is the editable authentication
scene. The 1920x1080 PNG is its packaged wallpaper master and the top-down
24-bit BMP is the deterministic framebuffer asset. No controls or text are
baked into it.

`infinity-shell-wallpaper-v3-source.png` is the editable authenticated desktop
scene. Its PNG is the packaged wallpaper master and its BMP is embedded in the
live and freshly installed kernels. Native UI overlays remain interactive,
resolution-aware, and independently damage tracked.

`infinity-onboarding-wallpaper-v1.png` is the 2048x1152 first-boot master. It
keeps low-detail negative space behind the onboarding card and moves the branded
infinity light sculpture to the right side of the composition. Its preserved
ImageGen source is in `../source-artwork/`; the 24-bit BMP is the deterministic
framebuffer form embedded in both live and freshly installed kernels.

`infinity-topbar-icon-v2.png` is the slimmer cyan-to-pearl desktop navigation
mark matched to the compact glass-rail reference. Its 32-bit BMP derivative is
the live framebuffer asset; the wordmark and every control remain native.
