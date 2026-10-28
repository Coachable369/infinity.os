"""Native-only explicit exit; nonzero status is not exception evidence."""
import sys
assert __name__ == "__main__" and sys.argv == [__file__]
sys.exit(1)
