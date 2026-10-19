"""Explicit unavailable descriptor polling boundary for isolated Python."""
import errno

error = OSError


# ------------------------=
# FUNC: select
# DESC: Rejects descriptor polling rather than claiming unsupported descriptors are ready.
# ------------------=
def select(readers, writers, exceptional, timeout=None):
    raise OSError(errno.ENOTSUP, "Descriptor polling is unavailable in this native Python profile")
