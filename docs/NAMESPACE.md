# Namespace

The Namespace Service projects human roots such as `/system`, `/home`, `/apps`, `/shared`, `/recovery`, and `/trash` over stable Object identities. These roots are not a Unix root filesystem.

Interactive Console and File Navigator instances retain an explicit `CurrentNamespaceRef` in their own session/UI state. Machine operations always receive a NamespaceRef, ObjectRef, ObjectId, or ReferenceId explicitly. Namespace deletion rejects protected roots and non-empty containers by default; no recursive scope is inferred.
