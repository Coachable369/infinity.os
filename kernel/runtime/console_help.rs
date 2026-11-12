//! Central Console documentation: operation schemas plus registered native front ends.
use super::{console_language as language, console_execution::{Bytes, OUTPUT_BYTES}};
pub struct Entry { pub name: &'static [u8], pub usage: &'static [u8], pub description: &'static [u8] }
macro_rules! entry { ($n:literal,$u:literal,$d:literal) => { Entry{name:$n,usage:$u,description:$d} }; }
pub static COMMANDS: &[Entry] = &[
    entry!(b"ai memory",b"ai memory status | stats | search QUERY | inspect ID | remember TEXT | update ID to TEXT | forget ID",b"Owner-private persistent AIKit memory. Alias: infinity memory. Open Infinity Memory for semantic search and editing. memory status still reports RAM."),
    entry!(b"exit",b"exit",b"Leave Command Window and return to the appropriate desktop or startup surface."),
    entry!(b"hello",b"hello",b"Display the built-in system greeting."),
    entry!(b"system status",b"system status",b"Inspect kernel and driver readiness."),
    entry!(b"system info",b"system info",b"Inspect the current machine and system build."),
    entry!(b"system generation",b"system generation",b"Inspect the active installed system generation."),
    entry!(b"system boot",b"system boot",b"Inspect boot handoff and selected boot source."),
    entry!(b"memory status",b"memory status",b"Inspect measured native memory availability."),
    entry!(b"device list",b"device list",b"List devices discovered by native drivers."),
    entry!(b"tasks",b"tasks",b"Alias for task list."),
    entry!(b"task list",b"task list",b"Inspect registered processes and live resource usage."),
    entry!(b"task inspect",b"task inspect HANDLE",b"Inspect one numeric task handle from task list."),
    entry!(b"task launch",b"task launch APP",b"Start a registered native application."),
    entry!(b"task end",b"task end HANDLE",b"End a selected process. Protected system tasks cannot be ended. Unsaved editor work retains its confirmation."),
    entry!(b"task pause",b"task pause HANDLE",b"Pause a runnable task selected from task list."),
    entry!(b"task resume",b"task resume HANDLE",b"Resume a paused task."),
    entry!(b"task relaunch",b"task relaunch HANDLE",b"Relaunch a supported built-in task. Reopen native Python/C applications normally."),
    entry!(b"task throttle",b"task throttle HANDLE cpu=N memory=N queue=N io=N",b"Update a task resource budget; inspect the task first and choose valid positive limits."),
    entry!(b"service",b"service list | inspect|logs|start|stop|kill|restart|enable|disable|delete NAME",b"Inspect and control native managed Python daemons. Changes require an administrator. Stop/kill pause automatic execution; start resumes it. Disable preserves an active run."),
    entry!(b"service create",b"service create NAME /path.py [start=manual|boot] [delay=SECONDS] [restart=never|on-failure|always] [enabled=true|false]",b"Persist a managed Python daemon and its startup policy."),
    entry!(b"service edit",b"service edit NAME [/path.py] [start=manual|boot] [delay=SECONDS] [restart=never|on-failure|always] [enabled=true|false]",b"Update the selected daemon's script or startup policy."),
    entry!(b"taskscheduler",b"taskscheduler list | inspect|logs|run|stop|kill|restart|enable|disable|delete NAME",b"Inspect and control persisted scheduled Python tasks from Command Window or Task Manager."),
    entry!(b"taskscheduler create",b"taskscheduler create NAME /path.py [cron=\"MIN HOUR DOM MONTH DOW\" | every=SECONDS | at=YYYY-MM-DDTHH:MM:SSZ] [enabled=true|false]",b"Create a native Python task using five-field UTC cron. Lists, ranges and steps work. Default: 0 * * * *. schedule is a supported compatibility alias."),
    entry!(b"taskscheduler cron",b"taskscheduler create NAME MIN HOUR DOM MONTH DOW /path.py",b"Standard five-field cron in UTC: minute 0-59, hour 0-23, day 1-31, month 1-12, weekday 0-7 (Sunday 0 or 7). Use *, comma lists, inclusive ranges or */N steps. Both restricted day fields use OR."),
    entry!(b"taskscheduler edit",b"taskscheduler edit NAME [/path.py] [cron=\"MIN HOUR DOM MONTH DOW\" | every=SECONDS | at=YYYY-MM-DDTHH:MM:SSZ] [enabled=true|false]",b"Change a scheduled task's script, trigger, or enabled state."),
    entry!(b"runtime contexts",b"runtime contexts",b"Inspect native execution contexts."),
    entry!(b"capability list",b"capability list",b"Inspect current capability grants without creating authority."),
    entry!(b"event subscriptions",b"event subscriptions",b"Inspect registered event subscriptions."),
    entry!(b"event trace",b"event trace",b"Inspect retained event tracing records."),
    entry!(b"system resources",b"system resources",b"Inspect native resource accounting."),
    entry!(b"voice stop",b"voice stop",b"Stop active conversation/listening. Alias: voice conversation stop."),
    entry!(b"voice status",b"voice status",b"Inspect voice pipeline state without starting capture."),
    entry!(b"ai ask",b"ai ask PROMPT | llm ask PROMPT",b"Ask the selected local Hermes or Ministral model; its response returns to this terminal when complete."),
    entry!(b"llm",b"llm PROMPT | llm ask PROMPT | llm decide QUESTION | llm response | llm cancel | llm new | llm help",b"Local language-model prompt and response commands. Advice never grants authority to execute OS actions."),
    entry!(b"ai new",b"ai new | llm new",b"Start a fresh idle conversation owned by this session. Cancel any active generation first."),
    entry!(b"ai decide",b"ai decide QUESTION",b"Compare supplied facts, constraints and measured telemetry. Numeric choices use 2-8 options, 1-8 criteria, integer weights totaling 100 and whole scores 0-1000 (higher is better); eligible masks enforce constraints. Ties and no eligible option are reported."),
    entry!(b"ai response",b"ai response",b"Display this Command Window session's current AI reply, including partial output."),
    entry!(b"ai cancel",b"ai cancel",b"Cancel only the current AI generation initiated by this Command Window session."),
    entry!(b"ai status",b"ai status",b"Inspect inference availability and measured timing."),
    entry!(b"ai trace",b"ai trace",b"Inspect recent AI execution tracing."),
    entry!(b"model list",b"model list",b"List installed local model descriptors."),
    entry!(b"model inspect",b"model inspect local-intent-v1",b"Inspect the built-in local intent model descriptor."),
    entry!(b"agent list",b"agent list",b"List registered native AI agents."),
    entry!(b"storage usage",b"storage usage",b"Inspect storage byte usage and capacity."),
    entry!(b"user add",b"user add handle=NAME display-name=\"DISPLAY NAME\"",b"Create a user with privately entered and confirmed password. Alias of user create. Administrators can update, enable, disable and delete users through the user domain."),
    entry!(b"group add",b"group add name=NAME [display-name=\"DISPLAY NAME\"] | group add group:NAME user=user:HANDLE",b"Create a group using name=, or add a user to an existing group using a group: target. The built-in administrators group cannot be renamed or deleted. The installer account starts as an administrator."),
    entry!(b"PATH",b"PATH | PATH=$PATH;/another/directory",b"Show or replace this session's semicolon-separated script search directories. Default: /System."),
    entry!(b"motd",b"motd | motd set MESSAGE",b"Read /System/motd, or customize the terminal greeting as an administrator."),
    entry!(b"grep",b"grep [-in] PATTERN [FILE ...]",b"Print matching lines; -i ignores case, -n includes line numbers. Without files, read stdin."),
    entry!(b"sed",b"sed 's/pattern/replacement/g' [FILE ...]",b"Print transformed lines. Omit g to replace only the first match on each line."),
    entry!(b"awk",b"awk [-F SEP] '{print $1,$2}' [FILE ...]",b"Print numbered fields, the whole record ($0), NR or NF."),
    entry!(b"figlet",b"figlet [-f standard|small] [-o FILE] TEXT",b"Render ASCII letters using a packaged FIGfont. -o creates a new personal text object."),
    entry!(b"help",b"help [commands|DOMAIN|DOMAIN ACTION|COMMAND|search WORD]",b"Search the command catalog as you type. Enter opens a manual, Tab inserts an editable example, and Esc returns. Named topics print their manual."),
    entry!(b"disk",b"disk",b"Explore the system disk and partitions. Enter inspects a partition, arrows select. D moves your file to Trash or deletes a selected Trash entry after confirmation. U restores a selected Trash file. R refreshes; Q exits."),
    entry!(b"top",b"top",b"Live local process monitor: use arrows to select, p to pause or resume, k to end, r to relaunch a built-in app, q to return."),
    entry!(b"ipconfig",b"ipconfig [--all]",b"All native interfaces, link state, MAC, MTU, IP/prefix, gateway, DNS and traffic counters. Read only."),
    entry!(b"list",b"list [-l|--long] [-a|--all] [-R|--tree] [PATH]",b"List native Type, Owner, Size (B), Modified (UTC), and Name columns. ls and dir are aliases; directories end with /. Legacy timestamps are not recorded. -l remains accepted; -a includes hidden entries and -R includes descendants."),
    entry!(b"ls",b"ls [-laR] [PATH]",b"Alias for list; identical options and permissions."),
    entry!(b"dir",b"dir [-laR] [PATH]",b"Alias for list; identical options and permissions."),
    entry!(b"edit",b"edit FILE",b"Local terminal text editor, new or existing file (16 KiB ASCII). Arrows/Home/End move; Ctrl+S/O saves, Ctrl+X exits, Ctrl+W finds, Ctrl+G goes to a line, Ctrl+K cuts a line, Ctrl+U pastes it, Ctrl+Z/Y undo/redo. Unsaved exit asks Save/Discard/Cancel."),
    entry!(b"cd",b"cd [PATH|home|parent|previous]",b"Change this Console's namespace. idir is an alias; relative paths are resolved from the current location."),
    entry!(b"idir",b"idir [PATH|home|parent|previous]",b"Native namespace navigation; alias of cd."),
    entry!(b"path",b"path",b"Print the current namespace."),
    entry!(b"find",b"find TEXT",b"Search bounded namespace paths case-insensitively."),
    entry!(b"cat",b"cat [-f] [-n LINES] FILE",b"Read an authorized text or log object (up to 16 KiB). -f follows appended content once per second; Ctrl+C or Escape stops. Paths with spaces may be quoted. Replaced or truncated files restart from the beginning."),
    entry!(b"tail",b"tail [-f] [-n LINES] FILE",b"The shared cat reader, defaulting to the last 10 lines. Example: tail -f /Logs/ai.audiopipeline/pipeline-00.log. Requires an authenticated session and file read permission. Displays exact appended bytes; -n 0 starts with future additions only."),
    entry!(b"examine",b"examine PATH",b"Inspect object identity, type, reference count, version and size."),
    entry!(b"resolve",b"resolve PATH",b"Resolve a namespace path to its immutable ObjectId."),
    entry!(b"versions",b"versions PATH",b"Inspect object version history."),
    entry!(b"references",b"references PATH",b"List namespace references to an object."),
    entry!(b"relationships",b"relationships PATH",b"Inspect typed object relationships."),
    entry!(b"open",b"open PATH",b"Open a supported resource with its registered application."),
    entry!(b"preview",b"preview PATH",b"Preview a supported file using the native preview service."),
    entry!(b"wallpaper",b"wallpaper IMAGE_PATH",b"Set the current user's desktop wallpaper."),
    entry!(b"ssh",b"ssh USER@HOST [-p PORT] | ssh status | ssh enable [PORT] [LIMIT] | ssh disable",b"Connect to a remote host with private password entry and an interactive terminal. Ctrl+] disconnects. The listener defaults to enabled on port 22 with password or public-key login. ssh status shows the listening address and a connection example. Configuration requires an active administrator. Local and SSH sessions use the same account permissions. Use help ssh key for registration."),
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
    entry!(b"compute",b"compute list | inspect ID | authorize scope=N seconds=N confirm=true | authorize-cancel task=N seconds=N confirm=true | revoke CAPABILITY",b"Active administrators can approve scoped compute grants from local or SSH sessions. Submission and cancellation require explicit grants held by that session. See help compute submit/cancel."),
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
        for op in language::OPERATIONS {out.write_segments(&[op.domain,b" ",op.action,b" - ",op.description]);}
        return true;
    }
    if let Some(query)=topic.strip_prefix(b"search ") {
        if query.is_empty(){return false;}
        for entry in COMMANDS {if contains(entry.name,query)||contains(entry.description,query){out.write_line(entry.usage);}}
        for op in language::OPERATIONS {if contains(op.domain,query)||contains(op.action,query)||contains(op.description,query){out.write_segments(&[op.domain,b" ",op.action,b" - ",op.description]);}}
        return true;
    }
    if let Some(entry)=COMMANDS.iter().find(|entry|entry.name.eq_ignore_ascii_case(topic)) {
        let index = COMMANDS.iter().position(|item| core::ptr::eq(item, entry)).unwrap();
        manual(index, out); return true;
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

// ------------------------=
// FUNC: catalog_count
// DESC: Includes native front ends and all typed service operations in local Help.
// ------------------=
pub fn catalog_count() -> usize { COMMANDS.len() + language::OPERATIONS.len() }

// ------------------------=
// FUNC: catalog_name
// DESC: Formats the canonical command name without executing or authorizing it.
// ------------------=
pub fn catalog_name(index: usize, out: &mut Bytes<128>) {
    if let Some(entry) = COMMANDS.get(index) { out.write(entry.name); }
    else if let Some(op) = language::OPERATIONS.get(index.saturating_sub(COMMANDS.len())) {
        out.write(op.domain); out.write(b" "); out.write(op.action);
    }
}

// ------------------------=
// FUNC: catalog_matches
// DESC: Searches names, syntax and descriptions case insensitively as the user types.
// ------------------=
pub fn catalog_matches(index: usize, query: &[u8]) -> bool {
    if index >= catalog_count() { return false; }
    let mut name = Bytes::<128>::new(); catalog_name(index, &mut name);
    if index >= COMMANDS.len() && COMMANDS.iter().any(|entry| entry.name.eq_ignore_ascii_case(name.bytes())) { return false; }
    if query.is_empty() { return true; }
    if contains(name.bytes(), query) { return true; }
    if let Some(entry) = COMMANDS.get(index) {
        contains(entry.usage, query) || contains(entry.description, query)
    } else if let Some(op) = language::OPERATIONS.get(index.saturating_sub(COMMANDS.len())) {
        contains(op.description, query) || contains(op.example, query)
    } else { false }
}

// ------------------------=
// FUNC: filtered_entry
// DESC: Resolves a visible catalog row to a stable entry without allocating.
// ------------------=
pub fn filtered_entry(query: &[u8], row: usize) -> Option<usize> {
    (0..catalog_count()).filter(|index| catalog_matches(*index, query)).nth(row)
}

// ------------------------=
// FUNC: command_draft
// DESC: Returns a documented editable example; the caller must never execute this automatically.
// ------------------=
pub fn command_draft(index: usize) -> &'static [u8] {
    if let Some(entry) = COMMANDS.get(index) { details(entry.name).0 }
    else { language::OPERATIONS.get(index.saturating_sub(COMMANDS.len())).map_or(b"", |op| op.example) }
}

// ------------------------=
// FUNC: manual
// DESC: Renders the complete selected page from authoritative schema and native front end documentation.
// ------------------=
pub fn manual(index: usize, out: &mut Bytes<OUTPUT_BYTES>) {
    if let Some(entry) = COMMANDS.get(index) {
        out.write_line(entry.usage); out.write_line(entry.description);
        let (example, permissions, result) = details(entry.name);
        out.write_segments(&[b"Permissions: ", permissions]);
        out.write_segments(&[b"Example: ", example]);
        out.write_segments(&[b"Expected output: ", result]);
    } else if let Some(op) = language::OPERATIONS.get(index.saturating_sub(COMMANDS.len())) {
        super::console_execution::render_help(out, None, Some(op));
    }
}

#[path = "console_help_details.rs"] mod documentation;
use documentation::details;

/// Input actions shared by the native controller and behavioral harness.
#[derive(Clone,Copy)]
pub enum Navigation { Up, Down, Open, Insert, Back, Erase, Type(u8), Ignore }
// ------------------------=
// FUNC: navigate
// DESC: Advances interactive Help and returns only an editable draft, never an executable dispatch.
// ------------------=
pub fn navigate(s:&mut crate::ui::command_terminal::State,key:Navigation)->Option<&'static [u8]> {
    let selected=filtered_entry(&s.help_query[..s.help_query_length],s.help_selected);
    let draft=match key {Navigation::Insert=>selected.map(command_draft),Navigation::Open if s.help_detail=>selected.map(command_draft),_=>None};
    match key {
        Navigation::Back=>if s.help_detail{s.help_detail=false;s.help_scroll=0;}else{s.help=false;},
        Navigation::Up if s.help_detail=>s.help_scroll=s.help_scroll.saturating_sub(1),
        Navigation::Down if s.help_detail=>{
            let mut page=Bytes::<OUTPUT_BYTES>::new();if let Some(index)=selected{manual(index,&mut page);}
            s.help_scroll=s.help_scroll.saturating_add(1).min(manual_lines(page.bytes(),s.columns).count().saturating_sub(s.rows.saturating_sub(2)));
        }
        Navigation::Up=>s.help_selected=s.help_selected.saturating_sub(1),
        Navigation::Down=>if filtered_entry(&s.help_query[..s.help_query_length],s.help_selected+1).is_some(){s.help_selected+=1;},
        Navigation::Open if selected.is_some()=>{s.help_detail=true;s.help_scroll=0;},
        Navigation::Type(byte @ 32..=126) if !s.help_detail && s.help_query_length<s.help_query.len()=>{
            s.help_query[s.help_query_length]=byte;s.help_query_length+=1;s.help_selected=0;
        }
        Navigation::Erase if !s.help_detail=>{s.help_query_length=s.help_query_length.saturating_sub(1);s.help_selected=0;},
        _=>{}
    }
    if draft.is_some(){s.help=false;}s.changed();draft
}
#[derive(Clone)]
pub struct ManualLines<'a>{remaining:&'a[u8],columns:usize}
// ------------------------=
// FUNC: manual_lines
// DESC: Wraps manual paragraphs on word boundaries while preserving explicit newlines.
// ------------------=
pub fn manual_lines(bytes:&[u8],columns:usize)->ManualLines<'_>{ManualLines{remaining:bytes,columns:columns.max(1)}}
impl<'a> Iterator for ManualLines<'a>{
    type Item=&'a[u8];
    // ------------------------=
    // FUNC: next
    // DESC: Returns the next bounded line, consuming a delimiter without dropping command characters.
    // ------------------=
    fn next(&mut self)->Option<Self::Item>{
        if self.remaining.is_empty(){return None;}
        let end=self.remaining.iter().position(|b|*b==b'\n').unwrap_or(self.remaining.len());
        let (length,advance)=if end<=self.columns{(end,(end+1).min(self.remaining.len()))}else{
            match self.remaining[..self.columns+1].iter().rposition(|b|*b==b' '){Some(n)if n>0=>(n,n+1),_=>(self.columns,self.columns)}
        };
        let line=&self.remaining[..length];self.remaining=&self.remaining[advance..];Some(line)
    }
}
