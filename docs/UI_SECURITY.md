# InfinityUI Security

Skins are appearance resources, never authority. They cannot register IOP
operations, access credentials, replace trusted security meaning, or obtain
window surfaces without capabilities. Trusted surfaces include authentication,
lock, capability consent, destructive confirmation, and recovery.

Secure input uses an exclusive bounded lease owned by a trusted context.
Clipboard payloads are typed and bounded; read and write require an external
capability decision. Window mutation and pointer capture validate context
ownership. Invalid skin activation rolls back without restarting callers.

Window ownership and skin rollback are TESTED. Secure-input and clipboard
mechanisms are IMPLEMENTED BUT UNTESTED in a live VM. Hardware-backed secure
attention and signed third-party skin distribution are PLANNED.
