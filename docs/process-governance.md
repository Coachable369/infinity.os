# Application and service lifecycle authority

Task Manager and `top` read the same execution registry. Built-in applications
register when opened; bootstrap services register during runtime initialization.
Native compiler jobs, Python scripts, child processes, and automation workers
register in `native_compiler_task::Task::start_with_heap` before their first
execution slice. Registration failure prevents execution.

Native processes use separate named records, including startup before a window
exists. Pause, resume, termination and resource budgets are observed by the native
polling path. A termination request retains the record until resource cleanup;
its slot cannot be reused while the old owner remains alive. Process records use
bounded reuse, so repeated launches do not exhaust the registry.

Active applications appear before service rows. Keyboard navigation and the mouse
wheel reach the full process catalog. Critical system services retain their
existing protection and service lifecycle policy. Internal service work remains
attributed to its owning service rather than represented as a separate application.

The interactive Python host currently supports one foreground Python application
at a time. The busy message directs users to Task Manager or `python-cancel`.
Bundled GUI startup has a 30-second deadline; failure is logged under `/Logs` and
releases the slot. This is not concurrent execution of multiple Python GUI apps.

Tk uses the compositor's usable client height for initial and requested window
bounds. A full-display geometry request is clamped before native window creation,
so it cannot silently leave an unmapped app running behind the desktop.

A Python app receives Command Window stdin only when it has requested a read.
Merely running a GUI app does not intercept shell commands such as `task end`.

## Behavioral verification

`tools/task-manager-test.rs` exercises registration, hung-state visibility,
pause/resume/end, reservation through cleanup, resource reset and repeated reuse.
`tools/native-python/process-governance-installed.py` exercises the installed ARM
kernel through real launcher, keyboard and pointer input: both application
windows, pause/resume counters, termination, complete memory reclamation and
subsequent relaunch. It captures Task Manager, `top` and application framebuffers.
Run it through `./build-kit run` against a disposable installed disk fixture.
The fixture is booted without an ISO; the test never writes guest process state.
