# Unix Alias Profile

`compat.unix` is a deliberately narrow command-vocabulary layer over Infinity Native operations. It includes `pwd`, `ls`, `cp`, `mv`, `rm`, `mkdir`, `cat`, and `find`; native `cd` remains independent of the profile.

These names retain InfinityOS Object identity, Namespace reference, capability, and Trash semantics. They do not introduce a Unix VFS, process CWD, inode model, or historical Unix execution behavior.
