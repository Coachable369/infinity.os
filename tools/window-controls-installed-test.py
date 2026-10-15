"""Exercise real title-button input and tray restoration on a disposable installed OS."""
import argparse
import importlib.util
import json
from pathlib import Path
import shutil
import time

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location("native_editor",ROOT/"tools/native-editor-installed-test.py")
native=importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)

# ------------------------=
# FUNC: state
# DESC: Reads the versioned non-secret window lifecycle projection without injecting state.
# ------------------=
def state(guest):
    value=native.read(guest,"INFINITY_WINDOW_CONTROL_SNAPSHOT",81)
    assert value[:2]==(0x494e465743545231,1)
    return value

# ------------------------=
# FUNC: wait
# DESC: Waits for observable lifecycle changes following one real pointer press.
# ------------------=
def wait(guest,predicate):
    until=time.monotonic()+30
    while time.monotonic()<until:
        value=state(guest)
        if predicate(value):return value
        time.sleep(.05)
    raise AssertionError({"window_state":state(guest)})

# ------------------------=
# FUNC: button
# DESC: Clicks a runtime-published title-button center exactly once.
# ------------------=
def button(guest,app,index):
    s=state(guest);at=8+app*12+6+index*2
    native.adaptive_click(guest,s[at],s[at+1])

# ------------------------=
# FUNC: verify
# DESC: Checks maximize/restore, hidden identity, exact tray restoration and closure for every shell app.
# ------------------=
def verify(guest):
    results=[]
    for query,app,shelf,mode in [("file navigator",0,0,5),("terminal",1,6,5),("text",2,7,5),
                                ("task manager",3,8,5),("settings",4,9,8),("browser",5,10,5)]:
        guest.launch(query,mode)
        at=8+app*12
        before=wait(guest,lambda s:s[at]==1 and s[4]==app)
        if before[at+1]:button(guest,app,1);before=wait(guest,lambda s:s[at+1]==0)
        bounds=before[at+2:at+6]
        button(guest,app,1)
        enlarged=wait(guest,lambda s:s[at+1]==1)
        assert enlarged[at+4]>=bounds[2]
        button(guest,app,1)
        restored=wait(guest,lambda s:s[at+1]==0)
        assert restored[at+2:at+6]==bounds
        button(guest,app,0)
        hidden=wait(guest,lambda s:s[at]==0 and s[3]&(1<<shelf)!=0)
        display=guest.state();width=display[11]
        assert hidden[5]>width//2
        rail_width=min(max(width*6//100,72),width//4)
        row=(hidden[3]&((1<<shelf)-1)).bit_count()
        native.adaptive_click(guest,hidden[5]+rail_width//2,
                              hidden[6]+rail_width+row*hidden[7])
        shown=wait(guest,lambda s:s[at]==1 and s[3]&(1<<shelf)==0)
        assert shown[at+2:at+6]==bounds
        guest.screenshot(f"window-controls-{app}")
        button(guest,app,2)
        closed=wait(guest,lambda s:s[at]==0)
        assert closed[3]&(1<<shelf)==0
        results.append({"app":app,"maximize_restore":True,"minimize_restore":True,"close":True})
    return results

# ------------------------=
# FUNC: main
# DESC: Installs to a disposable disk, boots with media detached and records structured lifecycle evidence.
# ------------------=
def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--arch",choices=("aarch64","x86_64"),required=True)
    parser.add_argument("--iso",type=Path,required=True)
    parser.add_argument("work",type=Path)
    args=parser.parse_args()
    work=args.work.resolve();work.relative_to(ROOT/"build");work.mkdir()
    artifacts=work/"artifacts";artifacts.mkdir()
    media=args.iso.resolve();media.relative_to(ROOT)
    shutil.copyfile(media,artifacts/"installer.iso")
    packaged=native.desktop.extract_media_kernels(artifacts/"installer.iso",artifacts,args.arch)
    guest=native.NativeGuest(work,1,f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd",width=1024,height=768,memory_mb=12288)
    guest.arch=args.arch;guest.acceleration="hvf" if args.arch=="aarch64" else "tcg"
    guest.patched=True;guest.boot_timeout_seconds=600
    report={"accepted":False,"arch":args.arch,"media":str(media),"packaged":packaged}
    try:
        guest.install();guest.stop();report["identity"]=guest.onboard()
        report["apps"]=verify(guest);report["accepted"]=True
    finally:
        guest.stop()
        (work/"evidence.json").write_text(json.dumps(report,indent=2))
    print(json.dumps(report,indent=2))

if __name__=="__main__":main()
