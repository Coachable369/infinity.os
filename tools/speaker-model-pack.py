#!/usr/bin/env python3
"""Pack the pinned WeSpeaker ResNet34 trunk for the native bounded Rust executor.
The statistical pooling and projection head remain explicit in speaker-core.
This is a build-time converter, not a guest ONNX dependency.
"""
import hashlib
import importlib.util
import math
import os
from pathlib import Path
import struct

PIN = '5ef208a9da1453335308a6b6f4e6dfbd7e183a38b604de0a57664f45d257fe94'
FRAMES = 200

# ------------------------=
# FUNC: main
# DESC: Validates pinned model bytes and emits bounded native trunk and projection data.
# ------------------=
def main():
    if os.environ.get('INFINITY_BUILD_KIT_ACTIVE') != '1':
        raise SystemExit('Use ./build-kit run python3 tools/speaker-model-pack.py')
    root = Path(__file__).resolve().parents[1]
    spec = importlib.util.spec_from_file_location('face_pack', root/'tools/face-model-pack.py')
    p = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(p)
    raw = (root/'model-cache/speaker/resnet34.onnx').read_bytes()
    if hashlib.sha256(raw).hexdigest() != PIN:
        raise ValueError('speaker model digest mismatch')
    graph = list(p.fields(p.one(list(p.fields(raw)), 7)))
    constants = {}
    for number, raw_tensor in graph:
        if number != 5:
            continue
        f = list(p.fields(raw_tensor))
        name, dtype = p.one(f, 8).decode(), p.one(f, 2)
        if dtype != 1:
            raise ValueError('unexpected non-float initializer')
        data = p.one(f, 9)
        shape = p.integers(f, 1)
        if math.prod(shape)*4 != len(data):
            raise ValueError('initializer size')
        constants[name] = (shape, data)
    tensors, nodes, ids, weights = [], [], {}, bytearray()
    arena = 0

    # ------------------------=
    # FUNC: add
    # DESC: Allocates a bounded tensor with immutable weight or private activation storage.
    # ------------------=
    def add(name, shape, data=None):
        nonlocal arena
        size = p.count(shape)
        offset = len(weights)//4 if data is not None else arena
        if data is not None:
            weights.extend(data)
        else:
            arena += size
        ids[name] = len(tensors)
        tensors.append((len(shape), *(shape+[1]*(4-len(shape))), int(data is not None), offset, size))
        return ids[name]

    # Input is already transposed to NCHW by the native filterbank frontend.
    last = None
    for number, raw_node in graph:
        if number != 1:
            continue
        f = list(p.fields(raw_node))
        op = p.one(f, 4).decode()
        inputs = [v.decode() for k, v in f if k == 1]
        output = p.one(f, 2).decode()
        if op == 'Unsqueeze':
            add(output, [1, 1, 80, FRAMES])
            continue
        if not tensors:
            continue
        if op == 'ReduceMean':
            break
        if op not in ('Conv', 'Relu', 'Add'):
            raise ValueError(('unexpected trunk operation', op))
        attr = p.attributes(f)
        source = tensors[ids[inputs[0]]]
        shape = list(source[1:1+source[0]])
        params = [0]*8
        if op == 'Conv':
            kh, kw = attr['kernel_shape']
            sy, sx = attr.get('strides', [1,1])
            top, left, bottom, right = attr.get('pads', [0]*4)
            group = attr.get('group', 1)
            ws = constants[inputs[1]][0]
            if attr.get('dilations', [1,1]) != [1,1] or group != 1 or ws[1] != shape[1]:
                raise ValueError('convolution shape')
            shape = [1, ws[0], (shape[2]+top+bottom-kh)//sy+1, (shape[3]+left+right-kw)//sx+1]
            params = [kh,kw,sy,sx,top,left,group,0]
        for name in inputs:
            if name not in ids:
                add(name, *constants[name])
        last = add(output, shape)
        inp = [ids[name] for name in inputs]
        nodes.append((p.OPS[op],last,len(inp),*(inp+[0]*(5-len(inp))),*params))
    if tensors[last][1:5] != (1,256,10,25):
        raise ValueError(('trunk output', tensors[last]))
    if arena > 32*1024*1024 or len(nodes)>256 or len(tensors)>512:
        raise ValueError('native graph budget')
    header = b'IFACE001'+bytes.fromhex(PIN)+struct.pack('<6I',2,len(tensors),len(nodes),1,arena,len(weights))
    trunk = header+b''.join(struct.pack('<8I',*t) for t in tensors)
    trunk += b''.join(struct.pack('<16I',*n) for n in nodes)+struct.pack('<I',last)+weights
    head = bytearray(b'ISPKHEAD'+bytes.fromhex(PIN))
    for name, shape in [('model.seg_1.weight',[256,5120]),('model.seg_1.bias',[256]),('mean_vec',[256])]:
        actual, data = constants[name]
        if actual != shape:
            raise ValueError((name, actual))
        head.extend(data)
    out = root/'build/speaker/models'
    out.mkdir(parents=True, exist_ok=True)
    (out/'resnet34.iface').write_bytes(trunk)
    (out/'resnet34.head').write_bytes(head)
    print({'trunk_bytes':len(trunk),'head_bytes':len(head),'arena_bytes':arena*4,'nodes':len(nodes)})

if __name__ == '__main__':
    main()
