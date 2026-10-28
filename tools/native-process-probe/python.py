import sys
from collections import Counter

class Calculator:
    # ------------------------=
    # FUNC: total
    # DESC: Computes a deterministic native arithmetic result for the isolated process probe.
    # ------------------=
    def total(self, values):
        return sum(x * x for x in values)

counts = Counter(['a', 'b', 'a'])
assert counts['a'] == 2
assert 'Infinity'.upper() == 'INFINITY'
print('Hello from native Python')
sys.exit(Calculator().total([1, 2, 7]))
