"""Behavioral playback regression with an unseekable fresh track and a finite audio sink."""
import runpy
import sys
import types
from pathlib import Path

class Widget:
    # ------------------------=
    # FUNC: __init__
    # DESC: Stores control values and timer callbacks for deterministic playback transitions.
    # ------------------=
    def __init__(self, *args, **kw):
        self.value = kw.get("value", 0)
        self.pending = {}
        self.serial = 0
    # ------------------------=
    # FUNC: __getattr__
    # DESC: Supplies inert decoration operations while retaining real player playback logic.
    # ------------------=
    def __getattr__(self, name):
        return lambda *args, **kw: None
    # ------------------------=
    # FUNC: set
    # DESC: Records a slider or bound variable mutation.
    # ------------------=
    def set(self, value): self.value = value
    # ------------------------=
    # FUNC: get
    # DESC: Reads the current control value.
    # ------------------=
    def get(self): return self.value
    # ------------------------=
    # FUNC: width
    # DESC: Models the large source artwork that previously crowded out transport controls.
    # ------------------=
    def width(self): return 1024
    # ------------------------=
    # FUNC: height
    # DESC: Models the source artwork dimensions.
    # ------------------=
    def height(self): return 1024
    # ------------------------=
    # FUNC: subsample
    # DESC: Returns a retained artwork derivative.
    # ------------------=
    def subsample(self, *args): return self
    # ------------------------=
    # FUNC: after
    # DESC: Enqueues callbacks without executing recursive event-loop work.
    # ------------------=
    def after(self, delay, callback):
        self.serial += 1
        self.pending[self.serial] = callback
        return self.serial
    # ------------------------=
    # FUNC: after_cancel
    # DESC: Cancels a pending player callback.
    # ------------------=
    def after_cancel(self, ticket): self.pending.pop(ticket, None)

class Codec(types.ModuleType):
    # ------------------------=
    # FUNC: __init__
    # DESC: Creates an audio-only source with one 600ms PCM block.
    # ------------------=
    def __init__(self):
        super().__init__("_infinity_media")
        self.sent = 0
        self.frames = []
        self.seeks = 0
    # ------------------------=
    # FUNC: io
    # DESC: Models authorized file reads, hardware rate, and an immediately drained PCM sink.
    # ------------------=
    def io(self, op, data, index):
        if op in (0, 7): return None
        if op == 1: return b"test"
        if op == 2: return 48000
        if op == 3: self.sent += len(data)
        return 0
    # ------------------------=
    # FUNC: open
    # DESC: Opens a fresh audio decoder and reports absence of video as the native decoder does.
    # ------------------=
    def open(self, raw, video, rate):
        if video: return None
        self.frames = [(0.0, b"\x01\x00" * 57600)]
        return object(), .6
    # ------------------------=
    # FUNC: next
    # DESC: Returns finite decoded frames followed by EOF.
    # ------------------=
    def next(self, stream): return self.frames.pop(0) if self.frames else None
    # ------------------------=
    # FUNC: seek
    # DESC: Rejects seeks to verify fresh playback never depends on seek support.
    # ------------------=
    def seek(self, stream, position):
        self.seeks += 1
        raise ValueError("Unseekable")

sys.modules["tkinter"] = types.SimpleNamespace(**{name: Widget for name in
    ("Frame", "Label", "PhotoImage", "Scale", "IntVar", "StringVar", "Button")}, TclError=RuntimeError)
codec = Codec()
sys.modules["_infinity_media"] = codec
Player = runpy.run_path(str(Path(__file__).resolve().parents[1] / "sdk/system/media-player.py"))["Player"]
root = Widget()
player = Player(root, "/home/Alex/music/Chime.wav")
root.pending.pop(1)()
assert player.playing and codec.sent == 115200 and codec.seeks == 0
player.tick()
assert not player.playing and player.position == .6
player.open("/home/Alex/music/Next.mp3")
assert player.playing and codec.sent == 230400 and codec.seeks == 0
player.close()
assert not player.playing
print("PASS: initial double-click, full drain, replacement open and close transitions")
