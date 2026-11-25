"""Native Fabric management; all observations and mutations come from the OS authority."""
import json
import tkinter as tk
from tkinter import ttk
from _infinity_app import fabric_request
from infinity_uikit import workspace_controls, workspace_menu, MENU_HEIGHT
from infinity_uikit_artwork import COLORS

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
        font = ('Inter', -18)
        root.option_add('*Font', font)
        style = ttk.Style(root)
        style.theme_use('clam')
        style.configure('TLabel', background=COLORS['panel'], foreground=COLORS['text'], font=font)
        style.configure('TFrame', background=COLORS['panel'])
        style.configure('Fabric.Treeview', background=COLORS['panel'], fieldbackground=COLORS['panel'],
                        foreground=COLORS['text'], font=font, rowheight=40, borderwidth=0,
                        lightcolor=COLORS['panel'], darkcolor=COLORS['panel'], bordercolor=COLORS['panel'])
        style.configure('Fabric.Treeview.Heading', background=COLORS['panel'], foreground=COLORS['text'], font=font,
                        lightcolor=COLORS['panel'], darkcolor=COLORS['panel'], bordercolor=COLORS['border'], relief='flat')
        style.map('Fabric.Treeview', background=[('selected', COLORS['accent'])])
        self.images = workspace_controls(root, style, 1.0)
        style.configure('Workspace.TMenubutton', background=COLORS['panel'], borderwidth=0,
                        lightcolor=COLORS['panel'], darkcolor=COLORS['panel'], bordercolor=COLORS['panel'],
                        arrowcolor=COLORS['muted'], relief='flat')
        style.map('Workspace.TMenubutton', background=[('active', COLORS['accent'])])
        menu_bar = ttk.Frame(root, height=MENU_HEIGHT)
        menu_bar.pack(fill='x'); menu_bar.pack_propagate(False)
        for title, entries in (('File', (('Refresh', self.refresh), ('Close', root.destroy))),
                               ('Help', (('Resource units', self.explain),))):
            button = ttk.Menubutton(menu_bar, text=title, takefocus=True, style='Workspace.TMenubutton')
            menu = tk.Menu(button, tearoff=False, bg=COLORS['panel'], fg=COLORS['text'],
                           activebackground=COLORS['accent'], font=font)
            for label, action in entries: menu.add_command(label=label, command=action)
            workspace_menu(button, menu)
            button.pack(side='right' if title=='Help' else 'left', fill='y', padx=8)
        toolbar = ttk.Frame(root, height=48)
        toolbar.pack(fill='x'); toolbar.pack_propagate(False)
        ttk.Label(toolbar, text='Fabric resources', font=('Inter', -22)).pack(side='left', padx=16)
        ttk.Button(toolbar, text='Refresh', command=self.refresh).pack(side='right', padx=12, pady=4)
        self.status = tk.StringVar(value='Connecting to Fabric service…')
        ttk.Label(root, textvariable=self.status, padding=12).pack(side='bottom', fill='x')
        body = ttk.Frame(root, padding=12)
        body.pack(fill='both', expand=True)
        body.columnconfigure(0, weight=1)
        body.rowconfigure(1, weight=3, minsize=110)
        body.rowconfigure(2, weight=2, minsize=100)
        cards = ttk.Frame(body)
        cards.grid(row=0, column=0, sticky='ew', pady=(0,12))
        self.totals = []
        for i, heading in enumerate(('CPU slots', 'Memory allowance', 'Accelerators')):
            cards.columnconfigure(i, weight=1)
            card = ttk.Frame(cards, padding=(12,8))
            card.grid(row=0, column=i, sticky='nsew', padx=(0,8))
            ttk.Label(card, text=heading).pack(anchor='w')
            value = tk.StringVar(value='Unavailable')
            ttk.Label(card, textvariable=value, font=('Inter', -32)).pack(anchor='w', pady=8)
            self.totals.append(value)
        center = ttk.Frame(body)
        center.grid(row=1, column=0, sticky='nsew')
        ttk.Label(center, text='Nodes', font=('Inter',-22), padding=8).pack(anchor='w')
        self.nodes = ttk.Treeview(center, columns=('state','cpu','memory','trust'), show='tree headings', style='Fabric.Treeview', selectmode='browse', height=2)
        self.nodes.heading('#0', text='Node'); self.nodes.column('#0', width=180, minwidth=140)
        for key, label, width in [('state','Connection',110),('cpu','CPU slots',100),('memory','Memory',130),('trust','Trust',100)]:
            self.nodes.heading(key,text=label); self.nodes.column(key,width=width,minwidth=80)
        self.node_scroll = ttk.Scrollbar(center, orient='vertical', command=self.nodes.yview)
        self.node_scroll.pack(side='right', fill='y')
        self.nodes.configure(yscrollcommand=self.node_scroll.set)
        self.nodes.pack(fill='both', expand=True)
        node_horizontal = ttk.Scrollbar(center, orient='horizontal', command=self.nodes.xview)
        node_horizontal.pack(fill='x')
        self.nodes.configure(xscrollcommand=node_horizontal.set)
        self.nodes.bind('<<TreeviewSelect>>', self.select)
        self.detail = tk.StringVar(value='Select a node to inspect its resources.')
        # Full node identity stays accessible without consuming the table's rows.
        ttk.Label(toolbar, textvariable=self.detail, padding=(12,0), wraplength=420).pack(side='left')
        jobs_panel = ttk.Frame(body, padding=8)
        jobs_panel.grid(row=2, column=0, sticky='nsew', pady=(12,0))
        job_header = ttk.Frame(jobs_panel)
        job_header.pack(fill='x', pady=(0,8))
        ttk.Label(job_header, text='Job activity', font=('Inter',-20)).pack(side='left')
        self.cancel_button = ttk.Button(job_header, text='Cancel job', command=self.cancel_job, state='disabled')
        self.cancel_button.pack(side='right')
        self.jobs = ttk.Treeview(jobs_panel, columns=('node','state','cpu','memory'),
                                show='tree headings', style='Fabric.Treeview', height=2)
        self.jobs.heading('#0', text='Job'); self.jobs.column('#0', width=90, minwidth=60)
        for key,label,width in [('node','Node',140),('state','Status',120),('cpu','CPU units',90),('memory','Reserved memory',150)]:
            self.jobs.heading(key,text=label); self.jobs.column(key,width=width,minwidth=70)
        job_scroll = ttk.Scrollbar(jobs_panel, orient='vertical', command=self.jobs.yview)
        job_scroll.pack(side='right',fill='y')
        self.jobs.configure(yscrollcommand=job_scroll.set)
        self.jobs.pack(fill='both',expand=True)
        self.jobs.bind('<<TreeviewSelect>>', self.select_job)
        self.job_status = tk.StringVar()
        ttk.Label(jobs_panel,textvariable=self.job_status).pack(anchor='w')
        inspector_host = ttk.Frame(body, width=260)
        inspector_host.grid(row=0,column=1,rowspan=3,sticky='ns',padx=(12,0))
        inspector_host.grid_propagate(False)
        inspector_host.rowconfigure(0, weight=1)
        inspector_host.columnconfigure(0, weight=1)
        self.inspector_canvas = tk.Canvas(inspector_host, width=244, highlightthickness=0, bg=COLORS['panel'])
        self.inspector_canvas.grid(row=0,column=0,sticky='nsew')
        inspector_scroll = ttk.Scrollbar(inspector_host, orient='vertical', command=self.inspector_canvas.yview)
        inspector_scroll.grid(row=0,column=1,sticky='ns')
        self.inspector_canvas.configure(yscrollcommand=inspector_scroll.set)
        inspector = ttk.Frame(self.inspector_canvas, padding=(8,12))
        self.inspector_window = self.inspector_canvas.create_window(0,0,window=inspector,anchor='nw')
        inspector.bind('<Configure>', self.inspector_layout)
        self.inspector_canvas.bind('<Configure>', self.inspector_layout)
        ttk.Label(inspector,text='This computer',font=('Inter',-20)).grid(row=0,column=0,sticky='w',pady=(0,16))
        self.enabled = tk.BooleanVar()
        self.cpu = tk.StringVar(); self.memory = tk.StringVar(); self.tasks = tk.StringVar()
        self.toggle = ttk.Checkbutton(inspector,text='Contribute resources',variable=self.enabled,command=self.mark_dirty)
        self.toggle.grid(row=1,column=0,sticky='w',pady=8)
        self.entries = []
        for row,(label,var) in enumerate((('CPU scheduling units',self.cpu),('Memory limit (MiB)',self.memory),('Concurrent jobs',self.tasks)),2):
            ttk.Label(inspector,text=label).grid(row=row*2,column=0,sticky='w',pady=(12,4))
            entry = ttk.Entry(inspector,textvariable=var,width=20)
            entry.grid(row=row*2+1,column=0,sticky='ew')
            entry.bind('<KeyRelease>',self.mark_dirty)
            self.entries.append(entry)
        self.apply_button = ttk.Button(inspector,text='Apply contribution',command=self.apply)
        self.apply_button.grid(row=10,column=0,sticky='ew',pady=20)
        ttk.Button(inspector,text='Discard edits and reload',command=self.reload_policy).grid(row=11,column=0,sticky='ew',pady=(0,12))
        self.usage = tk.StringVar()
        ttk.Label(inspector,textvariable=self.usage,wraplength=240).grid(row=12,column=0,sticky='w')
        ttk.Label(inspector,text='Selected node',font=('Inter',-20)).grid(row=13,column=0,sticky='w',pady=(20,8))
        self.node_detail = tk.StringVar(value='Select a node to inspect it.')
        ttk.Label(inspector,textvariable=self.node_detail,wraplength=220,justify='left').grid(row=14,column=0,sticky='w')
        self.refresh()
        root.after(2000,self.tick)

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
                else: self.nodes.insert('', 'end', iid=ident,text=label,values=values)
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
            self.select()
            self.select_job()
            self.status.set(self.notice or f"{len(snapshot['nodes'])} nodes • Live resource observations")
        except (OSError,ValueError,KeyError) as error:
            self.snapshot = None
            self.job_records = {}
            self.nodes.delete(*self.nodes.get_children())
            self.jobs.delete(*self.jobs.get_children())
            self.select()
            self.select_job()
            for value in self.totals: value.set('Unavailable')
            for control in [self.toggle,self.apply_button]+self.entries:
                control.configure(state='disabled')
            self.status.set(f'Fabric service unavailable: {error}')

    # ------------------------=
    # FUNC: select
    # DESC: Drills into the selected authoritative node without changing its trust or contribution.
    # ------------------=
    def select(self, event=None):
        selected = self.nodes.selection()
        self.selected_node = None
        if not selected or not self.snapshot:
            self.detail.set('Select a node to inspect its resources.')
            self.node_detail.set('Select a node to inspect it.')
            return
        node = next((n for n in self.snapshot['nodes'] if n['id']==selected[0]),None)
        if node:
            self.selected_node = node
            self.detail.set(f"{node['id'][:12]} · {node['trust']} · {node['reachability']}")
            lines = [node['id'], f"{node['reachability']} · {node['trust']}"]
            for label, resource, formatter in zip(('CPU slots','Memory allowance','Accelerators'),node['resources'],(str,size,str)):
                lines.append(f"\n{label}\n{formatter(resource['available'])} available / {formatter(resource['capacity'])} advertised\n"
                             f"{formatter(resource['provider_reserved'])} provider reserved\n{formatter(resource['consumer_reserved'])} consumer reserved")
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
