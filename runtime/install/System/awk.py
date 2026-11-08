"""awk [-F SEP] '{print $1,$2}' [FILE ...] -- print fields, NR or NF."""
import argparse
import re
import sys

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("-F")
p.add_argument("program")
p.add_argument("files", nargs="*")
a = p.parse_args()
match = re.fullmatch(r"\s*\{\s*print(?:\s+(.*?))?\s*\}\s*", a.program)
if not match:
    sys.exit(__doc__)
fields = [s.strip() for s in (match[1] or "$0").split(",")]
if any(not re.fullmatch(r"NR|NF|\$\d+", s) for s in fields):
    sys.exit(__doc__)
nr = 0
for path in a.files or ["-"]:
    stream = sys.stdin if path == "-" else open(path, encoding="utf-8")
    try:
        for line in stream:
            nr += 1
            line = line.rstrip("\n")
            words = re.split(a.F, line) if a.F else line.split()
            row = [line] + words
            print(*(nr if s == "NR" else len(words) if s == "NF" else
                    row[int(s[1:])] if int(s[1:]) < len(row) else "" for s in fields))
    finally:
        if stream is not sys.stdin:
            stream.close()
