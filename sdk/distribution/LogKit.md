# LogKit — native Python and C diagnostics

LogKit emits bounded JSON Lines through the native application console. It is
available in both languages and uses the same record schema. It does not create
a privileged central logging daemon or promise persistence after application exit.

## Quick start

Copy `../Examples/Python/logging_demo.py` or `../Examples/C/logging_demo.c` to
Documents and run in Text Editor. The terminal displays one JSON record with
version 1, level 1, component `demo` and a message. No permission beyond normal
application console output is required. Logging does not configure global Python
logging or change other applications' handlers.

## Reference

Python: `from infinity_log import log, DEBUG, INFO, WARNING, ERROR`;
`log(level, component, message, sink=None)`. The default sink is native stderr;
an explicit text sink must implement `write` and `flush`. Invalid fields raise
ValueError; incomplete writes and sink exceptions propagate.

C: `#include <infinity/log.h>`;
`infinity_log(level, component, message)` returns zero on complete delivery or a
negative error. Constants `INFINITY_LOG_DEBUG/INFO/WARNING/ERROR` are 0/1/2/3.
The header uses the native checked console syscall, with no printf-format-string
interpretation of the message. A short write returns -5; invalid arguments -22;
native capacity/permission errors are preserved.

Record: `{"version":1,"level":1,"component":"demo","message":"Started"}`.
Each record ends with exactly one newline; embedded newlines, quotes and control
characters are JSON-escaped. Component is at most 32 UTF-8 bytes and message at
most 256. C callers supply valid UTF-8, NUL-terminated strings; Python validates
encoding. Oversized records are rejected, not silently truncated. No wall-clock
timestamp is fabricated; applications may include their own correlation ID or
measured duration in the message.

## Operational guidance

Log state transitions and error codes, not every rendered frame or audio sample.
Use DEBUG sparingly. Console buffers are bounded: on capacity errors, yield and
reduce log volume; never busy-loop retrying. Logging failures must not be treated
as successful business operations. Do not log passwords, tokens, private prompts,
face images or raw microphone samples. Redact sensitive values before calling.
Use a user-authorized persistent sink when persistence is required; retention and
rotation are the application's responsibility. LogKit does not upload telemetry.

## Examples and validation

Both shipped examples use the actual SDK entry points. Tests intercept the C
syscall boundary and Python sink to check encoded records, invalid input, short
writes and error propagation. They do not claim a persistent log collector exists.
