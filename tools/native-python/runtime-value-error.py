"""Native-only parser failure fixture: no host execution."""
import sys
assert __name__ == "__main__" and sys.argv == [__file__]
int("invalid integer input")
