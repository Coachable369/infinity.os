# Object References

A Namespace reference is a human-readable path relationship to a stable ObjectId. `reference create` attaches an additional path to the same ObjectId. `reference delete` removes exactly the selected path and never implies underlying object destruction.

File Navigator exposes authorized references in its Inspector. Move, Copy, and Create Reference remain separate actions: move preserves identity and relocates one reference, copy creates a new identity, and reference creation preserves identity while adding a location.
