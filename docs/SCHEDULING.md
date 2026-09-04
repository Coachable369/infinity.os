# Scheduling and Resources

The Milestone 4 scheduler is cooperative and architecture-neutral. It selects `Runnable` contexts with weighted rotation across Critical, System, Normal, and Background classes, then rotates fairly within a class. Contexts yield or block; message, timer, and dependency readiness are typed wake reasons. No polling sleep synchronizes services.

TESTED: runnable contexts, background non-starvation, blocking/wakeup, idle accounting, memory admission, queue admission, and CPU tick accounting.

Policy includes memory limit, CPU weight, message limit, and I/O priority. Current enforcement rejects memory/queue accounting above limits. CPU weight is scheduling metadata rather than precise proportional share. I/O priority awaits asynchronous storage dispatch.

Preemption, register save/restore, page-table switching, multicore synchronization, and hard CPU-time enforcement are SCAFFOLDED or PLANNED. This is not process or MMU isolation.

