"""Validate and format JSON from a native object snapshot or standard input."""
import argparse
import json
import sys

# ------------------------=
# FUNC: main
# DESC: Produces readable or compact JSON, rejecting invalid documents with a nonzero exit code.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compact', action='store_true')
    parser.add_argument('--sort-keys', action='store_true')
    parser.add_argument('file', nargs='?', default='-')
    args = parser.parse_args()
    stream = sys.stdin if args.file == '-' else open(args.file, encoding='utf-8')
    try:
        document = json.load(stream)
    except ValueError as error:
        parser.exit(1, str(error) + '\n')
    finally:
        if args.file != '-': stream.close()
    print(json.dumps(document, indent=None if args.compact else 2,
                     separators=(',', ':') if args.compact else None, sort_keys=args.sort_keys))

if __name__ == '__main__':
    main()
