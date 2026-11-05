"""SSH transport acceptance against a fresh installed guest and macOS OpenSSH.

Run only through build-kit. Installs unchanged media; never patches guest memory,
keys, identities, or installed executables. Provisioning uses the local Console.
"""
import argparse
import array
import base64
import importlib.util
import fcntl
import os
import pty
import select
import signal
import struct
import hashlib
import threading
import termios
import json
from pathlib import Path
import shutil
import socket
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("ssh_accounts", ROOT / "tools/accounts-installed-test.py")
accounts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accounts)
native = accounts.native


class Guest(accounts.AccountsGuest):
    # ------------------------=
    # FUNC: boot
    # DESC: Attests ISO-detached boot and exposes the guest listener only on host loopback through QEMU's NIC.
    # ------------------=
    def boot(self, installer):
        super().boot(installer)
        if not installer:
            self.qmp("human-monitor-command", {"command-line": f"hostfwd_add tcp:127.0.0.1:{self.ssh_port}-:22"})


# ------------------------=
# FUNC: state
# DESC: Reads coherent typed service counters without treating CLI output or log prose as acceptance evidence.
# ------------------=
def state(guest):
    result = native.read(guest, "INFINITY_SSH_DIAGNOSTIC_SNAPSHOT", 20)
    assert result[:2] == (0x494e465353483031, 1)
    assert result[6] <= result[12] <= 4 and result[13] <= 4
    return result


# ------------------------=
# FUNC: wait_state
# DESC: Waits for explicit SSH state transitions under a bounded deadline.
# ------------------=
def wait_state(guest, predicate, timeout=40):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        current = state(guest)
        if predicate(current):
            return current
        time.sleep(.1)
    guest.screenshot("ssh-failure")
    raise AssertionError({"ssh_snapshot": current})


# ------------------------=
# FUNC: command
# DESC: Sends local provisioning through the production Console and real keyboard events.
# ------------------=
def command(guest, value):
    guest.launch("command", 5)
    guest.command(value)


# ------------------------=
# FUNC: keyscan
# DESC: Decodes the peer's actual SSH public wire key and retains a known_hosts record for strict client verification.
# ------------------=
def keyscan(port):
    result = subprocess.run(["/usr/bin/ssh-keyscan", "-T", "10", "-t", "ed25519", "-p", str(port), "127.0.0.1"],
                            capture_output=True, timeout=15, check=True)
    records = [line.split() for line in result.stdout.splitlines() if line and not line.startswith(b"#")]
    assert len(records) == 1 and len(records[0]) == 3
    blob = base64.b64decode(records[0][2], validate=True)
    assert len(blob) == 51 and blob[:19] == b"\0\0\0\x0bssh-ed25519\0\0\0\x20"
    return blob, result.stdout


# ------------------------=
# FUNC: client
# DESC: Runs standard macOS OpenSSH with an isolated key and strict host-key checking.
# ------------------=
def client(work, port, key="client", extra=()):
    return ["/usr/bin/ssh", "-F", "/dev/null", "-E", str(work / "openssh-diagnostics.log"), "-p", str(port), "-i", str(work / key),
            "-oBatchMode=yes", "-oIdentitiesOnly=yes", "-oConnectTimeout=6", "-oConnectionAttempts=1",
            "-oStrictHostKeyChecking=yes", "-oUserKnownHostsFile=" + str(work / "known_hosts"),
            *extra, "ms9node01@127.0.0.1"]


# ------------------------=
# FUNC: master
# DESC: Requires OpenSSH's authenticated multiplex control socket instead of interpreting its debug messages.
# ------------------=
def master(work, port, number):
    control = work / f"control-{number}"
    log = (work / f"client-{number}.log").open("wb")
    process = subprocess.Popen(client(work, port, extra=("-N", "-M", "-S", str(control))), stdout=log, stderr=log)
    try:
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise AssertionError({"client_exit": process.returncode})
            if control.exists():
                check = subprocess.run(client(work, port, extra=("-S", str(control), "-O", "check")), capture_output=True, timeout=5)
                if check.returncode == 0:
                    return process, log, control
            time.sleep(.1)
        raise AssertionError("OpenSSH authenticated control connection timed out")
    except BaseException:
        process.terminate()
        process.wait(timeout=10)
        log.close()
        raise


# ------------------------=
# FUNC: disconnect
# DESC: Requests a clean OpenSSH transport shutdown and reaps its host process.
# ------------------=
def disconnect(work, port, handle):
    process, log, control = handle
    try:
        subprocess.run(client(work, port, extra=("-S", str(control), "-O", "check")), capture_output=True, timeout=5, check=True)
        assert process.poll() is None, {"premature_master_exit": process.returncode}
        # OpenSSH clientloop.c sends SSH_MSG_DISCONNECT and returns zero for
        # deliberate SIGTERM of SessionType=none. A multiplex -O exit has no
        # remote command status and can instead return 255.
        process.terminate()
        code = process.wait(timeout=10)
        assert code == 0, {"master_disconnect_exit": code}
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=10)
        log.close()


# ------------------------=
# FUNC: exercise
# DESC: Verifies default-on public-key-only access, authentication, AEAD policy, limits, cleanup and reboot persistence.
# ------------------=
def exercise(guest, work):
    port = guest.ssh_port
    initial = wait_state(guest, lambda s: s[3:5] == (1, 1) and s[6] == 0 and s[13] == 4)
    for name in ("client", "unregistered"):
        subprocess.run(["/usr/bin/ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(work / name)], check=True, timeout=10)
    original_key, known_hosts = keyscan(port)
    (work / "known_hosts").write_bytes(known_hosts)
    wait_state(guest, lambda s: s[6] == 0)
    rejected = subprocess.run(client(work, port, extra=("-N",)), capture_output=True, timeout=20)
    assert rejected.returncode == 255
    public = (work / "client.pub").read_text().split()
    command(guest, "ssh key add ms9node01 " + " ".join(public[:2]))
    wait_state(guest, lambda s: s[6] == 0)
    first = master(work, port, 1)
    second = None
    try:
        wait_state(guest, lambda s: s[6] == 1 and s[8] >= 1)
        denied = subprocess.run(client(work, port, key="unregistered", extra=("-N",)), capture_output=True, timeout=20)
        assert denied.returncode == 255
        wait_state(guest, lambda s: s[6] == 1)
        for cipher in ("aes256-ctr", "aes128-ctr"):
            rejected = subprocess.run(client(work, port, extra=("-N", "-c", cipher)), capture_output=True, timeout=20)
            assert rejected.returncode == 255
            wait_state(guest, lambda s: s[6] == 1)
        channel = subprocess.run(client(work, port, extra=("-S", str(first[2]), "-s")) + ["sftp"], capture_output=True, timeout=10)
        assert channel.returncode == 255
        wait_state(guest, lambda s: s[9] >= 2)
        second = master(work, port, 2)
        full = wait_state(guest, lambda s: s[6] == 2 and s[8] >= 2)
        overflow = subprocess.run(client(work, port, extra=("-N",)), capture_output=True, timeout=15)
        assert overflow.returncode == 255
        assert state(guest)[6] == 2
    finally:
        try:
            if second is not None:
                disconnect(work, port, second)
        finally:
            disconnect(work, port, first)
    baseline = wait_state(guest, lambda s: s[6] == 0 and s[14] == 2)
    with socket.create_connection(("127.0.0.1", port), timeout=6) as silent:
        waiting = wait_state(guest, lambda s: s[6] == 1)
        before = guest.state()
        started = time.monotonic()
        guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {"axis": "x", "value": -4}}]})
        guest.wait(lambda s: s[13] != before[13], "Pointer responds during incomplete SSH handshake", timeout=5)
        latency = time.monotonic() - started
        wait_state(guest, lambda s: s[6] == 0 and s[10] > waiting[10], timeout=40)
        silent.settimeout(5)
        # The server may have sent its banner before expiring the handshake.
        received = 0
        while True:
            try:
                chunk = silent.recv(4096)
            except ConnectionResetError:
                break
            if not chunk:
                break
            received += len(chunk)
            assert received <= 35000
    baseline = wait_state(guest, lambda s: s[6] == 0 and s[14] == 2)
    for index in range(20):
        with socket.create_connection(("127.0.0.1", port), timeout=6) as stream:
            stream.sendall(b"SSH-1.0-invalid\r\n")
        wait_state(guest, lambda s: s[6] == 0 and s[10] >= baseline[10] + index + 1)
    clean = wait_state(guest, lambda s: s[6] == 0 and s[13:15] == (4, 2))
    guest.stop()
    guest.boot(False)
    guest.authenticate()
    wait_state(guest, lambda s: s[3:5] == (1, 1))
    reboot_key, _ = keyscan(port)
    assert reboot_key == original_key
    wait_state(guest, lambda s: s[6] == 0)
    after_reboot = master(work, port, 3)
    disconnect(work, port, after_reboot)
    wait_state(guest, lambda s: s[6] == 0)
    (work / "transport-progress.json").write_text(json.dumps({"authentication": True,
        "rejected_authentication": True, "non_aead_rejected": True, "connection_limit": True,
        "malformed_cleanup": True, "host_key_reboot_persistence": True,
        "full_snapshot": full, "clean_snapshot": clean, "revocation_pending": True}, indent=2) + "\n")
    console_result = exercise_console(guest, work) if guest.console_test else None
    if console_result is not None:
        (work / "console-result.json").write_text(json.dumps(console_result, indent=2) + "\n")
    if guest.password_client:
        (work / "password-client-result.json").write_text(json.dumps(exercise_password_client(guest, work), indent=2) + "\n")
    interactive = exercise_interactive_terminal(guest, work) if guest.interactive else None
    regressions = exercise_regressions(guest, work) if guest.regressions else None
    disabled = finish_provisioning(guest, work)
    return {"initial": initial, "full": full, "clean": clean, "disabled": disabled, "console": console_result, "regressions": regressions, "interactive_terminal": interactive,
            "peak_protocol_nanoseconds": clean[15], "protocol_steps": clean[16],
            "public_host_key": base64.b64encode(original_key).decode(), "pointer_latency_seconds": latency}




# ------------------------=
# FUNC: client_text
# DESC: Types into the private SSH prompt using actual keyboard input, without expecting or exposing local command-history text.
# ------------------=
def client_text(guest, value):
    aliases={" ":"spc","-":"minus",".":"dot","/":"slash","=":"equal"}
    shifted={">":"dot","<":"comma","_":"minus",":":"semicolon","@":"2"}
    for character in value:
        if character in shifted:
            guest.key("shift",shifted[character])
        elif character.isupper():
            guest.key("shift", character.lower())
        else:
            guest.key(aliases.get(character,character))


# ------------------------=
# FUNC: client_state
# DESC: Reads two matching samples of the client state/counter ABI, which has no publication sequence fields.
# ------------------=
def client_state(guest):
    address, size = native.base.symbol(guest.work.parent / "artifacts/installed-kernel.elf",
                                       "INFINITY_SSH_CLIENT_SNAPSHOT")
    assert size == 64
    previous = guest.memory(address, size)
    for _ in range(20):
        current = guest.memory(address, size)
        if current == previous:
            state = struct.unpack("<8Q", current)
            assert state[:2] == (0x494e46535348434c, 1)
            return state
        previous = current
    raise AssertionError("Client counters did not stabilize")


# ------------------------=
# FUNC: exercise_password_client
# DESC: Verifies native password login and guest-originated SSH using real input and binary diagnostic state.
# ------------------=
def exercise_password_client(guest, work):
    askpass = work / "fixture-askpass.sh"
    askpass.write_text("#!/bin/sh\nprintf '%s\\n' 'MeshProof901'\n")
    askpass.chmod(0o700)
    before = console_state(guest)
    environment = dict(os.environ, SSH_ASKPASS=str(askpass), SSH_ASKPASS_REQUIRE="force", DISPLAY="infinity-test")
    try:
        result = subprocess.run(["/usr/bin/ssh", "-p", str(guest.ssh_port),
            "-o", "StrictHostKeyChecking=yes", "-o", f"UserKnownHostsFile={work / 'known_hosts'}",
            "-o", "PreferredAuthentications=password", "-o", "PubkeyAuthentication=no",
            "-o", "NumberOfPasswordPrompts=1", "-o", "ConnectTimeout=15",
            "ms9node01@127.0.0.1", "system status"], env=environment,
            stdin=subprocess.DEVNULL, capture_output=True, start_new_session=True, timeout=30)
        assert result.returncode == 0, {"ssh_exit": result.returncode}
    finally:
        askpass.unlink(missing_ok=True)
    assert console_state(guest)[3] > before[3]
    wait_state(guest, lambda s: s[6] == 0)
    command(guest, f"ssh ms9node01@10.0.2.2 -p {guest.ssh_port}")
    # ------------------------=
    # FUNC: wait_client
    # DESC: Waits for typed native-client transitions without parsing terminal text.
    # ------------------=
    def wait_client(expected):
        deadline = time.monotonic() + 40
        while time.monotonic() < deadline:
            value = client_state(guest)
            assert value[:2] == (0x494e46535348434c, 1)
            if value[2] == expected:
                return value
            time.sleep(.1)
        raise AssertionError({"client_state": value, "expected": expected})
    wait_client(2)
    guest.key("y")
    wait_client(3)
    client_text(guest, "MeshProof901")
    guest.key("ret")
    connected = wait_client(5)
    before = console_state(guest)
    client_text(guest, "system status")
    guest.key("ret")
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        current = client_state(guest)
        if current[3] > connected[3] and current[4] >= 14 and console_state(guest)[3] > before[3]:
            break
        time.sleep(.1)
    else:
        raise AssertionError({"client_io": current})
    guest.screenshot("ssh-outbound-password-shell")
    guest.key("ctrl", "bracket_right")
    wait_client(0)
    return {"inbound_password": True, "outbound_password": True, "native_remote_command": True,
            "client_snapshot": current, "iso_attached": False}


# ------------------------=
# FUNC: exercise_interactive_terminal
# DESC: Verifies actual installed keyboard help/preferences and external PTY shell execution with filesystem side effects.
# ------------------=
def exercise_interactive_terminal(guest,work):
    # ------------------------=
    # FUNC: tty
    # DESC: Reads the non-secret terminal state published by the production renderer.
    # ------------------=
    def tty():
        result=native.read(guest,"INFINITY_COMMAND_TERMINAL_SNAPSHOT",16)
        assert result[:2]==(0x494e465454593031,1)
        return result
    # ------------------------=
    # FUNC: wait
    # DESC: Waits for a structured state transition without interpreting command output text.
    # ------------------=
    def wait(predicate):
        deadline=time.monotonic()+45
        while time.monotonic()<deadline:
            if predicate(): return
            time.sleep(.05)
        raise AssertionError({"terminal":tty(),"client":client_state(guest)})
    command(guest,"help")
    wait(lambda:tty()[9]==1)
    assert tty()[5]>6
    guest.key("down");guest.key("ret")
    wait(lambda:tty()[10:12]==(1,1))
    guest.screenshot("terminal-help-detail")
    guest.key("esc");wait(lambda:tty()[9:11]==(1,0))
    guest.key("esc");wait(lambda:tty()[9]==0)
    guest.key("ctrl","comma");wait(lambda:tty()[8]!=0)
    font=tty()[12];guest.key("right");wait(lambda:tty()[12]==(font+1)%4)
    guest.screenshot("terminal-preferences")
    guest.key("esc");wait(lambda:tty()[8]==0)
    command(guest,"help commands")
    wait(lambda:tty()[6]>6)
    guest.screenshot("terminal-scrollback")
    fixture=work/"external-pty";fixture.mkdir()
    port=22444
    log=(fixture/"server.log").open("wb")
    server=subprocess.Popen([str(ROOT/"build/ssh-pty-env/bin/python"),str(ROOT/"tools/ssh-pty-server.py"),str(fixture),"--port",str(port)],stdout=log,stderr=subprocess.STDOUT)
    try:
        wait(lambda:(fixture/"ready.json").exists())
        command(guest,f"ssh fixture@10.0.2.2 -p {port}")
        wait(lambda:client_state(guest)[2]==2)
        # This server's generated key belongs exclusively to this disposable fixture.
        guest.key("y");wait(lambda:client_state(guest)[2]==3)
        client_text(guest,"TerminalProof901");guest.key("ret")
        wait(lambda:client_state(guest)[2]==5 and tty()[7]==1)
        wait(lambda:json.loads((fixture/"pty-events.json").read_text())["shell"] if (fixture/"pty-events.json").exists() else False)
        guest.screenshot("terminal-external-shell-prompt")
        client_text(guest,"printf proof > accepted");guest.key("ret")
        wait(lambda:(fixture/"accepted").exists())
        assert (fixture/"accepted").read_bytes()==b"proof"
        client_text(guest,"printf arrow > arrow-proox");guest.key("backspace");guest.key("f");guest.key("ret")
        wait(lambda:(fixture/"arrow-proof").exists())
        assert (fixture/"arrow-proof").read_bytes()==b"arrow"
        client_text(guest,"sleep 30");guest.key("ret");time.sleep(.5);guest.key("ctrl","c")
        client_text(guest,"printf resumed > interrupt-proof");guest.key("ret")
        wait(lambda:(fixture/"interrupt-proof").exists())
        assert (fixture/"interrupt-proof").read_bytes()==b"resumed"
        guest.screenshot("terminal-external-shell-execution")
        guest.key("ctrl","bracket_right");wait(lambda:client_state(guest)[2]==0 and tty()[7]==0)
        state=json.loads((fixture/"pty-events.json").read_text())
        assert state["password"] and state["pty"] and state["shell"]
        return {"dynamic_rows":True,"help_navigation":True,"preferences":True,"external_shell":True,"editing":True,"interrupt":True,"pty":state}
    finally:
        if server.poll() is None:server.terminate()
        server.wait(timeout=15);log.close()


# ------------------------=
# FUNC: finish_provisioning
# DESC: Revokes the fixture key through native administration, verifies rejection, and removes listener authority.
# ------------------=
def finish_provisioning(guest, work):
    public = (work / "client.pub").read_text().split()
    command(guest, "ssh key revoke ms9node01 " + " ".join(public[:2]))
    rejected = subprocess.run(client(work, guest.ssh_port, extra=("-N",)), capture_output=True, timeout=20)
    assert rejected.returncode == 255
    command(guest, "ssh disable")
    return wait_state(guest, lambda s: s[3:5] == (0, 0) and s[6] == 0 and s[13:15] == (0, 0))


# ------------------------=
# FUNC: console_state
# DESC: Reads remote-session identity, geometry, counters and typed completion without interpreting terminal prose.
# ------------------=
def console_state(guest):
    words = native.read(guest, "INFINITY_SSH_CONSOLE_SNAPSHOT", 48)
    assert words[:2] == (0x494e465353484332, 1) and words[7] <= 4
    return words


# ------------------------=
# FUNC: drain_terminal
# DESC: Keeps the real OpenSSH PTY consuming output under a fixed test budget.
# ------------------=
def drain_terminal(handle):
    process, master, captured = handle
    while select.select([master], [], [], 0)[0]:
        try:
            data = os.read(master, 4096)
        except (BlockingIOError, OSError):
            break
        if not data:
            break
        captured.extend(data)
        assert len(captured) <= 2_000_000


# ------------------------=
# FUNC: wait_console
# DESC: Waits for native structured session changes while draining independent OpenSSH terminal streams.
# ------------------=
def wait_console(guest, handles, predicate, timeout=30):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        for handle in handles:
            drain_terminal(handle)
        words = console_state(guest)
        rows = [words[8+i*9:17+i*9] for i in range(4)]
        if predicate(words, rows):
            return words, rows
        time.sleep(.05)
    raise AssertionError({"console": words})


# ------------------------=
# FUNC: exercise_console
# DESC: Verifies macOS OpenSSH exec status/streams, interactive isolation, history, resize, interruption and clean closure on the installed guest.
# ------------------=
def exercise_console(guest, work):
    port = guest.ssh_port
    initial = console_state(guest)
    assert initial[7] == 0
    receipts = []
    for command_text in ("help", "system status", "node list", "ai status"):
        before = console_state(guest)[3]
        try:
            result = subprocess.run(client(work, port) + [command_text], capture_output=True, timeout=30)
        except subprocess.TimeoutExpired as error:
            (work / "exec-timeout-stdout.bin").write_bytes(error.stdout or b"")
            (work / "exec-timeout-stderr.bin").write_bytes(error.stderr or b"")
            (work / "exec-timeout.json").write_text(json.dumps({"command": command_text,
                "console": console_state(guest), "transport": state(guest)}, indent=2) + "\n")
            raise
        assert result.returncode == 0 and result.stdout and not result.stderr
        wait_state(guest, lambda s: s[6] == 0)
        current, _ = wait_console(guest, [], lambda s, rows: s[3] == before+1 and s[7] == 0)
        receipts.append({"command": command_text, "exit_status": result.returncode,
                         "stdout_bytes": len(result.stdout), "stderr_bytes": len(result.stderr), "completion_count": current[3]})
        (work / "exec-progress.json").write_text(json.dumps(receipts, indent=2) + "\n")
    denied_before = console_state(guest)[5]
    denied = subprocess.run(client(work, port) + ["voice status"], capture_output=True, timeout=30)
    assert denied.returncode == 126 and not denied.stdout and denied.stderr
    wait_state(guest, lambda s: s[6] == 0)
    wait_console(guest, [], lambda s, rows: s[5] == denied_before+1 and s[7] == 0)
    handles = []
    try:
        for index in range(2):
            master_fd, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
            process = subprocess.Popen(client(work, port, extra=("-tt",)), stdin=slave, stdout=slave, stderr=slave,
                                       env={**os.environ, "TERM": "xterm-256color"}, start_new_session=True)
            os.close(slave)
            os.set_blocking(master_fd, False)
            handles.append((process, master_fd, bytearray()))
            wait_console(guest, handles, lambda s, rows: sum(bool(row[6] & 2) for row in rows) == index+1)
        _, rows = wait_console(guest, handles, lambda s, rows: s[7] == 2)
        active = [row for row in rows if row[0]]
        assert len(active) == 2 and active[0][0] != active[1][0]
        os.write(handles[0][1], b"system status\r")
        _, rows = wait_console(guest, handles, lambda s, rows: any(row[4] == 1 and row[7] == 0x8005 for row in rows))
        first_id = next(row[0] for row in rows if row[7] == 0x8005)
        os.write(handles[1][1], b"node list\r")
        _, rows = wait_console(guest, handles, lambda s, rows: any(row[4] == 1 and row[7] == 0xd001 for row in rows))
        second_id = next(row[0] for row in rows if row[7] == 0xd001)
        for handle, identity, operation in [(handles[0], first_id, 0x8005), (handles[1], second_id, 0xd001)]:
            os.write(handle[1], b"\x1b[A\r")
            wait_console(guest, handles, lambda s, rows: any(row[0] == identity and row[4] == 2 and row[7] == operation for row in rows))
        fcntl.ioctl(handles[0][1], termios.TIOCSWINSZ, struct.pack("HHHH", 43, 132, 0, 0))
        os.kill(handles[0][0].pid, signal.SIGWINCH)
        wait_console(guest, handles, lambda s, rows: any(row[0] == first_id and row[2:4] == (132, 43) for row in rows))
        interrupts = console_state(guest)[4]
        os.write(handles[0][1], b"system stat\x03")
        wait_console(guest, handles, lambda s, rows: s[4] == interrupts+1 and any(row[0] == first_id and row[5] == 130 for row in rows))
        os.write(handles[0][1], b"help\r")
        wait_console(guest, handles, lambda s, rows: any(row[0] == first_id and row[4] == 3 and row[5] == 0 for row in rows))
        before = guest.state()
        started = time.monotonic()
        guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {"axis": "x", "value": 4}}]})
        guest.wait(lambda s: s[13] != before[13], "Pointer responds with two active SSH consoles", timeout=5)
        latency = time.monotonic() - started
        os.write(handles[0][1], b"\x04")
        wait_console(guest, handles, lambda s, rows: s[7] == 1)
        assert handles[0][0].wait(timeout=15) == 0
        _, rows = wait_console(guest, handles, lambda s, rows: any(row[0] == second_id and row[4] == 2 for row in rows))
        os.write(handles[1][1], b"help\r")
        wait_console(guest, handles, lambda s, rows: any(row[0] == second_id and row[4] == 3 for row in rows))
        os.write(handles[1][1], b"\x04")
        wait_console(guest, handles, lambda s, rows: s[7] == 0)
        assert handles[1][0].wait(timeout=15) == 0
        wait_state(guest, lambda s: s[6] == 0)
        final = console_state(guest)
        return {"exec": receipts, "denied_status": denied.returncode, "initial": initial,
                "final": final, "pointer_latency_seconds": latency, "peak_execution_nanoseconds": final[6]}
    finally:
        for index, handle in enumerate(handles):
            process, master_fd, captured = handle
            drain_terminal(handle)
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)
            os.close(master_fd)
            (work / f"interactive-{index}.bin").write_bytes(captured)


# ------------------------=
# FUNC: verify_ai_responsiveness
# DESC: Exercises actual composer input, generation admission, Escape cancellation and recovery without assuming a particular Studio status-menu layout.
# ------------------=
def verify_ai_responsiveness(guest, widget):
    guest.launch("text editor", 5)
    editor_before = native.features(guest)[7]
    native.editor_window_control(guest, "minimize")
    deadline = time.monotonic() + 15
    while native.read(guest, "INFINITY_WINDOW_CONTROL_SNAPSHOT", 81)[32]:
        assert time.monotonic() < deadline, "Editor did not minimize"
        time.sleep(.1)
    guest.key("ctrl", "j")
    widget.wait(guest, native, lambda s: s[3] == 1 and s[4] == 2 and s[10] == 1, timeout=600)
    guest.key("ctrl", "x")
    widget.wait(guest, native, lambda s: s[5:7] == (0, 0))
    prompt = "Explain how rain forms in one sentence."
    native.ui.text(guest, prompt)
    widget.wait(guest, native, lambda s: s[5:8] == (len(prompt), len(prompt), 1))
    guest.key("ret")
    widget.wait(guest, native, lambda s: s[9] == 1 and s[5] == 0)
    started = time.monotonic()
    guest.key("esc")
    widget.wait(guest, native, lambda s: s[9] != 1 and s[15] == 0)
    cancellation_seconds = time.monotonic() - started
    guest.key("ctrl", "j")
    native.ui.text(guest, "Ready again")
    widget.wait(guest, native, lambda s: s[5:8] == (11, 11, 1))
    assert native.features(guest)[7] == editor_before
    guest.screenshot("ssh-ai-recovered-after-stop")
    guest.key("ctrl", "x")
    return {"composer_input": True, "generation_admitted": True, "cancel_and_readmit": True,
            "cancellation_seconds": cancellation_seconds, "editor_unchanged": True,
            "microphone_recognition_verified": False, "status_menu_tested": False}


# ------------------------=
# FUNC: exercise_regressions
# DESC: Exercises outbound HTTPS and real AI-widget input while standard OpenSSH clients use the same native runtime.
# ------------------=
def exercise_regressions(guest, work):
    reference_file = work / "https-reference.bin"
    reference = subprocess.run(["/usr/bin/curl", "--ipv4", "--http1.1", "--noproxy", "*",
        "--silent", "--show-error", "--max-time", "30", "--output", str(reference_file),
        "--write-out", "%{http_code}", "https://example.com/"], capture_output=True, timeout=40, check=True)
    expected = (int(reference.stdout), reference_file.stat().st_size,
                hashlib.sha256(reference_file.read_bytes()).digest())
    handle = master(work, guest.ssh_port, 4)
    try:
        command(guest, "https authorize confirm=true")
        command(guest, "geturl https://example.com/")
        elf = guest.work.parent / "artifacts/installed-kernel.elf"
        exit_address, size = native.base.symbol(elf, "INFINITY_GETURL_EXIT")
        body_address, body_size = native.base.symbol(elf, "INFINITY_GETURL_RESPONSE")
        assert size == 4 and body_size == 48
        deadline = time.monotonic() + 180
        while True:
            code = struct.unpack("<I", guest.memory(exit_address, size))[0]
            if code != 0xffffffff:
                break
            assert time.monotonic() < deadline, "Outbound HTTPS did not complete"
            time.sleep(.1)
        response = struct.unpack("<6Q", guest.memory(body_address, body_size))
        assert code == 0 and (response[0], response[1], struct.pack("<4Q", *response[2:])) == expected
        guest.screenshot("ssh-concurrent-outbound-https")
        (work / "network-result.json").write_text(json.dumps({"status": expected[0],
            "length": expected[1], "sha256": expected[2].hex(), "ssh_active": state(guest)[6]}, indent=2) + "\n")
    finally:
        disconnect(work, guest.ssh_port, handle)
    wait_state(guest, lambda s: s[6] == 0)
    # The network test leaves Console in front of the AI composer. Close it via
    # its published titlebar hit target before attempting pointer editing.
    chrome = native.read(guest, "INFINITY_WINDOW_CONTROL_SNAPSHOT", 81)
    assert chrome[:2] == (0x494e465743545231, 1) and chrome[20] == 1
    native.ui.click(guest, chrome[30], chrome[31])
    deadline = time.monotonic() + 15
    while native.read(guest, "INFINITY_WINDOW_CONTROL_SNAPSHOT", 81)[20]:
        assert time.monotonic() < deadline, "Console did not close"
        time.sleep(.1)
    spec = importlib.util.spec_from_file_location("ssh_widget", ROOT / "tools/ai-widget-installed-test.py")
    widget = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(widget)
    stop = threading.Event()
    queries, failures = [], []

    # ------------------------=
    # FUNC: query_during_ui
    # DESC: Drives bounded real SSH traffic during graphical AI input without touching the guest debugger from another thread.
    # ------------------=
    def query_during_ui():
        try:
            for _ in range(150):
                if stop.is_set():
                    break
                began = time.monotonic()
                response = subprocess.run(client(work, guest.ssh_port) + ["system status"], capture_output=True, timeout=20)
                assert response.returncode == 0 and response.stdout and not response.stderr
                queries.append({"started": began, "finished": time.monotonic(), "seconds": time.monotonic() - began})
                if stop.wait(2):
                    break
        except BaseException as error:
            failures.append(repr(error))

    worker = threading.Thread(target=query_during_ui, name="ssh-ui-acceptance")
    worker.start()
    try:
        widget_result = verify_ai_responsiveness(guest, widget)
        (work / "widget-result.json").write_text(json.dumps(widget_result, indent=2) + "\n")
        audio_before = native.read(guest, "INFINITY_NATIVE_WORKFLOW_AUDIO_SNAPSHOT", 16)
        audio_start = guest.audio_capture.stat().st_size
        audio_began = time.monotonic()
        command(guest, "voice say Native speech remains responsive during remote Console access.")
        deadline = time.monotonic() + 180
        while True:
            audio_state = native.read(guest, "INFINITY_NATIVE_WORKFLOW_AUDIO_SNAPSHOT", 16)
            assert audio_state[4] != 4, {"speech_failed": audio_state}
            if audio_state[4] == 7 and audio_state[11] > audio_before[11] and audio_state[10] > 0:
                break
            assert time.monotonic() < deadline, {"speech_timeout": audio_state}
            time.sleep(.1)
        audio_ended = time.monotonic()
        assert any(query["started"] >= audio_began and query["finished"] <= audio_ended for query in queries)
        audio_result = {"start_byte": audio_start, "end_byte": guest.audio_capture.stat().st_size,
                        "seconds": audio_ended - audio_began, "snapshot": audio_state}
    finally:
        stop.set()
        worker.join(timeout=25)
        (work / "concurrent-queries.json").write_text(json.dumps({"queries": queries,
            "failures": failures, "worker_stopped": not worker.is_alive()}, indent=2) + "\n")
    assert not worker.is_alive() and not failures and len(queries) >= 2, {"queries": queries, "failures": failures}
    wait_state(guest, lambda s: s[6] == 0)
    return {"outbound_https": {"status": expected[0], "length": expected[1], "sha256": expected[2].hex()},
            "widget": widget_result, "concurrent_exec": queries, "audio": audio_result,
            "audio_playback_verified": False}


# ------------------------=
# FUNC: main
# DESC: Creates a new disposable installation, verifies media parity and records only actual installed acceptance results.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("work", type=Path)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), required=True)
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--console", action="store_true")
    parser.add_argument("--password-client", action="store_true")
    parser.add_argument("--interactive", action="store_true",help="Verify Command Window UI and external real-shell PTY")
    parser.add_argument("--regressions", action="store_true")
    parser.add_argument("--resume-regressions", action="store_true",
                        help="Retry only regressions on this harness's unchanged, verified fresh installation")
    parser.add_argument("--resume-password-client", action="store_true",
                        help="Retry password-client checks on this harness's unchanged verified installation")
    args = parser.parse_args()
    resuming = args.resume_regressions or args.resume_password_client
    work, media = args.work.resolve(), args.iso.resolve()
    if not __debug__ or not work.is_relative_to(ROOT / "build") or work == ROOT / "build" or (work.exists() != resuming):
        parser.error("Use assertions and a nonexistent work directory strictly inside build/")
    if not media.is_relative_to(ROOT) or not media.is_file():
        parser.error("ISO must be an existing repository-local installer")
    if args.resume_regressions and not args.regressions:
        parser.error("Regression resumption requires --regressions")
    if args.resume_password_client and (args.resume_regressions or args.regressions):
        parser.error("Resume one acceptance group at a time")
    work.mkdir(parents=True, exist_ok=resuming)
    artifacts = work / "artifacts"
    artifacts.mkdir(exist_ok=resuming)
    if resuming:
        previous = json.loads((work / "result.json").read_text())
        assert previous["architecture"] == args.arch
        for image in (media, artifacts / "installer.iso"):
            with image.open("rb") as stream:
                assert hashlib.file_digest(stream, "sha256").hexdigest() == previous["source_media_sha256"]
        media_hash = previous["source_media_sha256"]
        assert json.loads((work / "transport-progress.json").read_text())["host_key_reboot_persistence"]
        assert json.loads((work / "console-result.json").read_text())["final"][7] == 0
        packaged = {"verified_existing_artifacts": True}
    else:
        media_hash = accounts.archive_media(media, artifacts / "installer.iso")
        packaged = native.desktop.extract_media_kernels(artifacts / "installer.iso", artifacts, args.arch)
    guest = Guest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd", reuse=resuming,
                  width=1920, height=1080, memory_mb=12288)
    guest.console_test = args.console
    guest.password_client = args.password_client
    guest.interactive = args.interactive
    guest.regressions = args.regressions
    if args.regressions:
        guest.audio_capture = guest.work / "ssh-audio.wav"
    guest.arch = args.arch
    guest.acceleration = "hvf" if args.arch == "aarch64" else "tcg"
    guest.patched = True
    guest.detached_boots = []
    # Full model payload preparation under cross-architecture TCG is slower than
    # hardware virtualization; this bounds the harness, not SSH service deadlines.
    guest.boot_timeout_seconds = 1200 if args.arch == "x86_64" else 600
    guest.install_timeout_seconds = 3600 if args.arch == "x86_64" else 1800
    with socket.socket() as reserve:
        reserve.bind(("127.0.0.1", 0))
        guest.ssh_port = reserve.getsockname()[1]
    report_path = work / ("password-client-result.json" if args.resume_password_client else
                          "regressions-result.json" if args.resume_regressions else "result.json")
    try:
        if resuming:
            guest.ssh_port = int((work / "known_hosts").read_text().split("]:", 1)[1].split()[0])
            installed = accounts.verify_disk(guest)
            guest.boot(False)
            guest.authenticate()
            wait_state(guest, lambda s: s[3:5] == (1, 1))
            results = ({"password_client": exercise_password_client(guest, work)} if args.resume_password_client
                       else {"regressions": exercise_regressions(guest, work)})
            results["disabled"] = finish_provisioning(guest, work)
        else:
            guest.install()
            guest.stop()
            installed = accounts.verify_disk(guest)
            guest.onboard()
            results = exercise(guest, work)
        guest.stop()
        if args.regressions:
            audio = results["regressions"]["audio"]
            raw = guest.audio_capture.read_bytes()
            assert raw[:4] == b"RIFF" and raw[8:16] == b"WAVEfmt " and raw[36:40] == b"data"
            begin = max(44, audio["start_byte"])
            end = min(len(raw), audio["end_byte"])
            pcm = array.array("h", raw[begin:end - (end - begin) % 2])
            assert len(pcm) >= 4800 and sum(value != 0 for value in pcm) >= 1000
            audio.update({"nonzero_samples": sum(value != 0 for value in pcm), "sample_count": len(pcm),
                          "sha256": hashlib.sha256(raw).hexdigest()})
            results["regressions"]["audio_playback_verified"] = True
        assert accounts.verify_disk(guest) == installed
        report_path.write_text(json.dumps({"passed": True, "architecture": args.arch,
            "source_media_sha256": media_hash, "packaged": packaged, "installed_kernel": installed,
            "detached_boots": guest.detached_boots, "ssh": results}, indent=2) + "\n")
    except BaseException as error:
        report_path.write_text(json.dumps({"passed": False, "architecture": args.arch,
            "source_media_sha256": media_hash, "failure": repr(error),
            "detached_boots": guest.detached_boots}, indent=2) + "\n")
        raise
    finally:
        guest.stop()


if __name__ == "__main__":
    main()
