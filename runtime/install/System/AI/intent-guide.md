# InfinityOS application intents

Requests pass through AIKit RequestApi, IntentRouter, inference when needed,
ExecutionBroker, and the native application dispatcher. A reply is not evidence
of execution: the dispatcher reports the actual result.

## Application operations

Name the app and describe the operation. Examples:

- In Text Editor write "I love you kai".
- `Please type Meeting at noon into Text Editor`
- In Text Editor undo.
- In File Navigator open folder /personal/documents.
- In Browser search Google for native Rust applications.

Explicit literal writing inserts at the caret and replaces the selection. It is
undoable and does not silently save or overwrite a file. A closed target app is
opened; an existing document is preserved. Unsupported operations are rejected.
The installed /system/service-registry.json is the current action inventory.
The LLM can infer phrasing beyond the fast parser using this same inventory.

## Teach a personal intent

Say or type:

Remember that when I say greeting for Kai, I mean in text editor write I love you kai

This stores a Procedural reference in your persistent memory, only after a
successful durable write. "Greeting for Kai" then resolves to that registered
action. Rephrased requests can use memory-grounded model inference. References
are scoped to your account and respect the learning preference. Inspect, edit,
or delete them in Infinity Memory. Conflicting exact references are not executed
automatically. Deleted or unsupported actions cannot be resurrected by a memory.

A new phrase may refer to an existing action. A genuinely new executable action
must be implemented and registered by its application; the LLM and vector store
cannot grant permissions or invent system APIs. Initial installations include
the native action registry and this reference; ordinary personal facts remain
separate from actionable procedural references.
