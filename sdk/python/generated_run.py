"""Trusted generated-application entry; exception reports are diagnostics, not authority."""
import builtins
import io
import os
import sys
import types
from importlib.machinery import SourceFileLoader
from _infinity_runtime import capture_exception


# ------------------------=
# FUNC: main
# DESC: Preserves normal script argv, __main__, scoped imports and real tracebacks while reporting uncaught exception metadata separately.
# ------------------=
def main():
    source = sys.argv[1]
    if not source.startswith("/"):
        raise ValueError("Generated application source must be an absolute granted path")
    sys.argv[:] = sys.argv[1:]
    directory = os.path.dirname(source)
    if directory not in sys.path:
        sys.path.append(directory)
    application = types.ModuleType("__main__")
    application.__file__ = source
    application.__cached__ = None
    application.__loader__ = SourceFileLoader("__main__", source)
    application.__package__ = None
    application.__spec__ = None
    application.__builtins__ = builtins
    # Keep the genuine application module installed through atexit/finalizers,
    # unlike runpy.run_path's temporary __main__ replacement.
    sys.modules["__main__"] = application
    try:
        with io.open_code(source) as stream:
            code = compile(stream.read(), source, "exec", dont_inherit=True)
        exec(code, application.__dict__, application.__dict__)
    except Exception as error:
        try:
            capture_exception(source, error)
        except BaseException:
            # Diagnostic transport failure cannot replace the original exception.
            pass
        raise


if __name__ == "__main__":
    main()
