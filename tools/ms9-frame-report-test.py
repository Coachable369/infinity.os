"""Behavioral checks of the installed diagnostic reader using explicitly synthetic samples."""
import importlib.util
import pathlib
import struct
import tempfile

# ------------------------=
# FUNC: main
# DESC: Verifies frame-ring filtering, exact statistics and debugger pause/resume without claiming guest performance.
# ------------------=
def main():
    spec = importlib.util.spec_from_file_location("installed_acceptance", pathlib.Path(__file__).with_name("ms9-installed-acceptance.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    guest = object.__new__(module.Guest)
    guest.installer = False
    guest.frames_address = 0
    guest.frames_length = 10802 * 8
    words = [0] * 10802
    words[:2] = [1, 4]
    words[2:14] = [1, 10, 1, 2, 20, 2, 3, 30, 3, 4, 40, 4]
    guest.memory = lambda *_: struct.pack("<10802Q", *words)
    calls = []
    guest.qmp = lambda operation, *_: calls.append(operation)
    with tempfile.TemporaryDirectory(prefix="ms9-frame-report-") as directory:
        guest.work = pathlib.Path(directory)
        result = guest.frame_report("initial")
        assert (result["sample_count"], result["average_ns"], result["p95_ns"], result["worst_ns"]) == (4, 25, 40, 40)
        assert not result["performance_acceptance"]
        assert calls == ["stop", "cont"]
        words[1] = 3601
        words[2:5] = [3601, 100, 5]
        result = guest.frame_report("wrapped")
        assert (result["sample_count"], result["average_ns"], result["worst_ns"]) == (4, 47, 100)
        words[3] = 0xffffffffffffffff
        result = guest.frame_report("unavailable")
        assert result["sample_count"] == 3
        words[1] = 7200
        result = guest.frame_report("stale")
        assert result["sample_count"] == 0
        assert "average_ns" not in result
        words[0] = 0
        result = guest.frame_report("not-initialized")
        assert not result["available"] and result["sample_count"] == 0
    print("Diagnostic reader behavior passed; all samples synthetic.")

if __name__ == "__main__":
    main()
