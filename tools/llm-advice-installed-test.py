#!/usr/bin/env python3
"""Real Command Window and widget inference on a private, media-detached installation."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import platform
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("llm_native_editor", ROOT / "tools/native-editor-installed-test.py")
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)
widget_spec = importlib.util.spec_from_file_location("llm_widget", ROOT / "tools/ai-widget-installed-test.py")
widget = importlib.util.module_from_spec(widget_spec)
widget_spec.loader.exec_module(widget)

class AdviceGuest(native.NativeGuest):
    # ------------------------=
    # FUNC: text
    # DESC: Types prompts through the existing real-keyboard punctuation adapter without writing guest memory.
    # ------------------=
    def text(self, value):
        native.ui.text(self, value)

# ------------------------=
# FUNC: sha256
# DESC: Binds reusable private fixtures and final receipts to the exact immutable installer bytes.
# ------------------=
def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()

# ------------------------=
# FUNC: snapshot
# DESC: Reads coherent content-free admission, inference, action-audit and terminal-delivery state from the installed kernel.
# ------------------=
def snapshot(guest):
    state = native.read(guest, "INFINITY_LLM_ADVICE_SNAPSHOT", 24)
    if state[:2] != (0x494e464c4c4d3031, 1):
        return None
    names = ("authorized", "origin", "mode", "admitted_turn", "accepted_requests", "chat_turn",
             "generation", "response_bytes", "native_tokens", "model", "native_ready",
             "control_pending", "audit_count", "audit_fingerprint", "command_turn", "delivered_turn",
             "command_terminal", "terminal_rows", "command_owned", "selected_ready")
    return dict(sequence=state[2], **dict(zip(names, state[3:23])))

# ------------------------=
# FUNC: wait
# DESC: Requires typed transitions and a live guest within a bounded deadline, preserving numeric failure evidence.
# ------------------=
def wait(guest, predicate, label, timeout=180, audit=None):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        assert guest.process.poll() is None, dict(stage=label, exit=guest.process.returncode)
        last = snapshot(guest)
        if last is not None:
            if audit is not None:
                assert last["control_pending"] == 0, dict(stage=label, snapshot=last)
                assert (last["audit_count"], last["audit_fingerprint"]) == audit, dict(stage=label, snapshot=last)
            if predicate(last):
                return last
        time.sleep(.2)
    guest.screenshot("llm-advice-failure")
    raise AssertionError(dict(stage=label, snapshot=last))

# ------------------------=
# FUNC: detached
# DESC: Verifies the installed boot flag and actual QEMU block-device inventory contain no inserted removable media.
# ------------------=
def detached(guest):
    assert guest.installer is False and guest.state()[3] == 1
    devices = guest.qmp("query-block")
    assert not any(device.get("removable", False) and device.get("inserted") for device in devices), devices
    return [dict(device=item.get("device"), removable=item.get("removable", False),
                 inserted=bool(item.get("inserted"))) for item in devices]

# ------------------------=
# FUNC: completed
# DESC: Requires real native output tokens and response bytes for the exact admitted turn, with no new OS execution audit records.
# ------------------=
def completed(guest, admitted, audit, timeout, terminal):
    # ------------------------=
    # FUNC: terminal_state
    # DESC: Recognizes completion for the exact request and requires terminal delivery when requested.
    # ------------------=
    def terminal_state(state):
        same = state["admitted_turn"] == admitted["admitted_turn"] == state["chat_turn"]
        return same and state["generation"] in (2, 3, 4, 5) and (not terminal or state["delivered_turn"] == state["chat_turn"])
    result = wait(guest, terminal_state, "native reply completed and delivered", timeout, audit)
    assert result["generation"] == 2 and result["native_tokens"] > 0 and result["response_bytes"] > 0, result
    assert result["native_ready"] == result["selected_ready"] == 1, result
    if terminal:
        assert result["command_owned"] == result["command_terminal"] == 1, result
        assert result["terminal_rows"] >= admitted["terminal_rows"], result
    return result

# ------------------------=
# FUNC: exercise
# DESC: Executes actual terminal and widget prompts, verifies owner-bound response delivery and independent advisory admission, and captures production screenshots.
# ------------------=
def exercise(guest, proof, timeout):
    proof["detached_devices"] = detached(guest)
    guest.launch("command", 5)
    before = wait(guest, lambda state: state["authorized"] == state["native_ready"] == state["selected_ready"] == 1
                  and state["generation"] != 1 and not state["control_pending"], "native model ready", 600)
    audit = (before["audit_count"], before["audit_fingerprint"])
    guest.command("ai ask Explain a scheduled task in one short sentence.")
    admitted = wait(guest, lambda state: state["origin"] == state["mode"] == 1
                    and state["admitted_turn"] > before["chat_turn"]
                    and state["accepted_requests"] == before["accepted_requests"] + 1,
                    "Command Window prompt admitted", audit=audit)
    assert admitted["command_turn"] == admitted["admitted_turn"] and admitted["command_owned"] == admitted["command_terminal"] == 1
    proof["screenshots"].append(str(guest.screenshot("llm-command-generating")))
    command_result = completed(guest, admitted, audit, timeout, True)
    widget.wait(guest, native, lambda state: state[14] == command_result["chat_turn"] and state[9] == 2)
    assert command_result["terminal_rows"] > before["terminal_rows"], command_result
    proof["command"] = dict(admitted=admitted, completed=command_result)
    proof["screenshots"].append(str(guest.screenshot("llm-command-response")))

    guest.key("ctrl", "j")
    widget.wait(guest, native, lambda state: state[4] == 2 and state[10] == 1)
    guest.key("shift", "tab")
    widget.wait(guest, native, lambda state: state[4] == 1 and state[10] == 1)
    guest.key("tab")
    widget.wait(guest, native, lambda state: state[4] == 2 and state[10] == 1)
    proof["widget_reverse_focus"] = True
    guest.key("ctrl", "x")
    widget.wait(guest, native, lambda state: state[5:7] == (0, 0))
    prompt = "decide weights=20,80 scores=9,4/4,9 Reply with exactly CHOICE:1 or CHOICE:2."
    native.ui.text(guest, prompt)
    widget.wait(guest, native, lambda state: state[5:7] == (len(prompt), len(prompt)))
    guest.key("ret")
    submitted = wait(guest, lambda state: state["origin"] == state["mode"] == 2
                     and state["admitted_turn"] > command_result["chat_turn"]
                     and state["accepted_requests"] == command_result["accepted_requests"] + 1,
                     "widget decision admitted", audit=audit)
    result = completed(guest, submitted, audit, timeout, False)
    widget.wait(guest, native, lambda state: state[14] == result["chat_turn"] and state[9] == 2 and state[5] == 0)
    # The widget reply must not be copied into the earlier terminal request.
    assert result["command_turn"] == result["delivered_turn"] == command_result["delivered_turn"], result
    assert result["terminal_rows"] == command_result["terminal_rows"], result
    proof["widget_decision"] = dict(admitted=submitted, completed=result)
    proof["screenshots"].append(str(guest.screenshot("llm-widget-decision-response")))
    guest.launch("command", 5)
    guest.command("ai new")
    cleared = wait(guest, lambda state: state["chat_turn"] > result["chat_turn"]
                   and state["generation"] == 0 and state["response_bytes"] == 0
                   and state["command_owned"] == 0,
                   "completed advisory conversation reset", audit=audit)
    assert cleared["accepted_requests"] == result["accepted_requests"], cleared
    guest.command("llm ask Explain a daemon in one short sentence.")
    fresh = wait(guest, lambda state: state["origin"] == state["mode"] == 1
                 and state["admitted_turn"] > cleared["chat_turn"]
                 and state["accepted_requests"] == cleared["accepted_requests"] + 1,
                 "fresh conversation admitted through llm alias", audit=audit)
    fresh_result = completed(guest, fresh, audit, timeout, True)
    proof["conversation_reset"] = dict(cleared=cleared, admitted=fresh, completed=fresh_result)
    proof["screenshots"].append(str(guest.screenshot("llm-fresh-conversation-response")))
    proof["no_os_actions"] = True
    proof["response_ownership"] = True
    proof["native_inference"] = True
    proof["decision_correctness_scope"] = "Installed admission and generation verified; exact choice correctness is covered by the separate real-model host protocol suite."

# ------------------------=
# FUNC: main
# DESC: Installs unchanged ISO bytes into a new private fixture, or explicitly reuses a validated private harness fixture, then records failure-preserving detached-runtime evidence.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/llm-advice-installed-test.py --arch ARCH --iso PATH")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), required=True)
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--reuse", type=Path, help="Explicitly reuse a private LLM or automation harness fixture; no fresh-install claim")
    parser.add_argument("--firmware", type=Path)
    parser.add_argument("--timeout", type=int, default=240, help="Maximum seconds to observe each model response")
    args = parser.parse_args()
    assert 1 <= args.timeout <= 900
    iso = args.iso.resolve()
    assert iso.is_relative_to(ROOT) and iso.is_file()
    reuse = args.reuse is not None
    work = args.reuse.resolve() if reuse else ROOT / "build" / f"llm-advice-installed-{args.arch}-{time.time_ns()}"
    assert work.parent == ROOT / "build" and work.name.startswith((f"llm-advice-installed-{args.arch}-", f"automation-installed-{args.arch}-"))
    artifacts = work / "artifacts"
    marker = work / "harness.json"
    iso_digest = sha256(iso)
    if not reuse:
        artifacts.mkdir(parents=True)
        native.desktop.copy_artifact(iso, artifacts / "installer.iso")
        media = native.desktop.extract_media_kernels(iso, artifacts, args.arch)
        marker.write_text(json.dumps(dict(harness="llm-advice-installed-v1", architecture=args.arch,
                                         iso_sha256=iso_digest, media=media), indent=2) + "\n")
    else:
        assert iso_digest == sha256(artifacts / "installer.iso")
        if marker.is_file():
            identity = json.loads(marker.read_text())
            assert identity["harness"] == "llm-advice-installed-v1" and identity["architecture"] == args.arch
            assert identity["iso_sha256"] == iso_digest
            media = identity["media"]
            assert media["installed-kernel.elf"]["sha256"] == sha256(artifacts / "installed-kernel.elf")
        else:
            # A separate automation run can supply the private disk, but its
            # extracted artifacts are independently checked against this ISO.
            assert work.name.startswith(f"automation-installed-{args.arch}-")
            validation = work / f"llm-media-validation-{time.time_ns()}"
            validation.mkdir()
            media = native.desktop.extract_media_kernels(iso, validation, args.arch)
            for filename in ("kernel.elf", "installed-kernel.elf"):
                assert media[filename]["sha256"] == sha256(artifacts / filename)
            marker.write_text(json.dumps(dict(harness="llm-advice-installed-v1", architecture=args.arch,
                                             iso_sha256=iso_digest, media=media), indent=2) + "\n")
    firmware = args.firmware or Path(f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd")
    guest = AdviceGuest(work, 1, str(firmware), reuse=reuse, width=1024, height=768, memory_mb=12288)
    guest.arch = args.arch
    guest.acceleration = "hvf" if args.arch == "aarch64" and platform.system() == "Darwin" and platform.machine() == "arm64" else "tcg"
    guest.patched = True
    guest.install_timeout_seconds = 1800 if args.arch == "x86_64" else 900
    guest.boot_timeout_seconds = 600
    proof = dict(architecture=args.arch, installed_acceptance=False, fresh_install=False,
                 reused_private_disk=reuse, source_media=str(iso), iso_sha256=iso_digest,
                 media_kernels=media, screenshots=[], work=str(work), acceleration=guest.acceleration)
    destination = ROOT / "builds/validation/llm-advice" / args.arch / "installed.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        if not reuse:
            guest.install()
            guest.stop()
            proof["fresh_install"] = True
        subprocess.run(["python3", str(ROOT / "tools/update-installed-clone.py"), str(guest.disk),
                        str(artifacts / "installed-kernel.elf"), "--verify-only"], check=True)
        proof["installed_kernel_matches_media"] = True
        proof["identity"] = guest.onboard()
        assert proof["identity"]["cold_boot_identity"]
        exercise(guest, proof, args.timeout)
        proof["installed_acceptance"] = True
    except BaseException as error:
        proof["failure"] = str(error)
        if guest.process is not None and guest.process.poll() is None:
            try:
                proof["last_snapshot"] = snapshot(guest)
                proof["screenshots"].append(str(guest.screenshot("llm-advice-failure-final")))
            except BaseException as capture_error:
                proof["capture_failure"] = str(capture_error)
        raise
    finally:
        guest.stop()
        report = json.dumps(proof, indent=2) + "\n"
        destination.write_text(report)
        (work / "evidence.json").write_text(report)
        print(json.dumps(dict(report=str(destination), installed_acceptance=proof["installed_acceptance"])), flush=True)

if __name__ == "__main__":
    main()
