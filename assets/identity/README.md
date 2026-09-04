# InfinityOS Identity Artwork

`first-boot-node-v1-source.png` is the editable AI-generated source. The 16:9
`first-boot-node-v1.png` is the review/export image and
`first-boot-node-v1.bmp` is the deterministic boot-time framebuffer asset.

The artwork contains no baked interface text. Native controls, typography,
focus, pointer state, and live values are rendered by `kernel/core/bootstrap.rs`.
This lets onboarding, authentication, the desktop, and Settings reuse the same
visual language while remaining accessible and responsive.
