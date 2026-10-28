"""Entry point for a real isolated CPython PEP 517 child, never an in-process hook."""
import json
import os
import runpy
import sys

# ------------------------=
# FUNC: _private_path
# DESC: Rejects traversal and embedded NULs before a child path can reach inherited private filesystem views.
# ------------------=
def _private_path(path, roots=('/tmp/', '/work/', '/packages/')):
    return isinstance(path, str) and '\0' not in path and '..' not in path.split('/') \
        and path.startswith(roots)

# ------------------------=
# FUNC: _validate_record
# DESC: Validates a bounded child invocation before applying its environment or changing its working directory.
# ------------------=
def _validate_record(record):
    argv, environment, cwd = record['argv'], record['env'], record['cwd']
    if not isinstance(argv, list) or not 2 <= len(argv) <= 32 or argv[0] != '/system/python':
        raise ValueError('Only the installed native Python executable can run a build child')
    if not all(isinstance(arg, str) and '\0' not in arg and len(arg) <= 4096 for arg in argv):
        raise ValueError('Invalid child argument')
    if not isinstance(environment, dict) or any(not isinstance(k, str) or not isinstance(v, str)
            or '\0' in k or '=' in k or '\0' in v for k, v in environment.items()):
        raise ValueError('Invalid child environment')
    if cwd != '/' and not _private_path(cwd):
        raise PermissionError('Child working directory is outside its inherited grants')
    for directory in environment.get('PYTHONPATH', '').split(os.pathsep):
        if directory and not _private_path(directory, ('/tmp/',)):
            raise PermissionError('Build environment path is not in private scratch')
    args = argv[1:]
    while args and args[0] in ('-u', '-B'):
        args = args[1:]
    if not args:
        raise ValueError('Missing child Python entry point')
    if args[0] in ('-m', '-c'):
        if len(args) < 2:
            raise ValueError('Missing child Python entry point')
    elif not _private_path(args[0]):
        raise PermissionError('Child entry point is not an inherited Python source')
    return args, environment, cwd

# ------------------------=
# FUNC: main
# DESC: Restores only the pip parent's scoped guest environment and executes its requested Python hook in this separate native process.
# ------------------=
def main():
    record_path = sys.argv[1]
    if not _private_path(record_path, ('/tmp/',)):
        raise PermissionError('Child launch records must be in private guest scratch')
    with open(record_path, 'rb') as stream:
        raw = stream.read(16385)
    if len(raw) > 16384:
        raise ValueError('Child launch record exceeds 16 KiB')
    record = json.loads(raw)
    args, environment, cwd = _validate_record(record)
    flags = record['argv'][1:len(record['argv']) - len(args)]
    if '-u' in flags:
        sys.stdout.reconfigure(write_through=True)
        sys.stderr.reconfigure(write_through=True)
    if '-B' in flags:
        sys.dont_write_bytecode = True
    os.environ.clear()
    os.environ.update(environment)
    os.chdir(cwd)
    # A build hook must consume declared build dependencies, not accidentally
    # resolve an unrelated package from the initiating user's installation.
    sys.path[:] = [path for path in sys.path if path != '/packages']
    # Pip creates its private build environment's sitecustomize inside /tmp.
    # Host environment and global site files never enter this isolated process.
    for directory in reversed(environment.get('PYTHONPATH', '').split(os.pathsep)):
        if not directory:
            continue
        sys.path.insert(0, directory)
    if environment.get('PYTHONPATH'):
        import sitecustomize
    if args[0] == '-m' and len(args) >= 2:
        module = args[1]
        sys.argv = args[1:]
        if module == 'pip':
            from _infinity_network import configure_pip
            configure_pip()
        runpy.run_module(module, run_name='__main__', alter_sys=True)
    elif args[0] == '-c' and len(args) >= 2:
        sys.argv = ['-c'] + args[2:]
        exec(compile(args[1], '<native-build-child>', 'exec'), {'__name__': '__main__'})
    elif _private_path(args[0]):
        sys.argv = args
        sys.path.insert(0, os.path.dirname(args[0]))
        runpy.run_path(args[0], run_name='__main__')
    else:
        raise PermissionError('Child entry point is not an inherited Python source')

if __name__ == '__main__':
    main()
