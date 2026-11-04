# SSH password login and outbound connections

InfinityOS accepts an active native account's password or registered Ed25519
public key. Passwords are checked against the existing salted account verifier;
there is no separate SSH password database. Disabled accounts and revoked or
replaced credentials lose access. Repeated failed password attempts incur an
account-scoped delay, including attempts over new TCP connections.

From your Mac, with the default VirtualBox forwarding rule:

```sh
ssh -p 22222 Jonathan@127.0.0.1
```

Verify the displayed host fingerprint against `ssh status` in InfinityOS before
accepting a new host. Enter your InfinityOS account password when prompted.
Passwords are not command arguments. SSH must be enabled, and the account must
already exist in the installed system. Replacing an ISO does not update the
kernel of an existing installed VM.

Inbound sessions retain the restricted native Console permissions used by key
authentication: `help`, `system status`, `node list`, and `ai status`. Password
authentication does not grant a Unix shell, administrative permissions, file
transfer, forwarding, or access to the desktop session.

## Connect from InfinityOS

Open Command Window while signed in locally:

```text
ssh user@server.example.com
ssh user@192.168.1.20 -p 2222
```

Names resolve through the native network service. Before sending credentials,
the terminal displays the server's Ed25519 SHA-256 fingerprint. Verify it with
the server administrator, then press **Y**; **N** or **Escape** cancels. Trust is
limited to that connection, so the next connection asks again. The password
prompt masks input and bypasses local command history, transcript, completion,
and clipboard shortcuts. Three rejected passwords end the attempt.

This initial client provides a **line-mode remote shell**: type a command and
press Enter; Escape disconnects. It is not a full-screen VT terminal and does
not offer SCP/SFTP, forwarding, keyboard-interactive authentication, or remote
public-key authentication. Servers must offer Ed25519 host keys and SSH password
authentication. Logging out or locking the local session revokes the connection.

TCP/SSH processing is bounded and shares the native NIC pump. Established SSH
connections have independent buffers and do not reserve the browser actor.
Only name resolution briefly uses the shared DNS service; a busy resolver is
reported as a connection-start failure rather than cancelling another request.

## Verification

`./build-kit ssh-tests` exercises real encrypted client/server password exchange,
host confirmation, incorrect-password retry, partial network delivery, shell
input/output, credential revocation, native account throttling, and restricted
session permissions. Native architecture checks and installed-VM acceptance
are separate from these protocol tests; consult the build manifest and recorded
VM results rather than assuming an older ISO contains this implementation.
