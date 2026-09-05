# Linux Alias Profile

`compat.linux` provides command-name familiarity only. Linux and Unix Shell Profiles map familiar command names onto InfinityOS-native Object, Namespace, capability, and Trash semantics.

Initial mappings include `pwd` to `path`, `ls` to `list`, `tree` to `list tree`, `stat` to `examine`, `cp` to `object copy`, `mv` to `namespace move`, `ln` to `reference create`, `unlink` to `reference delete`, `rm` to safe `object delete`, and `mkdir` to `namespace create`. Native `cd` is available independently and retains InfinityOS session-namespace semantics.

This profile does not promise GNU/Linux behavior, POSIX paths, process CWD, byte-stream utilities, or ambient root authority.
