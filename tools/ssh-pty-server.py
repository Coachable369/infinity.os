"""Independent loopback SSH server and real PTY for acceptance; never shipped in InfinityOS.
Requires test-only AsyncSSH in build/ssh-pty-env. Matches the production AEAD cipher policy.
"""
import argparse
import asyncio
import fcntl
import json
import os
from pathlib import Path
import signal
import struct
import subprocess
import termios
import asyncssh

class Authority(asyncssh.SSHServer):
    # ------------------------=
    # FUNC: begin_auth
    # DESC: Always requires authentication for the isolated fixture.
    # ------------------=
    def begin_auth(self, username):
        return True

    # ------------------------=
    # FUNC: password_auth_supported
    # DESC: Enables the fixture's non-production password authentication.
    # ------------------=
    def password_auth_supported(self):
        return True

    # ------------------------=
    # FUNC: validate_password
    # DESC: Validates an explicit fixture credential without logging secrets.
    # ------------------=
    def validate_password(self, username, password):
        return username == 'fixture' and password == 'TerminalProof901'

# ------------------------=
# FUNC: serve
# DESC: Bridges one authenticated independent SSH connection to a real controlling shell PTY.
# ------------------=
async def serve(work, port):
    key=asyncssh.generate_private_key('ssh-ed25519')
    raw=key.export_public_key('openssh').split()[1]
    import base64
    (work/'host-key.raw').write_bytes(base64.b64decode(raw)[-32:])
    (work/'host-key.pub').write_bytes(key.export_public_key('openssh'))
    done=asyncio.Event()
    # ------------------------=
    # FUNC: process
    # DESC: Preserves raw bytes, window changes and terminal signals; records typed protocol events.
    # ------------------=
    async def process(proc):
        master,slave=os.openpty()
        state={'password':True,'pty':list(proc.term_size[:2]),'shell':False,'resizes':[],'input_bytes':0}
        # ------------------------=
        # FUNC: record
        # DESC: Publishes structured fixture events atomically for the acceptance driver.
        # ------------------=
        def record():
            temporary=work/'pty-events.tmp'
            temporary.write_text(json.dumps(state));temporary.replace(work/'pty-events.json')
        # ------------------------=
        # FUNC: resize
        # DESC: Applies protocol dimensions to the actual PTY.
        # ------------------=
        def resize(width,height):
            fcntl.ioctl(master,termios.TIOCSWINSZ,struct.pack('HHHH',height,width,0,0))
        # ------------------------=
        # FUNC: setup
        # DESC: Installs the child's controlling terminal for real interactive job control.
        # ------------------=
        def setup():
            os.setsid();fcntl.ioctl(slave,termios.TIOCSCTTY,0)
        assert proc.term_type=='vt100' and all(state['pty'])
        resize(*state['pty'])
        child=subprocess.Popen(['/bin/sh','-i'],stdin=slave,stdout=slave,stderr=slave,cwd=work,
            env=dict(os.environ,TERM='vt100',PS1='fixture$ '),preexec_fn=setup)
        os.close(slave)
        state['shell']=True;record()
        # ------------------------=
        # FUNC: receive
        # DESC: Sends SSH channel input and resize events to the PTY without local echo.
        # ------------------=
        async def receive():
            while True:
                try:
                    data=await proc.stdin.read(4096)
                except asyncssh.TerminalSizeChanged as change:
                    resize(change.width,change.height);state['resizes'].append([change.width,change.height]);record();continue
                if not data:return
                state['input_bytes']+=len(data);record()
                while data:
                    n=await asyncio.to_thread(os.write,master,data);data=data[n:]
        # ------------------------=
        # FUNC: transmit
        # DESC: Preserves shell output and SSH backpressure.
        # ------------------=
        async def transmit():
            while child.poll() is None:
                try:data=await asyncio.to_thread(os.read,master,4096)
                except OSError:return
                if not data:return
                proc.stdout.write(data);await proc.stdout.drain()
        tasks=[asyncio.create_task(receive()),asyncio.create_task(transmit())]
        try:
            await asyncio.wait(tasks,timeout=300,return_when=asyncio.FIRST_COMPLETED)
        finally:
            if child.poll() is None:os.killpg(child.pid,signal.SIGHUP)
            await asyncio.to_thread(child.wait)
            for task in tasks:task.cancel()
            await asyncio.gather(*tasks,return_exceptions=True)
            os.close(master);record();proc.exit(max(0,child.returncode));done.set()
    async with await asyncssh.create_server(Authority,'127.0.0.1',port,server_host_keys=[key],
        process_factory=process,encoding=None,line_editor=False,encryption_algs=['chacha20-poly1305@openssh.com']):
        (work/'ready.json').write_text(json.dumps({'port':port}))
        await asyncio.wait_for(done.wait(),300)

# ------------------------=
# FUNC: main
# DESC: Runs a disposable fixture exclusively under the repository's build directory.
# ------------------=
def main():
    parser=argparse.ArgumentParser();parser.add_argument('work',type=Path);parser.add_argument('--port',type=int,default=22444)
    args=parser.parse_args();args.work.mkdir(parents=True,exist_ok=True);asyncio.run(serve(args.work.resolve(),args.port))

if __name__=='__main__':main()
