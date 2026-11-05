"""InfinityOS Media Player. Media bytes and sound output belong to the signed-in session."""
import sys
import time
import tkinter as tk
import _infinity_media as codec

NAVY, PANEL, BLUE, LIGHT, MUTED = "#081426", "#132441", "#155be6", "#f4f8ff", "#a7bddc"

# ------------------------=
# FUNC: timestamp
# DESC: Formats position and duration without requiring a wall clock or time zone.
# ------------------=
def timestamp(seconds):
    minutes, seconds = divmod(max(0, int(seconds)), 60)
    return f"{minutes}:{seconds:02d}"


class Player:
    # ------------------------=
    # FUNC: __init__
    # DESC: Builds the native player window and loads the selected object or opens its library.
    # ------------------=
    def __init__(self, root, initial=None):
        self.root = root
        root.title("Media Player")
        root.configure(bg=NAVY)
        root.geometry("580x530")
        root.minsize(430, 450)
        self.raw = self.audio = self.video = self.pending_video = None
        self.pcm = bytearray()
        self.target = self.duration = self.position = 0.0
        self.playing = self.dragging = self.eof = False
        self.buttons = {}
        self.paths = []
        self.current = None
        self.ticket = None
        self.rate = max(16000, codec.io(2, b"", 0))
        self.title = tk.StringVar(value="Media Player")
        self.status = tk.StringVar(value="Choose a file from your library")
        head = tk.Frame(root, bg=NAVY)
        head.pack(fill="x", padx=20, pady=10)
        tk.Label(head, textvariable=self.title, anchor="w", font=("Inter", 15, "bold"),
                 bg=NAVY, fg=LIGHT).pack(side="left", expand=True, fill="x")
        self.button(head, "Library", self.library).pack(side="right")
        self.screen = tk.Label(root, bg=NAVY)
        self.screen.pack(expand=True, fill="both")
        self.art = tk.PhotoImage(file="/System/media-cover.png")
        self.screen.configure(image=self.art)
        self.note = tk.Label(root, textvariable=self.status, bg=NAVY, fg=MUTED)
        self.note.pack()
        self.slider = tk.Scale(root, from_=0, to=1, resolution=.1, orient="horizontal",
                               showvalue=False, bg=NAVY, fg=LIGHT, troughcolor=PANEL,
                               activebackground=BLUE, highlightthickness=0)
        self.slider.pack(fill="x", padx=24)
        self.slider.bind("<ButtonPress-1>", lambda e: setattr(self, "dragging", True))
        self.slider.bind("<ButtonRelease-1>", self.drop_seek)
        self.volume = tk.IntVar(value=85)
        self.time = tk.Label(root, text="0:00 / 0:00", bg=NAVY, fg=MUTED)
        self.time.pack()
        bar = tk.Frame(root, bg=NAVY)
        bar.pack(pady=(4, 12))
        self.button(bar, "Previous", lambda: self.step(-1)).pack(side="left", padx=5)
        self.transport = self.button(bar, "Play", self.toggle, primary=True)
        self.transport.pack(side="left", padx=5)
        self.button(bar, "Next", lambda: self.step(1)).pack(side="left", padx=5)
        volume = tk.Frame(root, bg=NAVY)
        volume.pack(pady=(0, 8))
        tk.Label(volume, text="Volume", fg=MUTED, bg=NAVY).pack(side="left", padx=8)
        tk.Scale(volume, from_=0, to=100, variable=self.volume, length=140, orient="horizontal",
                 showvalue=False, bg=NAVY, fg=LIGHT, troughcolor=PANEL, highlightthickness=0).pack(side="left")
        root.bind("<space>", lambda e: self.toggle())
        root.protocol("WM_DELETE_WINDOW", self.close)
        self.scan()
        if initial:
            root.after(1, lambda: self.open(initial))
        else:
            root.after(1, self.library)

    # ------------------------=
    # FUNC: button
    # DESC: Creates a keyboard-accessible transport control.
    # ------------------=
    def button(self, parent, label, action, primary=False):
        if primary not in self.buttons:
            self.buttons[primary] = [tk.PhotoImage(data=codec.io(6, b"", int(primary)+2*state),
                                                  format="PPM") for state in range(3)]
        normal, hover, pressed = self.buttons[primary]
        button = tk.Button(parent, text=label, command=action, image=normal, compound="center",
                           bg=NAVY, fg=LIGHT, activebackground=NAVY, activeforeground=LIGHT,
                           relief="flat", borderwidth=0, padx=0, pady=0, highlightthickness=0,
                           font=("Inter", 11))
        for event, image in (("<Enter>",hover),("<Leave>",normal),("<ButtonPress-1>",pressed),
                             ("<ButtonRelease-1>",hover),("<FocusIn>",hover),("<FocusOut>",normal)):
            button.bind(event, lambda e, img=image: button.configure(image=img), add="+")
        return button

    # ------------------------=
    # FUNC: scan
    # DESC: Refreshes only the files the kernel currently allows this account to open.
    # ------------------=
    def scan(self):
        self.paths.clear()
        for i in range(128):
            path = codec.io(0, b"", i)
            if path is None:
                break
            self.paths.append(path)
        self.paths.sort(key=str.casefold)

    # ------------------------=
    # FUNC: library
    # DESC: Opens a single reusable chooser of accessible media objects.
    # ------------------=
    def library(self):
        if getattr(self, "picker", None) and self.picker.winfo_exists():
            self.picker.lift()
            return
        self.scan()
        self.picker = win = tk.Toplevel(self.root, bg=NAVY)
        win.title("Media Library")
        tk.Label(win, text="Media Library", font=("Inter", 16, "bold"), bg=NAVY, fg=LIGHT).pack(anchor="w", padx=16, pady=10)
        items = tk.Listbox(win, width=50, height=9, bg=PANEL, fg=LIGHT, selectbackground=BLUE,
                           font=("Inter", 11), exportselection=False)
        items.pack(fill="both", expand=True, padx=12)
        for path in self.paths:
            items.insert("end", path)
        def choose(event=None):
            if items.curselection():
                path = self.paths[items.curselection()[0]]
                win.destroy()
                self.open(path)
        items.bind("<Return>", choose)
        items.bind("<Double-Button-1>", choose)
        if self.paths:
            items.selection_set(0)
            self.button(win, "Play selection", choose, True).pack(pady=8)
        else:
            tk.Label(win, text="No supported media files in your personal storage.", bg=NAVY, fg=MUTED).pack(pady=8)
        win.bind("<Escape>", lambda e: win.destroy())
        items.focus_set()

    # ------------------------=
    # FUNC: open
    # DESC: Opens independent audio and video tracks and starts playback after a successful decode.
    # ------------------=
    def open(self, path):
        self.pause()
        self.audio = self.video = self.raw = None
        self.current = path
        self.title.set(path.rsplit("/", 1)[-1])
        self.screen.configure(image=self.art)
        try:
            self.raw = codec.io(1, path.encode(), 0)
            self.rate = max(16000, codec.io(2, b"", 0))
            a = codec.open(self.raw, 0, self.rate)
            v = codec.open(self.raw, 1, self.rate)
            if a and codec.io(2, b"", 0) < 0:
                raise OSError("No audio output device available")
            self.audio, self.video = a and a[0], v and v[0]
            if not (a or v):
                raise ValueError("No playable audio or video track")
            self.duration = max((a or (0,0))[1], (v or (0,0))[1])
            self.slider.configure(to=max(1, self.duration))
            self.position = self.target = 0
            self.play()
        except (ValueError, OSError, MemoryError) as exc:
            self.status.set(str(exc))

    # ------------------------=
    # FUNC: play
    # DESC: Restarts the playback clock from the selected position and schedules the first frame.
    # ------------------=
    def play(self):
        if not self.audio and not self.video:
            self.library()
            return
        self.pending_video = None
        self.pcm.clear()
        self.eof = False
        self.audio_done = self.audio is None
        self.video_done = self.video is None
        self.wait_start = None
        if self.position >= self.duration and self.duration > 0:
            self.position = 0
        self.target = self.position
        self.submitted = self.chunk_start = self.position
        try:
            for stream in (self.audio, self.video):
                if stream:
                    codec.seek(stream, self.position)
        except ValueError as exc:
            self.status.set(str(exc))
            return
        self.clock = time.monotonic() - self.position
        self.playing = True
        self.status.set("Playing")
        self.transport.configure(text="Pause")
        self.tick()

    # ------------------------=
    # FUNC: pause
    # DESC: Cancels scheduled decoding and releases this player's audio lease.
    # ------------------=
    def pause(self):
        if self.ticket is not None:
            self.root.after_cancel(self.ticket)
            self.ticket = None
        if self.playing and not self.audio:
            self.position = max(0.0, time.monotonic() - self.clock)
        self.playing = False
        codec.io(4, b"", 0)
        self.transport.configure(text="Play")
        self.status.set("Paused")

    # ------------------------=
    # FUNC: toggle
    # DESC: Applies Play or Pause consistently from the keyboard and transport.
    # ------------------=
    def toggle(self):
        self.pause() if self.playing else self.play()

    # ------------------------=
    # FUNC: step
    # DESC: Opens the adjacent file in the user's sorted library.
    # ------------------=
    def step(self, delta):
        if self.paths:
            i = self.paths.index(self.current) if self.current in self.paths else -1
            self.open(self.paths[(i + delta) % len(self.paths)])

    # ------------------------=
    # FUNC: drop_seek
    # DESC: Changes playback position after releasing the scrubber.
    # ------------------=
    def drop_seek(self, event):
        active = self.playing
        self.pause()
        self.dragging = False
        self.position = self.slider.get()
        if active:
            self.play()
        else:
            self.time.configure(text=f"{timestamp(self.position)} / {timestamp(self.duration)}")

    # ------------------------=
    # FUNC: tick
    # DESC: Paces decoded audio batches and timestamped video frames without blocking on the speaker.
    # ------------------=
    def tick(self):
        self.ticket = None
        if not self.playing:
            return
        try:
            if self.audio_done:
                self.position = time.monotonic() - self.clock
            if self.audio and not self.eof and len(self.pcm) < self.rate * 2:
                for _ in range(8):
                    frame = codec.next(self.audio)
                    if frame is None:
                        self.eof = True
                        break
                    pts, samples = frame
                    skip = min(len(samples)//4, max(0, round((self.target - pts)*self.rate)))
                    self.pcm.extend(samples[skip*4:])
                    if len(self.pcm) >= self.rate * 2:
                        break
            busy = codec.io(5, b"", 0)
            if self.audio and not self.audio_done:
                if busy > 0:
                    self.position = min(self.submitted, self.chunk_start + (busy-1)/self.rate)
                elif busy == 0:
                    self.position = self.submitted
                if busy < 0:
                    if self.wait_start is None:
                        self.wait_start = time.monotonic()
                    elif time.monotonic()-self.wait_start > 5:
                        raise OSError("Audio output is busy")
                else:
                    self.wait_start = None
            if self.pcm and (len(self.pcm) >= self.rate*2 or self.eof) and not busy:
                count = min(len(self.pcm), self.rate*4)
                result = codec.io(3, memoryview(self.pcm)[:count], self.volume.get())
                if result == 0:
                    self.chunk_start = self.submitted
                    self.submitted += count / (4 * self.rate)
                    del self.pcm[:count]
                    busy = 1
                elif result != -11:
                    raise OSError("Audio output unavailable")
            if self.audio and not self.audio_done and self.eof and not self.pcm and not busy:
                if self.video and not self.video_done:
                    self.clock = time.monotonic() - self.position
                self.audio_done = True
            if self.video and not self.video_done:
                for _ in range(3):
                    if self.pending_video is None:
                        self.pending_video = codec.next(self.video)
                    if self.pending_video is None:
                        self.video_done = True
                        break
                    pts, pixels = self.pending_video
                    if pts > self.position:
                        break
                    self.pending_video = None
                    if pts + .15 >= self.position:
                        self.picture = tk.PhotoImage(data=pixels, format="PPM")
                        self.screen.configure(image=self.picture)
            self.time.configure(text=f"{timestamp(self.position)} / {timestamp(self.duration)}")
            if not self.dragging:
                self.slider.set(self.position)
            if self.video_done and (not self.audio or self.eof and not self.pcm and not busy):
                self.pause()
                self.position = self.duration
                self.status.set("Finished")
                return
            self.ticket = self.root.after(16, self.tick)
        except (ValueError, OSError, tk.TclError, MemoryError) as exc:
            self.pause()
            self.status.set(str(exc))

    # ------------------------=
    # FUNC: close
    # DESC: Stops playback before destroying the native window.
    # ------------------=
    def close(self):
        self.pause()
        self.root.destroy()


if __name__ == "__main__":
    root = tk.Tk()
    Player(root, sys.argv[1] if len(sys.argv)>1 else None)
    root.mainloop()
