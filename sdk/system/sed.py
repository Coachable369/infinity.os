"""sed 's/old/new/[g]' [FILE ...] -- basic line substitution."""
import re
import sys

if len(sys.argv) < 2 or not sys.argv[1].startswith("s") or len(sys.argv[1]) < 4:
    sys.exit(__doc__)
delimiter = sys.argv[1][1]
parts = sys.argv[1][2:].split(delimiter)
if len(parts) != 3 or parts[2] not in ("", "g"):
    sys.exit(__doc__)
pattern = re.compile(parts[0])
for path in sys.argv[2:] or ["-"]:
    stream = sys.stdin if path == "-" else open(path, encoding="utf-8")
    try:
        for line in stream:
            sys.stdout.write(pattern.sub(parts[1], line, count=0 if parts[2] else 1))
    finally:
        if stream is not sys.stdin:
            stream.close()
