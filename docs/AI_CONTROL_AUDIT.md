# AI control coverage audit — 2026-10-11

## Acceptance checklist

- Inventory installed launcher entries, native windows, Settings categories and desktop surfaces.
- Resolve explicit actions through native app controllers, with the same path for desktop text/voice and attached panels.
- Keep capability checks, unsaved-document dialogs, session isolation and cancellation.
- Provide a reusable attached panel with close, clipped scrolling, editing and bounded layout.
- Distinguish code/harness evidence from installed visual and microphone evidence.

## Inventory and current coverage

“Window controls” below means maximize, restore, minimize and close from the attached panel or a contextual desktop request. Named open/focus/close commands exist for the six native window families. Save dialogs are not bypassed. Voice uses the existing authenticated chat submission path; these additions do not create a second voice execution engine.

| App / launcher entry | Native owner | Granular AI commands implemented | Gaps / limits |
|---|---|---|---|
| Infinity Browser | Browser, panel 0 | Google search, back/forward, new/close tab, zoom, reload; window controls | DOM/form interaction, bookmarks, downloads and permissions management are not general AI tools |
| Command Window | Terminal, panel 1 | `type COMMAND` drafts the command; window controls | Execution intentionally requires review and Enter in the terminal |
| Text Editor | Editor, panel 2 | Insert, find, undo/redo, select all, save; generated insert/replace proposals; window controls | Generated edits require Apply; bounded document context; filename dialogs may need interaction |
| Task Manager | Tasks, panel 3 | Select next/previous task, refresh; window controls | Terminate/relaunch/pause/throttle still use native controls |
| Settings | Settings, panel 4 | Open categories, cycle theme/icon set/local model; window controls | Not every Settings field is exposed as a typed AI action |
| File Navigator | Navigator, panels 5–10 | Find exact filename in current subtree, open absolute folder, list/grid, refresh; window controls | Search examines at most 256 records; file mutation/rename/delete and arbitrary content search are not AI tools |
| Documents | Navigator alias | Same Navigator controls, launches Documents | Same limits |
| Projects | Navigator alias | Same Navigator controls, launches Projects | Same limits |
| Media | Navigator alias | Same Navigator controls, launches Media | Not a separate media-player control interface |
| Recycle Bin | Navigator alias | Same Navigator controls, launches Trash | Restore/permanent deletion not exposed to AI |
| AI & Voice | Settings alias | Opens AI category; next model | Mic/voice configuration remains native Settings controls |
| Security | Settings alias | Opens Privacy & Security | Permission grants are not bypassed |
| Storage | Settings alias | Opens Storage | Policy/placement edits remain native controls |
| About | Settings alias | Opens About | Read-only screen |
| Network | Settings alias | Opens Network | IP/DNS/profile edits remain native controls |
| Nodes & Mesh | Settings alias | Opens Nodes & Mesh | Pairing/trust/revocation remain native controls |
| Holographic Desktop | Spatial, panel 12 | Previous/next item, open selected, zoom, close | Not a normal maximizable window |
| World Shift | Spatial, panel 12 | Previous/next selection, activate selected world, close | World authoring remains native controls |
| App Launcher | Launcher, panel 11 | Search catalog, launch named entry, close | Reordering remains drag/drop |
| Gravity Wall | Spatial, panel 12 | Select items, activate, zoom, open Add Idea/New Category editor, close | Editing/saving category/idea contents and deletion remain native editor controls |
| Matter Shelf | Spatial, panel 12 | Select/activate items, close | Link, collection and drop workflows remain native controls |
| Constellations | Spatial, panel 12 | Select/activate items, close | Connection editing remains native controls |

Launcher names are resolved against `LAUNCHER_APPS`, rather than a second independently maintained app list. Settings aliases and collection entries therefore launch their actual registered action.

### All Settings categories

General; Themes & Skins; Users & Accounts; AI & Voice; Privacy & Security; Devices; Network; Nodes & Mesh; Storage; About; Input. All can be opened from either AI entry point using the registered name or documented alias. Theme, icon and model cycling dispatch the native Settings row action. Other edits are **not** counted as implemented AI controls merely because the category can be opened.

### General desktop / top navigation

- Open audio, network, Bluetooth, power, window, search, account, Help and calendar menus.
- Calendar: previous month, next month, today.
- Windows: tile left/right, center, grow/shrink, next window.
- Presentation: toggle Focus Lens; Peek Through.
- System Overview: show/hide and checkpoint its visibility.
- Dock and launcher destinations: open registered apps through normal launch workflows.
- Running-app drawer: existing named focus/close controls reach native window management; drawer placement/reordering remains pointer-driven.
- Clipboard/history, widget positioning, notifications, lock/restart/shutdown, identity administration and permission grants are not newly exposed AI mutation tools.
- Login, installer and lock screens deliberately do not have an authenticated app-control assistant.

## Shared implementation

`runtime/ai/control.rs` recognizes bounded explicit text/voice requests, preserving argument case. The authenticated pending-control boundary still owns dispatch. `console_assistant.rs` resolves contextual requests using the same `Panel` actions as attached panels, then calls normal browser, editor, Navigator, Settings, launcher or spatial controllers. Explicit supported requests do not wait for LLM generation.

Generated prose cannot become an arbitrary command. Attached models can propose one canonical `ACTION` record, but the app ownership allowlist must accept it and the user must Apply it. Incomplete records, combined commands and cross-app operations are rejected. Desktop model output can propose a bounded `OS_CONTEXT` record through the existing authenticated pending-control boundary. Existing generated editor operations remain staged, revision-checked and applied explicitly. Background generation stays associated with the original open app; foreground switching/minimization alone no longer cancels it. Closing the owner/panel, session boundaries and AI revocation still cancel it.

## Panel design

Reuses the existing default Infinity UI kit, `assistant_tab` material, native rounded controls and font metrics. No new icon family or placeholder imagery was necessary. The kit recipe is `assets/ui-design-kit/default/ai-assistant-panel.md`.

Added: header close control; clipped request/response/proposal transcript; proportional scrollbar with wheel/track/drag input; fixed footer actions; caret-based insertion/deletion and navigation; I-beam and caret presentation integration. Transcript buffers remain bounded. Ordinary hover reads compact state instead of copying conversation buffers.

## Evidence and outstanding acceptance

Behavioral harnesses exercise scoped actions, preserved arguments, one-shot action consumption, session/authority rejection, close hit regions, scaled geometry and scroll-thumb bounds. ARM64 and x86_64 kernel checks are run through the build kit. These are not installed voice or screenshot evidence.

Passing run: `builds/manifests/20261011T053410982624Z-3138.json` (exit 0).
It ran the editor/assistant behavioral executable, `ai-test`, and both native-browser kernel architecture checks. Diagnostics are in `build/assistant-ai-test.log`, `build/assistant-aarch64-check.log`, and `build/assistant-x86-check.log`. Existing compiler warnings remain; this was not a warning-free build or a full release build.

The comprehensive request is **not yet fully accepted**: the gaps above, installed speech dispatch, visual screenshot review, and cold-installed parity verification remain open. The changes are shared kernel code and reuse already-packaged assets; no live-only auxiliary service or host automation is introduced. No new release ISO is claimed by this audit.
