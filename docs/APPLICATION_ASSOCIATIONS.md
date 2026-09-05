# Application Associations

Opening an object inspects its typed metadata, resolves an authorized registered application association, validates launch authority, and dispatches `Application.Launch`. Filename extensions are hints only and unknown content is never blindly executed.

File Navigator registers `BrowseNamespace`, `RevealObject`, and `OpenObject` intents under `app.infinity.file-navigator`.
