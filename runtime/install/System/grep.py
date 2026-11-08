"""grep [-in] PATTERN [FILE ...] -- print matching lines."""
import argparse
import re
import sys

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("-i", action="store_true")
p.add_argument("-n", action="store_true")
p.add_argument("pattern")
p.add_argument("files", nargs="*")
a = p.parse_args()
pattern = re.compile(a.pattern, re.I if a.i else 0)
found = False
for path in a.files or ["-"]:
    stream = sys.stdin if path == "-" else open(path, encoding="utf-8")
    try:
        for number, line in enumerate(stream, 1):
            if pattern.search(line):
                found = True
                print((path + ":" if len(a.files) > 1 else "") +
                      (str(number) + ":" if a.n else "") + line.rstrip("\n"))
    finally:
        if stream is not sys.stdin:
            stream.close()
sys.exit(0 if found else 1)
