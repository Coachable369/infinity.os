"""Generated account-key acceptance on unchanged fresh installed ARM or x86 media.

No full kernel byte comparisons. Native key objects are read from a stopped
disposable guest solely to supply standard OpenSSH client fixtures.
"""
import argparse
import base64
import importlib.util
import hashlib
import json
import os
from pathlib import Path
import select
import socket
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("ssh_acceptance", ROOT / "tools/ssh-installed-test.py")
ssh = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ssh)
accounts, native = ssh.accounts, ssh.native
automation_spec = importlib.util.spec_from_file_location("ssh_automation_acceptance", ROOT / "tools/automation-installed-test.py")
automation = importlib.util.module_from_spec(automation_spec)
automation_spec.loader.exec_module(automation)


# ------------------------=
# FUNC: prepare_helpers
# DESC: Builds the native read-only artifact verifiers inside the active build-kit run instead of relying on stale host binaries.
# ------------------=
def prepare_helpers(artifacts):
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    (ROOT / "build/tools").mkdir(parents=True, exist_ok=True)
    with (artifacts / "host-verifiers.log").open("wb") as log:
        subprocess.run(["rustc", "--edition=2021", "-A", "warnings", str(ROOT / "tools/installed-object-test.rs"),
                        "-o", str(ROOT / "build/tools/installed-object-test")], check=True, stdout=log, stderr=subprocess.STDOUT)
        subprocess.run(["cargo", "build", "--quiet", "--manifest-path", str(ROOT / "tools/behavior-harness/Cargo.toml"),
                        "--bin", "installed-pool-verify"], check=True, stdout=log, stderr=subprocess.STDOUT,
                       env={**os.environ, "CARGO_TARGET_DIR": str(ROOT / "build/cargo")})


# ------------------------=
# FUNC: export
# DESC: Reads only generated identity/key records from a stopped guest, then verifies OpenSSH derives the registered public key.
# ------------------=
def export(guest, work, handle):
    assert guest.process is None
    key = work / handle
    exported = key if not key.exists() else work / (handle + "-verify-" + str(time.time_ns()))
    result = subprocess.run([str(ROOT / "build/cargo/debug/installed-pool-verify"), "--ssh-account-key", "--vm-stopped",
        str(guest.disk), handle, str(exported)], capture_output=True, text=True, check=True, timeout=30)
    record = json.loads(result.stdout)
    assert record["registered"] is True
    if exported != key:
        try:
            assert exported.read_bytes() == key.read_bytes()
            assert exported.with_suffix(".pub").read_bytes() == key.with_suffix(".pub").read_bytes()
        finally:
            exported.unlink()
            exported.with_suffix(".pub").unlink()
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
    result = subprocess.run(argv + [command], capture_output=True, timeout=30)
    if result.returncode != 255:
        stream = result.stderr if result.stderr else result.stdout
        if not stream.endswith((b"\n\n", b"\r\n\r\n")):
            guest.output_delimiter_failures.append(dict(user=handle, command=command,
                exit_status=result.returncode, stdout_bytes=len(result.stdout), stderr_bytes=len(result.stderr)))
        if command == "list /System":
            (work / (handle + "-system-list.txt")).write_bytes(result.stdout)
    return result


# ------------------------=
# FUNC: account_client
# DESC: Builds an isolated non-PTY OpenSSH channel for binary streams and application-owned keyboard input.
# ------------------=
def account_client(guest, work, handle, command):
    argv = ssh.client(work, guest.ssh_port, key=handle, extra=("-T",))
    argv[-1] = handle + "@127.0.0.1"
    return argv + [command]


# ------------------------=
# FUNC: edit_source
# DESC: Exercises the native remote editor through ANSI mode transitions and real save/exit control bytes.
# ------------------=
def edit_source(guest, work, path, source, label):
    process = subprocess.Popen(account_client(guest, work, "ms9node01", "edit " + path),
                               stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    received = bytearray()
    transcript = work / (label + ".bin")
    try:
        deadline = time.monotonic() + 30
        while b"\x1b[?1049h" not in received:
            assert process.poll() is None, {"editor_exit": process.returncode}
            assert time.monotonic() < deadline, {"editor_mode_bytes": len(received)}
            if select.select([process.stdout], [], [], .1)[0]:
                chunk = os.read(process.stdout.fileno(), 16384)
                assert chunk
                received.extend(chunk)
                assert len(received) <= 65536
        # The transport must deliver Ctrl-A, source bytes, save and exit in
        # order even when SSH stdin EOF immediately follows the same packet.
        payload = b"\x01" + source + b"\x13\x18"
        assert len(payload) < 512
        stdout, stderr = process.communicate(payload, timeout=120 if guest.arch == "x86_64" else 45)
        received.extend(stdout)
        assert len(received) <= 2_000_000
        assert process.returncode == 0 and not stderr, {"exit": process.returncode, "stderr": stderr}
        assert b"\x1b[?1049l" in received and b"\x1b[?25h" in received
        transcript.write_bytes(received)
        return {"exit_status": process.returncode, "alternate_screen_entered": True,
                "alternate_screen_left": True, "protocol_bytes": len(received),
                "transcript": str(transcript), "sha256": hashlib.sha256(received).hexdigest()}
    finally:
        transcript.write_bytes(received)
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)
        for stream in (process.stdin, process.stdout, process.stderr):
            stream.close()


# ------------------------=
# FUNC: remote_python_and_editor
# DESC: Verifies native remote source creation/revision, two piped input lines, EOF, binary streams and actual process status.
# ------------------=
def remote_python_and_editor(guest, work):
    path = automation.DOCS + "ssh-input-" + str(time.time_ns()) + ".py"
    source = (b"import sys\na=int(input())\nb=int(input())\n"
              b"assert sys.stdin.read()==''\n"
              b"sys.stdout.buffer.write(bytes([0,255,a,b,a+b,0]))\n"
              b"sys.stderr.buffer.write(bytes([0,254,b,a,0]))\n"
              b"sys.exit(a+b)\n")
    original = source.replace(b"sys.exit(a+b)", b"sys.exit(a+b-1)")
    created = edit_source(guest, work, path, original, "ssh-editor-create")
    edited = edit_source(guest, work, path, source, "ssh-editor-save")
    before = native.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10)
    response = subprocess.run(account_client(guest, work, "ms9node01", "python " + path),
                              input=b"37\n5\n", capture_output=True, timeout=300 if guest.arch == "x86_64" else 90)
    (work / "ssh-python-stdout.bin").write_bytes(response.stdout)
    (work / "ssh-python-stderr.bin").write_bytes(response.stderr)
    assert response.returncode == 42, {"exit": response.returncode, "stderr": response.stderr}
    # These are binary application protocol frames, not diagnostic-copy checks.
    frames = response.stdout.split(b"\0")
    assert len(frames) == 3 and frames[1] == bytes([255, 37, 5, 42])
    assert response.stderr.split(b"\0") == [b"", bytes([254, 5, 37]), b""]
    finished = native.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10)
    assert finished[:2] == (0x494e46505954484e, 1) and finished[2] > before[2]
    assert finished[3:7] == (2, 42, 6, 5), finished
    return {"path": path, "create": created, "edit": edited, "native_process": finished,
            "exit_status": response.returncode, "stdin_lines": 2, "stdin_eof": True,
            "binary_stdout": [0, 255, 37, 5, 42, 0], "binary_stderr": [0, 254, 5, 37, 0]}, (original, source)


# ------------------------=
# FUNC: verify_saved_source
# DESC: Matches both actual editor revisions against read-only native disk extraction after stopping the disposable guest.
# ------------------=
def verify_saved_source(guest, evidence, expected):
    assert guest.process is None
    exported = native.export_generated_artifacts(guest, None, guest.arch)
    rows = [item for item in exported["artifacts"]
            if bytes.fromhex(item["path_hex"]).decode("ascii") == evidence["path"]]
    assert len(rows) == 2 and sum(bool(item["current"]) for item in rows) == 1
    rows.sort(key=lambda item: item["version"])
    for row, source in zip(rows, expected):
        content = (Path(exported["directory"]) / row["file"]).read_bytes()
        assert content == source and row["bytes"] == len(source)
    assert rows[0]["object"] == rows[1]["object"] and rows[0]["version"] < rows[1]["version"]
    assert rows[-1]["current"]
    return {"directory": exported["directory"], "revisions": rows, "exact_bytes_verified": True}


# ------------------------=
# FUNC: remote_password_input
# DESC: Creates an account through private exec stdin and confirms a committed identity without accepting password echo or command prose.
# ------------------=
def remote_password_input(guest, work):
    accounts.open_accounts(guest)
    before = accounts.directory(accounts.accounts(guest))
    assert len(before["users"]) == 3
    password = accounts.TEST_PASSWORD.encode("ascii")
    created = []
    for index in range(5):
        handle = "sshpipe" if index == 0 else "sshpipe" + str(index)
        response = subprocess.run(account_client(guest, work, "ms9node01",
                                  "user create handle=" + handle + " display-name=" + handle),
                                  input=password + b"\n" + password + b"\n", capture_output=True,
                                  timeout=120 if guest.arch == "x86_64" else 45)
        assert password not in response.stdout and password not in response.stderr
        (work / (handle + "-create-stdout.txt")).write_bytes(response.stdout)
        (work / (handle + "-create-stderr.txt")).write_bytes(response.stderr)
        assert response.returncode == 0 and not response.stderr, (handle, response.returncode, response.stderr)
        accounts.open_accounts(guest)
        identifier = accounts.created(guest, 0, before, require_feedback=False)
        created.append(dict(handle=handle, user=identifier))
        before = accounts.directory(accounts.accounts(guest))
    assert len(before["users"]) == 8
    return {"passed": True, "created": created, "supported_accounts": 8, "exit_status": 0,
            "password_echo_bytes": 0, "private_piped_confirmation": True}


# ------------------------=
# FUNC: remote_ai_detached
# DESC: Requires private AI exec completion while the desktop is logged out and verifies no reply survives into another authenticated session.
# ------------------=
def remote_ai_detached(guest, work):
    login = guest.wait(lambda state: state[3] == 1 and state[4] in (9, 10), "logged-out remote AI")
    ssh.wait_state(guest, lambda state: state[3:5] == (1, 1))
    handles = ("sshsettings", "sshconsole")
    empty = {}
    for handle in handles:
        response = client(guest, work, handle, "ai response")
        assert response.returncode == 0 and not response.stderr and response.stdout
        empty[handle] = response.stdout
    before = ssh.console_state(guest)
    response = subprocess.run(account_client(guest, work, handles[0],
        "ai ask Explain why backups should be tested in one short sentence."), capture_output=True,
        timeout=900 if guest.arch == "x86_64" else 180)
    assert response.returncode == 0 and not response.stderr and response.stdout, (response.returncode, response.stderr)
    assert len(response.stdout) > len(empty[handles[0]])
    (work / "ssh-private-ai-response.bin").write_bytes(response.stdout)
    ssh.wait_state(guest, lambda state: state[6] == 0)
    completed = ssh.console_state(guest)
    assert completed[3] == before[3] + 1 and completed[7] == 0
    assert guest.state()[4] in (9, 10) and guest.state()[9] & 2 == 0
    for handle in handles:
        after = client(guest, work, handle, "ai response")
        assert after.returncode == 0 and not after.stderr and after.stdout == empty[handle]
    return {"request_user": handles[0], "desktop_logged_out": True,
            "exit_status": response.returncode, "channel_stdout_bytes": len(response.stdout),
            "channel_sha256": hashlib.sha256(response.stdout).hexdigest(),
            "new_session_reply_erased": True, "other_user_reply_isolated": True,
            "native_token_counters": "Covered by the separate real-native remote-advice harness",
            "login_mode": login[4], "completion_count": completed[3]}


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
# FUNC: remote_created
# DESC: Creates through stock SSH and resolves the committed stable identifier from native catalog memory.
# ------------------=
def remote_created(guest, work, command, kind):
    before = set(automation.wait(guest, lambda _: True, "remote catalog ready")["rows"])
    response = client(guest, work, "ms9node01", command)
    assert response.returncode == 0 and not response.stderr, (response.returncode, response.stderr)
    current = automation.wait(guest, lambda s: len(set(s["rows"]) - before) == 1, "SSH-created catalog row")
    identifier = (set(current["rows"]) - before).pop()
    assert current["rows"][identifier]["kind"] == kind
    return identifier


# ------------------------=
# FUNC: remote_management
# DESC: Verifies same-user remote CRUD and native execution while ordinary accounts cannot mutate shared jobs.
# ------------------=
def remote_management(guest, work):
    script = automation.HELLO
    service = remote_created(guest, work, f"service create sshworker {script} start=manual enabled=false", 0)
    cron = remote_created(guest, work, f'taskscheduler create sshcron {script} cron="* * * * *" enabled=false', 1)
    assert automation.snapshot(guest)["rows"][cron]["next"] % 60 == 0
    for command in ("service edit sshworker restart=on-failure", "service run sshworker"):
        response = client(guest, work, "ms9node01", command)
        assert response.returncode == 0 and not response.stderr, (command, response.returncode, response.stderr)
    completion = automation.completed(guest, service, 1)
    for handle in ("sshsettings", "sshconsole"):
        before = automation.snapshot(guest)
        for command in (f"service create denied {script}", "service start sshworker",
                        f'taskscheduler create denied {script} cron="*/5 * * * *"',
                        "taskscheduler enable sshcron", "service logs sshworker"):
            response = client(guest, work, handle, command)
            assert response.returncode == 126, (handle, command, response.returncode)
            after = automation.wait(guest, lambda s: s["revision"] >= before["revision"], "denied remote command settled")
            assert after["revision"] == before["revision"] and after["rows"] == before["rows"]
    guest.launch("task manager", 5)
    guest.key("tab")
    guest.key("tab")
    guest.key("ret")
    view = automation.wait_ui(guest, 1, 0)
    assert view[6] == 1
    service_screen = str(guest.screenshot("ssh-created-local-service"))
    guest.key("tab")
    guest.key("ret")
    view = automation.wait_ui(guest, 2, 0)
    assert view[6] == 1
    cron_screen = str(guest.screenshot("ssh-created-local-cron"))
    response = client(guest, work, "ms9node01", "service delete sshworker")
    assert response.returncode == 0
    automation.wait(guest, lambda s: service not in s["rows"], "remote daemon deletion")
    saved = automation.snapshot(guest)["rows"][cron]
    guest.stop()
    guest.boot(False)
    guest.authenticate()
    ssh.wait_state(guest, lambda s: s[3:5] == (1, 1))
    restored = automation.wait(guest, lambda s: cron in s["rows"], "cron cold reboot")["rows"][cron]
    for field in ("id", "kind", "runs", "next", "enabled"):
        assert restored[field] == saved[field], (field, restored, saved)
    assert client(guest, work, "ms9node01", "taskscheduler enable sshcron").returncode == 0
    enabled = automation.wait(guest, lambda s: s["rows"][cron]["enabled"], "remote cron enabled")["rows"][cron]
    assert enabled["next"] % 60 == 0
    completed = automation.completed(guest, cron, restored["runs"] + 1)
    assert client(guest, work, "ms9node01", "taskscheduler disable sshcron").returncode == 0
    automation.wait(guest, lambda s: not s["rows"][cron]["enabled"], "remote cron disabled")
    assert client(guest, work, "ms9node01", "taskscheduler delete sshcron").returncode == 0
    automation.wait(guest, lambda s: cron not in s["rows"], "remote cron deletion")
    return dict(service_id=service, cron_id=cron, service_completion=completion,
                cron_completion=completed, cron_reboot=restored, ordinary_mutations_denied=True,
                screenshots=[service_screen, cron_screen])


# ------------------------=
# FUNC: main
# DESC: Installs immutable media, creates users through onboarding, Settings and Console, and authenticates with their automatically provisioned keys.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("work", type=Path)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), default="aarch64")
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--resume", action="store_true", help="Resume only this harness's unchanged freshly installed disk")
    args = parser.parse_args()
    work, media = args.work.resolve(), args.iso.resolve()
    assert work.exists() == args.resume and work.is_relative_to(ROOT / "build") and work != ROOT / "build"
    assert media.is_file() and media.is_relative_to(ROOT)
    artifacts = work / "artifacts"
    if args.resume:
        previous = json.loads((work / "result.json").read_text())
        assert previous["architecture"] == args.arch and previous["detached_boots"] and not previous["passed"]
        packaged = previous["packaged"]
    else:
        artifacts.mkdir(parents=True)
        subprocess.run(["cp", "-c", str(media), str(artifacts / "installer.iso")], check=True)
        packaged = native.desktop.extract_media_kernels(artifacts / "installer.iso", artifacts, args.arch)
    prepare_helpers(artifacts)
    guest = ssh.Guest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd", reuse=args.resume,
                      width=1920, height=1080, memory_mb=20480)
    guest.arch, guest.acceleration = args.arch, "hvf" if args.arch == "aarch64" else "tcg"
    guest.patched, guest.detached_boots = True, []
    guest.output_delimiter_failures = []
    guest.boot_timeout_seconds = 1200 if args.arch == "x86_64" else 600
    guest.install_timeout_seconds = 3600 if args.arch == "x86_64" else 1800
    with socket.socket() as reserve:
        reserve.bind(("127.0.0.1", 0))
        guest.ssh_port = reserve.getsockname()[1]
    result = {"passed": False, "architecture": args.arch, "acceleration": guest.acceleration,
              "memory_mib": guest.memory_mb, "media": str(media), "packaged": packaged,
              "full_kernel_comparison": False, "fresh_install": not args.resume,
              "reused_private_disk": args.resume}
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
        ui_id, cli_id = None, None
        current = accounts.directory(accounts.accounts(guest))
        if args.resume and len(current["users"]) in (2, 3):
            prior = previous["accounts_snapshot"]
            prior_directory = accounts.directory(prior)
            assert current["groups"] == prior_directory["groups"]
            assert all(current["users"].get(identifier) == flags for identifier, flags in prior_directory["users"].items())
            ordinary = [identifier for identifier, flags in prior_directory["users"].items() if flags == 1]
            assert len(ordinary) == 1 and prior[5] == accounts.BROWSE
            ui_id = ordinary[0]
            result["settings_creation_prior_evidence"] = str(work / "result.json")
            added = set(current["users"]) - set(prior_directory["users"])
            if added:
                assert len(added) == 1
                cli_id = added.pop()
                assert current["users"][cli_id] == 1
                result["resumed_existing_cli_fixture"] = cli_id
        if ui_id is None:
            assert len(accounts.directory(accounts.accounts(guest))["users"]) == 1
            ui_id = create_in_settings(guest)
        if cli_id is None:
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
        result["keys"] = keys
        guest.boot(False)
        guest.authenticate()
        ssh.wait_state(guest, lambda s: s[3:5] == (1, 1))
        reboot_host, _ = ssh.keyscan(guest.ssh_port)
        assert reboot_host == host
        receipts = []
        result["requests"] = receipts
        for handle in handles:
            for command in ("help taskscheduler", "system status", "node list", "list /System", "taskscheduler list", "voice status"):
                response = client(guest, work, handle, command)
                expected = 126 if command == "node list" and handle != "ms9node01" else 0
                assert response.returncode == expected, (handle, command, response.returncode)
                assert bool(response.stderr) == bool(expected) and (response.stdout or response.stderr)
                receipts.append({"user": handle, "command": command, "status": response.returncode,
                                 "stdout_bytes": len(response.stdout), "stderr_bytes": len(response.stderr)})
        management = remote_management(guest, work)
        result["remote_management"] = management
        try:
            result["private_password_input"] = remote_password_input(guest, work)
        except AssertionError as error:
            result["private_password_input"] = dict(passed=False, failure=repr(error))
        execution, sources = remote_python_and_editor(guest, work)
        result["remote_editor_python"] = execution
        guest.stop()
        if result["private_password_input"]["passed"]:
            for account in result["private_password_input"]["created"]:
                keys[account["handle"]] = export(guest, work, account["handle"])
            assert len(keys) == 8 and len({item["fingerprint"] for item in keys.values()}) == 8
        execution["stored_source"] = verify_saved_source(guest, execution, sources)
        guest.boot(False)
        result["private_remote_ai"] = remote_ai_detached(guest, work)
        guest.authenticate()
        wrong = client(guest, work, "sshsettings", "help", key="sshconsole")
        assert wrong.returncode == 255
        public = (work / "sshconsole.pub").read_text().split()
        revoke = client(guest, work, "ms9node01", f"ssh key revoke sshconsole {public[0]} {public[1]}")
        assert revoke.returncode == 0
        assert client(guest, work, "sshconsole", "system status").returncode == 255
        ssh.wait_state(guest, lambda s: s[6] == 0)
        ssh.command(guest, "ssh disable")
        ssh.wait_state(guest, lambda s: s[3:5] == (0, 0) and s[13:15] == (0, 0))
        guest.stop()
        guest.boot(False)
        guest.authenticate()
        disabled = ssh.state(guest)
        assert disabled[3:5] == (0, 0) and disabled[13:15] == (0, 0)
        assert result["private_password_input"]["passed"], result["private_password_input"]
        assert not guest.output_delimiter_failures, guest.output_delimiter_failures
        result.update(passed=True, initial=initial, keys=keys, requests=receipts,
            remote_management=management, revoked_key_rejected=True,
            blank_line_command_delimiters=True,
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
        result["output_delimiter_failures"] = guest.output_delimiter_failures
        result["detached_boots"] = guest.detached_boots
        (work / ("resume-result.json" if args.resume else "result.json")).write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
