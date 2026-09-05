# File Navigator

File Navigator is the native `app.infinity.file-navigator` application. It launches from App Launcher, the Files dock action, and `navigator [NamespaceRef|ObjectId]`. Its Normal Window Server surface is independently focusable, movable, resizable, minimizable, and maximizable; default size is 780 x 560 and minimum size is 640 x 420.

The frosted object-aware interface provides Back, Forward, Up, explicit location, semantic object presentation, List/Grid state, a collapsible Inspector, and sections for Identity, Metadata, Storage, Security, References, Versions, and Relationships. Only real configured locations and available metadata are shown. Enumeration uses bounded batches and viewport ranges so 100, 1,000, and 10,000 item result sets do not require unbounded UI allocation.

GUI actions share the same typed operations as Console: Object.Copy, Namespace.Move, Namespace.Attach/Detach, safe Trash.Move, explicit Object.Destroy, Object.Search, Object.History, ApplicationAssociation.Resolve, and Application.Launch. Open in Console Here initializes a new Console session with the explicit active NamespaceRef; no subprocess CWD is created.
