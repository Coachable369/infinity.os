# Infinity Native Commands

Infinity Native is the canonical human command language. `path` reports the current Console session's human `CurrentNamespaceRef`; it is not object identity or a POSIX working directory. `idir` and the native cross-platform command `cd` call the same typed Namespace navigation operation. `cd` remains available with every compatibility profile disabled. `pwd` exists only as a Shell Profile alias to `path`.

Navigation and inspection: `path`, `idir`, `cd`, `list`, `list tree`, `examine`, `resolve`, `references`, `versions`, `relationships`, `find`, `open`, and `navigator`.

Lifecycle operations: `namespace create/delete/move/list`, `object create/copy/delete/inspect/history/relationships/destroy`, `reference create/delete/list`, and `trash add/list/restore/delete/empty`. Copy creates a new ObjectId. Move and rename alter a Namespace reference while retaining ObjectId. Reference creation adds a path to the same ObjectId. Ordinary object deletion moves the reference into Trash; explicit confirmed destruction targets the underlying Object identity.

All commands terminate at the typed operation registry and capability checks. They do not invoke hidden POSIX APIs.
