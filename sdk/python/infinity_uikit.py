"""Reusable native application chrome from the InfinityOS UIKit specification."""

MENU_HEIGHT = 36
TOOLBAR_HEIGHT = 48

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
    key=round(scale*100)
    cache=getattr(root,"_infinity_control_images",{})
    if key not in cache:
        raw=zlib.decompress(base64.b64decode(SURFACES))
        width,height=round(80*scale),round(40*scale)
        images=[]
        for state in range(9):
            pixels=raw[state*9600:(state+1)*9600]
            rows=[]
            for y in range(height):
                sy=min(39,int(y/scale))
                rows.append(b"".join(pixels[(sy*80+min(79,int(x/scale)))*3:(sy*80+min(79,int(x/scale)))*3+3] for x in range(width)))
            data=f"P6\n{width} {height}\n255\n".encode()+b"".join(rows)
            images.append(tk.PhotoImage(master=root,data=data,format="PPM"))
        cache[key]=images;root._infinity_control_images=cache
        for prefix,offset in (("Secondary",0),("Primary",3),("Field",6)):
            style.element_create(f"UIKit{key}.{prefix}","image",images[offset],
                ("pressed",images[offset+2]),("focus",images[offset+1]),("active",images[offset+1]),
                border=round(8*scale),padding=(round(12*scale),round(8*scale)),sticky="nsew")
    for name,prefix in (("TButton","Secondary"),("Accent.TButton","Primary")):
        style.layout(name,[(f"UIKit{key}.{prefix}",{"sticky":"nsew","children":[("Button.label",{"sticky":"nsew"})]})])
    style.layout("TEntry",[(f"UIKit{key}.Field",{"sticky":"nsew","children":[("Entry.textarea",{"sticky":"nsew"})]})])
    return cache[key]

# ------------------------=
# FUNC: workspace_tabs
# DESC: Apply the reusable soft-shoulder UIKit tab silhouette with native themed state images.
# ------------------=
def workspace_tabs(root, style, scale=1):
    import tkinter as tk
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
                             border=(round(32*scale),round(40*scale)-1,round(32*scale),0), padding=(round(24*scale),round(9*scale),round(24*scale),round(9*scale)), sticky="nsew")
    style.layout("Workspace.TNotebook.Tab", [(element, {"sticky":"nsew", "children":[("Notebook.label", {"sticky":"nswe"})]})])
    style.configure("Workspace.TNotebook", background="#090F26", borderwidth=0, tabmargins=(0,0,0,0))
    style.configure("Workspace.TNotebook.Tab", padding=(round(32*scale),round(9*scale)), font=("Inter",-round(18*scale)), foreground="#E6EAF2")
    style.map("Workspace.TNotebook.Tab", foreground=[("selected","#FFFFFF"),("active","#FFFFFF")])
    return images

# ------------------------=
# FUNC: _tab_images
# DESC: Rasterize each UIKit state at the chosen display scale, retaining smooth shoulders without vertical tiling.
# ------------------=
def _tab_images(root, scale):
    import tkinter as tk
    width,height=round(80*scale),round(40*scale)
    # Image borders retain the shoulder geometry while Tk stretches only the flat center.
    profile=(100,80,73,69,67,65,63,61,59,57,55,53,51,49,47,45,43,41,39,37,35,33,31,29,27,25,23,21,19,17,14,11,8,5,3,1,0)
    images = []
    for fill, edge in (("#101B3A", "#536984"), ("#243651", "#92AEC8"), ("#2E4263", "#AAC8E2"), ("#243651", "#C9D2E0")):
        fill_rgb=tuple(int(fill[i:i+2],16) for i in (1,3,5))
        edge_rgb=tuple(int(edge[i:i+2],16) for i in (1,3,5))
        pixels=bytearray()
        for y in range(height):
            t=y*36/(height-1)
            i=min(35,int(t));fraction=t-i
            inset=(profile[i]*(1-fraction)+profile[i+1]*fraction)*height/144
            for x in range(width):
                distance=min(x+0.5-inset,width-0.5-inset-x)
                coverage=min(1,max(0,distance+0.5))
                interior=0 if y==0 else min(1,max(0,distance-0.5))
                color=tuple(round(bg*(1-coverage)+e*(coverage-interior)+f*interior)
                            for bg,e,f in zip((9,15,38),edge_rgb,fill_rgb))
                pixels.extend(color)
        data=f"P6\n{width} {height}\n255\n".encode()+pixels
        images.append(tk.PhotoImage(master=root,data=data,format="PPM"))
    return images
