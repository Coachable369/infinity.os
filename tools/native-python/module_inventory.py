"""Build-time facts for the immutable native interpreter module directory."""
import re

# Importable compatibility wrappers must not be advertised as implemented
# facilities. Remove entries only with real native backend acceptance evidence.
LIMITATIONS = {
    "select": "native descriptor polling is not implemented",
    "subprocess": "general native child-process execution is unavailable; only authorized pip PEP 517 children have a bounded native adapter",
    "multiprocessing": "native Python child-process spawning is not implemented",
    "threading": "native Python worker-thread creation is not implemented",
    "concurrent.futures": "native Python worker-thread and child-process execution are not implemented",
    "concurrent.futures.process": "native Python child-process spawning is not implemented",
    "concurrent.futures.thread": "native Python worker-thread creation is not implemented",
    "asyncio": "native Python selector and socket event loop is not implemented",
    "http.server": "native Python listening sockets are not implemented",
    "socketserver": "native Python listening sockets are not implemented",
    "ftplib": "native Python socket protocols are not implemented",
    "imaplib": "native Python socket protocols are not implemented",
    "poplib": "native Python socket protocols are not implemented",
    "smtplib": "native Python socket protocols are not implemented",
    "tkinter": "the native Tcl/Tk windowing backend is not installed",
    "turtle": "the native Tcl/Tk windowing backend is not installed",
    "idlelib": "the native Tcl/Tk windowing backend is not installed",
    "ensurepip": "pip is already bundled as an immutable system distribution",
    "venv": "native Python virtual environment creation is not implemented",
}

SUPPORTED_SCOPES = {
    "mmap": "process-private anonymous mappings only, at most 1 MiB committed; read/write, slicing, buffer views and reuse; no executable mappings, file-backed persistence, resize or cross-process sharing",
    "ctypes": "native LP64 types, private memory and outbound calls to explicitly granted InfinityOS ELF libraries using libffi, including structures and variadic calls; no executable Python callbacks, ambient CDLL(None), foreign ABI libraries or early unloading",
    "socket": "explicit --allow-network grant; IPv4 TCP client and DNS through the native actor, one active connection; no UDP, IPv6, listeners or descriptor polling",
    "ssl": "native client TLS 1.3 AES-128-GCM with mandatory hostname and validity verification; immutable system roots or exact certifi 2026.06.17 root intersection; no server TLS, arbitrary custom roots, client certificates, negotiated ALPN, BIOs or certificate-inspection API",
    "http.client": "synchronous HTTP/1.1 using native TCP and verified TLS 1.3 under an explicit network grant",
    "urllib.request": "synchronous native HTTP/HTTPS reads under an explicit network grant; TLS uses immutable system trust",
    "subprocess": "pip-only isolated Python build child, bounded to one child, scoped filesystem and inherited subset authorization; source-build acceptance remains pending",
}

REQUIRED_BACKENDS = {
    "bz2": ("_bz2",), "lzma": ("_lzma",), "sqlite3": ("_sqlite3",),
    "ctypes": ("_ctypes",), "curses": ("_curses",), "readline": ("readline",),
    "infinity": ("_infinity_app",),
    "tkinter": ("_tkinter",), "turtle": ("_tkinter",),
    "socket": ("_socket", "_infinity_socket"), "ssl": ("_ssl", "_infinity_socket"),
    "http.client": ("_socket", "_ssl", "_infinity_socket"),
    "urllib.request": ("_socket", "_ssl", "_infinity_socket"),
}

# Deliberate task-oriented defaults, not every frozen wrapper. Backend and
# inclusion filters below prevent a missing compiled unit becoming advice.
GENERATION_LIBRARIES = {
    "arithmetic": ("math", "decimal", "fractions", "statistics"),
    "dates": ("datetime", "calendar", "zoneinfo"),
    "data": ("json", "csv", "configparser", "tomllib"),
    "models": ("collections", "itertools", "functools", "dataclasses", "enum", "heapq", "bisect", "random"),
    "text": ("re", "textwrap", "html", "urllib.parse", "xml.etree.ElementTree"),
    "compression": ("zlib", "gzip", "zipfile", "tarfile", "bz2", "lzma"),
    "native_ui": ("infinity",),
    "in_memory_sql": ("sqlite3",),
    "templates": ("jinja2", "markupsafe"),
    "configuration": ("yaml", "tomli", "tomli_w", "toml", "dotenv"),
    "html_processing": ("bs4", "soupsieve"),
    "package_utilities": ("packaging", "six", "typing_extensions"),
    "data_utilities": ("attrs", "more_itertools", "toolz", "sortedcontainers", "dateutil"),
    "terminal_text": ("pygments", "tabulate", "colorama"),
}


# ------------------------=
# FUNC: module_inventory
# DESC: Derives included and missing modules from the exact pinned CPython source and native build inputs.
# ------------------=
def module_inventory(source, version, architecture, frozen, builtins):
    declared = set(re.findall(r'^"([\w.]+)",$',
        (source / "Python/stdlib_module_names.h").read_text(), re.MULTILINE))
    builtin_set = set(builtins) | {"sys", "builtins", "_ast", "_imp", "_string",
        "_tokenize", "_warnings", "gc", "marshal"}
    all_names = declared | set(frozen) | builtin_set | {"_infinity_inventory"}
    modules = []
    for name in sorted(all_names):
        kind = "builtin" if name in builtin_set else "frozen" if name in frozen \
            or name == "_infinity_inventory" else "not-included"
        modules.append(dict(name=name, kind=kind, standard_library=name.split(".")[0] in declared,
                            included=kind != "not-included"))
    return dict(schema_version=1, python_version=version, architecture=architecture,
                modules=modules, limitations=dict(LIMITATIONS),
                supported_scopes=dict(SUPPORTED_SCOPES),
                required_backends=dict(REQUIRED_BACKENDS),
                generation_libraries={purpose: tuple(name for name in names
                    if (name in builtin_set or name in frozen) and name not in LIMITATIONS
                    and all(backend in builtin_set or backend in frozen
                            for backend in REQUIRED_BACKENDS.get(name, ())))
                    for purpose, names in GENERATION_LIBRARIES.items()})
