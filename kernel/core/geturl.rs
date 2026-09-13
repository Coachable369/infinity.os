//! Console adapter for the native transfer actor; never grants ambient permissions.
use super::*;
use crate::http_transport::geturl::{Command, Options};
use crate::runtime::{capability::CapabilityType, execution::SecurityIdentity};
use core::sync::atomic::{AtomicU32, Ordering};

#[no_mangle]
pub static INFINITY_GETURL_EXIT: AtomicU32 = AtomicU32::new(0);
static mut PENDING: Option<(SecurityIdentity, bool, bool, bool)> = None;

// ------------------------=
// FUNC: authorize
// DESC: Requires explicit authenticated operator consent for a sixty-second session network lease; endpoint policy remains mandatory.
// ------------------=
pub(super) fn authorize(console: &mut ConsoleRuntime, confirmed: bool) {
    if !confirmed {
        console.output.write_line(
            b"Grant this session connect/send/receive/DNS capabilities for 60 seconds.",
        );
        console.output.write_line(
            b"Scope: all destinations still allowed by Network policy. Not persistent.",
        );
        console
            .output
            .write_line(b"To approve: https authorize confirm=true");
        return;
    }
    let Ok(lease) = crate::runtime::node_client::begin_capability_input(
        console.current_user,
        console.current_session,
    ) else {
        console
            .output
            .write_line(b"HTTPS authorization requires an authenticated privileged operator.");
        return;
    };
    let owner = SecurityIdentity(console.current_session.0);
    let result = crate::runtime::node_client::clock()
        .and_then(|now| {
            crate::runtime::with_runtime(|runtime| {
                let kinds = [
                    CapabilityType::NetworkConnect,
                    CapabilityType::NetworkSend,
                    CapabilityType::NetworkReceive,
                    CapabilityType::NetworkResolve,
                ];
                let mut created = [None; 4];
                for (i, kind) in kinds.into_iter().enumerate() {
                    if (0..runtime.capabilities.count())
                        .filter_map(|i| runtime.capabilities.nth(i))
                        .any(|c| {
                            runtime
                                .capabilities
                                .validate(c.id, owner, kind, 0, 1, 0, now)
                                .is_ok()
                        })
                    {
                        continue;
                    }
                    match runtime.capabilities.grant(
                        kind,
                        0,
                        1,
                        0,
                        owner,
                        owner,
                        Some(now.saturating_add(60)),
                        0,
                    ) {
                        Ok(id) => created[i] = Some(id),
                        Err(_) => {
                            for id in created.into_iter().flatten() {
                                let _ = runtime.capabilities.revoke(id);
                            }
                            return false;
                        }
                    }
                }
                true
            })
        })
        .unwrap_or(false);
    crate::runtime::with_runtime(|runtime| {
        let _ = runtime.ui.trusted.release_secure_input(lease);
    });
    console.output.write_line(if result {
        b"Temporary network capabilities granted; Network policy still applies."
    } else {
        b"Network lease could not be granted."
    });
}

// ------------------------=
// FUNC: execute
// DESC: Dispatches one native HTTPS transfer using only valid capabilities already held by the authenticated console session.
// ------------------=
pub(super) fn execute(console: &mut ConsoleRuntime, command: &[u8]) -> bool {
    if command != b"geturl" && !command.starts_with(b"geturl ") {
        return false;
    }
    let text = core::str::from_utf8(command).unwrap_or("");
    let parsed = crate::http_transport::geturl::parse(text.split_ascii_whitespace().skip(1));
    let options = match parsed {
        Ok(Command::Help) => {
            INFINITY_GETURL_EXIT.store(0, Ordering::Release);
            console
                .output
                .write_line(b"geturl [-fsS4] [--http1.1] https://HOST/PATH");
            console.output.write_line(b"Initial HTTPS GET only; 8 KiB limit, 30-second deadline. Not full curl compatibility.");
            console.output.write_line(
                b"Requires existing session network permissions. Other options fail explicitly.",
            );
            return true;
        }
        Ok(Command::Version) => {
            INFINITY_GETURL_EXIT.store(0, Ordering::Release);
            console
                .output
                .write_line(b"geturl 0.1 native; HTTPS TLS1.3 HTTP1.1 IPv4; limited compatibility");
            return true;
        }
        Ok(Command::Get(options)) => options,
        Err(error) => {
            INFINITY_GETURL_EXIT.store(error.exit_code() as u32, Ordering::Release);
            console.output.write_line(
                b"geturl: unsupported option/protocol or invalid URL. See geturl --help.",
            );
            return true;
        }
    };
    if unsafe { (&*(&raw const PENDING)).is_some() } {
        fail(
            console,
            options,
            2,
            b"geturl: a transfer is already pending.",
        );
        return true;
    }
    let owner = SecurityIdentity(console.current_session.0);
    let now = crate::ui::performance::monotonic_ns().unwrap_or(0) / 1_000_000_000;
    let capabilities = crate::runtime::with_runtime(|runtime| {
        if !(0..crate::runtime::identity::MAX_SESSIONS)
            .filter_map(|i| runtime.identity.session_nth(i))
            .any(|s| {
                s.id == console.current_session
                    && s.user == console.current_user
                    && s.state == crate::runtime::identity::SessionState::Active
            })
        {
            return None;
        }
        let mut ids = [0; 4];
        for (i, kind) in [
            CapabilityType::NetworkConnect,
            CapabilityType::NetworkSend,
            CapabilityType::NetworkReceive,
            CapabilityType::NetworkResolve,
        ]
        .into_iter()
        .enumerate()
        {
            ids[i] = (0..runtime.capabilities.count())
                .filter_map(|i| runtime.capabilities.nth(i))
                .find(|c| {
                    runtime
                        .capabilities
                        .validate(c.id, owner, kind, 0, 1, 0, now)
                        .is_ok()
                })?
                .id;
        }
        Some(ids)
    })
    .flatten();
    let Some(ids) = capabilities else {
        fail(
            console,
            options,
            7,
            b"geturl: network permission required; see https authorize.",
        );
        return true;
    };
    match crate::drivers::https::get(
        owner,
        ids[0],
        ids[1],
        ids[2],
        ids[3],
        options.host,
        options.target,
    ) {
        Ok(()) => {
            unsafe {
                PENDING = Some((owner, options.fail, options.silent, options.show_error));
            }
            INFINITY_GETURL_EXIT.store(u32::MAX, Ordering::Release);
        }
        Err(_) => fail(
            console,
            options,
            7,
            b"geturl: network policy, configuration or HTTPS service unavailable.",
        ),
    }
    true
}
// ------------------------=
// FUNC: fail
// DESC: Records a typed status and respects silent/show-error when displaying transfer admission failures.
// ------------------=
fn fail(console: &mut ConsoleRuntime, options: Options<'_>, code: u32, message: &[u8]) {
    INFINITY_GETURL_EXIT.store(code, Ordering::Release);
    if !options.silent || options.show_error {
        console.output.write_line(message);
    }
}
// ------------------------=
// FUNC: poll
// DESC: Collects completed authenticated content without blocking input or exposing a former session's response after lock/logout.
// ------------------=
pub(super) fn poll(console: &mut ConsoleRuntime) {
    let Some((owner, fail_http, silent, show_error)) = (unsafe { PENDING }) else {
        return;
    };
    let active = owner.0 == console.current_session.0
        && crate::runtime::with_runtime(|r| {
            (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|i| r.identity.session_nth(i))
                .any(|s| {
                    s.id.0 == owner.0 && s.state == crate::runtime::identity::SessionState::Active
                })
        })
        .unwrap_or(false);
    if !active {
        let _ = crate::drivers::https::cancel(owner);
        let _ = crate::drivers::https::take(owner);
        unsafe {
            PENDING = None;
        }
        INFINITY_GETURL_EXIT.store(42, Ordering::Release);
        return;
    }
    let result = match crate::drivers::https::take(owner) {
        Ok(None) => return,
        Ok(Some(value)) => value,
        Err(_) => Err(crate::drivers::https::Failure::Denied),
    };
    unsafe {
        PENDING = None;
    }
    let mut code = 0;
    match result {
        Ok(response) if !fail_http || response.status < 400 => {
            for line in response.bytes[..response.length].split(|b| *b == b'\n') {
                if line.is_empty() {
                    console.output.write_line(b"");
                    continue;
                }
                for chunk in line.chunks(LINE_CAPACITY) {
                    let mut safe = [0; LINE_CAPACITY];
                    for (i, b) in chunk.iter().enumerate() {
                        safe[i] = if b.is_ascii_control() && *b != b'\t' {
                            b' '
                        } else {
                            *b
                        };
                    }
                    console.output.write_line(&safe[..chunk.len()]);
                }
            }
        }
        Ok(_) => {
            code = 22;
            if !silent || show_error {
                console
                    .output
                    .write_line(b"geturl: HTTP request failed (--fail).");
            }
        }
        Err(error) => {
            code = match error {
                crate::drivers::https::Failure::Timeout => 28,
                crate::drivers::https::Failure::Resolution => 6,
                crate::drivers::https::Failure::Cancelled => 42,
                _ => 7,
            };
            if !silent || show_error {
                console
                    .output
                    .write_line(b"geturl: native HTTPS transfer failed.");
            }
        }
    }
    INFINITY_GETURL_EXIT.store(code, Ordering::Release);
    console.redraw();
}
