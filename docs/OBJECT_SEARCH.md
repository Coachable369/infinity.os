# Object Search

`find` and File Navigator search consume bounded authoritative Namespace/Object results. Results are batched and virtualized and may be cancelled by the caller. Search never enumerates names beyond the caller's capability scope and does not use rendered UI text as an authority.
