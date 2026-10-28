"""Native InfinityOS UIKit and capability-checked application control hooks.

Publish truthful state and legal actions; use the same handler for pointer and AI
events. Native services require an authenticated owner and explicit AI authority.
"""
from _infinity_app import window, state, choice, text_action, publish, event, done, yield_native
from _infinity_app import element as _element

CLICK, CLOSE, AI = 1, 2, 3

# ------------------------=
# FUNC: label
# DESC: Updates one native label with a stable positive element identifier.
# ------------------=
def label(identifier, x, y, width, height, text):
    return _element(1, identifier, x, y, width, height, text)

# ------------------------=
# FUNC: display
# DESC: Updates a themed inset UIKit display, not a host-rendered substitute.
# ------------------=
def display(identifier, x, y, width, height, text):
    return _element(2, identifier, x, y, width, height, text)

# ------------------------=
# FUNC: button
# DESC: Updates one real native button using shared pointer and theme behavior.
# ------------------=
def button(identifier, x, y, width, height, text):
    return _element(3, identifier, x, y, width, height, text)

# ------------------------=
# FUNC: run
# DESC: Shares pointer and AI handling and acknowledges actions only after publishing changed state.
# ------------------=
def run(render, handle):
    render()
    while True:
        message = event()
        if message is not None:
            if message["kind"] == CLOSE:
                return
            applied = bool(handle(message))
            if applied:
                render()
            if message["kind"] == AI:
                done(message["token"], applied)
        yield_native()
