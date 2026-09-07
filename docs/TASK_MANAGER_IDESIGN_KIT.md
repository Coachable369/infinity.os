# InfinityOS Task Manager — IDesign Kit

## Product identity

- Application identity: `app.infinity.task-manager`.
- The existing Activity/graph semantic icon is extended into the launcher catalog and active icon family; it uses connected blue-white telemetry nodes and remains consistent at every installed scale.
- Task Manager observes authoritative Execution Manager contexts. It does not fabricate host processes or parse display strings.

## Window composition

- A normal movable, resizable, minimizable, maximizable layered window uses the active frosted glass recipe and white-blue focus edge.
- Three restrained telemetry cards show accounted CPU activity, live memory consumption, and registered installed-image disk footprint. Their fills use a moving highlight driven by the compositor frame clock while the reported values remain authoritative.
- A conventional `Task` menu replaces the action-button strip. It groups Launch, Relaunch, Pause/Resume, Throttle, End Task, and Refresh, with the destructive action visually separated.
- The task table exposes application/service name, state, proportional accounted CPU share, live memory usage, and installed disk footprint without parsing presentation strings.
- Selection remains visually stable while Task-menu actions operate on the highlighted row. System-owned rows remain visible while the typed runtime rejects destructive lifecycle or resource actions against them.

## Interaction and motion

- Selection, hover, focus, and pressed states use the same semantic skin roles as the launcher and File Navigator.
- Telemetry meters use a restrained moving highlight on the one-second presentation clock without moving the table geometry.
- Keyboard traversal covers the task table. Up/Down select, Enter or Space pauses/resumes, Delete ends, R relaunches, T throttles, and L launches File Navigator.
- Pointer targets derive from the live window geometry. The dashboard maintains a presentation-safe minimum while remaining resizable and maximizable.

## Safety and parity

- App tasks may be launched, ended, paused, resumed, relaunched, reprioritized, and throttled. Core services are inspectable but lifecycle-controlled only by their existing typed Service operations.
- The Command Window exposes the same typed task list, inspect, launch, end, relaunch, pause, resume, priority, and throttle transitions.
- The runtime, launcher entry, console surface, UI surface, and behavioral tests ship in both live and freshly installed System Generations.
