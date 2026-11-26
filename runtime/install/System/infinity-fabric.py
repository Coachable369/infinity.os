"""Native Fabric management; all observations and mutations come from the OS authority."""
import json
import sys
import tkinter as tk
from tkinter import ttk
from _infinity_app import fabric_request, fabric_node_request
from infinity_uikit import workspace_controls, workspace_menu, MENU_HEIGHT
from infinity_uikit_artwork import COLORS
from infinity_uikit_dashboard import dashboard_controls

# ------------------------=
# FUNC: size
# DESC: Formats resource bytes without implying remotely addressable memory.
# ------------------=
def size(value):
    return f"{value / (1024 ** 3):.1f} GiB" if value >= 1024 ** 3 else f"{value / (1024 ** 2):.1f} MiB"

class FabricManager:
    # ------------------------=
    # FUNC: __init__
    # DESC: Constructs anchored cluster and node views using packaged native UIKit controls.
    # ------------------=
    def __init__(self, root):
        self.root = root
        self.snapshot = None
        self.selected = None
        self.dirty = False
        self.notice = None
        self.policy_error = False
        self.job_records = {}
        self.selected_node = None
        root.title("Infinity Fabric")
        maximum_width, maximum_height = root.maxsize()
        width = min(1400, root.winfo_screenwidth()-64, maximum_width)
        height = min(880, root.winfo_screenheight()-140, maximum_height)
        root.geometry(f"{width}x{height}")
        root.minsize(min(1024, width), min(700, height))
        root.configure(bg=COLORS['canvas'])
        self.zoom = 1.0
        font = ('Inter', -18)
        root.option_add('*Font', font)
        style = self.style = ttk.Style(root)
        style.theme_use('clam')
        style.configure('TLabel', background=COLORS['panel'], foreground=COLORS['text'], font=font)
        style.configure('TFrame', background=COLORS['panel'])
        style.configure('Fabric.Treeview', background=COLORS['panel'], fieldbackground=COLORS['panel'],
                        foreground=COLORS['text'], font=font, rowheight=40, borderwidth=0,
                        lightcolor=COLORS['panel'], darkcolor=COLORS['panel'], bordercolor=COLORS['panel'])
        style.configure('Fabric.Treeview.Heading', background='#19334E', foreground=COLORS['text'], font=('Inter',-16,'bold'), padding=(8,4),
                        lightcolor=COLORS['panel'], darkcolor=COLORS['panel'], bordercolor=COLORS['border'], relief='flat')
        style.map('Fabric.Treeview', background=[('selected', '#164F79')], foreground=[('selected','#FFFFFF')])
        self.images = workspace_controls(root, style, 1.0)
        self.dashboard_images = dashboard_controls(root, style)
        style.configure('Workspace.TMenubutton', background=COLORS['panel'], borderwidth=0,
                        lightcolor=COLORS['panel'], darkcolor=COLORS['panel'], bordercolor=COLORS['panel'],
                        arrowcolor=COLORS['muted'], relief='flat')
        style.map('Workspace.TMenubutton', background=[('active', COLORS['accent'])])
        style.layout('Workspace.TMenubutton', [('Menubutton.padding', {'sticky':'nswe', 'children':[('Menubutton.label', {'sticky':'w'})]})])
        menu_bar = self.menu_bar = ttk.Frame(root, height=MENU_HEIGHT)
        self.menus = []
        menu_bar.pack(fill='x'); menu_bar.pack_propagate(False)
        for title, entries in (('File', (('Refresh', self.refresh), ('Close', root.destroy))),
                               ('View', (('Zoom in   Cmd +', lambda: self.set_zoom(self.zoom + .1)),
                                         ('Zoom out   Cmd −', lambda: self.set_zoom(self.zoom - .1)),
                                         ('Actual size   Cmd 0', lambda: self.set_zoom(1.0)))),
                               ('Help', (('Connect nodes…', self.connection_guide), ('Resource units', self.explain)))):
            button = ttk.Menubutton(menu_bar, text=title, takefocus=True, style='Workspace.TMenubutton')
            menu = tk.Menu(button, tearoff=False, bg=COLORS['panel'], fg=COLORS['text'],
                           activebackground=COLORS['accent'], font=font)
            for label, action in entries: menu.add_command(label=label, command=action)
            self.menus.append(menu)
            workspace_menu(button, menu)
            button.pack(side='right' if title=='Help' else 'left', fill='y', padx=8)
        toolbar = self.toolbar = ttk.Frame(root, height=48)
        toolbar.pack(fill='x'); toolbar.pack_propagate(False)
        ttk.Label(toolbar, text='Overview', font=('Inter', -22)).pack(side='left', padx=16)
        ttk.Button(toolbar, text='Refresh', command=self.refresh).pack(side='right', padx=12, pady=4)
        ttk.Button(toolbar, text='Connect nodes…', command=self.connection_guide).pack(side='right', padx=8, pady=4)
        self.status = tk.StringVar(value='Connecting to Fabric service…')
        ttk.Label(root, textvariable=self.status, padding=(16,8)).pack(side='bottom', fill='x')
        body = ttk.Frame(root, padding=16)
        body.pack(fill='both', expand=True)
        body.columnconfigure(0, weight=1)
        body.rowconfigure(1, weight=3, minsize=110)
        body.rowconfigure(2, weight=2, minsize=100)
        cards = ttk.Frame(body)
        cards.grid(row=0, column=0, sticky='ew', pady=(0,12))
        self.totals = []
        self.card_icons = []
        self.cards = cards
        for i, heading in enumerate(('CPU slots', 'Memory allowance', 'Accelerators')):
            cards.columnconfigure(i, weight=1, uniform='resource')
            card = ttk.Frame(cards, padding=12, style='DashboardCard.TFrame')
            card.grid(row=0, column=i, sticky='nsew', padx=(0,8))
            icon = ttk.Label(card, image=self.dashboard_images[('cpu','memory','gpu')[i]], style='DashboardCard.TLabel')
            icon.pack(side='left', padx=(0,12))
            self.card_icons.append(icon)
            style.configure('DashboardCardInterior.TFrame', background='#112D48')
            summary = ttk.Frame(card, style='DashboardCardInterior.TFrame')
            summary.pack(side='left', fill='x', expand=True)
            ttk.Label(summary, text=heading, style='DashboardCard.TLabel').pack(anchor='w')
            value = tk.StringVar(value='Unavailable')
            ttk.Label(summary, textvariable=value, font=('Inter', -24, 'bold'), style='DashboardCard.TLabel').pack(anchor='w', pady=(8,0))
            self.totals.append(value)
        cards.bind('<Configure>', self.card_layout)
        center = ttk.Frame(body, padding=12, style='DashboardPanel.TFrame')
        center.grid(row=1, column=0, sticky='nsew')
        ttk.Label(center, text='Nodes', font=('Inter',-22,'bold'), padding=(8,4)).pack(anchor='w')
        self.nodes = ttk.Treeview(center, columns=('state','cpu','memory','trust'), show='tree headings', style='Fabric.Treeview', selectmode='browse', height=2)
        self.nodes.heading('#0', text='Node',anchor='w'); self.nodes.column('#0', width=160, minwidth=140)
        for key, label, width in [('state','Connection',100),('cpu','CPU slots',80),('memory','Memory',100),('trust','Trust',80)]:
            self.nodes.heading(key,text=label,anchor='w'); self.nodes.column(key,width=width,minwidth=80)
        self.node_scroll = ttk.Scrollbar(center, orient='vertical', command=self.nodes.yview)
        self.node_scroll.pack(side='right', fill='y')
        self.nodes.configure(yscrollcommand=self.node_scroll.set)
        self.nodes.pack(fill='both', expand=True)
        self.nodes.bind('<<TreeviewSelect>>', self.select)
        self.detail = tk.StringVar(value='Select a node to inspect its resources.')
        # Full node identity stays accessible without consuming the table's rows.
        # Node identity is shown in the inspector rather than crowded into the toolbar.
        jobs_panel = ttk.Frame(body, padding=12, style='DashboardPanel.TFrame')
        jobs_panel.grid(row=2, column=0, sticky='nsew', pady=(12,0))
        job_header = ttk.Frame(jobs_panel)
        job_header.pack(fill='x', pady=(0,8))
        ttk.Label(job_header, text='Job activity', font=('Inter',-20,'bold')).pack(side='left')
        self.cancel_button = ttk.Button(job_header, text='Cancel job', command=self.cancel_job, state='disabled')
        self.cancel_button.pack(side='right')
        self.jobs = ttk.Treeview(jobs_panel, columns=('node','state','cpu','memory'),
                                show='tree headings', style='Fabric.Treeview', height=2)
        self.jobs.heading('#0', text='Job',anchor='w'); self.jobs.column('#0', width=90, minwidth=60)
        for key,label,width in [('node','Node',140),('state','Status',120),('cpu','CPU units',90),('memory','Reserved memory',150)]:
            self.jobs.heading(key,text=label,anchor='w'); self.jobs.column(key,width=width,minwidth=70)
        job_scroll = ttk.Scrollbar(jobs_panel, orient='vertical', command=self.jobs.yview)
        job_scroll.pack(side='right',fill='y')
        self.jobs.configure(yscrollcommand=job_scroll.set)
        self.jobs.bind('<<TreeviewSelect>>', self.select_job)
        self.job_status = tk.StringVar()
        self.empty_jobs = ttk.Label(self.jobs,textvariable=self.job_status,image=self.dashboard_images['jobs'],compound='top',justify='center',padding=8)
        self.empty_jobs.place(relx=.5,rely=.65,anchor='center')
        self.jobs.pack(fill='both',expand=True)
        inspector_host = self.inspector_host = ttk.Frame(body, width=312)
        inspector_host.grid(row=0,column=1,rowspan=3,sticky='ns',padx=(12,0))
        inspector_host.grid_propagate(False)
        inspector_host.rowconfigure(0, weight=1)
        inspector_host.columnconfigure(0, weight=1)
        self.inspector_canvas = tk.Canvas(inspector_host, width=264, highlightthickness=0, bg=COLORS['panel'])
        self.inspector_canvas.grid(row=0,column=0,sticky='nsew')
        inspector_scroll = ttk.Scrollbar(inspector_host, orient='vertical', command=self.inspector_canvas.yview)
        inspector_scroll.grid(row=0,column=1,sticky='ns')
        self.inspector_canvas.configure(yscrollcommand=inspector_scroll.set)
        inspector = ttk.Frame(self.inspector_canvas, padding=(12,16), style='DashboardPanel.TFrame')
        self.inspector_window = self.inspector_canvas.create_window(0,0,window=inspector,anchor='nw')
        inspector.bind('<Configure>', self.inspector_layout)
        self.inspector_canvas.bind('<Configure>', self.inspector_layout)
        inspector.columnconfigure(0, weight=1)
        ttk.Label(inspector,text='This computer',image=self.dashboard_images['monitor'],compound='left',font=('Inter',-20,'bold')).grid(row=0,column=0,columnspan=2,sticky='w',pady=(0,12))
        self.enabled = tk.BooleanVar()
        self.cpu = tk.StringVar(); self.memory = tk.StringVar(); self.tasks = tk.StringVar()
        self.toggle = ttk.Checkbutton(inspector,text='Contribute resources',variable=self.enabled,command=self.mark_dirty,style='Dashboard.TCheckbutton')
        self.toggle.grid(row=1,column=0,columnspan=2,sticky='ew',pady=(0,12))
        self.entries = []
        for row,(label,var) in enumerate((('CPU units',self.cpu),('Memory (MiB)',self.memory),('Concurrent jobs',self.tasks)),2):
            ttk.Label(inspector,text=label).grid(row=row,column=0,sticky='w',padx=(0,8),pady=6)
            entry = ttk.Entry(inspector,textvariable=var,width=7)
            entry.grid(row=row,column=1,sticky='ew',pady=6)
            entry.bind('<KeyRelease>',self.mark_dirty)
            self.entries.append(entry)
        self.apply_button = ttk.Button(inspector,text='Apply contribution',command=self.apply,style='Accent.TButton')
        self.apply_button.grid(row=10,column=0,columnspan=2,sticky='ew',pady=(12,8))
        ttk.Button(inspector,text='Discard edits',command=self.reload_policy).grid(row=11,column=0,columnspan=2,sticky='ew',pady=(0,12))
        self.usage = tk.StringVar()
        ttk.Label(inspector,textvariable=self.usage,wraplength=240).grid(row=12,column=0,columnspan=2,sticky='w')
        ttk.Separator(inspector,orient='horizontal').grid(row=13,column=0,columnspan=2,sticky='ew',pady=(12,12))
        ttk.Label(inspector,text='Selected node details',font=('Inter',-20,'bold')).grid(row=14,column=0,columnspan=2,sticky='w',pady=(0,12))
        self.node_fields={}
        self.node_field_labels={}
        for row,label in enumerate(('Identity','Trust','Connection'),15):
            ttk.Label(inspector,text=label).grid(row=row,column=0,sticky='w',pady=6)
            value=tk.StringVar(value='—'); self.node_fields[label]=value
            field=ttk.Label(inspector,textvariable=value,wraplength=132,compound='left')
            field.grid(row=row,column=1,sticky='w',pady=6)
            self.node_field_labels[label]=field
        ttk.Separator(inspector,orient='horizontal').grid(row=18,column=0,columnspan=2,sticky='ew',pady=12)
        self.node_detail = tk.StringVar(value='Select a node to inspect it.')
        ttk.Label(inspector,textvariable=self.node_detail,wraplength=220,justify='left').grid(row=19,column=0,columnspan=2,sticky='w')
        self.font_widgets = []
        self.collect_fonts(root)
        for modifier in ('Mod4', 'Control') + (('Command',) if sys.platform == 'darwin' else ()):
            root.bind(f'<{modifier}-plus>', lambda event: self.set_zoom(self.zoom + .1))
            root.bind(f'<{modifier}-equal>', lambda event: self.set_zoom(self.zoom + .1))
            root.bind(f'<{modifier}-minus>', lambda event: self.set_zoom(self.zoom - .1))
            root.bind(f'<{modifier}-0>', lambda event: self.set_zoom(1.0))
        self.set_zoom(1.0)
        from infinity_app_design import apply
        apply(root, "fabric", menu=self.menu_bar, content=body)
        self.refresh()
        root.after(2000,self.tick)

    # ------------------------=
    # FUNC: collect_fonts
    # DESC: Retains explicit typography roles so zoom never compounds font rounding.
    # ------------------=
    def collect_fonts(self, widget):
        if 'font' in widget.keys() and widget.cget('font'):
            import tkinter.font as tkfont
            font = tkfont.Font(root=self.root, font=widget.cget('font'))
            parts = self.root.tk.splitlist(widget.cget('font'))
            pixels = abs(int(parts[1])) if len(parts)>1 else abs(font.cget('size'))
            self.font_widgets.append((widget, max(18, pixels), font.cget('weight')))
        for child in widget.winfo_children():
            self.collect_fonts(child)

    # ------------------------=
    # FUNC: set_zoom
    # DESC: Zooms readable workspace content without changing resource state or compositor chrome.
    # ------------------=
    def set_zoom(self, value):
        self.zoom = min(1.75, max(.75, round(value, 2)))
        font = ('Inter', -round(18*self.zoom))
        self.images = workspace_controls(self.root, self.style, self.zoom)
        for name in ('.', 'TLabel', 'TEntry', 'TButton', 'Workspace.TMenubutton',
                     'DashboardCard.TLabel', 'DashboardPanel.TLabel', 'Dashboard.TCheckbutton', 'Fabric.Treeview'):
            self.style.configure(name, font=font)
        self.style.configure('Fabric.Treeview', rowheight=round(40*self.zoom))
        self.style.configure('Fabric.Treeview.Heading', font=font+('bold',))
        for widget, pixels, weight in self.font_widgets:
            widget.configure(font=('Inter', -round(pixels*self.zoom), weight))
        self.menu_bar.configure(height=round(MENU_HEIGHT*self.zoom))
        self.toolbar.configure(height=round(48*self.zoom))
        self.inspector_host.configure(width=round(312*self.zoom))
        self.card_layout()
        return 'break'

    # ------------------------=
    # FUNC: card_layout
    # DESC: Keeps resource values readable at small client widths without scaling typography or controls.
    # ------------------=
    def card_layout(self, event=None):
        spacious = self.cards.winfo_width() >= 690
        for icon in self.card_icons:
            if spacious and not icon.winfo_manager():
                icon.pack(side='left', padx=(0,12), before=icon.master.winfo_children()[1])
            elif not spacious and icon.winfo_manager():
                icon.pack_forget()

    # ------------------------=
    # FUNC: inspector_layout
    # DESC: Keeps contribution controls reachable at the installed desktop's constrained client height.
    # ------------------=
    def inspector_layout(self, event=None):
        self.inspector_canvas.itemconfigure(self.inspector_window, width=self.inspector_canvas.winfo_width())
        self.inspector_canvas.configure(scrollregion=self.inspector_canvas.bbox('all'))

    # ------------------------=
    # FUNC: mark_dirty
    # DESC: Keeps periodic observations from overwriting unapplied contribution edits.
    # ------------------=
    def mark_dirty(self, event=None):
        self.dirty = True
        self.notice = None
        self.policy_error = False

    # ------------------------=
    # FUNC: connection_guide
    # DESC: Presents live discovered peers and native identity verification without leaving Fabric.
    # ------------------=
    def connection_guide(self):
        guide = getattr(self, 'guide', None)
        if guide is not None and guide.winfo_exists():
            guide.lift()
            return
        guide = self.guide = ttk.Frame(self.root)
        guide.place(x=0, y=MENU_HEIGHT, relwidth=1, relheight=1, height=-MENU_HEIGHT)
        guide.lift()
        panel = ttk.Frame(guide, padding=20)
        panel.pack(fill='both', expand=True)
        header = ttk.Frame(panel); header.pack(fill='x')
        self.close_pair_button = ttk.Button(header, text='Back to overview', command=guide.destroy)
        self.close_pair_button.pack(side='right')
        ttk.Label(header, text='Nearby Fabric nodes', font=('Inter', -22, 'bold')).pack(side='left')
        ttk.Label(panel, text='Discovery updates automatically. Verify identity on both computers before sharing.',
                  wraplength=760).pack(anchor='w', pady=(8,12))
        self.peers = ttk.Treeview(panel, columns=('connection','trust'), show='tree headings', height=3,
                                  selectmode='browse', style='Fabric.Treeview')
        self.peers.heading('#0', text='Node identity'); self.peers.column('#0', width=280)
        for column in ('connection','trust'):
            self.peers.heading(column, text=column.title()); self.peers.column(column, width=160)
        self.peers.pack(fill='both', expand=True)
        self.peers.bind('<<TreeviewSelect>>', self.pair_selection)
        self.pair_details = tk.StringVar(value='Looking for nodes on your local network…')
        ttk.Label(panel, textvariable=self.pair_details, wraplength=760, justify='left').pack(fill='x', pady=12)
        actions = ttk.Frame(panel); actions.pack(fill='x')
        self.verify_button = ttk.Button(actions, text='Verify Identity', command=lambda:self.node_action(1))
        self.verify_button.pack(side='left', padx=(0,8))
        self.cancel_pair_button = ttk.Button(actions, text='Cancel pairing', command=lambda:self.node_action(3))
        self.cancel_pair_button.pack(side='left')
        confirmation = ttk.Frame(panel); confirmation.pack(fill='x', pady=12)
        ttk.Label(confirmation, text='Code from the other computer').pack(side='left', padx=(0,12))
        self.pair_code = tk.StringVar()
        ttk.Entry(confirmation, textvariable=self.pair_code, width=8).pack(side='left', padx=(0,12))
        self.confirm_button = ttk.Button(confirmation, text='Confirm matching identity', command=self.confirm_pair)
        self.confirm_button.pack(side='left')
        sharing = ttk.Frame(panel); sharing.pack(fill='x')
        self.sharing_buttons=[]
        for label, action in (('Allow resource sharing', self.allow_sharing),
                              ('Open Session', lambda:self.node_action(4)), ('Join Mesh', lambda:self.node_action(5))):
            button=ttk.Button(sharing,text=label,command=action); button.pack(side='left',padx=(0,8))
            self.sharing_buttons.append(button)
        self.pair_notice=tk.StringVar(value='')
        ttk.Label(panel,textvariable=self.pair_notice,wraplength=760).pack(fill='x',pady=12)
        self.pair_peer=None; self.pair_verification=None
        self.refresh_peers()

    # ------------------------=
    # FUNC: refresh_peers
    # DESC: Reconciles automatic discovery without resetting the user's selected peer or typed code.
    # ------------------=
    def refresh_peers(self):
        if not getattr(self,'guide',None) or not self.guide.winfo_exists(): return
        present=set()
        for node in (self.snapshot or {}).get('nodes',[]):
            if node['local']: continue
            ident=node['id']; present.add(ident)
            values=(node['reachability'],node['trust'])
            if self.peers.exists(ident): self.peers.item(ident,values=values)
            else: self.peers.insert('','end',iid=ident,text=ident[:20],values=values)
        for ident in self.peers.get_children():
            if ident not in present: self.peers.delete(ident)
        self.pair_selection()

    # ------------------------=
    # FUNC: pair_selection
    # DESC: Reads authenticated verification material for the exact selected peer and disables invalid actions.
    # ------------------=
    def pair_selection(self, event=None):
        selection=self.peers.selection(); peer=selection[0] if selection else None
        if peer!=self.pair_peer: self.pair_code.set(''); self.pair_notice.set('')
        self.pair_peer=peer; self.pair_verification=None
        node=next((n for n in (self.snapshot or {}).get('nodes',[]) if n['id']==peer),None)
        admin=bool((self.snapshot or {}).get('provider'))
        online=bool(node and node['reachability']=='Online')
        self.verify_button.configure(state='normal' if admin and online else 'disabled')
        for button in self.sharing_buttons:
            button.configure(state='normal' if admin and online and node['trust']=='Trusted' else 'disabled')
        if peer and admin:
            try: self.pair_verification=json.loads(fabric_node_request(0,peer))
            except OSError as error: self.pair_notice.set(str(error))
        v=self.pair_verification
        for button in (self.confirm_button,self.cancel_pair_button): button.configure(state='normal' if v else 'disabled')
        if v:
            fingerprint=' '.join(v['fingerprint'][i:i+8] for i in range(0,64,8))
            self.pair_details.set(f"Compare on both computers: {v['code']:06d} • {v['state']}\nFingerprint: {fingerprint}")
        elif peer: self.pair_details.set(f"Selected node: {peer}\nChoose Verify Identity, then compare the code and fingerprint on both computers.")
        else: self.pair_details.set('Select a discovered node. Both computers must be on a reachable local network with discovery enabled.')

    # ------------------------=
    # FUNC: node_action
    # DESC: Sends explicit actions to the native broker and reports failures without optimistic trust updates.
    # ------------------=
    def node_action(self, action, value=0, handle=None):
        if not self.pair_peer: return False
        if handle is None: handle=(self.pair_verification or {}).get('handle',0)
        try:
            fabric_node_request(action,self.pair_peer,handle,value)
            self.pair_notice.set('Request accepted. Complete verification and sharing consent on the other computer too.')
            self.refresh(automatic=True)
            return True
        except (OSError,ValueError) as error:
            self.pair_notice.set(str(error)); return False

    # ------------------------=
    # FUNC: confirm_pair
    # DESC: Requires a manually entered six-digit peer code; never automatically approves discovered identities.
    # ------------------=
    def confirm_pair(self):
        code=self.pair_code.get().strip()
        if len(code)!=6 or not code.isascii() or not code.isdigit():
            self.pair_notice.set('Enter the six-digit code shown on the other computer.'); return
        if self.node_action(2,int(code)): self.pair_code.set('')

    # ------------------------=
    # FUNC: allow_sharing
    # DESC: Explicitly grants only membership, compute and mesh policy categories required for resource sharing.
    # ------------------=
    def allow_sharing(self):
        for category in (1,2,10):
            if not self.node_action(6,1,category): return
        self.pair_notice.set('Membership, compute and mesh allowed. Open Session and Join Mesh on both computers.')

    # ------------------------=
    # FUNC: explain
    # DESC: Explains authoritative units and current execution limitations without overstating pooled hardware support.
    # ------------------=
    def explain(self):
        self.notice = 'CPU is measured in scheduling slots. Advertised memory is an allowance, not shared addressable RAM. GPU execution requires a compatible provider.'
        self.status.set(self.notice)

    # ------------------------=
    # FUNC: refresh
    # DESC: Reconciles native snapshots while preserving selection and in-progress policy edits.
    # ------------------=
    def refresh(self, automatic=False):
        if not automatic:
            self.notice = None
        try:
            snapshot = json.loads(fabric_request(0))
            present = set()
            for node in snapshot['nodes']:
                ident = node['id']; present.add(ident)
                cpu, memory = node['resources'][:2]
                values = (node['reachability'],str(cpu['available']),size(memory['available']),node['trust'])
                label = 'This computer' if node['local'] else ident[:12]
                if self.nodes.exists(ident): self.nodes.item(ident,text=label,values=values)
                else: self.nodes.insert('', 'end', iid=ident,text=label,values=values,image=self.dashboard_images['monitor'])
            for ident in self.nodes.get_children():
                if ident not in present: self.nodes.delete(ident)
            total = snapshot['totals']
            self.totals[0].set(str(total[0])); self.totals[1].set(size(total[1]))
            self.totals[2].set(str(total[2]) if total[2] else 'Unavailable')
            provider = snapshot['provider']
            for control in [self.toggle,self.apply_button]+self.entries:
                control.configure(state='normal' if provider else 'disabled')
            if provider:
                if not self.dirty:
                    self.enabled.set(provider['enabled']); self.cpu.set(str(provider['cpu']))
                    self.memory.set(str(provider['memory']//(1024*1024))); self.tasks.set(str(provider['tasks']))
                    self.edit_revision = provider['revision']
                self.usage.set(f"{provider['active']} active jobs\n{size(provider['allocated_memory'])} reserved")
            else: self.usage.set('Administrator access is required to change contribution policy.')
            if provider:
                try:
                    jobs = json.loads(fabric_request(2))
                    self.job_records = {str(job['id']):job for job in jobs}
                    states = {1:'Queued',2:'Placed',3:'Running',4:'Completed',5:'Failed',6:'Cancelled',7:'Node lost'}
                    current = set()
                    for job in jobs:
                        ident = str(job['id']); current.add(ident)
                        values = (job['node'][:12],states.get(job['state'],'Unknown'),job['cpu'],size(job['memory']))
                        if self.jobs.exists(ident): self.jobs.item(ident,text=ident,values=values)
                        else: self.jobs.insert('', 'end', iid=ident,text=ident,values=values)
                    for ident in self.jobs.get_children():
                        if ident not in current: self.jobs.delete(ident)
                    self.job_status.set('' if jobs else 'No jobs submitted on this computer.')
                except (OSError,ValueError,KeyError):
                    self.job_records = {}
                    self.jobs.delete(*self.jobs.get_children())
                    self.job_status.set('Job service unavailable.')
            else:
                self.job_records = {}
                self.jobs.delete(*self.jobs.get_children())
                self.job_status.set('Administrator access is required to inspect all jobs.')
            self.snapshot = snapshot
            self.refresh_peers()
            if not self.nodes.selection() and self.nodes.get_children():
                self.nodes.selection_set(self.nodes.get_children()[0])
            self.select()
            self.select_job()
            if self.jobs.get_children(): self.empty_jobs.place_forget()
            else: self.empty_jobs.place(relx=.5,rely=.65,anchor='center')
            self.status.set(self.notice or f"{len(snapshot['nodes'])} nodes | Live resource observations")
        except (OSError,ValueError,KeyError) as error:
            self.snapshot = None
            self.refresh_peers()
            self.job_records = {}
            self.nodes.delete(*self.nodes.get_children())
            self.jobs.delete(*self.jobs.get_children())
            self.select()
            self.select_job()
            for value in self.totals: value.set('Unavailable')
            for control in [self.toggle,self.apply_button]+self.entries:
                control.configure(state='disabled')
            self.job_status.set('Job service unavailable.')
            self.empty_jobs.place(relx=.5,rely=.65,anchor='center')
            self.status.set(f'Fabric service unavailable: {error}')

    # ------------------------=
    # FUNC: select
    # DESC: Drills into the selected authoritative node without changing its trust or contribution.
    # ------------------=
    def select(self, event=None):
        selected = self.nodes.selection()
        for ident in self.nodes.get_children():
            self.nodes.item(ident,image=self.dashboard_images['monitor-selected' if ident in selected else 'monitor'])
        self.selected_node = None
        for value in self.node_fields.values(): value.set('—')
        self.node_field_labels['Connection'].configure(image='')
        if not selected or not self.snapshot:
            self.detail.set('Select a node to inspect its resources.')
            self.node_detail.set('Select a node to inspect it.')
            return
        node = next((n for n in self.snapshot['nodes'] if n['id']==selected[0]),None)
        if node:
            self.selected_node = node
            self.detail.set(f"{node['id'][:12]} · {node['trust']} · {node['reachability']}")
            self.node_fields['Identity'].set('This computer' if node['local'] else node['id'][:12])
            self.node_fields['Trust'].set(node['trust'])
            self.node_fields['Connection'].set(node['reachability'])
            self.node_field_labels['Connection'].configure(image=self.dashboard_images['online' if node['reachability']=='Online' else 'offline'])
            lines = []
            for label, resource, formatter in zip(('CPU slots','Memory allowance','Accelerators'),node['resources'],(str,size,str)):
                lines.append(f"\n{label}\n{formatter(resource['available'])} available / {formatter(resource['capacity'])} advertised\n"
                             f"{formatter(resource['provider_reserved'])} provider reserved\n{formatter(resource['consumer_reserved'])} consumer reserved")
            lines.append('\nNode identity\n'+node['id'])
            self.node_detail.set('\n'.join(lines))

    # ------------------------=
    # FUNC: select_job
    # DESC: Enables cancellation only for a selected live job from the current authorized snapshot.
    # ------------------=
    def select_job(self, event=None):
        selected = self.jobs.selection()
        job = self.job_records.get(selected[0]) if selected else None
        self.cancel_button.configure(state='normal' if job and job['state'] in (1,2,3) else 'disabled')

    # ------------------------=
    # FUNC: cancel_job
    # DESC: Cancels the observed job epoch and surfaces rejected stale or unauthorized actions without optimistic state changes.
    # ------------------=
    def cancel_job(self):
        selected = self.jobs.selection()
        job = self.job_records.get(selected[0]) if selected else None
        if not job or job['state'] not in (1,2,3): return
        try:
            fabric_request(3,job['id'],job['epoch'])
            self.refresh()
            self.notice = 'Job cancellation requested.'
        except OSError as error:
            self.notice = f'Job was not cancelled: {error}. Refresh and retry.'
        self.status.set(self.notice)

    # ------------------------=
    # FUNC: apply
    # DESC: Submits explicit revision-checked contribution limits and preserves edits on rejection.
    # ------------------=
    def apply(self):
        if not self.snapshot or not self.snapshot.get('provider'):
            return
        try:
            cpu, memory, tasks = int(self.cpu.get()),int(self.memory.get()),int(self.tasks.get())
            if not (0 <= cpu <= 65535 and 0 <= tasks <= 65535 and 0 <= memory <= (2**64-1)//(1024*1024)):
                raise ValueError('Contribution limits are out of range')
            fabric_request(1,self.edit_revision,int(self.enabled.get()),memory*1024*1024,cpu|(tasks<<16))
            self.dirty=False; self.policy_error=False; self.refresh()
            self.notice = 'Contribution policy saved.'
            self.status.set(self.notice)
        except (ValueError,OSError) as error:
            self.policy_error = True
            self.notice = f'Not applied: {error}. Discard edits and reload to resolve a policy conflict.'
            self.status.set(self.notice)

    # ------------------------=
    # FUNC: reload_policy
    # DESC: Explicitly discards unapplied limits and reloads the current revision after a conflict.
    # ------------------=
    def reload_policy(self):
        self.dirty = False
        self.policy_error = False
        self.refresh()

    # ------------------------=
    # FUNC: tick
    # DESC: Refreshes with a bounded cadence rather than consuming the desktop event loop.
    # ------------------=
    def tick(self):
        self.refresh(automatic=True)
        self.root.after(2000,self.tick)

if __name__ == '__main__':
    root = tk.Tk()
    app = FabricManager(root)
    root.mainloop()
