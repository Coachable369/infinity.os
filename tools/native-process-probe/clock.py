"""Runs inside native CPython, not the build host; the probe supplies its UTC epoch."""
import sys
import time

assert sys.platform == "infinityos"
realtime = time.clock_gettime_ns(time.CLOCK_REALTIME)
monotonic = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
assert 1_800_000_000_000_000_000 <= realtime < 1_800_000_600_000_000_000
assert 0 <= monotonic < 86_400_000_000_000
assert realtime - monotonic > 1_700_000_000_000_000_000
assert 0 <= time.time_ns() - realtime < 10_000_000_000
assert 0 <= time.monotonic_ns() - monotonic < 10_000_000_000
assert time.clock_gettime_ns(time.CLOCK_REALTIME) >= realtime
assert time.clock_gettime_ns(time.CLOCK_MONOTONIC) >= monotonic
try:
    time.clock_gettime_ns(255)
except OSError as failure:
    assert failure.errno == 22
else:
    raise AssertionError("Unsupported clock identifier was accepted")
print(realtime, monotonic)
sys.exit(54)
