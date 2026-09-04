# User Sessions

Status: **TESTED** in the host harness; graphical login and lock are **TESTED**
in QEMU from the installed disk. Graphical unlock/logout remain **IMPLEMENTED
BUT UNTESTED** end to end.

A Session binds a stable user, machine, Personal Space, AI context, lifecycle,
and explicit capability set. States are Created, Authenticating, Active,
Locked, Closing, and Closed. A session is created only after successful
authentication. Lock preserves identity but hides the UI until reauthentication;
logout closes the session and clears its capabilities.

Sessions are intentionally transient and are not restored after reboot. Native
session IDs are not PIDs, cookies, or Unix login records. Ordinary sessions get
only self/profile/settings/AI/voice/display authority. Cross-user mutation
requires separately granted identity-management authority.
