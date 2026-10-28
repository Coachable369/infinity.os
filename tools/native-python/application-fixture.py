"""Native UIKit receipt fixture: inspect the real app registry and activate button 1.

The fixture exits 42 only after native pointer or authorized AI input changes its
state and commits a newer observation. Importing this file on a host is not proof.
"""
import sys
import infinity

infinity.window("Python Native Actions", 400, 280)
value = 0

# ------------------------=
# FUNC: render
# DESC: Publishes actual visible state and a legal action using native application contracts.
# ------------------=
def render():
    infinity.display(2, 20, 20, 360, 32, str(value))
    infinity.button(1, 20, 80, 160, 44, "Increment")
    infinity.state(True, False, "counter=" + str(value))
    infinity.choice(1, "counter.increment", "Increment counter")
    infinity.publish()

render()
while True:
    event = infinity.event()
    if event is not None:
        if event["kind"] == infinity.CLOSE:
            sys.exit(1)
        applied = event["id"] == 1
        if applied:
            value += 1
            render()
        if event["kind"] == infinity.AI:
            infinity.done(event["token"], applied)
        if applied:
            sys.exit(42)
    infinity.yield_native()
