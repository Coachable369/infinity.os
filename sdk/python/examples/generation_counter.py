import infinity as ui
value = 0

# ------------------------=
# FUNC: render
# DESC: Publishes current data and legal controls using native UIKit.
# ------------------=
def render():
    ui.state(False, False, str(value))
    ui.display(3, 20, 20, 280, 32, str(value))
    ui.button(1, 20, 72, 112, 48, "+")
    ui.button(2, 156, 72, 112, 48, "-")
    ui.choice(1, "counter.increase", "Increase")
    ui.choice(2, "counter.decrease", "Decrease")
    ui.publish()

# ------------------------=
# FUNC: handle
# DESC: Shares validated state updates between pointer and authorized AI actions.
# ------------------=
def handle(event):
    global value
    if event["kind"] not in (ui.CLICK, ui.AI):
        return False
    if event["id"] == 1:
        value += 1
    elif event["id"] == 2:
        value -= 1
    else:
        return False
    return True

if __name__ == "__main__":
    ui.window("Counter", 320, 240)
    ui.run(render, handle)
