# Offline Python package baseline

The native CPython 3.13 image includes 22 public distributions. No network,
host Python installation, or first-login download is required. The same
interpreter payload is included in the live system and installed System
Generation on ARM64 and x86_64.

| Purpose | Distributions |
| --- | --- |
| Package management and metadata | pip, packaging |
| Dates and time zones | tzdata, python-dateutil, six |
| Templates and safe HTML escaping | Jinja2, MarkupSafe |
| HTML parsing and CSS selection | beautifulsoup4, soupsieve, typing-extensions |
| Validated records and collections | attrs, more-itertools, toolz, sortedcontainers |
| Configuration | PyYAML, tomli, tomli-w, toml, python-dotenv |
| Syntax highlighting and terminal formatting | Pygments, tabulate, colorama |

Inspect the baseline with `python -m pip list` and check dependency consistency
with `python -m pip check`. Import names can differ from distribution names:
`dateutil`, `bs4`, `yaml`, `dotenv`, `more_itertools`, `tomli_w`,
and `typing_extensions` are examples.

The bundled baseline is immutable. Authorized packages in the user's package
store take precedence, including their submodules. To explicitly install a
separate version when a bundled version already satisfies the requirement, use
`python -m pip install --ignore-installed package==version`. Removing that user
copy exposes the bundled version again; it does not delete system content.

`sdk/python/packages.lock.json` pins each additional artifact by version,
upstream URL and SHA-256. Builds reject corrupt archives and foreign binary
wheels. Public packages retain their own upstream distribution metadata and
licenses; pip's private vendored dependencies are not counted as public packages.

MarkupSafe's actual C accelerator is compiled against the native CPython ABI
for each target. PyYAML uses its upstream Python implementation; its optional
libyaml accelerator is not claimed. TOMLI likewise uses its portable upstream
implementation, not a host-specific mypyc extension.

This baseline does not imply arbitrary PyPI compatibility. Native worker threads,
listening sockets, general subprocesses and foreign Linux/macOS/Windows extension
ABIs remain outside the current port. Native extension loading uses InfinityOS
ELF objects and existing capability-scoped ObjectStore package storage.

## Verification

`tools/native-python/bundled-tests.py` executes real parsing, rendering,
validation, collection operations and native C escaping. It checks exact public
distribution versions, dependency constraints and pip's own dependency checker.
`build.sh` runs this script in native hardware user mode on both architectures.
`tools/native-python/parity.py` verifies the expected distribution inventory and
byte-for-byte interpreter inclusion in live and installed kernels.

These checks are distinct from a detached-disk installed runtime test. Release
reports must state which were actually completed; packaging parity alone is not
installed-runtime proof.
