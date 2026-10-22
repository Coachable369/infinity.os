"""Inspect actual guest descriptor artifacts; never supplies capture or recognition."""
import hashlib
import json
import sys
from pathlib import Path


# ------------------------=
# FUNC: inspect
# DESC: Validates descriptor boundaries and extracts MJPEG modes from guest USB bytes.
# ------------------=
def inspect(device, config):
    if len(device) != 18 or device[:2] != b"\x12\x01":
        raise ValueError("invalid device descriptor")
    if len(config) < 9 or config[:2] != b"\x09\x02" or len(config) != int.from_bytes(config[2:4], "little"):
        raise ValueError("invalid configuration length")
    report = {"vid": int.from_bytes(device[8:10], "little"),
              "pid": int.from_bytes(device[10:12], "little"),
              "config_sha256": hashlib.sha256(config).hexdigest(),
              "interfaces": [], "endpoints": [], "mjpeg_modes": []}
    offset = 0
    interface = None
    while offset < len(config):
        size = config[offset]
        if size < 2 or offset + size > len(config):
            raise ValueError("invalid descriptor boundary")
        d = config[offset:offset + size]
        kind = d[1]
        if kind == 4:
            if size != 9:
                raise ValueError("invalid interface")
            interface = {"number": d[2], "alternate": d[3], "class": d[5], "subclass": d[6]}
            report["interfaces"].append(interface)
        elif kind == 5:
            if size < 7 or interface is None:
                raise ValueError("invalid endpoint")
            report["endpoints"].append({"interface": interface["number"], "address": d[2],
                                        "transfer_type": d[3] & 3,
                                        "max_packet": int.from_bytes(d[4:6], "little")})
        elif kind == 0x24 and interface and interface["class"] == 14:
            if size < 3:
                raise ValueError("invalid class descriptor")
            if interface["subclass"] == 1 and d[2] == 1:
                if size < 5:
                    raise ValueError("invalid control header")
                report["uvc_bcd"] = int.from_bytes(d[3:5], "little")
            if interface["subclass"] == 2 and d[2] == 7:
                if size < 26 or d[25] == 0 or size != 26 + 4 * d[25]:
                    raise ValueError("unsupported or invalid MJPEG intervals")
                intervals = [int.from_bytes(d[i:i+4], "little") for i in range(26, size, 4)]
                if not all(intervals):
                    raise ValueError("zero frame interval")
                report["mjpeg_modes"].append({"index": d[3], "width": int.from_bytes(d[5:7], "little"),
                    "height": int.from_bytes(d[7:9], "little"),
                    "max_frame_bytes": int.from_bytes(d[17:21], "little"),
                    "intervals_100ns": intervals})
        offset += size
    return report


# ------------------------=
# FUNC: main
# DESC: Checks actual descriptor structure and malformed inputs, then writes a machine-readable evidence report.
# ------------------=
def main():
    root = Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parents[2] / "builds/validation/camera/gpt"
    device = (root / "device.bin").read_bytes()
    config = (root / "config.bin").read_bytes()
    result = inspect(device, config)
    assert (result["vid"], result["pid"], result["uvc_bcd"]) == (0x80ee, 0x30, 0x100)
    assert len(result["mjpeg_modes"]) == 7
    assert any(m["width"] == 640 and m["height"] == 480 for m in result["mjpeg_modes"])
    assert any(e["address"] == 0x82 and e["transfer_type"] == 2 for e in result["endpoints"])
    malformed = [config[:i] for i in range(len(config))]
    bad = bytearray(config)
    bad[9] = 0
    malformed.append(bytes(bad))
    for candidate in malformed:
        try:
            inspect(device, candidate)
        except ValueError:
            continue
        raise AssertionError("accepted malformed configuration")
    result["malformed_inputs_rejected"] = len(malformed)
    result["capture_proven"] = False
    result["isolated_process_delivery_proven"] = False
    result["source"] = "VirtualBox guest EFI USB_IO; not host capture"
    (root / "descriptor-report.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
