"""Generated account-key acceptance on unchanged fresh installed ARM media.

No full kernel byte comparisons. Native key objects are read from a stopped
disposable guest solely to supply standard OpenSSH client fixtures.
"""
import argparse
import base64
import importlib.util
import json
from pathlib import Path
import socket
import subprocess

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("ssh_acceptance", ROOT / "tools/ssh-installed-test.py")
ssh = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ssh)
accounts, native = ssh.accounts, ssh.native


# ------------------------=
# FUNC: export
# DESC: Reads only generated identity/key records from a stopped guest, then verifies OpenSSH derives the registered public key.
# ------------------=
def export(guest, work, handle):
    assert guest.process is None
    key = work / handle
    result = subprocess.run([str(ROOT / "build/cargo/debug/installed-pool-verify"), "--ssh-account-key", "--vm-stopped",
        str(guest.disk), handle, str(key)], capture_output=True, text=True, check=True, timeout=30)
    record = json.loads(result.stdout)
    assert record["registered"] is True
    output = subprocess.run(["/usr/bin/ssh-keygen", "-y", "-f", str(key)], capture_output=True, check=True, timeout=10)
    derived = base64.b64decode(output.stdout.split()[1], validate=True)
    saved = base64.b64decode(key.with_suffix(".pub").read_bytes().split()[1], validate=True)
    assert len(derived) == 51 and derived == saved
    assert key.stat().st_mode & 0o777 == 0o600
    return record


# ------------------------=
# FUNC: client
# DESC: Uses generated per-account keys with stock OpenSSH and strict host identity verification.
# ------------------=
def client(guest, work, handle, command, key=None):
    argv = ssh.client(work, guest.ssh_port, key=key or handle)
    argv[-1] = handle + "@127.0.0.1"
    return subprocess.run(argv + [command], capture_output=True, timeout=30)


# ------------------------=
# FUNC: create_in_settings
# DESC: Exercises visible Settings controls with actual pointer input, independently of the broader keyboard-traversal helper.
# ------------------=
def create_in_settings(guest):
    before = accounts.directory(accounts.accounts(guest))
    for target, argument, value in [(accounts.NEW, 0, None),
            (accounts.FIELD, 0, "SSH Settings"), (accounts.FIELD, 1, "sshsettings"),
            (accounts.FIELD, 2, accounts.TEST_PASSWORD), (accounts.FIELD, 3, accounts.TEST_PASSWORD),
            (accounts.SAVE, 0, None)]:
        for _ in range(8):
            state = accounts.accounts(guest)
            _, _, x, y, width, height, _, visible = accounts.control(state, target, argument)
            if visible == 1:
                break
            # Target 18 is the existing detail-pane Down control. Reveal the
            # complete field through ordinary UI scrolling before clicking it.
            _, _, sx, sy, sw, sh, _, shown = accounts.control(state, 18)
            assert shown == 1
            native.adaptive_click(guest, sx + sw // 2, sy + sh // 2)
        assert visible == 1 and width > 0 and height > 0
        native.adaptive_click(guest, x + width // 2, y + height // 2)
        if target == accounts.NEW:
            accounts.wait_accounts(guest, lambda s: s[5] == accounts.CREATE)
        if value is not None:
            accounts.wait_accounts(guest, lambda s: s[13:15] == (target, argument))
            native.ui.text(guest, value)
    return accounts.created(guest, 0, before)


# ------------------------=
# FUNC: main
# DESC: Installs immutable media, creates users through onboarding, Settings and Console, and authenticates with their automatically provisioned keys.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("work", type=Path)
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--resume", action="store_true", help="Resume only this harness's unchanged freshly installed disk")
    args = parser.parse_args()
    work, media = args.work.resolve(), args.iso.resolve()
    assert work.exists() == args.resume and work.is_relative_to(ROOT / "build") and work != ROOT / "build"
    assert media.is_file() and media.is_relative_to(ROOT)
    artifacts = work / "artifacts"
    if args.resume:
        previous = json.loads((work / "result.json").read_text())
        assert previous["architecture"] == "aarch64" and previous["detached_boots"] and not previous["passed"]
        packaged = previous["packaged"]
    else:
        artifacts.mkdir(parents=True)
        subprocess.run(["cp", "-c", str(media), str(artifacts / "installer.iso")], check=True)
        packaged = native.desktop.extract_media_kernels(artifacts / "installer.iso", artifacts, "aarch64")
    guest = ssh.Guest(work, 1, "/opt/homebrew/share/qemu/edk2-aarch64-code.fd", reuse=args.resume, width=1920, height=1080, memory_mb=12288)
    guest.arch, guest.acceleration = "aarch64", "hvf"
    guest.patched, guest.detached_boots = True, []
    guest.boot_timeout_seconds, guest.install_timeout_seconds = 600, 1800
    with socket.socket() as reserve:
        reserve.bind(("127.0.0.1", 0))
        guest.ssh_port = reserve.getsockname()[1]
    result = {"passed": False, "architecture": "aarch64", "media": str(media), "packaged": packaged,
              "full_kernel_comparison": False}
    try:
        if args.resume:
            guest.boot(False)
            guest.authenticate()
        else:
            guest.install()
            guest.stop()
            guest.onboard()
        initial = ssh.wait_state(guest, lambda s: s[3:5] == (1, 1) and s[13] == 4)
        host, known_hosts = ssh.keyscan(guest.ssh_port)
        (work / "known_hosts").write_bytes(known_hosts)
        accounts.open_accounts(guest)
        assert len(accounts.directory(accounts.accounts(guest))["users"]) == 1
        ui_id = create_in_settings(guest)
        before = accounts.directory(accounts.accounts(guest))
        guest.launch("command", 5)
        guest.command("user create handle=sshconsole display-name=SSHConsole")
        for _ in range(2):
            native.ui.text(guest, accounts.TEST_PASSWORD)
            guest.key("ret")
            guest.wait(lambda state: state[71] == 1, "account prompt consumed", timeout=600)
        accounts.open_accounts(guest)
        cli_id = accounts.created(guest, 0, before, require_feedback=False)
        assert ui_id != cli_id
        guest.screenshot("generated-key-accounts")
        guest.stop()
        handles = ("ms9node01", "sshsettings", "sshconsole")
        keys = {handle: export(guest, work, handle) for handle in handles}
        assert len({item["fingerprint"] for item in keys.values()}) == len(handles)
        guest.boot(False)
        guest.authenticate()
        ssh.wait_state(guest, lambda s: s[3:5] == (1, 1))
        reboot_host, _ = ssh.keyscan(guest.ssh_port)
        assert reboot_host == host
        receipts = []
        for handle in handles:
            for command in ("help", "system status", "node list"):
                response = client(guest, work, handle, command)
                assert response.returncode == 0 and response.stdout and not response.stderr
                receipts.append({"user": handle, "command": command, "status": response.returncode,
                                 "stdout_bytes": len(response.stdout), "stderr_bytes": len(response.stderr)})
            denied = client(guest, work, handle, "voice status")
            assert denied.returncode == 126 and denied.stderr
        wrong = client(guest, work, "sshsettings", "help", key="sshconsole")
        assert wrong.returncode == 255
        ssh.wait_state(guest, lambda s: s[6] == 0)
        ssh.command(guest, "ssh disable")
        ssh.wait_state(guest, lambda s: s[3:5] == (0, 0) and s[13:15] == (0, 0))
        guest.stop()
        guest.boot(False)
        guest.authenticate()
        disabled = ssh.state(guest)
        assert disabled[3:5] == (0, 0) and disabled[13:15] == (0, 0)
        result.update(passed=True, initial=initial, keys=keys, requests=receipts,
            cross_user_authentication_rejected=True, saved_disable_survives_reboot=True,
            host_key_persistent=True, disabled=disabled)
    except BaseException as error:
        result["failure"] = repr(error)
        if guest.process is not None:
            guest.screenshot("account-enrollment-failure")
            result["accounts_snapshot"] = accounts.accounts(guest)
        raise
    finally:
        guest.stop()
        result["detached_boots"] = guest.detached_boots
        (work / ("resume-result.json" if args.resume else "result.json")).write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
