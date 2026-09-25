# InfinityOS Authentication Success Motion Kit v1

## Intent

Turn a successful password verification into one continuous 1.35-second visual response. The motion confirms success immediately, lets the authentication orb fall into the lake, resolves the impact through luminous water, and only then reveals the restored desktop.

## Layer recipe

| Layer | Asset | Resting treatment | Animated treatment |
| --- | --- | --- | --- |
| Environment | `infinity-auth-success-stage-v1.bmp` | Full-bleed clean scene with generated resting ripple composited once | Restored only inside the bounded orb lane from the pixel-identical stage |
| Orb | `infinity-auth-success-orb-v1.bmp` | 138 px nominal diameter at 1080p | Ease-in drop, subtle pre-drop lift, compression at contact, submerge fade |
| Water | `infinity-auth-success-ripple-v1.bmp` | One quiet 430 px nominal ring field | Two staggered expanding waves with independent opacity |
| Impact | `infinity-auth-success-splash-v1.bmp` | Hidden | Fast crown bloom, then graceful dissolve |

## Keyframes

| Time | Orb | Water | Interface |
| ---: | --- | --- | --- |
| 0 ms | Resting at 72% viewport height | Quiet ring at 82% | Password accepted; input ignored |
| 0-180 ms | 6 px lift and brighter core | Gentle contraction | Existing authentication card remains stable |
| 180-610 ms | Accelerating fall to contact | Ring tightens toward contact | Cursor remains coherent |
| 610-760 ms | Compresses and begins submerging | Impact crown blooms | White-blue contact flare |
| 760-1,180 ms | Fades beneath the surface | Primary wave expands; secondary wave follows | No additional input accepted |
| 1,180-1,350 ms | Gone | Waves dissolve | Desktop transition commits |

## Motion and performance rules

- Use elapsed-time progress, not frame counts, so slow guests do not stretch the sequence.
- Animate only the lower-right water/impact region; do not rebuild the full 4K scene per tick.
- Preserve the header and authentication card until the desktop commit, avoiding text shimmer.
- Keep the generated moving raster layers transparent and render them through the coherent software back buffer. The resting ripple is baked into the stage plate to prevent a restoration seam.
- Run the same sequence for initial sign-in and locked-session resume.
- Enter the desktop only after the final keyframe; failed credentials never start the effect.

## Acceptance

- A correct password starts progress at zero while the authentication surface remains active.
- Mid-sequence framebuffer state contains a lower orb position and a brighter contact region than the resting frame.
- Late-sequence framebuffer state contains expanded ripple energy with the orb substantially submerged.
- Incorrect passwords remain on the static authentication frame.
- Fresh-installed, ISO-detached login and resume paths both complete on the desktop after the animation.
