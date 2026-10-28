#!/usr/bin/env python3
"""Typed native-receipt decoder fixtures; no candidate or verifier execution."""
from pathlib import Path
import importlib.util
import json
import os
import struct


# ------------------------=
# FUNC: receipt
# DESC: Constructs the native probe's binary stream envelope independently of its decoder.
# ------------------=
def receipt(stdout=b"", stderr=b"", case=None, app_status=(1 << 64) - 1):
    status = 1 if case else 0
    data = struct.pack("<7Q", 0x434C414E47454C30, status,
                       len(stdout) + len(stderr), 3, 100, 100, len(stderr))
    data += stderr + bytes(-len(stderr) % 8)
    data += struct.pack("<Q", len(stdout)) + stdout + bytes(-len(stdout) % 8)
    data += struct.pack("<Q", app_status)
    if case:
        data += struct.pack("<5Q", 0x5059455843455031, 1, 1,
                            case["exception_class"], case["line"])
    return data


# ------------------------=
# FUNC: replace_word
# DESC: Mutates exactly one typed protocol word without relying on rendered diagnostics.
# ------------------=
def replace_word(data, offset, value):
    altered = bytearray(data)
    struct.pack_into("<Q", altered, offset, value)
    return bytes(altered)


# ------------------------=
# FUNC: rejected
# DESC: Requires explicit decoder rejection, rather than a false success or unrelated parsing exception.
# ------------------=
def rejected(decoder, data, expected_exit=0, case=None):
    try:
        decoder(data, expected_exit, case)
    except ValueError:
        return
    raise AssertionError("Malformed native receipt was accepted")


# ------------------------=
# FUNC: main
# DESC: Checks stdout/stderr padding, exact byte counts, native sentinels, exception identity, truncation and memory reclamation.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    spec = importlib.util.spec_from_file_location(
        "runtime_recovery_acceptance", Path(__file__).with_name("runtime-recovery-acceptance.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    decode = module.decode_native
    valid_cases = 0
    for output_length in (0, 1, 7, 8, 9, 64):
        for error_length in (0, 1, 7, 8, 9):
            stdout = bytes((index * 13) % 256 for index in range(output_length))
            stderr = bytes((index * 19) % 256 for index in range(error_length))
            observed, actual_error, actual_output = decode(receipt(stdout, stderr), 0, None)
            assert actual_output == stdout and actual_error == stderr
            assert observed == dict(exit_status=0, console_bytes=output_length + error_length,
                                    poll_visits=3, memory_reclaimed=True)
            valid_cases += 1
    for case in module.CASES:
        stdout, stderr = b"before failure\n", b"\x00\xff\x01\x02\x03"
        observed, actual_error, actual_output = decode(receipt(stdout, stderr, case), 1, case)
        assert observed["exit_status"] == 1
        assert actual_error == stderr and actual_output == stdout
        valid_cases += 1
        for relative, value in ((40, 0), (32, 0), (24, 2),
                                (16, case["exception_class"] + 1), (8, case["line"] + 1)):
            data = receipt(stdout, stderr, case)
            rejected(decode, replace_word(data, len(data) - relative, value), 1, case)
        rejected(decode, receipt(stdout, b"", case), 1, case)

    data = receipt(b"output-nine", b"error")
    invalid_cases = 0
    for length in range(len(data)):
        rejected(decode, data[:length])
        invalid_cases += 1
    mutations = (
        replace_word(data, 0, 0),                  # wrong native marker
        replace_word(data, 8, 1),                  # wrong process exit
        replace_word(data, 16, 0),                 # wrong accepted-byte count
        replace_word(data, 24, 1),                 # no observed process visits
        replace_word(data, 32, 99),                # leaked memory
        replace_word(replace_word(data, 32, 0), 40, 0),  # empty memory proof
        replace_word(data, 48, 65537),             # unbounded stderr
        replace_word(data, 64, 65537),             # unbounded stdout after padded stderr
        receipt(b"output", app_status=0),           # unexpected linked-app status
        data + b"\0",                             # trailing evidence
    )
    for changed in mutations:
        rejected(decode, changed)
        invalid_cases += 1
    print(json.dumps(dict(valid_receipts=valid_cases, invalid_receipts=invalid_cases,
                          exception_identity_rejections=len(module.CASES) * 6,
                          candidate_executed=False, native_execution=False), sort_keys=True))


if __name__ == "__main__":
    main()
