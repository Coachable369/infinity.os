"""Exercise the native launcher, real GUI edits and AIKit recall on private installed media."""
import argparse
import importlib.util
import json
import os
import subprocess
import socket
from pathlib import Path
import time

ROOT = Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: load
# DESC: Reuses the repository's keyboard, compositor and installed guest harnesses.
# ------------------=
def load(name, relative):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

memory = load("browser_memory_fixture", "tools/aikit-memory-installed-test.py")
tk = load("browser_tk_fixture", "tools/native-python/tk-installed.py")
python = load("browser_python_fixture", "tools/native-python/installed.py")
native, advice = memory.native, memory.advice
ssh = load("browser_ssh", "tools/ssh-installed-test.py")

# ------------------------=
# FUNC: inbound_terminal
# DESC: Checks an actual password-authenticated external OpenSSH PTY and native command completion on this disposable installation.
# ------------------=
def inbound_terminal(guest, work):
    with socket.socket() as reserve:
        reserve.bind(("127.0.0.1", 0))
        port = reserve.getsockname()[1]
    guest.qmp("human-monitor-command", {"command-line": f"hostfwd_add tcp:127.0.0.1:{port}-:22"})
    ssh.wait_state(guest, lambda state:state[3:5]==(1,1))
    handle = "qa" + str(time.time_ns())[-10:]
    guest.command(f"user create handle={handle} display-name=TerminalQA")
    for _ in range(2):
        native.ui.text(guest,ssh.accounts.TEST_PASSWORD)
        guest.key("ret")
        guest.wait(lambda state:state[71]==1,"fixture account input committed",timeout=90)
    _, key = ssh.keyscan(port)
    known = work/"pinned-guest-host"
    known.write_bytes(key)
    helper = work/"askpass"
    helper.write_text('#!/bin/sh\nprintf "%s\\n" "$INFINITY_QA_PASSWORD"\n')
    helper.chmod(0o700)
    args = ["/usr/bin/ssh","-F","/dev/null","-oStrictHostKeyChecking=yes",f"-oUserKnownHostsFile={known}",
            "-oPreferredAuthentications=password","-oPubkeyAuthentication=no","-oNumberOfPasswordPrompts=1",
            "-oConnectTimeout=10","-p",str(port),"-tt",f"{handle}@127.0.0.1"]
    env = dict(os.environ, SSH_ASKPASS=str(helper), SSH_ASKPASS_REQUIRE="force",DISPLAY="qa",
               INFINITY_QA_PASSWORD="incorrect-qa-password")
    proc = None
    try:
        rejected=subprocess.run(args+["system status"],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE,env=env,timeout=20,start_new_session=True)
        assert rejected.returncode==255
        env["INFINITY_QA_PASSWORD"] = ssh.accounts.TEST_PASSWORD
        proc=subprocess.Popen(args,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                              env=env,start_new_session=True)
        initial,rows=ssh.wait_console(guest,[],lambda state,rows:state[7]==1 and any(row[6]&2 for row in rows))
        terminal=next(row[0] for row in rows if row[6]&2)
        proc.stdin.write(b"system status\r")
        proc.stdin.flush()
        _,rows=ssh.wait_console(guest,[],lambda state,rows:state[3]>initial[3] and any(
            row[0]==terminal and row[4]==1 and row[5]==0 for row in rows))
        proc.stdin.write(b"\x04")
        proc.stdin.flush()
        out,_=proc.communicate(timeout=20)
        assert proc.returncode==0 and out
        return {"account":"separate new nonadministrator","password_authenticated":True,"bad_password_refused":True,"pty":True,
                "command":"system status","successful_commands":next(row[4] for row in rows if row[0]==terminal),"output_bytes":len(out),"exit":proc.returncode}
    finally:
        helper.unlink(missing_ok=True)
        if proc is not None and proc.poll() is None:
            proc.kill()
            proc.wait()

# ------------------------=
# FUNC: window
# DESC: Requires the expected native surface and a live successful Python task.
# ------------------=
def window(guest):
    return tk.wait(guest, python, lambda: tk.gui(guest, native),
                   lambda g: len(g["windows"]) == 1 and g["windows"][0]["visible"]
                   and g["windows"][0]["width"] >= 640, timeout=90)["windows"][0]

# ------------------------=
# FUNC: settled_editor
# DESC: Waits for actual painted input to settle before the independent virtual mouse changes focus.
# ------------------=
def settled_editor(guest, region, name):
    x, y, width, height = region
    deadline = time.monotonic() + 45
    previous, since = None, time.monotonic()
    while time.monotonic() < deadline:
        frame = tk.screenshot(guest, name)
        pitch = frame["width"] * 3
        rows = frame["pixels"]
        current = b"".join(rows[r*pitch+x*3:r*pitch+(x+width)*3] for r in range(y,y+height))
        # A blinking insertion caret can change a single vertical column.
        if previous is None or sum(a != b for a,b in zip(current, previous)) > 3 * height:
            since = time.monotonic()
        elif time.monotonic() - since >= 3:
            return frame["path"]
        previous = current
        time.sleep(.35)
    raise AssertionError(f"Native editor failed to settle in {region}")


# ------------------------=
# FUNC: main
# DESC: Runs launcher, recall and reboot checks using real inputs against the installed account.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    parser = argparse.ArgumentParser()
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--reuse", action="store_true")
    parser.add_argument("--verify-ssh", action="store_true")
    parser.add_argument("--kernel", type=Path, help="Test a corrected kernel on the stopped private QA disk")
    args = parser.parse_args()
    work = args.work.resolve()
    assert work.parent == ROOT / "build" and work.name.startswith("memory-browser-")
    artifacts = work / "artifacts"
    image = args.iso.resolve()
    assert image.is_relative_to(ROOT)
    if not args.reuse:
        artifacts.mkdir(parents=True)
        native.desktop.copy_artifact(image, artifacts / "installer.iso")
        native.desktop.extract_media_kernels(image, artifacts, "aarch64")
    else:
        assert advice.sha256(image) == advice.sha256(artifacts / "installer.iso")
    if args.kernel:
        assert args.reuse and args.kernel.resolve().is_relative_to(ROOT / "build")
        disk=work/"node-1/installed.raw"
        opened=subprocess.run(["lsof","-t",str(disk)],capture_output=True)
        assert opened.returncode==1 and not opened.stdout
        subprocess.run(["python3",str(ROOT/"tools/update-installed-clone.py"),str(disk),str(args.kernel),"--apply"],check=True)
        (artifacts/"installed-kernel.elf").unlink()
        native.desktop.copy_artifact(args.kernel.resolve(), artifacts/"installed-kernel.elf")
    guest = advice.AdviceGuest(work, 1, "/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
                               reuse=args.reuse, width=1024, height=768, memory_mb=20480)
    guest.arch, guest.acceleration, guest.patched = "aarch64", "hvf", True
    guest.boot_timeout_seconds, guest.install_timeout_seconds = 600, 1800
    proof = {"accepted":False, "updated_private_kernel":str(args.kernel) if args.kernel else None}
    output = work / "result.json" if args.verify_ssh else ROOT / "builds/validation/aikit-memory/browser-installed.json"
    try:
        if not args.reuse:
            guest.install()
            guest.stop()
        guest.onboard()
        proof["devices"] = advice.detached(guest)
        guest.launch("command", 5)
        if args.verify_ssh:
            proof["external_terminal"]=inbound_terminal(guest,work)
            (work/"external-terminal.json").write_text(json.dumps(proof["external_terminal"])+"\n")
        if args.reuse:
            # This disposable test owns these synthetic markers.
            for _ in range(16):
                prior=memory.command(guest,"search ORBIT")
                if prior[5]==0:
                    break
                assert memory.command(guest,f"forget {prior[6]}")[3]==1
        guest.command("ai new")
        repair=memory.command(guest,"remember Install the missing compiler dependency to fix the build")
        assert repair[3]==0
        command_bounds=native.features(guest)[21:25]
        previous = python.snapshot(guest)[2]
        guest.launch("Infinity Memory", 5)
        bounds = window(guest)
        time.sleep(2)
        proof["window"]=bounds
        wx,wy,ww,wh=(bounds[k] for k in ("x","y","width","height"))
        proof["initial"] = str(guest.screenshot("memory-browser-initial"))
        native.adaptive_click(guest,wx+240,wy+96)
        guest.text("The diagnostic project code is ORBIT-7331.")
        settled_editor(guest,(wx+65,wy+56,ww-90,65),"memory-browser-before-create")
        guest.key("end")
        for _ in range(5):
            guest.key("backspace")
        guest.text("7429.")
        proof["saved"] = settled_editor(guest,(wx+65,wy+56,ww-90,65),"memory-browser-record")
        guest.key("ret")
        native.adaptive_click(guest,wx+240,wy+150)
        guest.text("Compilation failed because a library was absent")
        proof["semantic"]=settled_editor(guest,(wx+58,wy+112,ww-370,74),"memory-browser-semantic")
        native.adaptive_click(guest, *tk.close_point(guest,window(guest)))
        proof["reclaimed"] = list(tk.finished(guest,python,previous))
        proof["after_close"]=str(guest.screenshot("memory-browser-after-close"))
        assert command_bounds[2]>200 and command_bounds[3]>100,command_bounds
        native.adaptive_click(guest, command_bounds[0]+100, command_bounds[1]+command_bounds[3]-40)
        lookup=memory.command(guest,"search ORBIT")
        assert lookup[5]==1 and lookup[6]>0,lookup
        identity=lookup[6]
        previous = python.snapshot(guest)[2]
        guest.launch("InfinityCode",5)
        code_window = window(guest)
        proof["code_window"]=code_window
        proof["code_image"]=settled_editor(guest,(code_window["x"]+8,code_window["y"]+6,400,130),"infinitycode-workspace")
        native.adaptive_click(guest,*tk.close_point(guest,code_window))
        tk.finished(guest,python,previous)
        native.adaptive_click(guest,command_bounds[0]+100,command_bounds[1]+command_bounds[3]-40)
        before=advice.wait(guest,lambda s:s["native_ready"]==s["selected_ready"]==1,"model available",300)
        guest.command("ai ask What is the diagnostic project code")
        admitted=advice.wait(guest,lambda s:s["admitted_turn"]>before["admitted_turn"],"separate question admitted",60)
        proof["answer_receipt"]=advice.completed(guest,admitted,(before["audit_count"],before["audit_fingerprint"]),240,True)
        proof["answer"]=str(guest.screenshot("memory-browser-model-answer"))
        guest.command("model select ministral")
        switched=advice.wait(guest,lambda s:s["model"]!=before["model"] and s["native_ready"]==s["selected_ready"]==1,
                             "second model selected",60)
        assert memory.command(guest,"search ORBIT")[5:7] == (1,identity)
        guest.command("model select hermes")
        restored=advice.wait(guest,lambda s:s["model"]==before["model"] and s["native_ready"]==s["selected_ready"]==1,
                             "original model restored",60)
        proof["model_switch"]=[switched["model"],restored["model"]]
        guest.stop()
        guest.boot(False)
        guest.authenticate()
        guest.launch("command",5)
        assert memory.command(guest,"search ORBIT")[5:7] == (1,identity)
        previous = python.snapshot(guest)[2]
        guest.launch("Infinity Memory", 5)
        bounds = window(guest)
        wx, wy = bounds["x"], bounds["y"]
        guest.key("ctrl","f")
        guest.text("ORBIT")
        settled_editor(guest,(wx+58,wy+112,ww-370,74),"memory-browser-filter")
        native.adaptive_click(guest,wx+145,wy+241)
        native.adaptive_click(guest,wx+int(ww*.35)+90,wy+wh-73)
        proof["delete_prompt"] = settled_editor(guest,(wx+320,wy+460,480,50),"memory-browser-confirm-delete")
        native.adaptive_click(guest,wx+int(ww*.35)+90,wy+wh-73)
        native.adaptive_click(guest,wx+int(ww*.35)+90,wy+wh-73)
        native.adaptive_click(guest,wx+ww-88,wy+wh-73)
        settled_editor(guest,(wx+320,wy+180,470,35),"memory-browser-after-delete")
        native.adaptive_click(guest,*tk.close_point(guest,window(guest)))
        tk.finished(guest,python,previous)
        native.adaptive_click(guest, command_bounds[0]+100, command_bounds[1]+command_bounds[3]-40)
        assert memory.command(guest,"search ORBIT")[5] == 0
        guest.stop()
        guest.boot(False)
        guest.authenticate()
        guest.launch("command",5)
        assert memory.command(guest,"search ORBIT")[5] == 0
        other=memory.command(guest,"search compiler")
        assert other[5]==1 and memory.command(guest,f"forget {other[6]}")[3]==1
        proof["accepted"]=True
    except BaseException as error:
        proof["failure"]=str(error)
        if guest.process and guest.process.poll() is None:
            proof["failure_image"]=str(guest.screenshot("memory-browser-failure"))
        raise
    finally:
        guest.stop()
        output.write_text(json.dumps(proof,indent=2)+"\n")

if __name__=="__main__":
    main()
