"""Installed native UIKit callback failure; click the real button to exercise."""
import infinity

# ------------------------=
# FUNC: render
# DESC: Presents one real UIKit button with a stable native event identifier.
# ------------------=
def render():
    infinity.window("Python callback recovery", 420, 160)
    infinity.button(1, 24, 48, 360, 48, "Exercise callback")

# ------------------------=
# FUNC: handle
# DESC: Raises a source-located TypeError only after a real user or authorized AI event.
# ------------------=
def handle(event):
    if event["kind"] == infinity.CLICK and event["id"] == 1:
        return 1 + "invalid operand"
    return False

infinity.run(render, handle)
