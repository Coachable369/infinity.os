# Object Model

ObjectId is stable identity. Object content is versioned data. Human paths are Namespace references to ObjectId and are never identity themselves.

Copy creates a distinct ObjectId even when storage internally deduplicates content. Move and reference rename preserve ObjectId. Creating a reference adds another authorized path to the same ObjectId. Removing a reference affects only that path. `Object.Delete` uses recoverable Trash semantics for ordinary user actions; `Object.Destroy` is a separate capability-checked and explicitly confirmed request that rejects protected System Objects and active protected relationships.
