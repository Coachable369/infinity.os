"""Compute SHA-256 digests for native object snapshots or standard input."""
import argparse
import hashlib
import sys

# ------------------------=
# FUNC: main
# DESC: Streams each file through SHA-256 with bounded memory and conventional digest output.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('files', nargs='*')
    args = parser.parse_args()
    for name in args.files or ['-']:
        stream = sys.stdin.buffer if name == '-' else open(name, 'rb')
        digest = hashlib.sha256()
        try:
            while True:
                chunk = stream.read(65536)
                if not chunk: break
                digest.update(chunk)
        finally:
            if name != '-': stream.close()
        print(digest.hexdigest(), ' ' + name)

if __name__ == '__main__':
    main()
