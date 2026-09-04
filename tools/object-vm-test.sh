#!/bin/sh
set -eu
disk=build/infinity-test-disk.raw
firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
test_dir=$(mktemp -d -t infinityos-object-vm.XXXXXX)
pid=""
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
emit_text(){
    printf %s "$1" | od -An -v -tu1 | tr -s ' ' '\n' | while IFS= read -r code;do
        test -n "$code"||continue
        case "$code" in
            32) key=spc;; 47) key=slash;; 45) key=minus;;
            65|66|67|68|69|70|71|72|73|74|75|76|77|78|79|80|81|82|83|84|85|86|87|88|89|90)
                lower=$((code+32));key="shift-$(printf "\\$(printf '%03o' "$lower")")";;
            *) key=$(printf "\\$(printf '%03o' "$code")");;
        esac
        echo "sendkey $key";sleep 0.018
    done
    echo 'sendkey ret';sleep 0.35
}

# ------------------------=
# FUNC: boot_session
# DESC: Starts and validates boot session.
# ------------------=
boot_session(){
    label=$1;log="$test_dir/$label.log";monitor="$test_dir/$label.sock";:>"$log"
    qemu-system-x86_64 -machine pc -m 256M -drive if=pflash,format=raw,readonly=on,file="$firmware" \
      -drive if=ide,index=0,format=raw,file="$disk" -boot order=c -serial file:"$log" \
      -display none -no-reboot -monitor unix:"$monitor",server=on,wait=off & pid=$!
    attempts=0;while ! grep -Fq '[ui] startup selection' "$log" 2>/dev/null;do attempts=$((attempts+1));
      test "$attempts" -lt 500||{ cat "$log";echo 'FAIL: installed object-store boot timed out' >&2;exit 1;};sleep .1;done
}

# ------------------------=
# FUNC: stop_session
# DESC: Stops stop session and waits for shutdown.
# ------------------=
stop_session(){ kill "$pid" 2>/dev/null||true;wait "$pid" 2>/dev/null||true;pid=""; }

boot_session mutate
{
  emit_text 3
  emit_text 'object create hello'
  emit_text 'object write /home/default/documents/hello Hello Infinity'
  emit_text 'object write /home/default/documents/hello Version 2'
  emit_text 'namespace link /home/default/documents/hello /home/default/archive/hello'
  emit_text 'namespace move /home/default/documents/hello /home/default/projects/hello'
  emit_text 'object history /home/default/archive/hello'
} | nc -U "$monitor" >/dev/null 2>&1
sleep 1;grep -Fq 'Versions retained: 3' "$log"||{ cat "$log";echo 'FAIL: VM object mutation/history failed' >&2;exit 1;}
object_id=$(sed -n 's/^Object ID: obj:\([0-9A-F]*\)$/\1/p' "$log"|head -1);test ${#object_id} -eq 32||{ cat "$log";echo 'FAIL: object identity not logged' >&2;exit 1;}
stop_session

boot_session restore
{
  emit_text 3
  emit_text 'object read /home/default/archive/hello'
  emit_text 'object inspect /home/default/projects/hello'
  emit_text 'object history /home/default/archive/hello'
  emit_text 'object restore /home/default/archive/hello 2'
} | nc -U "$monitor" >/dev/null 2>&1
sleep 1;grep -Fq 'Content: Version 2' "$log"||{ cat "$log";echo 'FAIL: reboot did not preserve current content' >&2;exit 1;}
test "$(grep -Fc "Object ID: obj:$object_id" "$log")" -ge 2||{ cat "$log";echo 'FAIL: both namespace relationships did not preserve object identity' >&2;exit 1;}
grep -Fq 'Restored as new version: 4' "$log"||{ cat "$log";echo 'FAIL: historical restore did not create version 4' >&2;exit 1;};stop_session

boot_session verify
{ emit_text 3;emit_text 'object read /home/default/archive/hello';emit_text 'object history /home/default/archive/hello'; } | nc -U "$monitor" >/dev/null 2>&1
sleep 1;grep -Fq 'Content: Hello Infinity' "$log"||{ cat "$log";echo 'FAIL: restored content did not survive second reboot' >&2;exit 1;}
grep -Fq 'Current version: 4' "$log"||{ cat "$log";echo 'FAIL: restored version metadata did not survive second reboot' >&2;exit 1;}
grep -Fq "Object ID: obj:$object_id" "$log"||{ cat "$log";echo 'FAIL: object identity changed across reboot/move' >&2;exit 1;};stop_session
echo 'PASS: installed VM object create/write/link/move/reboot/history/restore/reboot persistence'
