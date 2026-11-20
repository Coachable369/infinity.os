"""Reusable native application chrome from the InfinityOS UIKit specification."""

MENU_HEIGHT = 36
TOOLBAR_HEIGHT = 48

# ------------------------=
# FUNC: workspace_theme
# DESC: Applies the Studio-authored semantic palette and fixed spacing to standard native toolkit controls.
# ------------------=
def workspace_theme(root, style, scale=1):
    from infinity_uikit_artwork import COLORS
    c=COLORS
    body=("Inter",-round(16*scale))
    style.configure(".",background=c["panel"],foreground=c["text"],font=body)
    for name in ("TFrame","TLabel","TLabelframe","TLabelframe.Label"):
        style.configure(name,background=c["panel"],foreground=c["text"],borderwidth=0)
    for name in ("TEntry","TCombobox"):
        style.configure(name,fieldbackground=c["panel"],background=c["panel"],foreground=c["text"],
                        insertcolor=c["text"],arrowcolor=c["muted"],bordercolor=c["border"],
                        lightcolor=c["panel"],darkcolor=c["panel"],padding=round(8*scale))
        style.map(name,foreground=[("disabled",c["muted"]),("readonly",c["text"])],
                  fieldbackground=[("readonly",c["panel"])])
    for name in ("TButton","Accent.TButton","TMenubutton","Workspace.TMenubutton"):
        style.configure(name,foreground=c.get("primaryButtonText" if name=="Accent.TButton" else "buttonText",c["text"]),padding=(round(12*scale),round(8*scale)),font=body)
        style.map(name,foreground=[("disabled",c["muted"])])
    root.option_add("*TCombobox*Listbox.background",c["panel"])
    root.option_add("*TCombobox*Listbox.foreground",c["text"])
    root.option_add("*TCombobox*Listbox.selectBackground",c["accent"])
    return c

# ------------------------=
# FUNC: workspace_menu
# DESC: Open native workspace menus on a completed click so the opening release cannot select or dismiss the popup.
# ------------------=
def workspace_menu(button, menu):
    # ------------------------=
    # FUNC: press
    # DESC: Show a pressed state without transferring input to a popup before release.
    # ------------------=
    def press(event):
        if not button.instate(["disabled"]):
            button.state(["pressed"])
        return "break"
    # ------------------------=
    # FUNC: release
    # DESC: Post below the button only after a click ends within its bounds.
    # ------------------=
    def release(event):
        armed = button.instate(["pressed", "!disabled"])
        button.state(["!pressed"])
        if armed and 0 <= event.x < button.winfo_width() and 0 <= event.y < button.winfo_height():
            # A native grab can pump pending events. Complete this release's
            # dispatch before acquiring the popup grab to avoid re-entry.
            button.after_idle(post)
        return "break"
    # ------------------------=
    # FUNC: post
    # DESC: Acquire the popup grab after the opening release has left event dispatch.
    # ------------------=
    def post():
        if button.winfo_exists() and not button.instate(["disabled"]):
            button.focus_force()
            menu.tk_popup(button.winfo_rootx(), button.winfo_rooty()+button.winfo_height())
    button.configure(menu=menu, direction="below")
    button.bind("<ButtonPress-1>", press)
    button.bind("<ButtonRelease-1>", release)
    button.bind("<B1-Leave>", lambda event: "break")

# ------------------------=
# FUNC: workspace_controls
# DESC: Share production-rendered UIKit button and field states with native ttk consumers at their explicit content scale.
# ------------------=
def workspace_controls(root, style, scale=1):
    import base64
    import zlib
    import tkinter as tk
    from infinity_uikit_artwork import SURFACES
    workspace_theme(root,style,scale)
    key=round(scale*100)
    cache=getattr(root,"_infinity_control_images",{})
    if key not in cache:
        raw=zlib.decompress(base64.b64decode(SURFACES))
        width,height=round(80*scale),round(40*scale)
        images=[]
        for state in range(12):
            pixels=raw[state*9600:(state+1)*9600]
            rows=[]
            for y in range(height):
                sy=min(39,int(y/scale))
                rows.append(b"".join(pixels[(sy*80+min(79,int(x/scale)))*3:(sy*80+min(79,int(x/scale)))*3+3] for x in range(width)))
            data=f"P6\n{width} {height}\n255\n".encode()+b"".join(rows)
            images.append(tk.PhotoImage(master=root,data=data,format="PPM"))
        cache[key]=images;root._infinity_control_images=cache
        for prefix,offset in (("Secondary",0),("Primary",4),("Field",8)):
            style.element_create(f"UIKit{key}.{prefix}","image",images[offset],
                ("disabled",images[offset+3]),
                ("pressed",images[offset+2]),("focus",images[offset+1]),("active",images[offset+1]),
                border=round(8*scale),padding=(round(12*scale),round(8*scale)),sticky="nsew")
    for name,prefix in (("TButton","Secondary"),("Accent.TButton","Primary")):
        style.layout(name,[(f"UIKit{key}.{prefix}",{"sticky":"nsew","children":[("Button.label",{"sticky":"nsew"})]})])
    style.layout("TEntry",[(f"UIKit{key}.Field",{"sticky":"nsew","children":[("Entry.textarea",{"sticky":"nsew"})]})])
    style.layout("TCombobox",[(f"UIKit{key}.Field",{"sticky":"nsew","children":[
        ("Combobox.downarrow",{"side":"right","sticky":"ns"}),
        ("Combobox.padding",{"sticky":"nsew","children":[("Combobox.textarea",{"sticky":"nsew"})]})]})])
    workspace_scrollbars(root,style,scale)
    return cache[key]

# ------------------------=
# FUNC: workspace_tabs
# DESC: Apply the reusable soft-shoulder UIKit tab silhouette with native themed state images.
# ------------------=
def workspace_tabs(root, style, scale=1):
    import tkinter as tk
    from infinity_uikit_artwork import COLORS
    key=round(scale*100)
    cache=getattr(root,"_infinity_tab_images",{})
    element=f"Workspace{key}.tab"
    if key in cache:
        images=cache[key]
    else:
        images=_tab_images(root, scale)
        cache[key]=images
        root._infinity_tab_images=cache
        style.element_create(element, "image", images[0],
                             ("selected focus", images[3]), ("selected", images[1]), ("active", images[2]),
                             border=(round(28*scale),0,round(28*scale),0), padding=(round(24*scale),round(4*scale),round(24*scale),round(4*scale)), sticky="nsew")
    style.layout("Workspace.TNotebook.Tab", [(element, {"sticky":"nsew", "children":[("Notebook.label", {"sticky":"nswe"})]})])
    style.configure("Workspace.TNotebook", background=COLORS["canvas"], borderwidth=0, tabmargins=(0,0,0,0))
    style.configure("Workspace.TNotebook.Tab", padding=(round(32*scale),round(4*scale)), font=("Inter",-round(18*scale)), foreground=COLORS["text"])
    style.map("Workspace.TNotebook.Tab", foreground=[("selected","#FFFFFF"),("active","#FFFFFF")])
    return images

# ------------------------=
# FUNC: _tab_images
# DESC: Rasterize each UIKit state at the chosen display scale, retaining smooth shoulders without vertical tiling.
# ------------------=
def _tab_images(root, scale):
    import base64
    import zlib
    import tkinter as tk
    from infinity_uikit_tabs import WIDTH, HEIGHT, STATES
    raw=zlib.decompress(base64.b64decode(STATES))
    width,height=round(80*scale),round(32*scale)
    images=[]
    for state in range(4):
        pixels=raw[state*WIDTH*HEIGHT*3:(state+1)*WIDTH*HEIGHT*3]
        rows=[]
        for y in range(height):
            sy=min(HEIGHT-1,y*HEIGHT//height)
            rows.append(b"".join(pixels[(sy*WIDTH+min(WIDTH-1,x*WIDTH//width))*3:(sy*WIDTH+min(WIDTH-1,x*WIDTH//width))*3+3] for x in range(width)))
        images.append(tk.PhotoImage(master=root,data=f"P6\n{width} {height}\n255\n".encode()+b"".join(rows),format="PPM"))
    return images

# ------------------------=
# FUNC: workspace_scrollbars
# DESC: Apply shared rounded UIKit thumb states with native proportional dragging and track paging.
# ------------------=
def workspace_scrollbars(root,style,scale=1):
    import base64
    import zlib
    import tkinter as tk
    from infinity_uikit_artwork import COLORS, SCROLLBARS
    key=round(scale*100)
    cache=getattr(root,"_infinity_scrollbar_images",{})
    if key not in cache:
        raw=zlib.decompress(base64.b64decode(SCROLLBARS))
        images=[]
        for axis,(w,h) in enumerate(((14,32),(32,14))):
            width,height=round(w*scale),round(h*scale)
            for state in range(4):
                pixels=raw[(axis*4+state)*w*h*3:(axis*4+state+1)*w*h*3]
                rows=[]
                for y in range(height):
                    sy=min(h-1,int(y/scale))
                    rows.append(b"".join(pixels[(sy*w+min(w-1,int(x/scale)))*3:(sy*w+min(w-1,int(x/scale)))*3+3] for x in range(width)))
                images.append(tk.PhotoImage(master=root,data=f"P6\n{width} {height}\n255\n".encode()+b"".join(rows),format="PPM"))
            offset=axis*4
            style.element_create(f"UIKit{key}.{'Horizontal' if axis else 'Vertical'}.thumb","image",images[offset],
                ("disabled",images[offset+3]),("pressed",images[offset+2]),("active",images[offset+1]),
                border=round(6*scale),sticky="nsew")
        trough=tk.PhotoImage(master=root,width=1,height=1)
        trough.put(COLORS["canvas"],to=(0,0,1,1))
        images.append(trough)
        style.element_create(f"UIKit{key}.trough","image",trough,border=0,sticky="nsew")
        cache[key]=images
        root._infinity_scrollbar_images=cache
    for axis in ("Vertical","Horizontal"):
        style.layout(f"{axis}.TScrollbar",[(f"UIKit{key}.trough",{"sticky":"nswe","children":[
            (f"UIKit{key}.{axis}.thumb",{"sticky":"nswe"})]})])
        style.configure(f"{axis}.TScrollbar",troughcolor=COLORS["canvas"],background=COLORS["muted"],
                        bordercolor=COLORS["canvas"],borderwidth=0,arrowsize=0,width=round(14*scale))
    return cache[key]
