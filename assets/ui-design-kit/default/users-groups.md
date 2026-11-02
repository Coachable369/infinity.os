# Users and Groups

Reuse the approved Eclipse Precision panel, field, primary and secondary states,
Inter regular/medium atlases, Access check glyph, and themed Settings account icon.
No asset gap exists. The Settings compositor retains outer-frame ownership.

The section body has Users/Groups segments and a New command, 8px gutters,
40px selectable identity rows, and an unframed detail column. At body widths below
580 logical pixels, selecting a row replaces the list with details and a Back
command. Both columns have independent bounded scrolling and visible scroll
controls; keyboard focus reveals its control. The footer reserves two caption
lines for real validation and persistence failures.

Name and handle fields support select-all, copy, cut and atomic single-line
paste through the session-owned clipboard. Passwords accept 8-72 printable
characters in a dedicated private buffer; overflow is visibly rejected. Password
copy and clipboard history are never enabled. Drafts bind to their initiating
session and are erased when that session changes or loses authorization.

Fields use the shared Field recipe with a label above and a stable 36px entry
surface; buttons use Primary/Secondary, never Field. Membership rows use the
20px shared checkbox with a full-row target. Disabled and protected controls
use half coverage and cannot activate. Selection/focus use existing Precision
states without changing geometry. Labels clip within their assigned bounds.

Save/Cancel and Confirm/Cancel remain pinned side-by-side above status text while
fields scroll independently. Create and rename are explicit Save/Cancel forms.
Deletion requires a second,
named confirmation and states that personal files are retained. Password forms
include confirmation, display only masks, and erase controller-owned buffers
after cancellation, dispatch, lock, or leaving the section. Renderer snapshots
never contain passwords. All changes are proposed to authenticated account
dispatch; only persisted success updates feedback and committed data.

Tab/Shift-Tab traverse enabled controls, Enter activates, arrows navigate lists,
and Escape cancels a form or returns from narrow details. Pointer and keyboard
share targets. Parent runs the production painter fixture and both architecture
checks after coordinated build-lock release; no screenshot acceptance is implied
by this specification.
