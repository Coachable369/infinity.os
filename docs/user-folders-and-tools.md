# Personal folders, shortcuts, and system commands

Each configured identity owns Documents, Downloads, Pictures, Music, Videos,
Media, Projects, and Trash under its display-name home. Existing accounts gain
missing folders when their home is provisioned. The installer carries the same
code and bundled resources as live boot.

Music contains Chime.wav and Melody.mp3; Videos contains Motion.mp4. Media exposes
references to those same three objects. Samples are seeded only when the Media
folder is first created, so deliberate deletion does not recreate them on login.
Browser downloads also appear in the display-name Downloads folder while retaining
the identity-addressed reference used by the download manager.

Right-click a File Navigator sidebar location to edit its target using the
Location dialog. Targets must be existing accessible namespace folders. Mappings
are saved in the account-owned `.navigator-sidebar` object, survive reboot, and
do not alter another user's shortcuts. Infinity Storage defaults to `/`.

`/System` is the canonical system namespace. Its immutable installed tools include
`grep.py`, `sed.py`, `awk.py`, `figlet.py`, `wc.py`, `sha256sum.py`, and `jsonfmt.py`.
The standard command PATH discovers them from any working location:

```
wc -l /home/Your.Name/documents/example.py
sha256sum /home/Your.Name/documents/example.py
jsonfmt --sort-keys /home/Your.Name/documents/settings.json
```

The tool runner mounts only authorized input snapshots. These commands require no
host OS filesystem or services. `sdk/system` is a catalog dependency, and release
SDK parity checks enumerate its resources automatically.

Format 10 expands bounded object, namespace and version tables to 128 entries.
Two independent 40 KiB table banks are allocated through the native object
allocator. A commit writes the inactive checksummed bank before publishing its
root. Formats 4–9 remain readable and migrate on their next commit without moving
object content. Focused checks cover legacy migration, interrupted writes, table
corruption, ownership, remounts and capacity rollback.

Media Player participates in the same desktop stacking order as shell windows.
Native repaint notifications do not acquire focus; clicking an exposed window
raises it and invalidates composition once.
