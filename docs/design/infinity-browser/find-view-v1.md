# View and Edit menus

Reuse the native browser kit's menu row, Inter text, 8px gutters, sapphire
surfaces and cyan focus. Keep File and Settings adjacent; append Edit and View.
View lists up to eight open tabs in tab-strip order, marking the active tab.
Edit opens Find on Page, also available through Ctrl+F.

The native search strip sits immediately below the favorites rail, over the
page. It contains a bounded query field, current/total match status, previous,
next and close controls. Enter/Down advances, Up reverses and Escape closes.
Matches use real rendered document text, a DOM selection and centered scrolling.
Queries are encoded as UTF-16 escapes before evaluation; search does not use a
host browser or network service. Requests carry generations so late results
cannot overwrite a changed query or tab. The search excludes hidden text and
script/style/input content, and is bounded to two million text characters.

ASCII case-insensitive search is supported across adjacent text nodes in the
current document. Cross-origin frame contents are not searched.

## Verification

Browser-core behavior tests passed through the build kit. The disposable ARM
installation booted without installer media after an updated-kernel install.
`--find-view` verified two case-insensitive matches, next/previous selection,
View-menu switching between two real tabs, and existing Settings/AI controls.
Reviewed `installed-find-v1.png` and `installed-view-v1.png` against the existing
kit spacing, typography, menu styling and active-tab marker.

Passing manifest: `20260928T120112233462Z-35979.json`. This is updated installed
kernel evidence, not a fresh-ISO or x86 runtime claim. The regular ISO has not
been rebuilt for this change.
