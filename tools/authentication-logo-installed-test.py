"""Fresh-installed login and locked-session branding acceptance through real framebuffer pixels."""
import importlib.util
import pathlib
import shutil
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("installed_guest", ROOT / "tools/ms9-installed-acceptance.py")
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)


# ------------------------=
# FUNC: ppm_pixels
# DESC: Decodes one QEMU P6 framebuffer capture into its dimensions and exact RGB byte payload.
# ------------------=
def ppm_pixels(path):
    data = path.read_bytes()
    cursor = 0
    tokens = []
    while len(tokens) < 4:
        while cursor < len(data) and data[cursor] in b" \r\n\t":
            cursor += 1
        if cursor < len(data) and data[cursor] == ord("#"):
            cursor = data.index(b"\n", cursor) + 1
            continue
        end = cursor
        while end < len(data) and data[end] not in b" \r\n\t":
            end += 1
        tokens.append(data[cursor:end])
        cursor = end
    while cursor < len(data) and data[cursor] in b" \r\n\t":
        cursor += 1
    assert tokens[0] == b"P6" and tokens[3] == b"255"
    width, height = int(tokens[1]), int(tokens[2])
    pixels = data[cursor:]
    assert len(pixels) == width * height * 3
    return width, height, pixels


# ------------------------=
# FUNC: assert_illuminated_logo
# DESC: Requires the installed authentication card to contain the broad blue-white logo raster rather than a thin procedural path.
# ------------------=
def assert_illuminated_logo(path):
    width, height, pixels = ppm_pixels(path)
    fit = min(width * 1000 // 1536, height * 1000 // 1024)
    content_width, content_height = 1536 * fit // 1000, 1024 * fit // 1000
    offset_x, offset_y = (width - content_width) // 2, (height - content_height) // 2
    sx = lambda value: offset_x + value * fit // 1000
    sy = lambda value: offset_y + value * fit // 1000
    left, right = sx(235), sx(393)
    top, bottom = sy(173), sy(255)
    illuminated = saturated = 0
    for y in range(top, bottom):
        for x in range(left, right):
            at = (y * width + x) * 3
            red, green, blue = pixels[at:at + 3]
            illuminated += max(red, green, blue) >= 175
            saturated += blue >= 130 and blue >= red + 20
    scale_area = max(1, fit * fit // 1_000_000)
    assert illuminated >= 2200 * scale_area, (path, illuminated)
    assert saturated >= 650 * scale_area, (path, saturated)


# ------------------------=
# FUNC: assert_success_motion
# DESC: Requires a real lower-right framebuffer transition between the resting orb and water-impact keyframes.
# ------------------=
def assert_success_motion(resting_path, impact_path):
    resting_width, resting_height, resting = ppm_pixels(resting_path)
    impact_width, impact_height, impact = ppm_pixels(impact_path)
    assert (resting_width, resting_height) == (impact_width, impact_height)
    changed = brightened = 0
    for y in range(resting_height * 52 // 100, resting_height * 97 // 100):
        for x in range(resting_width * 48 // 100, resting_width * 98 // 100):
            at = (y * resting_width + x) * 3
            before = resting[at:at + 3]
            after = impact[at:at + 3]
            difference = sum(abs(int(after[index]) - int(before[index])) for index in range(3))
            changed += difference >= 42
            brightened += max(after) >= max(before) + 36 and after[2] >= after[0]
    scale_area = max(1, resting_width * resting_height // (1536 * 1024))
    assert changed >= 4_500 * scale_area, (impact_path, changed)
    assert brightened >= 700 * scale_area, (impact_path, brightened)


# ------------------------=
# FUNC: open_system_action
# DESC: Uses the native keyboard menu to execute one indexed session action and verifies its resulting mode.
# ------------------=
def open_system_action(guest, index, expected_mode):
    guest.key("ret")
    state = guest.wait(lambda value: value[4] == 7, "system menu opened")
    for _ in range(index):
        previous = state[8]
        guest.key("down")
        state = guest.wait(lambda value: value[4] == 7 and value[8] != previous, "system menu focus advanced")
    guest.key("ret")
    return guest.wait(lambda value: value[4] == expected_mode, "authentication surface opened")


# ------------------------=
# FUNC: submit_authentication_animation
# DESC: Enters the real password, captures the gated impact frame, and waits for the animation-owned desktop commit.
# ------------------=
def submit_authentication_animation(guest, expected_mode, resting_path, capture_name):
    state = guest.wait(lambda value: value[3] == 1 and value[4] == expected_mode,
                       "authentication ready")
    for _ in range(11):
        if state[8] == 1:
            break
        guest.key("tab")
        state = guest.wait(lambda value: value[8] != state[8], "password focus")
    assert state[8] == 1
    guest.text("MeshProof901")
    guest.key("ret")
    guest.wait(lambda value: value[4] == expected_mode and value[9] & 3 == 3,
               "success animation retained authentication surface")
    time.sleep(0.68)
    impact_path = guest.screenshot(capture_name)
    assert_success_motion(resting_path, impact_path)
    return guest.wait(lambda value: value[4] == 5 and value[9] & 3 == 3,
                      "success animation committed desktop")


# ------------------------=
# FUNC: main
# DESC: Installs, onboards, signs out, unlocks, locks, and proves the canonical raster logo on both authentication states.
# ------------------=
def main():
    assert len(sys.argv) in (2, 3)
    reuse = len(sys.argv) == 3 and sys.argv[2] == "--reuse-installed"
    work = pathlib.Path(sys.argv[1]).resolve()
    if reuse:
        assert work.is_dir()
    else:
        work.mkdir(exist_ok=False)
    artifacts = work / "artifacts"
    if not reuse:
        artifacts.mkdir()
    for source, name in [
        ("builds/InfinityOS-x86_64.iso", "installer.iso"),
        ("build/x86_64/kernel.elf", "kernel.elf"),
        ("build/x86_64/installed-kernel.elf", "installed-kernel.elf"),
    ]:
        if not reuse:
            shutil.copyfile(ROOT / source, artifacts / name)
    guest = base.Guest(work, 1, "/opt/homebrew/share/qemu/edk2-x86_64-code.fd", reuse=reuse,
                       width=1536, height=1024, memory_mb=8192)
    try:
        if not reuse:
            guest.install()
            guest.stop()
        guest.onboard()
        open_system_action(guest, 7, 9)
        signed_out = guest.screenshot("login-canonical-logo")
        assert_illuminated_logo(signed_out)
        submit_authentication_animation(
            guest, 9, signed_out, "login-success-water-impact"
        )
        open_system_action(guest, 6, 10)
        locked = guest.screenshot("resume-canonical-logo")
        assert_illuminated_logo(locked)
        submit_authentication_animation(
            guest, 10, locked, "resume-success-water-impact"
        )
        print("Installed login and resume surfaces render the canonical logo and water-impact success motion")
    finally:
        guest.stop()


if __name__ == "__main__":
    main()
