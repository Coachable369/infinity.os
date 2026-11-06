"""Extract the approved reusable account avatar from the UI kit reference."""
import os
from pathlib import Path
from PIL import Image, ImageDraw
assert os.environ.get('INFINITY_BUILD_KIT_ACTIVE'), 'Use ./build-kit run'
kit=Path(__file__).resolve().parents[1]/'assets/ui-design-kit/default'
im=Image.open(kit/'accounts-media-reference.png').convert('RGBA')
# Reference coordinates are normalized to the approved 1774x887 board.
w,h=im.size
box=tuple(round(v*(w/1774 if i%2==0 else h/887)) for i,v in enumerate((502,217,633,348)))
avatar=im.crop(box).resize((128,128),Image.Resampling.LANCZOS)
mask=Image.new('L',(512,512));ImageDraw.Draw(mask).ellipse((2,2,510,510),fill=255)
avatar.putalpha(mask.resize((128,128),Image.Resampling.LANCZOS))
(kit/'account-avatar-v1.rgba').write_bytes(avatar.tobytes())
avatar.save(kit/'account-avatar-v1.png')
