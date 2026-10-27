"""Pack licensed canonical face geometry for the allocation-free native painter."""
from pathlib import Path
import struct
import hashlib

# ------------------------=
# FUNC: main
# DESC: Packs static vertices and triangles; no runtime OBJ parser or framework is required.
# ------------------=
def main():
    root = Path(__file__).resolve().parents[1]
    folder = root / 'assets/ui-design-kit/default'
    vertices, faces = [], []
    source = (folder / 'face-canonical-v1.obj').read_bytes()
    assert hashlib.sha256(source).hexdigest() == '8bac80443397e113f41a8b565ea72c59390bc031d9defab289dba7bc0c54e618'
    for line in source.decode().splitlines():
        fields = line.split()
        if fields[0] == 'v':
            vertices.append(tuple(map(float, fields[1:4])))
        elif fields[0] == 'f':
            faces.append(tuple(int(x.split('/')[0])-1 for x in fields[1:4]))
    assert len(vertices) == 468 and all(max(f) < 468 for f in faces)
    data = struct.pack('<II', len(vertices), len(faces))
    data += b''.join(struct.pack('<fff', *v) for v in vertices)
    data += b''.join(struct.pack('<HHH', *f) for f in faces)
    (folder / 'face-canonical-v1.mesh').write_bytes(data)

if __name__ == '__main__':
    main()
