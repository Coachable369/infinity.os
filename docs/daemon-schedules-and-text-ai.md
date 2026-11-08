# Daemons, scheduled Python tasks, and text AI

Implementation acceptance checklist:

- One native daemon/schedule registry shared by Command Window and Task Manager.
- Daemon create, inspect, edit, delete, start, stop, kill, restart, enable, disable,
  startup trigger, and restart policy operate on actual isolated processes.
- Scheduled Python tasks support create, inspect, edit, delete, enable, disable,
  and observable execution receipts; definitions survive a cold Object Store mount.
- Task Manager reuses the approved design kit and provides daemon and schedule
  tabs with inline editors, keyboard input, and explicit operation results.
- Command Window submits prompts to the installed LLM and retrieves the response.
  Command and widget decision requests use known evidence, expose uncertainty,
  compare alternatives, and do not execute advice as an action plan.
- Direct AI audio capture, recognition, generation handoff, synthesis, playback,
  and startup/recovery code remain frozen.
- Behavioral checks cover authorization, lifecycle, scheduling, persistence,
  typed request routing, UI state, and generated artifact parity.
- Release handoff uses `./build-kit full`, ARM64 first, followed by x86_64;
  installed-runtime proof and rendered screenshots are recorded separately.

Kernel infrastructure service records describe embedded subsystem readiness.
They are distinct from managed executable daemons: changing a record alone is
not an implementation of process termination or restart.

## Commands

`service list` includes managed daemons and protected kernel infrastructure.
`service inspect NAME` shows the script, trigger, restart policy, enablement,
native process ID, run count, readable next UTC time or eligibility reason, and
last exit code. Lists also show the configured trigger and next-run status.

```text
service create worker /home/default/documents/worker.py start=boot delay=5 restart=on-failure
service start worker
service stop worker
service restart worker
service kill worker
service disable worker
service edit worker /home/default/documents/updated.py start=manual restart=never
service enable worker
service logs worker
service delete worker

schedule create report /home/default/documents/report.py every=3600
schedule create once /home/default/documents/report.py at="2026-12-01 09:00"
schedule inspect report
schedule edit report every=7200
schedule disable report
schedule run report
schedule stop report
schedule restart report
schedule kill report
schedule logs report
schedule enable report
schedule delete report
```

Task Manager exposes the same operations on its **Services** and **Scheduled
Tasks** tabs. Creation and editing use inline fields; Save must commit durable
state before the UI reports success. Stop a running job before editing or
deleting its definition. Boot/manual policies are available for daemons;
schedules also support fixed intervals and a one-time UTC date. Dates accept
`YYYY-MM-DDTHH:MM[:SS]Z`, quoted `YYYY-MM-DD HH:MM[:SS]`, or epoch seconds.
All dates use UTC; there is no implicit local-time conversion. Quote an entire
script path when it contains spaces. Spaces and tabs separate command tokens.
The GUI preserves unsaved drafts and rejects stale edits or deletion when that
job's definition changed; unrelated job activity and run counters do not conflict.

Configuration and lifecycle changes require an active local administrator.
Jobs run with their approving account's bounded source access after logout;
deactivation or loss of administrator membership cancels execution. Stop cancels
at the next native boundary; Kill immediately reclaims the suspended native task.
Both pause automatic admission until Start/Run, Enable, or a fresh boot restores
the configured trigger. Inspect the job to distinguish requested termination from
an observed process exit.
Disabling automatic dispatch does not terminate an already running process.
An explicit Start/Run works for a disabled definition. Restart-on-failure and
always-restart policies use a five-second backoff. Missed intervals coalesce
into one run, and the same job never overlaps itself.

The bounded native runner admits four processes concurrently, snapshots the
script and at most seven adjacent Python modules, and retains bounded output
for `logs` until that worker slot is reused. It includes the packaged native
Python runtime. Unattended jobs have no ambient network, writable filesystem,
GUI, interactive input, or user-installed package access. Scheduled runs have a
one-hour deadline; daemon lifetime is operator-controlled. These are explicit
execution limits, not simulated process state.

The versioned catalog is seeded into each fresh Object Store at
`/system/automation/state`. Older stores gain the empty object when first opened
by this runtime. Corrupt state blocks management instead of silently resetting
definitions. Run claims are durable before execution. A one-time job interrupted
by reboot is recorded as failed and requires an explicit Run to retry; it is not
silently replayed. Boot daemons are eligible again after their configured delay.

`ai ask PROMPT` (or `llm PROMPT` / `llm ask PROMPT`) submits to the selected local language model;
`ai decide QUESTION` supplies explicit decision guidance. `ai response` retrieves
the current command request and `ai cancel` cancels it. `ai new` / `llm new`
starts a fresh idle conversation after an active request finishes or is cancelled. See
[text AI behavior](command-llm-advice.md) for evidence and decision rules.
