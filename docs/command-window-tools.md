# Command Window tools

`top` displays live execution contexts, CPU share, memory use, state and name.
The selected row is cyan; active rows change color as CPU use increases.
Use the arrows to select a process and **Q** to return to the prompt.

The shell searches the session’s `PATH` for Python scripts after built-in
commands. Type `PATH` to inspect it. The default is `/System`. For example,
`PATH=$PATH;/home/default/documents` appends another search directory;
`PATH=/System` resets it. Relative entries resolve against the current
namespace. A script may be invoked with or without its `.py` extension.

Tab completes object names. For example, at the namespace root type
`CD A` and press Tab. Directory commands offer only directories. Multiple
matches are listed and the shared prefix is inserted; Tab does not execute
the command.

The shell reads `/System/motd` when opening a new Command Window.
An administrator can change it with `motd set Welcome to the lab`.

| Example | Operation |
|---|---|
| `grep -in 'warning' notes.txt` | Print matching lines with line numbers. |
| `sed 's/old/new/g' notes.txt` | Print text with substitutions; leave the source unchanged. |
| `awk -F ',' '{print $2,$1}' contacts.csv` | Print selected fields. Also supports NR, NF and $0. |
| `figlet -f small Infinity OS` | Render a full-width banner in the terminal. |
| `figlet -o banner.txt Infinity OS` | Create a new text object in the current personal directory. |

These are intentionally basic text tools, not complete GNU sed or POSIX awk.
Files are resolved in the OS object namespace. With no files the filters
read stdin; Ctrl+D signals EOF. FIGlet includes the standard and small FIGfonts
and never overwrites an existing output file. The SDK is at `/InfinitySDK`.
