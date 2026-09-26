"""Expose explicitly unavailable GPU contexts for the software-only native port."""
import importlib.util
import os
from pathlib import Path
import re
import subprocess
import tomllib

# ------------------------=
# FUNC: methods
# DESC: Maps pinned trait signatures into uninhabited native GPU types, never successful GPU stubs.
# ------------------=
def methods(source, kind):
    result = []
    for match in re.finditer(r'^    (unsafe )?fn ([a-z_]+)([\s\S]*?);', source, re.M):
        signature = match.group(0).strip()[:-1]
        signature = signature.replace('<Self::Connection as ConnectionInterface>::Adapter', 'Adapter')
        for name in ('Connection', 'ContextDescriptor', 'Context', 'SurfaceTexture', 'Surface', 'NativeWidget', 'Adapter', 'Device'):
            signature = signature.replace('Self::' + name, name)
        signature = re.sub(r'\bSelf\b', kind, signature)
        body = 'match *self {}' if '&self' in signature else 'Err(Error::Unimplemented)'
        result.append('// ------------------------=\n// FUNC: ' + match.group(2) + '\n// DESC: GPU contexts are unavailable in the native software-only backend.\n// ------------------=\npub ' + signature + ' { ' + body + ' }')
    return '\n'.join(result)

# ------------------------=
# FUNC: main
# DESC: Adds fail-closed GPU type declarations while preserving every upstream host backend.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    spec = importlib.util.spec_from_file_location("staging", Path(__file__).with_name("prepare-async-net.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    servo = root / "build/servo-port-audit"
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    directory, package = helper.stage(root, tomllib.loads(original)["package"], "surfman", "0.14.0")
    source = '''use crate::{Error, GLApi, ContextAttributes, ContextID, SurfaceAccess, SurfaceInfo, SurfaceType};
use crate::connection::Connection as ConnectionInterface;
use crate::implement_interfaces;
use euclid::default::Size2D;
use glow::Texture;
use std::os::raw::c_void;
'''
    for name in ('Connection', 'Device', 'Adapter', 'Context', 'ContextDescriptor', 'Surface', 'SurfaceTexture', 'NativeWidget'):
        source += '#[derive(Clone, Debug)]\npub enum ' + name + ' {}\n'
    for name in ('Connection', 'Device'):
        source += 'impl ' + name + ' {\n' + methods((directory / ('src/' + name.lower() + '.rs')).read_text(), name) + '\n}\n'
    for module, names in (('connection', 'Connection'), ('device', 'Adapter, Device'), ('context', 'Context, ContextDescriptor'), ('surface', 'Surface, SurfaceTexture, NativeWidget')):
        source += 'pub mod ' + module + ' { pub use super::{' + names + '}; }\n'
    source += 'implement_interfaces!();\n'
    (directory / "src/infinity.rs").write_text(source)
    path = directory / "src/lib.rs"
    path.write_text(path.read_text().replace('pub use default::connection::Connection;', '#[cfg(all(target_os = "none", infinity_native))]\npub mod infinity;\n#[cfg(all(target_os = "none", infinity_native))]\npub use infinity as default;\npub use default::connection::Connection;', 1))
    lock = servo / "Cargo.lock"
    header = 'name = "surfman"\nversion = "0.14.0"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))

if __name__ == "__main__":
    main()
