#!/bin/sh
# Configure durable host-side serial telemetry only while the user's VM is off.
set -eu
project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
test "$#" -eq 1 || { echo "Usage: $0 <VirtualBox-VM-name>" >&2; exit 2; }
info=$(VBoxManage showvminfo "$1" --machinereadable)
state=$(printf '%s\n' "$info" | sed -n 's/^VMState="\([^"]*\)"/\1/p')
uuid=$(printf '%s\n' "$info" | sed -n 's/^UUID="\([^"]*\)"/\1/p')
case "$state" in poweroff) ;; *) echo "Shut down the VM first; this tool never stops a running VM." >&2; exit 1 ;; esac
case "$uuid" in ''|*[!a-fA-F0-9-]*) echo "Invalid VM UUID" >&2; exit 1 ;; esac
directory="$project_root/builds/voice-logs/$uuid"
mkdir -p "$directory"
chmod 700 "$directory"
log="$directory/serial-$(date -u +%Y%m%dT%H%M%SZ).log"
VBoxManage modifyvm "$uuid" --uart1 0x3F8 4 --uartmode1 file "$log"
printf 'Voice telemetry log: %s\n' "$log"
printf 'After starting the updated OS, inspect with:\npython3 tools/voice-performance-report.py "%s"\n' "$log"
