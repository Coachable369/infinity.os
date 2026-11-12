"""Infinity Memory, the native signed-in AIKit catalog browser."""
import time
import tkinter as tk
from tkinter import ttk
from _infinity_app import memory_request

CATEGORIES = ("All memories", "Episodic", "Semantic", "Procedural")
NAVY, PANEL, WHITE, BLUE, MUTED = "#090f26", "#101b3a", "#ffffff", "#3e6fff", "#a7bddc"

# Downsampled from the approved Infinity Memory launcher glyph; no filesystem access at launch.
MEMORY_GLYPH = "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"

# ------------------------=
# FUNC: stamp
# DESC: Displays a stored UTC timestamp including the no-clock state.
# ------------------=
def stamp(value):
    return time.strftime("%Y-%m-%d %H:%M UTC", time.gmtime(value)) if value else "Unavailable"


# ------------------------=
# FUNC: message
# DESC: Maps native API failures to the relevant recovery action in the inspector.
# ------------------=
def message(error):
    detail = str(error)
    for code, explanation in ((-95, "Choose Keyword search for this text."),
                              (-28, "Memory is full. Remove a record and try again."),
                              (-13, "Access denied. Check your session and remove any credentials from the text."),
                              (-2, "This memory was removed. Refresh the list."),
                              (-5, "Memory storage is unavailable. Refresh to retry.")):
        if detail.endswith(f"({code})"):
            return explanation
    return detail


class Browser:
    # ------------------------=
    # FUNC: __init__
    # DESC: Creates the searchable collection and selected-record inspector in one native window.
    # ------------------=
    def __init__(self, root):
        self.surfaces = {}
        body_px = 17 if root.winfo_screenwidth() < 1500 else 20
        body_font = ("Inter", -body_px)
        self.button_font = ("Inter", -max(14, body_px-3))
        root.option_add("*Font", body_font)
        style = ttk.Style(root)
        style.theme_use("clam")
        style.configure("TLabel", background=NAVY, foreground=WHITE, font=body_font)
        style.configure("TEntry", fieldbackground=PANEL, foreground=WHITE, padding=8, font=body_font, bordercolor="#50779A", lightcolor=PANEL, darkcolor=PANEL, insertcolor=WHITE)
        style.configure("TCombobox", fieldbackground=PANEL, background=PANEL, foreground=WHITE, padding=6, font=body_font)
        style.configure("Library.Treeview", background=PANEL, fieldbackground=PANEL,
                        foreground=WHITE, font=body_font, rowheight=body_px+17, borderwidth=0,
                        lightcolor=PANEL, darkcolor=PANEL)
        style.map("Library.Treeview", background=[("selected", "#244F96")],
                  foreground=[("selected", WHITE)])
        style.configure("TCombobox", arrowcolor="#A8D5FF", bordercolor="#416084",
                        lightcolor=PANEL, darkcolor=PANEL)
        style.map("TEntry", bordercolor=[("focus", "#86C7FF")])
        style.map("TCombobox", fieldbackground=[("readonly", PANEL)], foreground=[("readonly", WHITE)])
        root.option_add("*TCombobox*Listbox.background", PANEL)
        root.option_add("*TCombobox*Listbox.foreground", WHITE)
        self.root = root
        root.title("Infinity Memory")
        width = min(1520, root.winfo_screenwidth() - 48, root.maxsize()[0])
        height = min(960, root.winfo_screenheight() - 124, root.maxsize()[1])
        root.geometry(f"{width}x{height}+{max(0,(root.winfo_screenwidth()-width)//2)}+{max(72,(root.winfo_screenheight()-height)//2)}")
        root.minsize(min(720,width), min(550,height))
        root.configure(bg=NAVY)
        self.record = self.page = self.total = 0
        self.pending = None
        self.confirmation = None
        self.original = ""
        self.original_category = "Semantic"
        self.ids = []
        self.query = tk.StringVar()
        self.filter = tk.StringVar(value=CATEGORIES[0])
        self.mode = tk.StringVar(value="Semantic")
        self.category = tk.StringVar(value="Semantic")
        self.status = tk.StringVar(value="Loading memories...")
        self.heading = tk.StringVar(value="Memory details")
        self.metadata = [tk.StringVar(value="—") for _ in range(4)]
        self.page_label = tk.StringVar()
        self.empty_hint = tk.StringVar()
        top = tk.Frame(root, bg=NAVY)
        top.pack(fill="x", padx=18, pady=(16, 8))
        self.glyph = tk.PhotoImage(data=MEMORY_GLYPH)
        tk.Label(top, image=self.glyph, bg=NAVY, bd=0).pack(side="left", padx=(0,12))
        tk.Label(top, text="Infinity Memory", font=("Inter", -int(body_px*1.85), "bold"), bg=NAVY, fg=WHITE).pack(side="left")
        tk.Label(top, text="Personal workspace", font=("Inter",-body_px), bg=NAVY, fg=MUTED).pack(side="right")
        self.fact_id = 0
        self.fact_job = None
        self.fact_last = ""
        self.fact = tk.StringVar()
        self.fact_state = tk.StringVar(value="Autosave on")
        capture = tk.Frame(root, bg=PANEL, padx=12, pady=8, highlightthickness=1, highlightbackground="#324D70")
        capture.pack(fill="x", padx=18, pady=4)
        tk.Label(capture, text="Add a fact", bg=PANEL, fg=WHITE, font=("Inter",-body_px,"bold")).pack(side="left", padx=(0,12))
        field = ttk.Entry(capture, textvariable=self.fact, font=body_font)
        field.pack(side="left", expand=True, fill="x")
        field.bind("<ButtonPress-1>", lambda event: field.focus_force(), add="+")
        field.bind("<Return>", lambda event: self.finish_fact())
        field.bind("<FocusOut>", lambda event: self.finish_fact())
        tk.Label(capture, textvariable=self.fact_state, bg=PANEL, fg="#8BDDB9", width=12).pack(side="right")
        self.fact.trace_add("write", self.fact_changed)
        controls = tk.Frame(root, bg=NAVY)
        controls.pack(fill="x", padx=18, pady=6)
        ttk.Label(controls, text="Search", width=10, anchor="w").pack(side="left", padx=(12, 4))
        self.search = ttk.Entry(controls, textvariable=self.query, font=body_font)
        self.search.bind("<ButtonPress-1>", lambda event: self.search.focus_force(), add="+")
        self.search.pack(side="left", expand=True, fill="x")
        for var, options in ((self.mode, ("Semantic", "Keyword")), (self.filter, CATEGORIES)):
            box = ttk.Combobox(controls, textvariable=var, values=options, width=11, state="readonly", font=body_font)
            box.pack(side="left", padx=(6, 0))
            box.bind("<<ComboboxSelected>>", self.changed)
        self.list_title = tk.StringVar(value="Memories")
        self.query.trace_add("write", self.changed)
        body = tk.PanedWindow(root, orient="horizontal", bg=NAVY, sashwidth=7, bd=0)
        body.pack(expand=True, fill="both", padx=18, pady=10)
        left, right = (tk.Frame(body, bg=PANEL, highlightthickness=1, highlightbackground="#365777") for _ in range(2))
        body.add(left, minsize=200, width=int(width*.35), stretch="always")
        body.add(right, minsize=290, width=int(width*.65), stretch="always")
        tk.Label(left,textvariable=self.list_title,bg=PANEL,fg=WHITE,font=("Inter",-body_px-2,"bold")).pack(anchor="w", padx=14, pady=(10,8))
        tk.Frame(left, height=1, bg="#31476A").pack(fill="x", padx=12)
        list_area = tk.Frame(left, bg=PANEL)
        list_area.pack(expand=True, fill="both", padx=8, pady=6)
        self.listbox = ttk.Treeview(list_area, show="tree", selectmode="browse", height=4, style="Library.Treeview")
        self.listbox.column("#0", width=160, minwidth=80, stretch=True)
        self.listbox.pack(fill="both", expand=True)
        self.empty = tk.Label(list_area, textvariable=self.empty_hint, bg=PANEL, fg=MUTED,
                              font=("Inter", -body_px), justify="left", wraplength=180)
        self.listbox.bind("<<TreeviewSelect>>", self.select)
        nav = tk.Frame(left, bg=PANEL)
        nav.pack(fill="x", padx=8, pady=8)
        self.prev = self.button(nav, "Previous", lambda: self.move(-1))
        self.next = self.button(nav, "Next", lambda: self.move(1))
        self.prev.pack(side="left")
        self.next.pack(side="right")
        tk.Label(nav, textvariable=self.page_label, bg=PANEL, fg=MUTED,
                 font=("Inter",-13)).pack(expand=True)
        tk.Label(right, textvariable=self.heading, bg=PANEL, fg=WHITE, font=("Inter", -body_px-5, "bold")).pack(anchor="w", padx=14, pady=(12, 6))
        tk.Label(right, text="Fact", bg=PANEL, fg=MUTED, font=("Inter",-14,"bold")).pack(anchor="w", padx=14, pady=(0,4))
        self.editor = tk.Text(right, height=3, wrap="word", bg=NAVY, fg=WHITE, insertbackground=WHITE,
                              font=body_font, padx=12, pady=10, relief="flat", bd=0, highlightthickness=1,
                              highlightbackground="#405D80", highlightcolor="#86C7FF")
        self.editor.bind("<ButtonPress-1>", lambda event: self.editor.focus_force(), add="+")

        row = tk.Frame(right, bg=PANEL)
        row.pack(fill="x", padx=14, pady=8)
        tk.Label(row, text="Category", bg=PANEL, fg=MUTED).pack(side="left")
        ttk.Combobox(row, values=CATEGORIES[1:], textvariable=self.category, state="readonly", width=14).pack(side="left", padx=8)
        tk.Frame(right, height=1, bg="#31476A").pack(fill="x", padx=14, pady=(0,5))
        meta = tk.Frame(right, bg=PANEL)
        meta.pack(fill="x", padx=14, pady=(2,5))
        for i,label in enumerate(("Source", "Record", "Created", "Updated")):
            col = i % 2
            meta.columnconfigure(col, weight=1, uniform="metadata")
            cell = tk.Frame(meta, bg=PANEL)
            cell.grid(row=i//2, column=col, sticky="ew", pady=2)
            tk.Label(cell, text=label, width=7, anchor="w", font=("Inter",-13),
                     fg=MUTED, bg=PANEL).pack(side="left")
            tk.Label(cell, textvariable=self.metadata[i], anchor="w", font=("Inter",-14),
                     fg=WHITE, bg=PANEL).pack(side="left", fill="x", expand=True)
        buttons = tk.Frame(right, bg=PANEL)
        self.delete = self.button(buttons, "Delete", self.remove)
        self.delete.configure(state="disabled")
        self.delete.pack(side="left")
        self.commit = self.button(buttons, "Save", self.save, True)
        self.commit.pack(side="right")
        # Reserve the actions and metadata before allocating the flexible editor.
        buttons.pack(side="bottom", fill="x", padx=14, pady=(4, 9))
        self.editor.pack(fill="both", expand=True, padx=14, before=row)
        root.bind("<Escape>", lambda event: self.cancel_confirmation())
        tk.Label(root, textvariable=self.status, bg="#101E38", fg=MUTED, anchor="w", padx=18, pady=5, font=("Inter",-14)).pack(fill="x")
        root.protocol("WM_DELETE_WINDOW", self.close)
        root.bind("<Control-n>", lambda event: self.new())
        root.bind("<Control-s>", lambda event: self.save())
        root.bind("<Control-f>", lambda event: self.search.focus_force())
        root.after_idle(self.open_catalog)
        root.after_idle(field.focus_force)

    # ------------------------=
    # FUNC: open_catalog
    # DESC: Displays the first actual record on launch while preserving the quick entry field for typing.
    # ------------------=
    def open_catalog(self):
        self.refresh()
        if self.ids:
            self.listbox.selection_set(str(self.ids[0]))
            self.show(self.ids[0])

    # ------------------------=
    # FUNC: fact_changed
    # DESC: Coalesces typing into one owner-scoped fact and leaves failed drafts editable.
    # ------------------=
    def fact_changed(self, *args):
        if self.fact_job is not None:
            self.root.after_cancel(self.fact_job)
            self.fact_job = None
        if self.fact.get().strip():
            self.fact_state.set("Saving...")
            self.fact_job = self.root.after(1000, self.store_fact)
        else:
            self.fact_id, self.fact_last = 0, ""

    # ------------------------=
    # FUNC: store_fact
    # DESC: Updates only a row created by this draft; a preexisting duplicate remains independently owned by its original record.
    # ------------------=
    def store_fact(self):
        self.fact_job = None
        value = self.fact.get().strip()
        if not value or value == self.fact_last:
            if value:
                self.fact_state.set("Autosaved")
            return True
        if len(value.encode()) > 512:
            self.fact_state.set("Too long")
            self.status.set("Facts can contain up to 512 UTF-8 bytes.")
            return False
        try:
            if self.fact_id:
                memory_request(5, self.fact_id, 2, value)
            else:
                identity, created = map(int, memory_request(7, 0, 2, value).split())
                self.fact_id = identity if created else 0
        except OSError as error:
            self.fact_state.set("Unsaved")
            self.status.set(message(error))
            return False
        self.fact_last = value
        self.fact_state.set("Autosaved")
        self.refresh()
        return True

    # ------------------------=
    # FUNC: finish_fact
    # DESC: Flushes the draft on Enter, focus transfer or close and starts a fresh capture after commit.
    # ------------------=
    def finish_fact(self):
        if self.fact_job is not None:
            self.root.after_cancel(self.fact_job)
            self.fact_job = None
        if not self.store_fact():
            return False
        self.fact.set("")
        return True

    # ------------------------=
    # FUNC: surface
    # DESC: Materializes each active UIKit state once, only when a button actually displays it.
    # ------------------=
    def surface(self, primary, state):
        key = int(primary) + 2 * state
        if key not in self.surfaces:
            from _infinity_media import io
            self.surfaces[key] = tk.PhotoImage(data=io(6, b"", key), format="PPM")
        return self.surfaces[key]

    # ------------------------=
    # FUNC: button
    # DESC: Renders the active Infinity UIKit button recipe for normal, hover and pressed states.
    # ------------------=
    def button(self, parent, label, action, primary=False):
        normal = self.surface(primary, 0)
        button = tk.Button(parent, text=label, command=action, image=normal, compound="center",
                           fg=WHITE, activeforeground=WHITE, bg=NAVY, activebackground=NAVY,
                           font=self.button_font, borderwidth=0, relief="flat", highlightthickness=0)
        for event, state in (("<Enter>", 1), ("<Leave>", 0), ("<ButtonPress-1>", 2),
                             ("<ButtonRelease-1>", 1), ("<FocusIn>", 1), ("<FocusOut>", 0)):
            button.bind(event, lambda e, s=state: button.configure(image=self.surface(primary, s)), add="+")
        return button

    # ------------------------=
    # FUNC: changed
    # DESC: Coalesces search input before querying the authorized index.
    # ------------------=
    def changed(self, *args):
        if self.pending is not None:
            self.root.after_cancel(self.pending)
        self.page = 0
        self.pending = self.root.after(320, self.refresh)

    # ------------------------=
    # FUNC: refresh
    # DESC: Loads at most six summaries and refreshes the authoritative counts.
    # ------------------=
    def refresh(self):
        self.pending = None
        try:
            raw = memory_request(2 if self.mode.get() == "Semantic" else 1, self.page,
                                 CATEGORIES.index(self.filter.get()), self.query.get()).decode()
            rows = raw.splitlines()
            _, count, total, embedded, size = rows.pop().split("\t")
            self.total = int(count)
            self.list_title.set(f"Library   ·   {count}")
            self.page_label.set(f"{self.page+1} / {(self.total+5)//6}" if self.total>6 else "")
            if self.page and self.page * 6 >= self.total:
                self.page = max(0, (self.total - 1) // 6)
                return self.refresh()
            records = [row.split("\t", 2) for row in rows]
            ids = [int(row[0]) for row in records]
            titles = tuple(row[2] for row in records)
            if records != getattr(self, "rows", None):
                self.ids, self.rows = ids, records
                self.listbox.delete(*self.listbox.get_children())
                for identity, title in zip(ids, titles):
                    self.listbox.insert("", "end", iid=str(identity), text=title)
                if self.record in ids:
                    self.listbox.selection_set(str(self.record))
            if ids:
                self.empty.place_forget()
            else:
                self.empty_hint.set("No matches. Try a different search or filter." if int(total)
                                    else "Your library is empty. Add your first fact above.")
                self.empty.place(x=12,y=12)
            self.prev.configure(state="normal" if self.page else "disabled")
            self.next.configure(state="normal" if (self.page + 1) * 6 < self.total else "disabled")
            prefix = {0: "No matching memories", 1: "1 match"}.get(self.total, f"{count} matches")
            summary = f"{prefix}    |    {total} stored    |    {embedded} embedded    |    {size} bytes"
            if self.confirmation is None and self.status.get() != summary:
                self.status.set(summary)
        except (OSError, ValueError) as error:
            self.ids.clear()
            self.rows = None
            self.listbox.delete(*self.listbox.get_children())
            self.status.set(message(error))

    # ------------------------=
    # FUNC: move
    # DESC: Navigates within the current filtered result set.
    # ------------------=
    def move(self, delta):
        self.page += delta
        self.refresh()

    # ------------------------=
    # FUNC: discard
    # DESC: Protects unsaved content and category edits during navigation or close.
    # ------------------=
    def discard(self, action):
        if self.confirmation is not None:
            return
        if self.editor.get("1.0", "end-1c") == self.original and self.category.get() == self.original_category:
            action()
        else:
            self.confirm("Discard unsaved changes?", "Discard", action)

    # ------------------------=
    # FUNC: confirm
    # DESC: Offers explicit choices on the same native surface while retaining the original record identity.
    # ------------------=
    def confirm(self, prompt, label, action):
        self.confirmation = action
        self.status.set(prompt)
        self.delete.configure(text="Cancel", state="normal", command=self.cancel_confirmation)
        self.commit.configure(text=label, command=self.accept_confirmation)
        self.delete.focus_force()

    # ------------------------=
    # FUNC: cancel_confirmation
    # DESC: Returns to ordinary editing without modifying durable memory or the current draft.
    # ------------------=
    def cancel_confirmation(self):
        if self.confirmation is None:
            return
        self.confirmation = None
        self.delete.configure(text="Delete", command=self.remove, state="normal" if self.record else "disabled")
        self.commit.configure(text="Save", command=self.save)
        self.refresh()

    # ------------------------=
    # FUNC: accept_confirmation
    # DESC: Performs the captured operation once after an explicit confirmation.
    # ------------------=
    def accept_confirmation(self):
        action = self.confirmation
        if action is not None:
            self.cancel_confirmation()
            action()

    # ------------------------=
    # FUNC: show
    # DESC: Displays one complete authorized record and its original metadata.
    # ------------------=
    def show(self, identity):
        try:
            header, model, source, content = memory_request(3, identity, 0, "").decode().split("\0")
            _, category, created, updated, dimensions = map(int, header.split("\t"))
        except (OSError, ValueError) as error:
            self.status.set(message(error))
            return
        self.record = identity
        self.original = content
        self.original_category = CATEGORIES[category]
        self.category.set(self.original_category)
        self.editor.delete("1.0", "end")
        self.editor.insert("1.0", content)
        self.heading.set("Memory details")
        for field, value in zip(self.metadata,(source, f"#{identity}  ·  {model or 'Keyword'}" +
                                                  (f" / {dimensions}d" if dimensions else ""),
                                                  stamp(created), stamp(updated))):
            field.set(value)
        self.delete.configure(state="normal")

    # ------------------------=
    # FUNC: select
    # DESC: Resolves the selected stable ID instead of inferring identity from a row position.
    # ------------------=
    def select(self, event):
        selection = self.listbox.selection()
        if selection:
            identity = int(selection[0])
            if identity == self.record or identity not in self.ids:
                return
            self.discard(lambda: self.show(identity))

    # ------------------------=
    # FUNC: new
    # DESC: Starts a blank record after protecting unsaved changes.
    # ------------------=
    def new(self):
        self.discard(self.reset)

    # ------------------------=
    # FUNC: reset
    # DESC: Clears selection and previous record metadata after deletion or new-record selection.
    # ------------------=
    def reset(self):
        self.record = 0
        self.original = ""
        self.original_category = "Semantic"
        self.category.set("Semantic")
        self.editor.delete("1.0", "end")
        self.listbox.selection_remove(*self.listbox.selection())
        self.editor.focus_force()
        self.heading.set("New memory")
        for field in self.metadata:
            field.set("—")
        self.delete.configure(state="disabled")

    # ------------------------=
    # FUNC: save
    # DESC: Updates the current stable record or creates one and displays its durable receipt.
    # ------------------=
    def save(self):
        if self.confirmation is not None:
            return
        text = self.editor.get("1.0", "end-1c").strip()
        if not 0 < len(text.encode()) <= 512:
            self.status.set("Enter between 1 and 512 UTF-8 bytes.")
            return
        try:
            identity = int(memory_request(5 if self.record else 4, self.record, CATEGORIES.index(self.category.get()), text))
        except OSError as error:
            self.status.set(message(error))
            return
        self.show(identity)
        self.refresh()

    # ------------------------=
    # FUNC: remove
    # DESC: Confirms permanent removal and refreshes the catalog only on success.
    # ------------------=
    def remove(self):
        if self.record:
            identity = self.record
            self.confirm(f"Permanently delete memory {identity}?", "Delete memory", lambda: self.erase(identity))

    # ------------------------=
    # FUNC: erase
    # DESC: Deletes the confirmed stable ID and resets the inspector only after durable success.
    # ------------------=
    def erase(self, identity):
        try:
            memory_request(6, identity, 0, "")
        except OSError as error:
            self.status.set(message(error))
            return
        self.reset()
        self.refresh()

    # ------------------------=
    # FUNC: close
    # DESC: Releases the native window only after resolving outstanding edits.
    # ------------------=
    def close(self):
        if not self.finish_fact():
            return
        self.cancel_confirmation()
        self.discard(self.root.destroy)


if __name__ == "__main__":
    window = tk.Tk()
    Browser(window)
    window.mainloop()
