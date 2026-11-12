"""Infinity Memory, the native signed-in AIKit catalog browser."""
import time
import tkinter as tk
from tkinter import ttk
from _infinity_app import memory_request

CATEGORIES = ("All memories", "Episodic", "Semantic", "Procedural")
NAVY, PANEL, WHITE, BLUE, MUTED = "#090f26", "#101b3a", "#ffffff", "#3e6fff", "#a7bddc"


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
        from _infinity_media import io as surface
        self.surfaces = [[tk.PhotoImage(data=surface(6, b"", primary + state * 2), format="PPM")
                          for state in range(3)] for primary in (0, 1)]
        body_px = 17 if root.winfo_screenwidth() < 1500 else 20
        body_font = ("Inter", -body_px)
        self.button_font = ("Inter", -max(14, body_px-3))
        root.option_add("*Font", body_font)
        style = ttk.Style(root)
        style.theme_use("clam")
        style.configure("TLabel", background=NAVY, foreground=WHITE, font=body_font)
        style.configure("TEntry", fieldbackground=PANEL, foreground=WHITE, padding=8, font=body_font, bordercolor="#50779A", lightcolor=PANEL, darkcolor=PANEL, insertcolor=WHITE)
        style.configure("TCombobox", fieldbackground=PANEL, background=PANEL, foreground=WHITE, padding=6, font=body_font)
        style.map("TEntry", bordercolor=[("focus", "#86C7FF")])
        style.map("TCombobox", fieldbackground=[("readonly", PANEL)], foreground=[("readonly", WHITE)])
        root.option_add("*TCombobox*Listbox.background", PANEL)
        root.option_add("*TCombobox*Listbox.foreground", WHITE)
        self.root = root
        root.title("Infinity Memory")
        width = min(1520, root.winfo_screenwidth() - 48, root.maxsize()[0])
        height = min(960, root.winfo_screenheight() - 124, root.maxsize()[1])
        root.geometry(f"{width}x{height}+{max(0,(root.winfo_screenwidth()-width)//2)}+{max(72,(root.winfo_screenheight()-height)//2)}")
        root.minsize(min(720,width), min(510,height))
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
        self.details = tk.StringVar()
        top = tk.Frame(root, bg=NAVY)
        top.pack(fill="x", padx=18, pady=(16, 8))
        tk.Label(top, text="Infinity Memory", font=("Inter", -int(body_px*2.1), "bold"), bg=NAVY, fg=WHITE).pack(side="left")
        self.fact_id = 0
        self.fact_job = None
        self.fact_last = ""
        self.fact = tk.StringVar()
        self.fact_state = tk.StringVar(value="")
        capture = tk.Frame(root, bg=PANEL, padx=12, pady=8, highlightthickness=1, highlightbackground="#567FA3")
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
        ttk.Label(controls, text="Search").pack(side="left", padx=(0, 6))
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
        body.add(left, minsize=200, width=int(width*.35))
        body.add(right, minsize=290)
        tk.Label(left,textvariable=self.list_title,bg=PANEL,fg=WHITE,font=("Inter",-body_px-3,"bold")).pack(anchor="w", padx=14, pady=(10,2))
        self.listbox = tk.Listbox(left, bg=PANEL, fg=WHITE, selectbackground=BLUE, font=body_font,
                                  exportselection=False, borderwidth=0, highlightthickness=0)
        self.listbox.pack(expand=True, fill="both", padx=10, pady=12)
        self.listbox.bind("<<ListboxSelect>>", self.select)
        nav = tk.Frame(left, bg=PANEL)
        nav.pack(fill="x", padx=8, pady=8)
        self.prev = self.button(nav, "Previous", lambda: self.move(-1))
        self.next = self.button(nav, "Next", lambda: self.move(1))
        self.prev.pack(side="left")
        self.next.pack(side="right")
        tk.Label(right, textvariable=self.heading, bg=PANEL, fg=WHITE, font=("Inter", -body_px-5, "bold")).pack(anchor="w", padx=14, pady=(12, 6))
        self.editor = tk.Text(right, height=5, wrap="word", bg=NAVY, fg=WHITE, insertbackground=WHITE,
                              font=body_font, padx=8, pady=8)
        self.editor.bind("<ButtonPress-1>", lambda event: self.editor.focus_force(), add="+")
        self.editor.pack(fill="both", expand=True, padx=14)
        row = tk.Frame(right, bg=PANEL)
        row.pack(fill="x", padx=14, pady=8)
        ttk.Label(row, text="Category").pack(side="left")
        ttk.Combobox(row, values=CATEGORIES[1:], textvariable=self.category, state="readonly", width=14).pack(side="left", padx=8)
        tk.Label(right, textvariable=self.details, bg=PANEL, fg=MUTED, anchor="w", justify="left").pack(fill="x", padx=14)
        buttons = tk.Frame(right, bg=PANEL)
        buttons.pack(fill="x", padx=14, pady=10)
        self.delete = self.button(buttons, "Delete", self.remove)
        self.delete.configure(state="disabled")
        self.delete.pack(side="left")
        self.commit = self.button(buttons, "Save", self.save, True)
        self.commit.pack(side="right")
        root.bind("<Escape>", lambda event: self.cancel_confirmation())
        tk.Label(root, textvariable=self.status, bg=NAVY, fg=MUTED, anchor="w").pack(fill="x", padx=18, pady=(0, 10))
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
            self.listbox.selection_set(0)
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
    # FUNC: button
    # DESC: Renders the active Infinity UIKit button recipe for normal, hover and pressed states.
    # ------------------=
    def button(self, parent, label, action, primary=False):
        normal, hover, pressed = self.surfaces[int(primary)]
        button = tk.Button(parent, text=label, command=action, image=normal, compound="center",
                           fg=WHITE, activeforeground=WHITE, bg=NAVY, activebackground=NAVY,
                           font=self.button_font, borderwidth=0, relief="flat", highlightthickness=0)
        for event, art in (("<Enter>", hover), ("<Leave>", normal), ("<ButtonPress-1>", pressed),
                           ("<ButtonRelease-1>", hover), ("<FocusIn>", hover), ("<FocusOut>", normal)):
            button.bind(event, lambda e, image=art: button.configure(image=image), add="+")
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
            self.list_title.set(f"Memories ({count})")
            if self.page and self.page * 6 >= self.total:
                self.page = max(0, (self.total - 1) // 6)
                return self.refresh()
            records = [row.split("\t", 2) for row in rows]
            ids = [int(row[0]) for row in records]
            titles = tuple(row[2] for row in records)
            if ids != self.ids or titles != self.listbox.get(0, "end"):
                self.ids = ids
                self.listbox.delete(0, "end")
                if titles:
                    self.listbox.insert("end", *titles)
                if self.record in ids:
                    self.listbox.selection_set(ids.index(self.record))
            self.prev.configure(state="normal" if self.page else "disabled")
            self.next.configure(state="normal" if (self.page + 1) * 6 < self.total else "disabled")
            prefix = {0: "No matching memories", 1: "1 match"}.get(self.total, f"{count} matches")
            summary = f"{prefix}    |    {total} stored    |    {embedded} embedded    |    {size} bytes"
            if self.confirmation is None and self.status.get() != summary:
                self.status.set(summary)
        except (OSError, ValueError) as error:
            self.ids.clear()
            self.listbox.delete(0, "end")
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
        self.heading.set(content.splitlines()[0][:48])
        self.details.set(f"Memory {identity}   |   {source}\nCreated  {stamp(created)}\nUpdated  {stamp(updated)}\n"
                         + (f"{model}  /  {dimensions} dimensions" if dimensions else "Keyword index only"))
        self.delete.configure(state="normal")

    # ------------------------=
    # FUNC: select
    # DESC: Resolves the selected stable ID instead of inferring identity from a row position.
    # ------------------=
    def select(self, event):
        selection = self.listbox.curselection()
        if selection:
            identity = self.ids[selection[0]]
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
        self.listbox.selection_clear(0, "end")
        self.editor.focus_force()
        self.heading.set("New memory")
        self.details.set("")
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
