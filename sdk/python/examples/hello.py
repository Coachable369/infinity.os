"""A real Python script for InfinityOS Documents and command-line acceptance."""

from collections import Counter

counts = Counter(["Infinity", "Python", "Infinity"])
assert counts["Infinity"] == 2
squares = [value * value for value in range(1, 6)]
assert sum(squares) == 55
print("Hello from native Python on InfinityOS!")
print("Squares:", squares)
