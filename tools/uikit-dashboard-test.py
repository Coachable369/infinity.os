"""Behavioral SVG raster and distribution parity checks for shared dashboard controls."""
import base64
import json
from pathlib import Path
import runpy
import sys
import zlib
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'sdk/python'))
from infinity_uikit_dashboard_artwork import SPRITES
render=runpy.run_path(str(ROOT/'tools/native-python/uikit-dashboard.py'))['render']
for name,(width,height,data) in SPRITES.items():
    raster=render(ROOT/'assets/ui-design-kit/default/fabric-workspace'/f'{name}.svg')
    assert raster.size==(width,height)
    assert raster.tobytes()==zlib.decompress(base64.b64decode(data))
assert SPRITES['toggle-on'][2]!=SPRITES['toggle-off'][2]
assert SPRITES['toggle-on'][2]!=SPRITES['toggle-hover-on'][2]
package=runpy.run_path(str(ROOT/'tools/package-sdk.py'))
manifest,payloads,entries=package['catalog']()
for filename in ('infinity_uikit_dashboard.py','infinity_uikit_dashboard_artwork.py'):
    assert payloads['UIKit/'+filename]==(ROOT/'sdk/python'/filename).read_bytes()
installed=package['install_tree'](ROOT)
assert any(row[0]=='/System/infinity-fabric.py' for row in installed)
# Actual native imports and widget rendering are exercised by the installed harness.
print('Dashboard SVG pixels and canonical SDK/install payload parity passed')
