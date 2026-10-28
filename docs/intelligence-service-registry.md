# IntelligenceKit service dictionary

Native System Space uses lowercase `/system` paths:

- `/system/service-registry.json`: versioned endpoint inventory, audited settings, argument contracts and availability categories.
- `/system/ai/instruction-mappings.json`: editable aliases mapped to allowlisted native targets.

Both are ordinary editable Text objects. Open them in Text Editor and save; malformed or unsupported targets are rejected before the editor write. Existing installations receive a missing dictionary during storage initialization without overwriting existing mappings or dictionaries. Fresh installations receive both current defaults. Each document remains within the native 16 KiB editor/storage bound.

The dictionary deliberately distinguishes executable actions from `uiOnly` and `readOnly` settings. An audit entry does not create an implementation. In particular, named WorldShift selection and network/node/storage mutations still require their existing UI workflows. Native authorization cannot be changed by editing JSON.

## Request flow

Voice and typed input share AIKit routing. Exact user aliases take the low-latency path. Otherwise Hermes receives the supported protocol directory and settings argument guidance to infer a control proposal. IntelligenceKit's capability directory exposes the dictionary path and the same native targets. The model's proposal is decoded and validated; it never directly invokes arbitrary system calls.

Settings execute through the existing authenticated settings boundary. Identity changes are persisted and primary/accent/background/skin projections are synchronized with the UI. Clock requests retain their existing clock service. A color-valued theme request such as `Set UI theme to purple` changes primary color; dark/light select the installed skins. Invalid values are rejected instead of reported as success.

Dictionary prose is documentation, not model instructions or authority. Changing supported argument ranges in JSON does not change native validation. To add an executable capability, implement its typed handler and authorization, register its target, add dictionary and alias entries, and add behavioral persistence/UI tests. Keep unavailable entries explicitly marked until that work is complete.

## Verification

The AI harness exercises alias resolution, typed theme mutation, serialization, locked-session denial, complete model-directory projection, output capacity and unknown registry target rejection. Object-store tests exercise dictionary provisioning, preservation and migration. Installed-object tests compare the installed dictionary bytes to the packaged source. These checks do not establish live model inference quality or installed-VM visual acceptance.
