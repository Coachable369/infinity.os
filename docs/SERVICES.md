# Services

## InfinityUI service chain

`Skin Registry` depends on Object and Settings. `Window Server` depends on the
Runtime and Device services. `InfinityUI` depends on Window Server, Skin
Registry, Font, and Session. `Clipboard` depends on InfinityUI and Session.
They expose typed operations and do not run shell scripts. Failure policy is
bounded retry for the visual core and on-failure for Clipboard.

The Organization Service (`service_id=13`) depends on Object and Namespace
services and provides typed Project and Collection operations. Console startup
depends on it and discovers providers by operation ID, not fixed address or
delay.

A version-1 `ServiceManifest` contains numeric service/type/image identities, service version, bounded dependency and operation lists, required capability classes, resource policy, criticality, and restart policy. It is a typed native structure; JSON or YAML is not a persistent format.

Lifecycle states are `Defined`, `Starting`, `Ready`, `Degraded`, `Stopping`, `Stopped`, `Failed`, and `Restarting`. Context creation is not readiness: each service explicitly announces readiness. The dependency graph is cycle-checked and no startup sleeps are used.

The registry supports bounded lookup and operation-to-ready-provider discovery. A hash index is PLANNED before it grows beyond the current 16-entry limit; callers never assume endpoint addresses.

Restart policies are `Never`, `OnFailure`, `Always`, and `BoundedRetry`. Retry backoff is 1, 2, 4, 8, 16, then 32 seconds. Non-critical failure destroys only the failed context, publishes `Service.StateChanged`, and applies policy. Critical failure enters controlled degraded state; it does not panic or reboot the kernel.

The live profile starts runtime, device, storage, object, namespace, event,
console, installer, Local ML, Infinity AI, Voice, and Agent services. The
installed profile uses identical contracts but omits installer startup.
Dependency order brings storage/object/namespace online before the AI services;
no service uses a startup sleep.

Milestone 6 service IDs are Local ML `9`, Infinity AI `10`, Voice `11`, and
Agent `12`. Model, intent, context, tool, voice, speech, and agent operations are
registered as stable numeric IOP operations. The Local ML and AI services are
functional in the colocated bring-up runtime; voice capture/speech providers are
honestly reported unsupported on current hardware.

Service components are currently statically linked, not independently loadable binaries. Their native manifests and policy are installed in System Space. Splitting them into MMU-isolated contexts is SCAFFOLDED.
# Milestone 7 service graph

`Identity -> Authentication -> Session -> Settings -> Onboarding/Shell` is
started by dependency readiness, with Object, Namespace, Event, AI, Voice, and
Console dependencies declared in versioned manifests. Installed systems start
the same service implementation as live media; onboarding is selected by
durable identity state rather than a second operating environment.
