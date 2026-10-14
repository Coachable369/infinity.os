# InfinityAIKit internal architecture

InfinityAIKit is the native, transport-neutral AI integration boundary for
InfinityOS. It does not expose or depend on MCP.

## Runtime flow

1. The request API creates a typed `RequestEnvelope` containing input, owner,
   origin, and correlation ID.
2. `IntentRouter` checks the installed editable instruction map and deterministic
   command parser before inference.
3. `InferenceBroker` selects either ordinary conversation or the bounded desktop
   tool prompt. Model output remains untrusted data.
4. `ContextBroker` releases only explicitly requested and authorized context
   classes.
5. `CapabilityDirectory` projects the authoritative 100-target instruction-map
   inventory; it does not maintain a duplicate command list.
6. `PolicyEngine` rechecks current authenticated ownership, AI enablement, and
   desktop permission immediately before dispatch.
7. `ExecutionBroker` stages at most one typed command, grants it exactly once to
   the native dispatcher, and accepts only the matching completion receipt.
8. `AuditLog` retains a bounded structured lifecycle history using correlation
   IDs and typed outcomes.
9. `ProviderRegistry` remains provider-neutral while the current default path is
   local native inference.

The native console dispatcher remains the sole authority for OS effects. It uses
the same application controllers, save prompts, capability checks, and state
verification as pointer and keyboard interaction. AI text cannot invoke a raw
system call, shell escape, or arbitrary application function.

## Extension contract

New AI-visible OS actions are added to the typed command protocol and the
authoritative editable mapping together. Application-specific actions must enter
through their native controller and return a verified execution result. Network
transports, remote providers, or future interoperability layers sit outside
AIKit and cannot bypass its policy or execution brokers.
