"""Compile original reusable surface artwork from the approved modern collection recipes."""
from pathlib import Path
import math
import os
from PIL import Image
ROOT = Path(__file__).resolve().parents[1]
KIT = ROOT / 'assets/ui-design-kit/default'
RES = ROOT / 'tools/installer-designer/Sources/InfinityInstallerStudio/Resources'
BUTTONS = ['Optical Glass','Liquid Pill','Frosted Slate','Solid Sapphire','Porcelain','Carbon','Aurora Silk','Clear Outline','Floating Surface','Tinted Glass','Text Action','Luminous Glass','Optical Capsule','Faceted Sapphire','Prism Glass']
FRAMES = ['Sapphire Hairline','Soft Shadow','Frosted Rim','Graphite Edge','Porcelain Edge','Satin Silver','Aurora Trace','Horizon Light','Inset Shadow','Cobalt Accent']

# ------------------------=
# FUNC: clamp
# DESC: Bounds a linear channel to the RGBA byte range.
# ------------------=
def clamp(v): return max(0, min(255, round(v)))

# ------------------------=
# FUNC: button
# DESC: Produces scalable material artwork; runtime owns antialiased silhouette and fixed corner geometry.
# ------------------=
def button(family, state):
    im = Image.new('RGBA',(256,64)); pixels=[]
    bases=[(52,81,108,225),(19,45,89,228),(51,67,87,245),(35,88,213,255),(235,235,229,255),(30,34,41,255),(63,51,133,255),(22,46,79,0),(24,35,53,255),(22,34,53,216),(23,51,88,0),(7,31,62,220),(7,35,65,225),(8,34,66,230),(14,25,63,230)]
    for y in range(64):
        for x in range(256):
            r,g,b,a=bases[family]; t=y/63; u=x/255
            light=0
            if family==0: light=60*(1-t)+ (31 if t < .70-.62*u else -3)
            elif family==1: light=24*math.exp(-t*7)-10*t
            elif family==2: light=5*(1-t)+0.65*math.sin(x*12.9898+y*78.233)
            elif family==4: light=10*(1-t)-4*t
            elif family==6: r+=25*u; g-=7*u; b+=25*u; light=9*(1-t)
            elif family==8: light=10*(1-t)-3*t
            elif family==9: light=13*(1-t)+13*(math.sin(u*9+t*4)+math.cos(u*14-t*7))*.5
            if family>=11:
                light=15*math.exp(-t*9)+8*t*t
                if family==12: light+=32*max(0,1-t*3) if t < .55-.5*u else 0
                if family==13: light+=10*abs(2*t-1)
                if family==14:
                    light+=23 if t < 1-u*1.8 else 0
                    r+=int(23*u*t); b+=int(35*u*t)
                if state==1: light+=14+12*t
                if state==2: light-=7
            if state==1: light+=8
            if state==2: light-=10
            if family in (7,10): a=0 if state==0 else (105 if state==1 else 170)
            pixels.append((clamp(r+light),clamp(g+light),clamp(b+light),a))
    im.putdata(pixels); return im

# ------------------------=
# FUNC: frame
# DESC: Builds continuous modern rectangular frame bands with preserved empty client area and active/inactive states.
# ------------------=
def frame(family,state):
    im=Image.new('RGBA',(64,64)); pixels=[]
    bases=[(76,157,224),(101,126,151),(157,190,218),(106,124,141),(224,234,245),(169,185,202),(94,142,223),(125,176,218),(49,72,94),(34,139,230)]
    # Edge alpha profiles, in physical pixels at 1x. No stacked rails or bevels.
    profiles=[(225,36,0),(125,38,15),(185,50,12),(150,22,0),(225,25,0),(190,32,0),(205,28,0),(185,40,10),(105,40,10),(230,34,0)]
    for y in range(64):
        for x in range(64):
            d=min(x,y,63-x,63-y); c=bases[family]
            a=profiles[family][d] if d<3 else 0
            light=8 if y<3 else -10 if y>60 else 0
            if family==6: c=(94+x//2,142-x//3,223)
            if family==7 and y>3: a=a*2//5
            if state==0: light-=12; a=a*3//5
            pixels.append(tuple(clamp(v+light) for v in c)+(a,))
    im.putdata(pixels);return im

# ------------------------=
# FUNC: main
# DESC: Publishes identical bounded RGBA assets to native UIKit and Studio without modifying existing themes.
# ------------------=
def main():
    assert os.environ.get('INFINITY_BUILD_KIT_ACTIVE'), 'Use ./build-kit run'
    for filename,names,render in [('modern-buttons-v1.rgba',BUTTONS,button),('modern-frames-v1.rgba',FRAMES,frame)]:
        data=b''.join(render(i,s).tobytes() for i in range(len(names)) for s in range(3))
        (KIT/filename).write_bytes(data);(RES/filename).write_bytes(data)
if __name__=='__main__': main()
