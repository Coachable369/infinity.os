#!/bin/sh
set -eu
disk=build/infinity-test-disk.raw
firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
test -s "$disk" || { echo 'ERROR: run make install-test first' >&2; exit 1; }
test_dir=$(mktemp -d -t infinityos-runtime-vm.XXXXXX)
log="$test_dir/runtime.log"; monitor="$test_dir/monitor.sock"; pid=""
# ------------------------=
# FUNC: cleanup
# DESC: Releases temporary resources created by this script.
# ------------------=
cleanup(){ if test -n "$pid" && kill -0 "$pid" 2>/dev/null;then kill "$pid" 2>/dev/null||true;fi;rm -rf "$test_dir"; }
trap cleanup EXIT INT TERM
# ------------------------=
# FUNC: emit_text
# DESC: Types emit text input into the virtual machine.
# ------------------=
emit_text(){ printf %s "$1"|od -An -v -tu1|tr -s ' ' '\n'|while IFS= read -r code;do test -n "$code"||continue;case "$code" in 32) key=spc;;45) key=minus;;*) key=$(printf "\\$(printf '%03o' "$code")");;esac;echo "sendkey $key";sleep .012;done;echo 'sendkey ret';sleep .3; }
:>"$log"
qemu-system-x86_64 -machine pc -m 256M -drive if=pflash,format=raw,readonly=on,file="$firmware" \
 -drive if=ide,index=0,format=raw,file="$disk" -boot order=c -serial file:"$log" -display none -no-reboot \
 -monitor unix:"$monitor",server=on,wait=off & pid=$!
attempt=0;while ! grep -Fq '[ui] startup selection' "$log" 2>/dev/null;do attempt=$((attempt+1));test "$attempt" -lt 500||{ cat "$log";echo 'FAIL: runtime VM boot timed out' >&2;exit 1;};sleep .1;done
{ emit_text 3;emit_text 'service list';emit_text 'service inspect object';emit_text 'runtime contexts';emit_text 'runtime inspect 1';emit_text 'capability list';emit_text 'event subscriptions';emit_text 'event trace';emit_text 'system resources';emit_text 'show me running services'; }|nc -U "$monitor" >/dev/null 2>&1
sleep 1
for evidence in '[operation] Service.List' '[operation] Service.Inspect' '[operation] Runtime.Contexts' \
 '[operation] Runtime.Inspect' '[operation] Capability.List' '[operation] Event.Subscriptions' \
 '[operation] Event.Trace' '[operation] Runtime.Resources' 'object  v1  Ready' \
 'No ambient administrator authority.';do grep -Fq "$evidence" "$log"||{ cat "$log";echo "FAIL: missing runtime console evidence: $evidence" >&2;exit 1;};done
echo 'PASS: installed runtime service discovery and console typed operations'
