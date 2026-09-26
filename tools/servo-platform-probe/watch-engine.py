"""Bounded native engine memory-write diagnosis through QEMU's debugger."""
import os
from pathlib import Path
import socket
import subprocess
import time
import argparse

# ------------------------=
# FUNC: packet
# DESC: Sends one debugger request and returns its checksummed response payload.
# ------------------=
def packet(connection, payload):
    data = payload.encode()
    connection.sendall(b"$" + data + b"#" + f"{sum(data) % 256:02x}".encode())
    while True:
        prefix = connection.recv(1)
        if prefix == b"$":
            break
        if not prefix:
            raise RuntimeError("Debugger disconnected")
    response = bytearray()
    while True:
        byte = connection.recv(1)
        if byte == b"#":
            break
        if not byte:
            raise RuntimeError("Debugger disconnected")
        response.extend(byte)
    checksum = connection.recv(2)
    if int(checksum, 16) != sum(response) % 256:
        raise RuntimeError("Debugger checksum mismatch")
    connection.sendall(b"+")
    return response.decode()

# ------------------------=
# FUNC: main
# DESC: Stops on bounded writes to a specified native address without changing guest memory.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    parser = argparse.ArgumentParser()
    parser.add_argument("address", type=lambda x: int(x, 0))
    parser.add_argument("--execute", action="store_true")
    args = parser.parse_args()
    point = f"{0 if args.execute else 2},{args.address:x},{4 if args.execute else 8}"
    root = Path(__file__).resolve().parents[2]
    output = root / "build/servo-platform-probe"
    endpoint = output / "engine-debug.sock"
    endpoint.unlink(missing_ok=True)
    process = subprocess.Popen(["qemu-system-aarch64", "-machine", "virt", "-accel", "tcg", "-cpu", "max",
        "-m", "2G", "-display", "none", "-serial", "file:" + str(output / "engine-watch.bin"),
        "-monitor", "none", "-S", "-gdb", "unix:" + str(endpoint) + ",server=on,wait=off",
        "-kernel", str(output / "engine-boot.elf")])
    try:
        deadline = time.monotonic() + 5
        while not endpoint.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
            connection.settimeout(30)
            connection.connect(str(endpoint))
            print(packet(connection, "?"), flush=True)
            print(packet(connection, "Z" + point), flush=True)
            for _ in range(8):
                print("stop", packet(connection, "c"), flush=True)
                print("value", packet(connection, f"m{args.address:x},8"), flush=True)
                registers = bytes.fromhex(packet(connection, "g"))
                print("registers", [hex(int.from_bytes(registers[i:i+8], "little")) for i in range(0, 264, 8)], flush=True)
                if args.execute:
                    address = int.from_bytes(registers[:8], "little")
                    argument = bytes.fromhex(packet(connection, f"m{address:x},80"))
                    print("argument", argument.hex(), flush=True)
                    for offset in (0, 8):
                        pointer = int.from_bytes(argument[offset:offset+8], "little")
                        print("indirect", hex(pointer), packet(connection, f"m{pointer:x},80"), flush=True)
                    break
                packet(connection, "z" + point)
                packet(connection, "s")
                packet(connection, "Z" + point)
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        endpoint.unlink(missing_ok=True)

if __name__ == "__main__":
    main()
