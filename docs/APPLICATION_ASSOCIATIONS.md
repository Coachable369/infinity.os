# Application Associations

Opening an object inspects its typed metadata, resolves an authorized registered application association, validates launch authority, and dispatches `Application.Launch`. Filename extensions are hints only and unknown content is never blindly executed.

File Navigator registers `BrowseNamespace`, `RevealObject`, and `OpenObject` intents under `app.infinity.file-navigator`.

The File Navigator context menu exposes `Open With...` for one-time launches and
`Always Open With...` for per-user defaults. Defaults are stored as a versioned,
checksummed private System object and are restored only for the authenticated
user/session. A default is ignored if its application is no longer installed or
no longer declares compatibility.

Built-in applications and installed applications use the same bounded registry.
An installed application manifest may declare:

```text
identity=app.example.markdown-studio
display-name=Markdown Studio
image-id=0xb001
open-content-types=utf8-text
open-extensions=md,markdown
```

`open-content-types` accepts `binary`, `utf8-text`, `namespace`, `system`,
`application`, and `metadata`. `open-extensions` contains at most six bounded
ASCII extensions. At least one type or extension is required. Registration also
registers the custom image with the task manager; built-in identities and image
IDs cannot be aliased by an installed manifest.

Launch creates a typed one-shot startup request containing the exact selected
NamespaceRef. The destination consumes that request by application identity.
The request is rejected before launch when the handler is incompatible, and File
Navigator reports a visible error when no compatible handler exists or the
selected installed image is unavailable.
