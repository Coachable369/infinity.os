#!/bin/sh
set -eu

test "$#" -eq 2 || {
    echo "usage: $0 INPUT.mp3 OUTPUT.pcm" >&2
    exit 2
}

input=$1
output=$2
temporary="${output}.partial"
trap 'rm -f -- "$temporary"' EXIT HUP INT TERM

ffmpeg -hide_banner -loglevel error -y -i "$input" -ac 1 -ar 16000 -f s16le "$temporary"
test -s "$temporary"
bytes=$(wc -c < "$temporary")
test $((bytes % 2)) -eq 0
mv "$temporary" "$output"
trap - EXIT HUP INT TERM
