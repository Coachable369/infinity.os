# UIKit quick start

1. Copy `/InfinitySDK/Examples/Python/counter.py` to your Documents folder.
2. Open it in Text Editor and press Run. A native Counter window appears with
   a display and two buttons. Click + and -; the value changes by one each time.
3. Stop the app. Change the title in `ui.window`, save and run again. Change a
   button's label or client geometry to see a controlled layout edit.
4. Keep IDs 1 and 2 linked to their existing handler actions. Closing the window
   must exit the event loop; it must not leave a background busy loop.

The equivalent C example is `/InfinitySDK/Examples/C/counter.c`. The calculator
example adds a larger grid of controls. These use the same OS renderer and
current theme as system applications; they do not recreate the theme in Python.

## Design workflow

Read `EclipsePrecision.md`, inspect `reference.png`, then use `catalog.json` and
`Controls.md` to identify reusable elements and states. `uikit.infinityui` is the
shared Studio design document; `atlas.rgba` and `styles.bin` are runtime resources,
not editable application documents. Use Infinity Studio to change the source
theme and its normal save/update workflow to deploy it. Do not overwrite the
immutable installed SDK or assume editing a reference image changes a running OS.

## Reference and limits

The public element kinds are label, display and button. See
`../APIKit/Reference.md` and `../APIKit/C/include/infinity/uikit.h` for calls,
structures and limits. `Controls.md` also describes OS-internal controls; that
catalog does not imply every control has a public Python factory.

Window borders, focus adorners and chrome belong to the OS compositor. Lay out
client contents only. Keep text readable and inside bounds, preserve theme
states, and test both pointer and AI activation. Do not stretch the whole atlas
as a button image or substitute an unrelated host theme.

## Expected results and troubleshooting

The display updates on each valid click, invalid IDs make no change, and close
ends execution. If elements overlap, check client dimensions and coordinates.
If styling differs, confirm the installed OS/theme version and Studio deployment;
the SDK reference is versioned, while a user may select a different active theme.
