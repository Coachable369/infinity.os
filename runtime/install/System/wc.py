"""Count lines, words and bytes in native object snapshots or standard input."""
import argparse
import sys

# ------------------------=
# FUNC: main
# DESC: Counts each input and reports a total without loading whole files into memory.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('-l', action='store_true', help='show lines')
    parser.add_argument('-w', action='store_true', help='show words')
    parser.add_argument('-c', action='store_true', help='show bytes')
    parser.add_argument('files', nargs='*')
    args = parser.parse_args()
    selected = (args.l, args.w, args.c) if args.l or args.w or args.c else (True, True, True)
    totals = [0, 0, 0]
    for name in args.files or ['-']:
        counts = [0, 0, 0]
        stream = sys.stdin.buffer if name == '-' else open(name, 'rb')
        try:
            for line in stream:
                counts[0] += line.count(b'\n')
                counts[1] += len(line.split())
                counts[2] += len(line)
        finally:
            if name != '-': stream.close()
        totals = [a+b for a,b in zip(totals,counts)]
        print(' '.join(str(n) for n, show in zip(counts,selected) if show), name)
    if len(args.files) > 1:
        print(' '.join(str(n) for n, show in zip(totals,selected) if show), 'total')

if __name__ == '__main__':
    main()
