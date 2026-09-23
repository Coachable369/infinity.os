#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

sizes="24 32 48 64 96 128 256"
families="crystal-blue-glass luminous-obsidian frosted-quartz aurora-harmony"
if [ "$#" -gt 0 ]; then
    families="$*"
    for requested in "$@"; do
        case "$requested" in
            crystal-blue-glass|luminous-obsidian|frosted-quartz|aurora-harmony) ;;
            *) echo "Unknown icon family: $requested" >&2; exit 2 ;;
        esac
    done
fi
base_names="home user folder folder-open documents downloads pictures music videos projects trash-empty trash-full drive-internal drive-external optical-disc usb-drive cloud-drive network server computer display printer camera microphone headphones terminal settings search information help lock unlock shield key power restart sleep wifi bluetooth battery volume clipboard mail calendar clock"
action_names="back forward up refresh new-file new-folder save cut copy paste undo redo add remove close"

if [ ! -f assets/icons/luminous-obsidian/master-base-v2.png ] || \
   [ ! -f assets/icons/luminous-obsidian/master-actions-v2.png ]; then
    python3 tools/assemble-luminous-obsidian-v2.py
fi

if [ ! -f assets/icons/aurora-harmony/master-base-v1.png ] || \
   [ ! -f assets/icons/aurora-harmony/master-actions-v1.png ]; then
    python3 tools/assemble-aurora-harmony.py
fi

# ------------------------=
# FUNC: build_group
# DESC: Slices one generated master grid into named, square, multi-resolution PNG icon resources.
# ------------------=
build_group() {
    family=$1
    group=$2
    rows=$3
    names=$4
    master="assets/icons/$family/master-$group.png"
    if test -s "assets/icons/$family/master-$group-v3.png"; then
        master="assets/icons/$family/master-$group-v3.png"
    elif test -s "assets/icons/$family/master-$group-v2.png"; then
        master="assets/icons/$family/master-$group-v2.png"
    elif test -s "assets/icons/$family/master-$group-v1.png"; then
        master="assets/icons/$family/master-$group-v1.png"
    fi
    python3 tools/slice-icon-atlas.py \
        "$master" "assets/icons/$family" "$group" "$rows" "$(printf '%s' "$sizes" | tr ' ' ',')"
}

# ------------------------=
# FUNC: build_runtime_atlas
# DESC: Packs ordered 128px resources into compact alpha-enabled PNG and BMP atlases for the framebuffer renderer.
# ------------------=
build_runtime_atlas() {
    family=$1
    group=$2
    layout=$3
    source="assets/icons/$family/128/$group"
    runtime="assets/icons/runtime/$family-$group"
    pattern="$source/*.png"
    ffmpeg -y -loglevel error -framerate 1 -pattern_type glob -i "$pattern" \
        -vf "tile=$layout:padding=0:margin=0,format=rgba" -frames:v 1 "$runtime.png"
    ffmpeg -y -loglevel error -i "$runtime.png" -pix_fmt bgra "$runtime.bmp"
}

for family in $families; do
    build_group "$family" base 9 "$base_names"
    build_group "$family" actions 3 "$action_names"
    build_runtime_atlas "$family" base 5x9
    build_runtime_atlas "$family" actions 5x3
done

python3 tools/build-launcher-icon-atlases.py $families

echo "Built selected 60-icon InfinityOS families at 24, 32, 48, 64, 96, 128, and 256 pixels."
