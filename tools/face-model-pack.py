#!/usr/bin/env python3
"""Build-time-only fixed YuNet/SFace packer. No ONNX/Python runtime in the guest.

Only the two audited hashes are accepted. Unsupported operators/attributes fail
closed; this is not a general ONNX importer. Invoke via ./build-kit run.
"""
import argparse
import hashlib
import math
import os
from pathlib import Path
import struct

PINS = {
    "yunet": "8f2383e4dd3cfbb4553ea8718107fc0423210dc964f9f4280604804ed2552fa4",
    "sface": "0ba9fbfa01b5270c96627c4ef784da859931e02f04419c829e83484087c34e79",
}
OPS = {"Conv": 1, "Relu": 2, "Sigmoid": 3, "MaxPool": 4, "Resize": 5,
       "Add": 6, "Sub": 7, "Mul": 8, "PRelu": 9, "Affine": 10,
       "Transpose": 11, "Gemm": 12, "Copy": 13}

# ------------------------=
# FUNC: varint
# DESC: Reads one bounded protobuf integer, rejecting truncation and overlong input.
# ------------------=
def varint(data, at):
    value = 0
    for shift in range(0, 70, 7):
        if at >= len(data):
            raise ValueError("truncated varint")
        byte = data[at]
        at += 1
        value |= (byte & 127) << shift
        if byte < 128:
            return value, at
    raise ValueError("overlong varint")

# ------------------------=
# FUNC: fields
# DESC: Decodes the nonrecursive protobuf wire fields needed by the pinned models.
# ------------------=
def fields(data):
    at = 0
    while at < len(data):
        key, at = varint(data, at)
        number, wire = key >> 3, key & 7
        if not number:
            raise ValueError("invalid field")
        if wire == 0:
            value, at = varint(data, at)
        else:
            if wire == 2:
                length, at = varint(data, at)
            elif wire in (1, 5):
                length = 8 if wire == 1 else 4
            else:
                raise ValueError("unsupported wire type")
            if length > len(data) - at:
                raise ValueError("truncated field")
            value = data[at:at + length]
            at += length
        yield number, value

# ------------------------=
# FUNC: integers
# DESC: Supports repeated and packed protobuf integer fields with signed int64 interpretation.
# ------------------=
def integers(items, field):
    result = []
    for number, value in items:
        if number != field:
            continue
        if isinstance(value, int):
            result.append(value)
        else:
            at = 0
            while at < len(value):
                v, at = varint(value, at)
                result.append(v)
    return [v - (1 << 64) if v >= (1 << 63) else v for v in result]

# ------------------------=
# FUNC: one
# DESC: Retrieves exactly one required field without silently choosing duplicate model metadata.
# ------------------=
def one(items, field):
    values = [v for n, v in items if n == field]
    if len(values) != 1:
        raise ValueError(("required field", field, len(values)))
    return values[0]

# ------------------------=
# FUNC: attributes
# DESC: Reads scalar/list attributes used by the pinned graph, rejecting unsupported attribute kinds.
# ------------------=
def attributes(node):
    result = {}
    for n, data in node:
        if n != 5:
            continue
        f = list(fields(data))
        name = one(f, 1).decode()
        kind = one(f, 20)
        if name in result:
            raise ValueError("duplicate attribute")
        if kind == 1:
            value = struct.unpack("<f", one(f, 2))[0]
        elif kind == 2:
            value = one(f, 3)
        elif kind == 3:
            value = one(f, 4).decode()
        elif kind == 7:
            value = integers(f, 8)
        else:
            raise ValueError(("unsupported attribute", kind))
        result[name] = value
    return result

# ------------------------=
# FUNC: count
# DESC: Checks positive fixed tensor dimensions and a per-tensor allocation ceiling.
# ------------------=
def count(shape):
    if not 1 <= len(shape) <= 4 or any(v <= 0 for v in shape):
        raise ValueError(("shape", shape))
    size = math.prod(shape)
    if size > 16 * 1024 * 1024:
        raise ValueError("tensor exceeds budget")
    return size

# ------------------------=
# FUNC: pack
# DESC: Converts only pinned model graphs into bounded float32 tensors and fixed native operation records.
# ------------------=
def pack(kind, path, destination):
    source = path.read_bytes()
    digest = hashlib.sha256(source).hexdigest()
    if digest != PINS[kind]:
        raise ValueError("model hash mismatch")
    graph = list(fields(one(list(fields(source)), 7)))
    constants = {}
    for n, raw in graph:
        if n != 5:
            continue
        f = list(fields(raw))
        name, dtype = one(f, 8).decode(), one(f, 2)
        shape = integers(f, 1) or [1]
        raw_values = [v for k, v in f if k == 9]
        if dtype == 1:
            if raw_values:
                values = list(struct.unpack("<" + "f" * (len(raw_values[0]) // 4), raw_values[0]))
            else:
                values = []
                for k, v in f:
                    if k == 4:
                        values.extend(struct.unpack("<" + "f" * (len(v) // 4), v))
        elif dtype == 7:
            values = list(struct.unpack("<" + "q" * (len(raw_values[0]) // 8), raw_values[0])) if raw_values else integers(f, 7)
        else:
            raise ValueError(("tensor type", dtype))
        if math.prod(shape) != len(values) or not all(math.isfinite(v) for v in values):
            raise ValueError("tensor values")
        if name in constants:
            raise ValueError("duplicate initializer")
        constants[name] = (shape, values)
    tensor = []
    ids = {}
    weights = bytearray()
    arena = 0

    # ------------------------=
    # FUNC: add
    # DESC: Registers a tensor in immutable weight storage or the explicitly bounded worker arena.
    # ------------------=
    def add(name, shape, values=None):
        nonlocal arena
        if name in ids:
            raise ValueError(("duplicate tensor", name))
        length = count(shape)
        storage = 1 if values is not None else 0
        offset = len(weights) // 4 if storage else arena
        if storage:
            if len(values) != length:
                raise ValueError("constant length")
            weights.extend(struct.pack("<" + "f" * length, *values))
        else:
            arena += length
        ids[name] = len(tensor)
        tensor.append((len(shape), *(shape + [1] * (4-len(shape))), storage, offset, length))
        return ids[name]

    real_inputs = [one(list(fields(v)), 1).decode() for n, v in graph if n == 11]
    real_inputs = [name for name in real_inputs if name not in constants]
    if len(real_inputs) != 1:
        raise ValueError("expected one image input")
    add(real_inputs[0], [1, 3, 320, 320] if kind == "yunet" else [1, 3, 112, 112])
    nodes = []
    for n, raw in graph:
        if n != 1:
            continue
        f = list(fields(raw))
        op, attr = one(f, 4).decode(), attributes(f)
        inputs = [v.decode() for k, v in f if k == 1]
        outputs = [v.decode() for k, v in f if k == 2]
        if len(outputs) != 1:
            raise ValueError("multi-output node")
        shape = list(tensor[ids[inputs[0]]][1:1+tensor[ids[inputs[0]]][0]])
        params = [0]*8
        if op in ("Reshape", "Flatten", "Dropout"):
            if op == "Reshape":
                target = list(constants[inputs[1]][1])
                target = [shape[i] if v == 0 else v for i, v in enumerate(target)]
                if target.count(-1) > 1:
                    raise ValueError("reshape")
                if -1 in target:
                    target[target.index(-1)] = math.prod(shape)//-math.prod(target)
                if math.prod(target) != math.prod(shape):
                    raise ValueError("reshape size")
                shape = target
            elif op == "Flatten":
                axis = attr.get("axis", 1)
                shape = [math.prod(shape[:axis]), math.prod(shape[axis:])]
            inputs = inputs[:1]
            op = "Copy"
        elif op == "BatchNormalization":
            gamma, beta, mean, variance = [constants[name][1] for name in inputs[1:]]
            epsilon = attr.get("epsilon", 1e-5)
            if any(v+epsilon <= 0 for v in variance):
                raise ValueError("batchnorm variance")
            scale = [g/math.sqrt(v+epsilon) for g, v in zip(gamma, variance)]
            bias = [b-m*s for b, m, s in zip(beta, mean, scale)]
            if not len(scale) == len(bias) == shape[1]:
                raise ValueError("batchnorm channels")
            a, b = outputs[0]+".scale", outputs[0]+".bias"
            constants[a] = ([len(scale)], scale)
            constants[b] = ([len(bias)], bias)
            inputs = [inputs[0], a, b]
            op = "Affine"
        elif op in ("Conv", "MaxPool"):
            if len(shape) != 4 or shape[0] != 1 or attr.get("dilations", [1, 1]) != [1, 1] or attr.get("ceil_mode", 0) != 0:
                raise ValueError("convolution geometry")
            kh, kw = attr["kernel_shape"]
            sy, sx = attr.get("strides", [1, 1])
            top, left, bottom, right = attr.get("pads", [0]*4)
            group = attr.get("group", 1)
            if op == "Conv":
                ws = constants[inputs[1]][0]
                if ws[1]*group != shape[1] or ws[2:] != [kh, kw] or ws[0]%group:
                    raise ValueError("convolution weights")
                shape[1] = ws[0]
            shape[2:] = [(shape[2]+top+bottom-kh)//sy+1, (shape[3]+left+right-kw)//sx+1]
            params = [kh, kw, sy, sx, top, left, group, 0]
        elif op == "Resize":
            if attr.get("mode") != "nearest" or attr.get("coordinate_transformation_mode") != "asymmetric" or attr.get("nearest_mode") != "floor":
                raise ValueError("resize mode")
            scales = constants[inputs[2]][1]
            if scales[:2] != [1, 1] or len(scales) != 4:
                raise ValueError("resize shape")
            shape = [int(v*s) for v, s in zip(shape, scales)]
            inputs = inputs[:1]
        elif op == "Transpose":
            perm = attr["perm"]
            if sorted(perm) != list(range(len(shape))):
                raise ValueError("permutation")
            shape = [shape[i] for i in perm]
            params[:len(perm)] = perm
        elif op == "Gemm":
            if attr.get("transA", 0) != 0 or attr.get("alpha", 1) != 1 or attr.get("beta", 1) != 1 or len(shape) != 2:
                raise ValueError("gemm mode")
            b = constants[inputs[1]][0]
            trans = attr.get("transB", 0)
            if len(b) != 2 or shape[1] != b[1 if trans else 0]:
                raise ValueError("gemm shape")
            shape = [shape[0], b[0 if trans else 1]]
            params[0] = trans
        elif op not in OPS:
            raise ValueError(("unsupported operator", op))
        for name in inputs:
            if name not in ids:
                add(name, *constants[name])
        output = add(outputs[0], shape)
        input_ids = [ids[name] for name in inputs]
        if not 1 <= len(input_ids) <= 5:
            raise ValueError("input arity")
        nodes.append((OPS[op], output, len(input_ids), *(input_ids+[0]*(5-len(input_ids))), *params))
    output_ids = [ids[one(list(fields(v)), 1).decode()] for n, v in graph if n == 12]
    if len(tensor) > 512 or len(nodes) > 256 or arena*4 > 128*1024*1024 or len(weights) > 40*1024*1024:
        raise ValueError(("worker budget exceeded", len(tensor), len(nodes), arena*4, len(weights)))
    header = b"IFACE001" + bytes.fromhex(digest) + struct.pack("<6I", 1 if kind == "yunet" else 2,
        len(tensor), len(nodes), len(output_ids), arena, len(weights))
    encoded = header + b"".join(struct.pack("<8I", *t) for t in tensor)
    encoded += b"".join(struct.pack("<16I", *node) for node in nodes)
    encoded += struct.pack("<"+"I"*len(output_ids), *output_ids) + weights
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary=destination.with_suffix(destination.suffix+f".{os.getpid()}.tmp")
    try:
        temporary.write_bytes(encoded)
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)
    print({"model": kind, "nodes": len(nodes), "tensors": len(tensor), "arena_bytes": arena*4,
           "packed_bytes": len(encoded), "sha256": hashlib.sha256(encoded).hexdigest()})

# ------------------------=
# FUNC: main
# DESC: Packs a pinned local model through repository build authority, without fetching or executing model code.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use the repository build kit")
    parser = argparse.ArgumentParser()
    parser.add_argument("model", choices=PINS)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    if root not in args.destination.resolve().parents:
        parser.error("output must stay inside the repository")
    pack(args.model, args.source, args.destination)

if __name__ == "__main__":
    main()
