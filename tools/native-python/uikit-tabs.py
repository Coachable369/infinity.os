"""Compile the reusable SVG tab states into the native UIKit artwork module."""
from pathlib import Path
import base64
import os
import re
import xml.etree.ElementTree as ET
import zlib
from PIL import Image, ImageDraw

# ------------------------=
# FUNC: render
# DESC: Rasterize the kit's restricted M/L/C SVG paths with supersampled curves, preserving the exact authored silhouette.
# ------------------=
def render(path, scale=1):
    svg=ET.parse(path).getroot()
    rect,shape=list(svg)
    factor=4*scale
    image=Image.new('RGB',(80*factor,32*factor),rect.attrib['fill'])
    tokens=re.findall(r'[MLC]|-?\d+(?:\.\d+)?',shape.attrib['d'])
    points=[];i=0;current=(0,0)
    while i<len(tokens):
        op=tokens[i];i+=1
        count=6 if op=='C' else 2
        values=list(map(float,tokens[i:i+count]));i+=count
        if op=='C':
            x0,y0=current;x1,y1,x2,y2,x3,y3=values
            for step in range(1,65):
                t=step/64;u=1-t
                points.append((u**3*x0+3*u*u*t*x1+3*u*t*t*x2+t**3*x3,u**3*y0+3*u*u*t*y1+3*u*t*t*y2+t**3*y3))
            current=(x3,y3)
        else:
            current=tuple(values);points.append(current)
    xy=[(round(x*factor),round(y*factor)) for x,y in points]
    draw=ImageDraw.Draw(image);draw.polygon(xy,fill=shape.attrib['fill'])
    draw.line(xy,fill=shape.attrib['stroke'],width=factor)
    return image.resize((80*scale,32*scale),Image.Resampling.LANCZOS)

# ------------------------=
# FUNC: main
# DESC: Publish one source-derived sprite module and a visual reference sheet under build-kit authority.
# ------------------=
def main():
    assert os.environ.get('INFINITY_BUILD_KIT_ACTIVE'), 'Use ./build-kit run'
    root=Path(__file__).resolve().parents[2]
    folder=root/'assets/ui-design-kit/default/workspace-tabs'
    states=['normal','selected','hover','focus']
    images=[render(folder/(state+'.svg'),2) for state in states]
    data=base64.b64encode(zlib.compress(b''.join(image.tobytes() for image in images),9)).decode()
    (root/'sdk/python/infinity_uikit_tabs.py').write_text('"""Generated from UIKit workspace-tabs SVG states; do not hand edit."""\nWIDTH=160\nHEIGHT=64\nSTATES='+repr(data)+'\n')
    sheet=Image.new('RGB',(640,96),'#090F26')
    for index,image in enumerate(images): sheet.paste(image,(index*160,16))
    sheet.save(folder/'states.png')

if __name__=='__main__': main()
