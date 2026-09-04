# InfinityUI Layout

Layout operates on `Point`, `Size`, `Rect`, `Insets`, and fixed-point `Scale`.
Rows and columns divide bounded content rectangles deterministically. Images use
explicit fit or cover behavior. Dialog placement clamps to the display work area.

The default login uses the reference 1536×1024 grid: 64-unit top bar, 521×754
authentication card at `(54,123)`, and 478×90 utility tray at `(529,914)`.
Proportional centering preserves this relationship at 1280×720, 1920×1080, and
larger modes without cutting off the infinity artwork or interactive controls.

Scale math and cover behavior at 1×, 1.25×, 1.5×, and 2× are TESTED. Complex
constraint solving and arbitrary transform layout are PLANNED.
