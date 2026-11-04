//! Central Console documentation: operation schemas plus registered native front ends.
use super::{console_language as language, console_execution::{Bytes, OUTPUT_BYTES}};
pub struct Entry { pub name: &'static [u8], pub usage: &'static [u8], pub description: &'static [u8] }
macro_rules! entry { ($n:literal,$u:literal,$d:literal) => { Entry{name:$n,usage:$u,description:$d} }; }
pub static COMMANDS: &[Entry] = &[
    entry!(b"help",b"help [commands|DOMAIN|DOMAIN ACTION|COMMAND|search WORD]",b"Explore the registered command catalog. Enter/Space advances local pages; Esc ends paging."),
    entry!(b"ipconfig",b"ipconfig [--all]",b"All native interfaces, link state, MAC, MTU, IP/prefix, gateway, DNS and traffic counters. Read only."),
    entry!(b"list",b"list [-l|--long] [-a|--all] [-R|--tree] [PATH]",b"List namespace entries. ls and dir are aliases. Long output includes owner, size, type, version and timestamps. Legacy timestamps are reported as unavailable, never invented."),
    entry!(b"ls",b"ls [-laR] [PATH]",b"Alias for list; identical options and permissions."),
    entry!(b"dir",b"dir [-laR] [PATH]",b"Alias for list; identical options and permissions."),
    entry!(b"edit",b"edit FILE",b"Local terminal text editor, new or existing file (16 KiB ASCII). Arrows/Home/End move; Ctrl+S/O saves, Ctrl+X exits, Ctrl+W finds, Ctrl+G goes to a line, Ctrl+K cuts a line, Ctrl+U pastes it, Ctrl+Z/Y undo/redo. Unsaved exit asks Save/Discard/Cancel."),
    entry!(b"cd",b"cd [PATH|home|parent|previous]",b"Change this Console's namespace. idir is an alias; relative paths are resolved from the current location."),
    entry!(b"idir",b"idir [PATH|home|parent|previous]",b"Native namespace navigation; alias of cd."),
    entry!(b"path",b"path",b"Print the current namespace."),
    entry!(b"find",b"find TEXT",b"Search bounded namespace paths case-insensitively."),
    entry!(b"examine",b"examine PATH",b"Inspect object identity, type, reference count, version and size."),
    entry!(b"resolve",b"resolve PATH",b"Resolve a namespace path to its immutable ObjectId."),
    entry!(b"versions",b"versions PATH",b"Inspect object version history."),
    entry!(b"references",b"references PATH",b"List namespace references to an object."),
    entry!(b"relationships",b"relationships PATH",b"Inspect typed object relationships."),
    entry!(b"open",b"open PATH",b"Open a supported resource with its registered application."),
    entry!(b"preview",b"preview PATH",b"Preview a supported file using the native preview service."),
    entry!(b"wallpaper",b"wallpaper IMAGE_PATH",b"Set the current user's desktop wallpaper."),
    entry!(b"ssh",b"ssh status | enable [port:N] [limit:N] | disable",b"Inspect/configure native SSH. Configuration requires a local administrator. Use help ssh key for registration."),
    entry!(b"ssh key",b"ssh key add|revoke USER ssh-ed25519 BASE64",b"Register/revoke a public key through IdentitySystem. Never paste a private key. New users receive ~/.ssh/id_ed25519 and .pub."),
    entry!(b"work",b"work help",b"Enterprise tools. Explore help work clipboard/history/checksum/compare/backup/restore/report/private."),
    entry!(b"work clipboard",b"work clipboard status|clear|ttl SECONDS|browser-read on|off|browser-write on|off",b"Session clipboard status, retention and explicit browser permissions."),
    entry!(b"work history",b"work history clear | work history find TEXT",b"Search or erase this session's command history."),
    entry!(b"work private",b"work private on|off",b"Disable or enable recording of command history."),
    entry!(b"work checksum",b"work checksum PATH",b"Compute the native content checksum for an authorized object."),
    entry!(b"work compare",b"work compare PATH PATH",b"Compare two authorized objects."),
    entry!(b"work backup",b"work backup SOURCE DESTINATION",b"Create an explicit local object backup."),
    entry!(b"work restore",b"work restore SOURCE DESTINATION SHA256",b"Restore a local backup with an expected content checksum."),
    entry!(b"work report",b"work report PATH",b"Write a bounded diagnostic report."),
    entry!(b"python",b"python SCRIPT [ARGS] | python -m MODULE [ARGS]",b"Run isolated native Python with bounded resources and explicit capabilities; no host shell."),
    entry!(b"python-cancel",b"python-cancel",b"Cancel the current native Python job."),
    entry!(b"pip",b"pip install PACKAGE | uninstall -y NAME | list | show NAME | check | freeze | help",b"Confined per-user native package installation. OS-managed packages remain protected."),
    entry!(b"cc",b"cc SOURCE",b"Compile native C with the packaged compiler. Use help run for execution; capability options are explicit."),
    entry!(b"cc-status",b"cc-status",b"Inspect the active native compile/run job."),
    entry!(b"cc-cancel",b"cc-cancel",b"Cancel the active native compile/run job."),
    entry!(b"run",b"run [--allow-network] FILE",b"Run a native application with bounded execution and explicit network consent."),
    entry!(b"infinitycode",b"infinitycode",b"Open InfinityCode."),
    entry!(b"apps",b"apps | apps launch NAME | apps hooks",b"List and launch registered native applications or inspect event hooks."),
    entry!(b"spatial",b"spatial",b"Open the spatial workspace overview."),
    entry!(b"spatialperf",b"spatialperf",b"Inspect spatial animation timing measurements."),
    entry!(b"inputdiag",b"inputdiag",b"Inspect native input button state."),
    entry!(b"uiperf",b"uiperf",b"Inspect measured frame timing and damaged pixels."),
    entry!(b"clock",b"clock | clock set HH:MM[:SS] | clock zone OFFSET",b"Read the native system clock. Clock setting operations require Settings authority."),
    entry!(b"commands",b"commands",b"Alias for help commands; the same central catalog."),
    entry!(b"navigator",b"navigator [PATH]",b"Open File Navigator at a native namespace."),
    entry!(b"reference",b"reference create|delete|list TARGET",b"Manage namespace references to native objects; see help namespace for typed operations."),
    entry!(b"shell",b"shell profile list|inspect|create|clone|enable|disable|set-default|delete; shell alias list|add|delete|resolve",b"Manage native shell profiles and aliases. Use help shell profile or help shell alias."),
    entry!(b"shell profile",b"shell profile create NAME | clone SOURCE NAME | inspect NAME | enable NAME | disable NAME | set-default NAME | delete NAME",b"Profile changes retain their native owner and policy checks."),
    entry!(b"shell alias",b"shell alias list PROFILE | add PROFILE ALIAS COMMAND | delete PROFILE ALIAS | resolve COMMAND",b"Aliases resolve to native commands, never an external shell."),
    entry!(b"camera",b"camera build | camera allow | camera stop",b"Build the isolated capture client, explicitly grant a short camera lease, or stop capture."),
    entry!(b"audio",b"audio status | audio tone | audio capture | audio stop",b"Inspect native audio; bounded test tone/capture requires an active local identity."),
    entry!(b"geturl",b"geturl [-fsS4] [--http1.1] https://HOST/PATH",b"Native HTTPS transfer. Use https authorize confirm=true for explicit network authority."),
    entry!(b"https",b"https authorize confirm=true | https get HOST PATH CONNECT SEND RECEIVE RESOLVE",b"Native authenticated HTTPS service using explicit capability IDs."),
    entry!(b"browse",b"browse https://HOST/PATH",b"Open the native browser with normal network consent and policy."),
    entry!(b"provider",b"provider list",b"Inspect registered local inference providers."),
    entry!(b"ai compute",b"ai compute",b"Inspect native inference compute capabilities."),
    entry!(b"ai profile",b"ai profile",b"Inspect the active native AI profile."),
    entry!(b"ai bench timing",b"ai bench timing",b"Inspect bounded inference benchmark timing."),
    entry!(b"ai workers",b"ai workers",b"Inspect native inference worker state."),
    entry!(b"ai timing",b"ai timing",b"Inspect inference performance counters."),
    entry!(b"voice conversation",b"voice conversation start|stop",b"Start or stop the permission-checked conversational voice pipeline."),
    entry!(b"voice listen",b"voice listen",b"Alias for voice conversation start."),
    entry!(b"voice say",b"voice say TEXT",b"Request native speech output subject to voice permissions."),
    entry!(b"voice devices",b"voice devices",b"Inspect available native input/output devices."),
    entry!(b"compute",b"compute list | inspect ID | authorize scope=N seconds=N confirm=true | authorize-cancel task=N seconds=N confirm=true | revoke CAPABILITY",b"Local administration of distributed compute authority; remote SSH does not inherit it. See help compute submit/cancel."),
    entry!(b"compute submit",b"compute submit scope=N capability=N kind=checksum|counter|accelerator locality=local|remote|prefer-local|prefer-remote durability=pinned|restartable work=N memory=N cpu=N deadline=N [node=ID request-grant=N cancel-grant=N] [fallback=ID fallback-request-grant=N fallback-cancel-grant=N]",b"Submit a bounded typed native compute job with explicit scoped authority."),
    entry!(b"compute cancel",b"compute cancel TASK_ID COMPUTE_CANCEL_CAP",b"Cancel exactly one job with its explicit cancellation grant."),
    entry!(b"plan",b"plan DOMAIN ACTION [TARGET] [key=value]",b"Validate an operation plan without execution. Typed pipelines use |>; rendered text is never executed."),
    entry!(b"clear",b"clear",b"Clear Console output."),
];
// ------------------------=
// FUNC: requested
// DESC: Recognizes help consistently before local or SSH command dispatch.
// ------------------=
pub fn requested(command: &[u8]) -> bool { command == b"help" || command.starts_with(b"help ") }
// ------------------------=
// FUNC: render
// DESC: Discovers every typed operation from its execution schema and every registered native front end without a parallel parser.
// ------------------=
pub fn render(command: &[u8], out: &mut Bytes<OUTPUT_BYTES>) -> bool {
    let topic=command.strip_prefix(b"help").unwrap_or(command).trim_ascii();
    if topic.is_empty() {
        out.write_line(b"InfinityOS Console reference");
        out.write_line(b"help commands | help DOMAIN | help DOMAIN ACTION | help COMMAND");
        out.write_line(b"help search WORD - find registered commands and operations");
        out.write_line(b"Local pages: Enter/Space next; Esc returns to the prompt.");
        out.write_line(b"Registered operation domains:");
        for d in language::DOMAINS {out.write_segments(&[d.name,b" - ",d.description]);}
        out.write_line(b"Use help commands for filesystem, editor, SSH, Python and other front ends.");return true;
    }
    if topic==b"commands" {
        for entry in COMMANDS {out.write_line(entry.usage);}
        return true;
    }
    if let Some(query)=topic.strip_prefix(b"search ") {
        if query.is_empty(){return false;}
        for entry in COMMANDS {if contains(entry.name,query)||contains(entry.description,query){out.write_line(entry.usage);}}
        for op in language::OPERATIONS {if contains(op.domain,query)||contains(op.action,query)||contains(op.description,query){out.write_segments(&[op.domain,b" ",op.action,b" - ",op.description]);}}
        return true;
    }
    if let Some(entry)=COMMANDS.iter().find(|entry|entry.name.eq_ignore_ascii_case(topic)) {
        out.write_line(entry.usage);out.write_line(entry.description);return true;
    }
    match language::parse(command) {
        Ok(language::ParseOutcome::Help(domain,operation)) => {super::console_execution::render_help(out,domain,operation);
            if operation.is_none() {if let Some(d)=domain {for entry in COMMANDS {
                if entry.name.starts_with(d.name)&&entry.name.get(d.name.len())==Some(&b' ') {out.write_line(entry.usage);}
            }}}true},
        _ => {out.write_line(b"No registered topic. Use help commands or help search WORD.");false}
    }
}
// ------------------------=
// FUNC: contains
// DESC: Performs bounded case-insensitive catalog matching without allocation.
// ------------------=
fn contains(text:&[u8],query:&[u8])->bool { !query.is_empty() && text.windows(query.len()).any(|w|w.eq_ignore_ascii_case(query)) }
