"""Reusable image-backed dashboard cards and accessible filled toggles."""
import base64
import tkinter as tk
from tkinter import ttk
import zlib
from infinity_uikit_dashboard_artwork import SPRITES

# ------------------------=
# FUNC: dashboard_controls
# DESC: Installs SVG-derived dashboard surfaces and toggle states without changing global button styles.
# ------------------=
def dashboard_controls(root, style):
    images=getattr(root,'_infinity_dashboard_images',None)
    if images is not None: return images
    images={}
    for name,(width,height,packed) in SPRITES.items():
        data=f'P6\n{width} {height}\n255\n'.encode()+zlib.decompress(base64.b64decode(packed))
        images[name]=tk.PhotoImage(master=root,data=data,format='PPM')
    root._infinity_dashboard_images=images
    for name,color in (('card','#112D48'),('panel','#0E1C36')):
        element='Dashboard.'+name
        style.element_create(element,'image',images[name],border=16,sticky='nsew')
        style.layout('Dashboard'+name.title()+'.TFrame',[(element,{'sticky':'nsew'})])
        style.configure('Dashboard'+name.title()+'.TLabel',background=color,foreground='#F2F6FC',font=('Inter',-16))
    style.element_create('Dashboard.toggle','image',images['toggle-off'],
                         ('disabled',images['toggle-disabled']),('selected active',images['toggle-hover-on']),
                         ('selected',images['toggle-on']),('active',images['toggle-hover-off']),sticky='')
    style.layout('Dashboard.TCheckbutton',[
        ('Checkbutton.padding',{'sticky':'nsew','children':[
            ('Dashboard.toggle',{'side':'right','sticky':'ns'}),
            ('Checkbutton.focus',{'side':'left','sticky':'nsew','children':[
                ('Checkbutton.label',{'sticky':'nsew'})]})]})])
    style.configure('Dashboard.TCheckbutton',background='#0E1C36',foreground='#F2F6FC',font=('Inter',-16),padding=(0,4))
    style.map('Dashboard.TCheckbutton',foreground=[('disabled','#73899E')],background=[('active','#0E1C36')])
    return images
