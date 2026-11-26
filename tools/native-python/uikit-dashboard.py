"""Compile reusable dashboard SVG controls into native dependency-free UIKit sprites."""
from pathlib import Path
import base64
import os
import xml.etree.ElementTree as ET
import zlib
from PIL import Image, ImageDraw, ImageColor

ROOT = Path(__file__).resolve().parents[2]
FOLDER = ROOT / 'assets/ui-design-kit/default/fabric-workspace'

# ------------------------=
# FUNC: render
# DESC: Rasterizes the documented SVG primitive subset with four-times antialiasing.
# ------------------=
def render(path):
    svg = ET.parse(path).getroot()
    w,h = int(svg.get('width')),int(svg.get('height'))
    image = Image.new('RGB',(w*4,h*4),svg.get('data-backdrop','#0E1C36'))
    draw = ImageDraw.Draw(image)
    gradients={}
    for definition in svg.findall('.//{*}linearGradient'):
        gradients[definition.get('id')]=[(float(stop.get('offset').rstrip('%'))/100, ImageColor.getrgb(stop.get('stop-color'))) for stop in definition]
    for shape in svg:
        a=shape.attrib; kind=shape.tag.split('}')[-1]
        if kind=='defs': continue
        fill=None if a.get('fill','none')=='none' else a['fill']
        gradient=gradients.get(fill[5:-1]) if fill and fill.startswith('url(#') else None
        if gradient: fill=None
        stroke=a.get('stroke'); width=round(float(a.get('stroke-width',1))*4)
        if kind=='rect':
            x,y=float(a.get('x',0)),float(a.get('y',0)); sw,sh=float(a['width']),float(a['height'])
            bounds=(x*4,y*4,(x+sw)*4,(y+sh)*4)
            if gradient:
                layer=Image.new('RGB',image.size); paint=ImageDraw.Draw(layer)
                mask=Image.new('L',image.size); ImageDraw.Draw(mask).rounded_rectangle(bounds,radius=float(a.get('rx',0))*4,fill=255)
                for row in range(round(y*4),round((y+sh)*4)+1):
                    t=(row-y*4)/(sh*4)
                    start,end=gradient[0],gradient[-1]
                    for lo,hi in zip(gradient,gradient[1:]):
                        if lo[0]<=t<=hi[0]: start,end=lo,hi; break
                    f=max(0,min(1,(t-start[0])/max(.0001,end[0]-start[0])))
                    color=tuple(round(a+(b-a)*f) for a,b in zip(start[1],end[1]))
                    paint.line((0,row,w*4,row),fill=color)
                image.paste(layer,(0,0),mask)
            draw.rounded_rectangle(bounds,radius=float(a.get('rx',0))*4,fill=fill,outline=stroke,width=width)
        elif kind=='circle':
            x,y,r=float(a['cx']),float(a['cy']),float(a['r'])
            draw.ellipse(((x-r)*4,(y-r)*4,(x+r)*4,(y+r)*4),fill=fill,outline=stroke,width=width)
        elif kind=='line':
            draw.line(tuple(float(a[k])*4 for k in ('x1','y1','x2','y2')),fill=stroke,width=width)
        else: raise ValueError(kind)
    return image.resize((w,h),Image.Resampling.LANCZOS)

# ------------------------=
# FUNC: main
# DESC: Compiles SVG source pixels into a small frozen module and a reviewable component board.
# ------------------=
def main():
    assert os.environ.get('INFINITY_BUILD_KIT_ACTIVE'), 'Use build-kit run'
    sprites={}
    board=Image.new('RGB',(640,240),'#0E1C36')
    for i,path in enumerate(sorted(FOLDER.glob('*.svg'))):
        image=render(path)
        sprites[path.stem]=(image.width,image.height,base64.b64encode(zlib.compress(image.tobytes(),9)).decode())
        board.paste(image,(i%6*105,i//6*80))
    destination=ROOT/'sdk/python/infinity_uikit_dashboard_artwork.py'
    source='"""Generated from UIKit fabric-workspace SVG assets."""\nSPRITES='+repr(sprites)+'\n'
    if not destination.exists() or destination.read_text()!=source:
        destination.write_text(source)
    board.save(FOLDER/'components.png')

if __name__=='__main__': main()
