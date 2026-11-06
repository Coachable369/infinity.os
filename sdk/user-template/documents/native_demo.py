"""Exercises CPython's native extension ABI inside InfinityOS."""
import infinity_native_test

result = infinity_native_test.add(19, 23)
assert result == 42
print("Native C extension result:", result)
