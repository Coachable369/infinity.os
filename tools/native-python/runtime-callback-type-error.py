"""Native-only actual stdlib callback failure; not a claim of installed pointer UI proof."""
# ------------------------=
# FUNC: callback
# DESC: Exercises an actual callback invocation that raises TypeError in application source.
# ------------------=
def callback(value):
    return value + "invalid operand"
sorted([1], key=callback)
