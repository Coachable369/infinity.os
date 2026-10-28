"""Bounded real native child-Python adapter for upstream pip's build hooks.

Not a shell, fork emulation or host subprocess bridge. The kernel admits it only
inside an explicitly authorized pip transaction and prohibits recursive children.
"""
import io
import json
import os
import tempfile
import time
import _infinity_process as _native

class Popen:
    # ------------------------=
    # FUNC: __init__
    # DESC: Starts a separately isolated native CPython child from a bounded private launch record, never evaluates the hook in this interpreter.
    # ------------------=
    def __init__(self, args, bufsize=-1, executable=None, stdin=None, stdout=None,
                 stderr=None, preexec_fn=None, close_fds=True, shell=False,
                 cwd=None, env=None, universal_newlines=None, startupinfo=None,
                 creationflags=0, restore_signals=True, start_new_session=False,
                 pass_fds=(), *, encoding=None, errors=None, text=None, **kwargs):
        import subprocess
        if shell or executable is not None or preexec_fn is not None or pass_fds or kwargs \
                or startupinfo is not None or creationflags or start_new_session:
            raise NotImplementedError('Only confined native Python build children are supported')
        args = list(args)
        if not args or args[0] != '/system/python' or not all(isinstance(arg, str) for arg in args):
            raise FileNotFoundError(2, 'Only /system/python is available for native build children')
        if stdin not in (None, subprocess.PIPE, subprocess.DEVNULL) \
                or stdout not in (None, subprocess.PIPE, subprocess.DEVNULL) \
                or stderr not in (None, subprocess.PIPE, subprocess.DEVNULL, subprocess.STDOUT):
            raise NotImplementedError('Native build children do not inherit arbitrary descriptors')
        self.args, self.returncode = args, None
        self._text = bool(text or universal_newlines or encoding or errors)
        self._encoding = 'utf-8' if encoding in (None, 'locale') else encoding
        self._errors = errors or 'strict'
        self._stdout_mode, self._stderr_mode = stdout, stderr
        self._stdout_data = self._stderr_data = None
        self._record = None
        self.stdin = _Input(self._text) if stdin == subprocess.PIPE else None
        self.stdout = _Output(self, False) if stdout == subprocess.PIPE else None
        self.stderr = _Output(self, True) if stderr == subprocess.PIPE else None
        record = json.dumps({'argv': args, 'env': dict(os.environ if env is None else env),
                             'cwd': os.getcwd() if cwd is None else os.fspath(cwd)}).encode('utf-8')
        if len(record) > 16384:
            raise ValueError('Child launch record exceeds 16 KiB')
        descriptor, path = tempfile.mkstemp(prefix='python-child-', suffix='.json')
        self._record = path
        try:
            with os.fdopen(descriptor, 'wb') as stream:
                stream.write(record)
            self.pid = _native.start(path.encode('utf-8'))
        except BaseException:
            os.unlink(path)
            self._record = None
            raise

    # ------------------------=
    # FUNC: poll
    # DESC: Reads the real child lifecycle receipt; success is never inferred from output or lack of an exception.
    # ------------------=
    def poll(self):
        if self.returncode is not None:
            return self.returncode
        receipt = _native.poll(self.pid)
        if receipt is None:
            return None
        status, out_size, err_size = receipt
        out = _native.read(self.pid, 0, 0, out_size)
        err = _native.read(self.pid, 1, 0, err_size)
        _native.release(self.pid)
        self.returncode = status
        import subprocess
        self._stdout_data = out + err if self._stderr_mode == subprocess.STDOUT else out
        self._stderr_data = err
        if self._record is not None:
            os.unlink(self._record)
            self._record = None
        if self._stdout_mode is None and out:
            os.write(1, out)
        if self._stderr_mode is None and err:
            os.write(2, err)
        return status

    # ------------------------=
    # FUNC: wait
    # DESC: Waits cooperatively for the actual isolated child while the desktop continues scheduling native slices.
    # ------------------=
    def wait(self, timeout=None):
        import subprocess
        deadline = None if timeout is None else time.monotonic() + timeout
        while self.poll() is None:
            if deadline is not None and time.monotonic() >= deadline:
                raise subprocess.TimeoutExpired(self.args, timeout)
            time.sleep(0.001)
        return self.returncode

    # ------------------------=
    # FUNC: communicate
    # DESC: Returns captured real child streams, rejecting unsupported interactive pipe input instead of discarding it.
    # ------------------=
    def communicate(self, input=None, timeout=None):
        if input:
            raise NotImplementedError('Native build children do not accept interactive stdin')
        self.wait(timeout)
        out, err = self._stdout_data, self._stderr_data
        if self._text:
            out, err = out.decode(self._encoding, self._errors), err.decode(self._encoding, self._errors)
        return (out if self.stdout is not None else None, err if self.stderr is not None else None)

    # ------------------------=
    # FUNC: kill
    # DESC: Cancels the exact native child and waits for its memory and file views to be reclaimed.
    # ------------------=
    def kill(self):
        if self.returncode is None:
            _native.cancel(self.pid)
    terminate = kill

    # ------------------------=
    # FUNC: __enter__
    # DESC: Preserves the context-manager contract used by standard subprocess helpers.
    # ------------------=
    def __enter__(self):
        return self

    # ------------------------=
    # FUNC: __exit__
    # DESC: Waits for real completion and closes retained stream wrappers on every context exit.
    # ------------------=
    def __exit__(self, *args):
        for stream in (self.stdin, self.stdout, self.stderr):
            if stream is not None:
                stream.close()
        self.wait()

class _Input(io.IOBase):
    # ------------------------=
    # FUNC: __init__
    # DESC: Represents the explicitly unsupported child input channel without silently buffering or discarding caller data.
    # ------------------=
    def __init__(self, text):
        self._text = text

    # ------------------------=
    # FUNC: write
    # DESC: Accepts an empty end-of-input only; rejects nonempty data because native build children currently receive EOF.
    # ------------------=
    def write(self, data):
        if self.closed:
            raise ValueError('I/O operation on closed file')
        if self._text:
            if not isinstance(data, str):
                raise TypeError('write() argument must be str')
        else:
            data = memoryview(data)
        if len(data):
            raise NotImplementedError('Native build children do not accept interactive stdin')
        return 0

class _Output(io.RawIOBase):
    # ------------------------=
    # FUNC: __init__
    # DESC: Represents actual captured child output with a lazy stream reader, not synthetic command text.
    # ------------------=
    def __init__(self, process, errors):
        self.process, self.errors, self.stream = process, errors, None

    # ------------------------=
    # FUNC: _ready
    # DESC: Loads bounded completed child bytes once and decodes only when the caller requested text mode.
    # ------------------=
    def _ready(self):
        if self.stream is None:
            self.process.wait()
            data = self.process._stderr_data if self.errors else self.process._stdout_data
            self.stream = io.StringIO(data.decode(self.process._encoding, self.process._errors)) \
                if self.process._text else io.BytesIO(data)

    # ------------------------=
    # FUNC: read
    # DESC: Reads only remaining actual child-output bytes or characters.
    # ------------------=
    def read(self, size=-1):
        self._ready()
        return self.stream.read(size)

    # ------------------------=
    # FUNC: readline
    # DESC: Preserves line boundaries for pip's existing subprocess diagnostics consumer.
    # ------------------=
    def readline(self, size=-1):
        self._ready()
        return self.stream.readline(size)

    # ------------------------=
    # FUNC: readable
    # DESC: Marks the captured output view as readable without claiming it is an OS-selectable pipe.
    # ------------------=
    def readable(self):
        return True

# ------------------------=
# FUNC: install
# DESC: Installs the native child implementation into upstream subprocess only for explicitly launched pip processes.
# ------------------=
def install():
    import subprocess
    subprocess.Popen = Popen
