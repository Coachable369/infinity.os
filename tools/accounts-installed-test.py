"""Fresh installed account acceptance through real input and non-secret binary state.

Run through build-kit run. This runner deliberately has no reuse, resume, or
offline-patch mode. The supplied media is installed unchanged on a new disk.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location(
    "accounts_native", ROOT / "tools/native-editor-installed-test.py")
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)

USERS, GROUPS, NEW, BACK, ROW = 1, 2, 3, 4, 5
RENAME, ENABLED, PASSWORD, DELETE, MEMBER = 6, 7, 8, 9, 10
FIELD, SAVE, CANCEL, CONFIRM = 11, 12, 13, 14
BROWSE, CREATE, EDIT, SECRET, REMOVE = 0, 1, 2, 3, 4
SAVED, DELETED = 1, 2
TEST_PASSWORD = "AccountProof901"
RESET_PASSWORD = "AccountReset902"
ADMIN_PASSWORD = "MeshProof901"


class AccountsGuest(native.NativeGuest):
    # ------------------------=
    # FUNC: wait
    # DESC: Allows slow native onboarding while retaining explicit short input deadlines from the shared helper.
    # ------------------=
    def wait(self, predicate, label, timeout=600):
        return super().wait(predicate, label, timeout)

    # ------------------------=
    # FUNC: boot
    # DESC: Disables inherited offline replacement and attests every installed boot through QMP block state.
    # ------------------=
    def boot(self, installer):
        assert self.patched is True, "Offline kernel replacement is forbidden"
        super().boot(installer)
        if not installer:
            blocks = self.qmp("query-block")
            inserted = [block["inserted"] for block in blocks if "inserted" in block]
            files = [Path(block["file"]).resolve() for block in inserted]
            # Empty optical drives are harmless; no inserted removable media is allowed.
            assert not any(block.get("removable") and "inserted" in block for block in blocks)
            assert self.disk.resolve() in files
            assert (self.work.parent / "artifacts/installer.iso").resolve() not in files
            self.detached_boots.append({"inserted_files": [str(path) for path in files]})


# ------------------------=
# FUNC: accounts
# DESC: Reads the coherent public account snapshot without injecting guest state or exposing credentials.
# ------------------=
def accounts(guest):
    state = native.read(guest, "INFINITY_ACCOUNTS_DIAGNOSTIC_SNAPSHOT", 768)
    assert state[:2] == (0x494e464143435431, 1)
    assert state[8] <= 16 and state[9] <= 16 and state[12] <= 64
    return state


# ------------------------=
# FUNC: wait_accounts
# DESC: Requires observed account state after real input, never rendered messages or command output.
# ------------------=
def wait_accounts(guest, predicate, timeout=60):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state = accounts(guest)
        if state[3] == 1 and state[15] == 1 and predicate(state):
            return state
        time.sleep(.1)
    guest.screenshot("accounts-failure")
    raise AssertionError({"account_header": state[:16]})


# ------------------------=
# FUNC: entries
# DESC: Decodes immutable account identifiers, flags and membership masks from the versioned binary contract.
# ------------------=
def entries(state, tab):
    start = 32 if tab == 0 else 96
    result = []
    for index in range(state[8 + tab]):
        low, high, flags, members = state[start + index * 4:start + index * 4 + 4]
        identifier = struct.pack("<2Q", low, high).hex()
        assert low or high
        result.append((identifier, flags, members))
    assert len({entry[0] for entry in result}) == len(result)
    return result


# ------------------------=
# FUNC: directory
# DESC: Normalizes membership by stable user IDs so slot reordering cannot fake persistence or deletion.
# ------------------=
def directory(state):
    users = entries(state, 0)
    groups = entries(state, 1)
    for _, _, mask in groups:
        assert mask >> len(users) == 0
    return {
        "users": {identifier: flags for identifier, flags, _ in users},
        "groups": {identifier: {"flags": flags, "members": sorted(
            user[0] for index, user in enumerate(users) if mask & (1 << index))}
            for identifier, flags, mask in groups},
    }


# ------------------------=
# FUNC: control
# DESC: Finds one live enabled control by its stable target and argument.
# ------------------=
def control(state, target, argument=0):
    matches = [state[160 + index * 8:168 + index * 8] for index in range(state[12])
               if state[160 + index * 8:162 + index * 8] == (target, argument)]
    assert len(matches) == 1, (target, argument, state[:16])
    assert matches[0][6] == 1, (target, argument, "disabled")
    return matches[0]


# ------------------------=
# FUNC: focus
# DESC: Traverses native Tab order and requires the requested enabled control to become focused and revealed.
# ------------------=
def focus(guest, target, argument=0):
    state = accounts(guest)
    control(state, target, argument)
    for _ in range(2 * state[12] + 2):
        if state[13:15] == (target, argument):
            return state
        guest.key("tab")
        state = accounts(guest)
    raise AssertionError(("unreachable keyboard target", target, argument))


# ------------------------=
# FUNC: activate
# DESC: Activates a real keyboard target or a fully visible production control through the native pointer driver.
# ------------------=
def activate(guest, target, argument=0, keyboard=True):
    state = focus(guest, target, argument)
    if keyboard:
        guest.key("ret")
        return
    _, _, x, y, width, height, _, visible = control(state, target, argument)
    screen = guest.state()
    assert visible == 1 and width > 0 and height > 0
    assert x + width <= screen[11] and y + height <= screen[12]
    native.adaptive_click(guest, x + width // 2, y + height // 2)


# ------------------------=
# FUNC: field
# DESC: Types into a focused native form field, replacing public text only through Ctrl+A.
# ------------------=
def field(guest, index, value, replace=False):
    focus(guest, FIELD, index)
    if replace:
        assert index in (0, 1)
        guest.key("ctrl", "a")
    native.ui.text(guest, value)


# ------------------------=
# FUNC: open_accounts
# DESC: Opens actual Settings and selects Users through the shared Settings navigation geometry.
# ------------------=
def open_accounts(guest, administrator=True):
    guest.launch("settings", 8)
    state = native.read(guest, "INFINITY_SETTINGS_DIAGNOSTIC_SNAPSHOT", 128)
    assert state[:2] == (0x494e465345545331, 1) and state[3] == 1
    native.adaptive_click(guest, state[93], state[94])
    return wait_accounts(guest, lambda state: state[7] == int(administrator) and state[5] == BROWSE)


# ------------------------=
# FUNC: select
# DESC: Selects an account by its immutable ID, resolving the current row index after every directory refresh.
# ------------------=
def select(guest, tab, identifier):
    activate(guest, USERS if tab == 0 else GROUPS)
    state = wait_accounts(guest, lambda value: value[4] == tab and value[5] == BROWSE)
    index = [entry[0] for entry in entries(state, tab)].index(identifier)
    activate(guest, ROW, index, keyboard=False)
    return wait_accounts(guest, lambda value: struct.pack("<2Q", *value[10:12]).hex() == identifier)


# ------------------------=
# FUNC: created
# DESC: Requires exactly one new stable ID, successful durable form completion and a selected result.
# ------------------=
def created(guest, tab, before, require_feedback=True):
    key = "users" if tab == 0 else "groups"
    state = wait_accounts(guest, lambda value: len(directory(value)[key]) == len(before[key]) + 1
                          and (not require_feedback or value[5:7] == (BROWSE, SAVED)), timeout=600)
    after = directory(state)
    added = set(after[key]) - set(before[key])
    assert len(added) == 1 and set(before[key]) <= set(after[key])
    identifier = added.pop()
    if require_feedback:
        assert struct.pack("<2Q", *state[10:12]).hex() == identifier
    return identifier


# ------------------------=
# FUNC: ui_create
# DESC: Creates an account with native keyboard form entry and verifies the new identity in authoritative state.
# ------------------=
def ui_create(guest, tab, handle, display):
    activate(guest, USERS if tab == 0 else GROUPS, keyboard=False)
    wait_accounts(guest, lambda state: state[4] == tab)
    before = directory(accounts(guest))
    activate(guest, NEW, keyboard=False)
    wait_accounts(guest, lambda state: state[5] == CREATE)
    field(guest, 0, display)
    field(guest, 1, handle)
    if tab == 0:
        field(guest, 2, TEST_PASSWORD)
        field(guest, 3, TEST_PASSWORD)
    activate(guest, SAVE)
    return created(guest, tab, before)


# ------------------------=
# FUNC: ui_rename
# DESC: Edits both public name fields without changing the stable account ID; later CLI resolution verifies the new handle.
# ------------------=
def ui_rename(guest, tab, identifier, handle, display):
    select(guest, tab, identifier)
    activate(guest, RENAME)
    wait_accounts(guest, lambda state: state[5] == EDIT)
    field(guest, 0, display, replace=True)
    field(guest, 1, handle, replace=True)
    activate(guest, SAVE)
    wait_accounts(guest, lambda state: state[5:7] == (BROWSE, SAVED)
                  and struct.pack("<2Q", *state[10:12]).hex() == identifier, timeout=600)


# ------------------------=
# FUNC: membership
# DESC: Checks one membership edge by immutable IDs rather than interpreting row labels or feedback prose.
# ------------------=
def membership(state, user, group):
    return user in directory(state)["groups"][group]["members"]


# ------------------------=
# FUNC: command
# DESC: Sends a real CLI command and returns refreshed Settings state for independent mutation verification.
# ------------------=
def command(guest, value):
    guest.launch("command", 5)
    guest.command(value)
    return open_accounts(guest)


# ------------------------=
# FUNC: checkpoint
# DESC: Saves non-secret structured identity evidence and a production screenshot for independent review.
# ------------------=
def checkpoint(guest, label):
    state = accounts(guest)
    result = {"directory": directory(state), "resolution": list(guest.state()[11:13])}
    (guest.work / f"{label}.json").write_text(json.dumps(result, indent=2) + "\n")
    guest.screenshot(label)
    return result["directory"]


# ------------------------=
# FUNC: reboot
# DESC: Starts a new media-detached emulator process and compares all account flags and membership edges after authentication.
# ------------------=
def reboot(guest, expected, label):
    guest.launch("command", 5)
    before = guest.state()
    guest.stop()
    guest.boot(False)
    after = guest.authenticate()
    assert after[16:20] == before[16:20]
    open_accounts(guest)
    assert checkpoint(guest, label) == expected


# ------------------------=
# FUNC: login_focus
# DESC: Reaches a documented login focus index through ordinary Tab input without reading credential fields.
# ------------------=
def login_focus(guest, target):
    state = guest.wait(lambda value: value[4] == 9, "account login surface")
    for _ in range(11):
        if state[8] == target:
            return state
        previous = state[8]
        guest.key("tab")
        state = guest.wait(lambda value: value[4] == 9 and value[8] != previous,
                           "login focus advanced", timeout=30)
    raise AssertionError(("unreachable login focus", target))


# ------------------------=
# FUNC: cold_login
# DESC: Selects a known public user slot on a detached cold boot and verifies actual login and self-service authority.
# ------------------=
def cold_login(guest, identifier, password, administrator, rejected_password=None):
    # Both the account directory and authentication selector enumerate user_nth.
    user_order = [entry[0] for entry in entries(accounts(guest), 0)]
    index = user_order.index(identifier)
    guest.launch("command", 5)
    node = guest.state()[16:20]
    guest.stop()
    guest.boot(False)
    state = guest.wait(lambda value: value[3] == 1 and value[4] == 9
                       and value[9] & 2 == 0, "unauthenticated cold login")
    assert state[16:20] == node
    state = login_focus(guest, 0)
    for _ in range(len(user_order)):
        assert state[7] < len(user_order)
        if state[7] == index:
            break
        previous = state[7]
        guest.key("down")
        state = guest.wait(lambda value: value[4] == 9 and value[8] == 0
                           and value[7] == (previous + 1) % len(user_order),
                           "selected next login account", timeout=30)
    assert state[7] == index
    login_focus(guest, 1)
    if rejected_password is not None:
        native.ui.text(guest, rejected_password)
        guest.key("ret")
        # A subsequent handled Tab is a completion barrier, not a timed absence
        # of success: successful sign-in gates input and already sets session flags.
        guest.key("tab")
        guest.wait(lambda value: value[4] == 9 and value[7] == index
                   and value[8] == 2 and value[9] & 2 == 0,
                   "old password rejected without a session", timeout=60)
        guest.key("shift", "tab")
        guest.wait(lambda value: value[4] == 9 and value[8] == 1,
                   "password field refocused", timeout=30)
    native.ui.text(guest, password)
    guest.key("ret")
    authenticated = guest.wait(lambda value: value[4] not in (9, 10)
                               and value[9] & 3 == 3, "selected account authenticated")
    assert authenticated[16:20] == node
    state = open_accounts(guest, administrator=administrator)
    assert [entry[0] for entry in entries(state, 0)] == user_order
    state = select(guest, 0, identifier)
    assert state[7] == int(administrator)
    # For a non-administrator, this control is enabled only for the actual actor.
    control(state, PASSWORD)
    return {"user": identifier, "selected_index": index, "administrator": administrator,
            "old_password_rejected": rejected_password is not None}


# ------------------------=
# FUNC: password_logins
# DESC: Proves first login, durable administrator reset, old-password denial and new-password login across cold boots.
# ------------------=
def password_logins(guest, expected, user, admin):
    initial = cold_login(guest, user, TEST_PASSWORD, administrator=False)
    assert checkpoint(guest, "accounts-new-user-login") == expected
    cold_login(guest, admin, ADMIN_PASSWORD, administrator=True)
    select(guest, 0, user)
    activate(guest, PASSWORD)
    wait_accounts(guest, lambda state: state[5] == SECRET)
    field(guest, 2, RESET_PASSWORD)
    field(guest, 3, RESET_PASSWORD)
    activate(guest, SAVE)
    wait_accounts(guest, lambda state: state[5:7] == (BROWSE, SAVED), timeout=600)
    assert directory(accounts(guest)) == expected
    reset = cold_login(guest, user, RESET_PASSWORD, administrator=False,
                       rejected_password=TEST_PASSWORD)
    assert checkpoint(guest, "accounts-reset-password-login") == expected
    cold_login(guest, admin, ADMIN_PASSWORD, administrator=True)
    assert directory(accounts(guest)) == expected
    return {"initial_login": initial, "reset_login": reset,
            "administrator_restored": True, "password_reset_persisted": True}


# ------------------------=
# FUNC: ui_delete
# DESC: Verifies default Enter cancels deletion, then explicitly confirms removal of the selected stable ID.
# ------------------=
def ui_delete(guest, tab, identifier):
    select(guest, tab, identifier)
    before = directory(accounts(guest))
    activate(guest, DELETE)
    wait_accounts(guest, lambda state: state[5] == REMOVE and state[13] == CANCEL)
    guest.key("ret")
    wait_accounts(guest, lambda state: state[5] == BROWSE)
    assert directory(accounts(guest)) == before
    activate(guest, DELETE)
    wait_accounts(guest, lambda state: state[5] == REMOVE)
    activate(guest, CONFIRM)
    key = "users" if tab == 0 else "groups"
    wait_accounts(guest, lambda state: state[5:7] == (BROWSE, DELETED)
                  and identifier not in directory(state)[key], timeout=600)


# ------------------------=
# FUNC: exercise
# DESC: Exercises CRUD, membership, cancellation, keyboard input, password login/reset and cold persistence.
# ------------------=
def exercise(guest):
    open_accounts(guest)
    baseline = checkpoint(guest, "accounts-baseline")
    assert len(baseline["users"]) == 1, "Fresh onboarding must create exactly one operator"
    admin = next(iter(baseline["users"]))
    activate(guest, USERS)
    activate(guest, NEW)
    wait_accounts(guest, lambda state: state[5] == CREATE)
    field(guest, 0, "Cancelled")
    guest.key("esc")
    wait_accounts(guest, lambda state: state[5] == BROWSE)
    assert directory(accounts(guest)) == baseline

    user = ui_create(guest, 0, "vmuser", "VM User")
    group = ui_create(guest, 1, "vmteam", "VM Team")
    state = select(guest, 1, group)
    user_index = [entry[0] for entry in entries(state, 0)].index(user)
    assert not membership(state, user, group)
    activate(guest, MEMBER, user_index)
    wait_accounts(guest, lambda state: membership(state, user, group))
    state = select(guest, 0, user)
    group_index = [entry[0] for entry in entries(state, 1)].index(group)
    activate(guest, MEMBER, group_index, keyboard=False)
    wait_accounts(guest, lambda state: not membership(state, user, group))
    activate(guest, MEMBER, group_index)
    wait_accounts(guest, lambda state: membership(state, user, group))
    for enabled in (False, True):
        activate(guest, ENABLED, keyboard=enabled)
        wait_accounts(guest, lambda state: bool(directory(state)["users"][user] & 1) == enabled)
    ui_rename(guest, 0, user, "vmrenamed", "VM Renamed")
    ui_rename(guest, 1, group, "vmrenamedteam", "VM Renamed Team")
    command(guest, "user disable user:vmrenamed")
    wait_accounts(guest, lambda state: directory(state)["users"][user] & 1 == 0)
    command(guest, "user enable user:vmrenamed")
    wait_accounts(guest, lambda state: directory(state)["users"][user] & 1 == 1)
    command(guest, "group remove group:vmrenamedteam user=user:vmrenamed")
    wait_accounts(guest, lambda state: not membership(state, user, group))
    command(guest, "group add group:vmrenamedteam user=user:vmrenamed")
    wait_accounts(guest, lambda state: membership(state, user, group))

    before = directory(accounts(guest))
    guest.launch("command", 5)
    guest.command("user create handle=vmcli display-name=CLIUser")
    for _ in range(2):
        native.ui.text(guest, TEST_PASSWORD)
        guest.key("ret")
        guest.wait(lambda state: state[71] == 1, "private account prompt consumed", timeout=600)
    open_accounts(guest)
    cli_user = created(guest, 0, before, require_feedback=False)
    before = directory(accounts(guest))
    command(guest, "group create name=vmcliteam display-name=CLITeam")
    cli_group = created(guest, 1, before, require_feedback=False)
    command(guest, "user update user:vmcli handle=vmclirenamed display-name=CLIRenamed")
    command(guest, "group update group:vmcliteam name=vmclirenamedteam display-name=CLIRenamedTeam")
    command(guest, "group add group:vmclirenamedteam user=user:vmclirenamed")
    wait_accounts(guest, lambda state: membership(state, cli_user, cli_group))
    expected = checkpoint(guest, "accounts-created")
    reboot(guest, expected, "accounts-restored")
    login_evidence = password_logins(guest, expected, user, admin)

    ui_delete(guest, 1, group)
    ui_delete(guest, 0, user)
    command(guest, "group remove group:vmclirenamedteam user=user:vmclirenamed")
    wait_accounts(guest, lambda state: not membership(state, cli_user, cli_group))
    command(guest, "user delete user:vmclirenamed")
    wait_accounts(guest, lambda state: cli_user not in directory(state)["users"])
    command(guest, "group delete group:vmclirenamedteam")
    wait_accounts(guest, lambda state: cli_group not in directory(state)["groups"])
    assert checkpoint(guest, "accounts-deleted") == baseline
    reboot(guest, baseline, "accounts-deletion-restored")
    return {"ui_user": user, "ui_group": group, "cli_user": cli_user, "cli_group": cli_group,
            "ui_and_cli_crud": True, "membership_both_views": True,
            "keyboard_forms_and_activation": True, "cancel_preserves_directory": True,
            "cold_boot_creation_and_deletion": True,
            "password_login": login_evidence,
            "limitations": ["Display-name bytes are not exposed by the public snapshot.",
                            "Reset revocation of another simultaneously active session is not exercised."]}


# ------------------------=
# FUNC: verify_disk
# DESC: Validates installed kernel bytes and referring records read-only while the disposable VM is stopped.
# ------------------=
def verify_disk(guest):
    assert guest.process is None
    result = subprocess.run([sys.executable, str(ROOT / "tools/update-installed-clone.py"),
        str(guest.disk), str(guest.work.parent / "artifacts/installed-kernel.elf"), "--verify-only"],
        check=True, capture_output=True, text=True, timeout=600)
    receipt = json.loads(result.stdout)
    assert receipt["installed_kernel_matches"] is True and receipt["references"] > 0
    return receipt


# ------------------------=
# FUNC: archive_media
# DESC: Copies the supplied installer to new run-local storage while hashing the exact archived bytes.
# ------------------=
def archive_media(source, destination):
    digest = hashlib.sha256()
    with source.open("rb") as incoming, destination.open("xb") as outgoing:
        while chunk := incoming.read(4 * 1024 * 1024):
            outgoing.write(chunk)
            digest.update(chunk)
    return digest.hexdigest()


# ------------------------=
# FUNC: main
# DESC: Creates one fresh install from explicit architecture-matching media and records detached account acceptance.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("work", type=Path)
    parser.add_argument("--arch", required=True, choices=("x86_64", "aarch64"))
    parser.add_argument("--iso", required=True, type=Path,
                        help="Repository-local installer; aarch64 requires QEMU-linked media")
    parser.add_argument("--accel", choices=("hvf", "tcg"))
    args = parser.parse_args()
    if not __debug__:
        parser.error("Assertions must remain enabled; do not use python -O")
    work, media = args.work.resolve(), args.iso.resolve()
    if not work.is_relative_to(ROOT / "build") or work == ROOT / "build" or work.exists():
        parser.error("work must be a nonexistent directory strictly inside build/")
    if not media.is_relative_to(ROOT) or not media.is_file():
        parser.error("--iso must be an existing repository-local file")
    if args.arch == "x86_64" and args.accel == "hvf":
        parser.error("The x86_64 helper supports TCG only")
    for executable in (f"qemu-system-{args.arch}", "xorriso", "mcopy"):
        if shutil.which(executable) is None:
            parser.error(f"Missing host prerequisite: {executable}")
    firmware = Path(f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd")
    if not firmware.is_file():
        parser.error(f"Missing firmware: {firmware}")
    work.mkdir(parents=True, exist_ok=False)
    artifacts = work / "artifacts"
    artifacts.mkdir()
    media_hash = archive_media(media, artifacts / "installer.iso")
    packaged = native.desktop.extract_media_kernels(artifacts / "installer.iso", artifacts, args.arch)
    for name in ("kernel.elf", "installed-kernel.elf"):
        for symbol, length in (("INFINITY_ACCOUNTS_DIAGNOSTIC_SNAPSHOT", 6144),
                               ("INFINITY_SETTINGS_DIAGNOSTIC_SNAPSHOT", 1024),
                               ("INFINITY_DIAGNOSTIC_SNAPSHOT", 4096)):
            assert native.base.symbol(artifacts / name, symbol)[1] == length
    guest = AccountsGuest(work, 1, str(firmware), width=1920, height=1080, memory_mb=12288)
    guest.arch = args.arch
    guest.acceleration = args.accel or ("hvf" if args.arch == "aarch64" else "tcg")
    guest.patched = True
    guest.detached_boots = []
    guest.boot_timeout_seconds = 600
    guest.install_timeout_seconds = 900
    try:
        guest.install()
        guest.stop()
        installed = verify_disk(guest)
        identity = guest.onboard()
        evidence = exercise(guest)
        guest.stop()
        final = verify_disk(guest)
        assert final == installed
        (work / "result.json").write_text(json.dumps({
            "architecture": args.arch, "acceleration": guest.acceleration,
            "fresh_install": True, "offline_patch": False,
            "source_media": str(media), "archived_media_sha256": media_hash,
            "packaged": packaged, "installed_kernel": final, "identity": identity,
            "detached_boots": guest.detached_boots, "accounts": evidence}, indent=2) + "\n")
        print("Installed accounts behavior passed", flush=True)
    finally:
        guest.stop()


if __name__ == "__main__":
    main()
