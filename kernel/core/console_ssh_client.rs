//! Private outbound SSH terminal input. Passwords never enter command history or transcript.
use super::{ConsoleRuntime, ConsoleKey, ConsoleMode, DesktopAppKind};
#[cfg(any(target_arch="aarch64",target_arch="x86_64"))]
use crate::drivers::ssh_client;
use crate::runtime::identity::StableId;
use infinity_ssh::client::State;

pub(super) struct Terminal {
    session: StableId,
    active: bool,
    shown: Option<State>,
    secret: [u8; 1024],
    length: usize,
    sending: bool,
    sent: usize,
}
impl Terminal {
    // ------------------------=
    // FUNC: new
    // DESC: Starts an inactive terminal with private bounded credential storage.
    // ------------------=
    pub const fn new() -> Self { Self { session: StableId::zero(), active: false, shown: None, secret: [0; 1024], length: 0, sending: false, sent: 0 } }
    // ------------------------=
    // FUNC: erase
    // DESC: Erases password or remote input on submit, cancellation and session changes.
    // ------------------=
    fn erase(&mut self) { zeroize::Zeroize::zeroize(&mut self.secret); self.length = 0; self.sending = false; self.sent = 0; }
}
#[cfg(any(target_arch="aarch64",target_arch="x86_64"))]
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: ssh_connect_command
    // DESC: Opens an outbound SSH terminal without accepting passwords in command arguments.
    // ------------------=
    pub(super) fn ssh_connect_command(&mut self, command: &[u8]) -> bool {
        let Ok(text) = core::str::from_utf8(command) else { return false; };
        let mut words = text.split_ascii_whitespace();
        if words.next() != Some("ssh") { return false; }
        let Some(target) = words.next() else { return false; };
        let Some((user, host)) = target.split_once('@') else { return false; };
        let port = match words.next() {
            None => Some(22), Some("-p") => words.next().and_then(|s| s.parse::<u16>().ok()), _ => None,
        };
        if words.next().is_some() || port.is_none_or(|p| p == 0) || host.is_empty() {
            self.output.write_line(b"Use ssh user@host [-p port]. Enter the password at the private prompt."); return true;
        }
        let now = crate::runtime::ai::qwen::workers::clock_ns() / 1_000_000;
        match ssh_client::begin_host(self.current_session, host, port.unwrap(), user, now) {
            Ok(()) => {
                self.ssh_terminal.erase(); self.ssh_terminal.active = true;
                self.ssh_terminal.session = self.current_session; self.ssh_terminal.shown = None;
                self.output.write_line(b"Connecting with SSH. Ctrl+] disconnects. Verify the server fingerprint before entering your password.");
            }
            Err(_) => self.output.write_line(b"SSH connection could not start. Check your session, network and destination."),
        }
        true
    }
    // ------------------------=
    // FUNC: ssh_terminal_input
    // DESC: Captures private authentication and remote input before local history, completion or echo.
    // ------------------=
    pub(super) fn ssh_terminal_input(&mut self, key: ConsoleKey) -> bool {
        if !self.ssh_terminal.active { return false; }
        if self.ssh_terminal.session != self.current_session { self.ssh_terminal_cancel(); return false; }
        let visible = matches!(self.mode, ConsoleMode::Console)
            || self.mode == ConsoleMode::Desktop && self.desktop_app == DesktopAppKind::CommandWindow;
        if !visible { self.ssh_terminal.erase(); return false; }
        if matches!(key, ConsoleKey::Shortcut(b']')) { self.ssh_terminal_cancel(); return true; }
        let Some(state) = ssh_client::state(self.current_session) else { self.ssh_terminal_cancel(); return true; };
        if state!=State::Connected&&matches!(key,ConsoleKey::Escape){self.ssh_terminal_cancel();return true;}
        let now = crate::runtime::ai::qwen::workers::clock_ns() / 1_000_000;
        match state {
            State::HostKey(key_bytes) => {
                if matches!(key, ConsoleKey::Character(b'y' | b'Y')) {
                    if ssh_client::confirm_host(self.current_session, key_bytes, now).is_err() { self.ssh_terminal_cancel(); }
                } else if matches!(key, ConsoleKey::Character(b'n' | b'N')) { self.ssh_terminal_cancel(); }
            }
            State::Password => {
                match key {
                    ConsoleKey::Character(byte) if byte >= 32 && byte != 127 && self.ssh_terminal.length < 1024 => {
                        self.ssh_terminal.secret[self.ssh_terminal.length] = byte; self.ssh_terminal.length += 1;
                    }
                    ConsoleKey::Backspace | ConsoleKey::Delete if self.ssh_terminal.length > 0 => {
                        self.ssh_terminal.length -= 1; self.ssh_terminal.secret[self.ssh_terminal.length] = 0;
                    }
                    ConsoleKey::Enter => {
                        let value = core::str::from_utf8(&self.ssh_terminal.secret[..self.ssh_terminal.length]);
                        let result = value.ok().and_then(|value| ssh_client::password(self.current_session, value, now).ok());
                        self.ssh_terminal.erase(); self.reset_input();
                        if result.is_none() { self.output.write_line(b"Password was not submitted. Retry or Escape to disconnect."); }
                    }
                    _ => (),
                }
                self.command.fill(0);
                let length = self.ssh_terminal.length.min(self.command.len());
                self.command[..length].fill(b'*'); self.command_length = length; self.command_cursor = length;
            }
            State::Connected => {
                let mut bytes=[0;4];
                let n=match key {
                    ConsoleKey::Character(b)=>{bytes[0]=b;1},
                    ConsoleKey::Shortcut(b) if b.is_ascii_alphabetic() =>{bytes[0]=b.to_ascii_uppercase()-b'A'+1;1},
                    ConsoleKey::Enter=>{bytes[0]=b'\r';1},ConsoleKey::Backspace=>{bytes[0]=127;1},ConsoleKey::Escape=>{bytes[0]=27;1},
                    ConsoleKey::Tab(false)=>{bytes[0]=9;1},
                    ConsoleKey::Up|ConsoleKey::Down|ConsoleKey::Right|ConsoleKey::Left|ConsoleKey::Home|ConsoleKey::End=>{
                        bytes[0]=27;bytes[1]=if crate::ui::terminal_vt::inspect(|t|t.application_cursor){b'O'}else{b'['};
                        bytes[2]=match key{ConsoleKey::Up=>b'A',ConsoleKey::Down=>b'B',ConsoleKey::Right=>b'C',ConsoleKey::Left=>b'D',ConsoleKey::Home=>b'H',_=>b'F'};3
                    },ConsoleKey::Delete=>{bytes.copy_from_slice(b"\x1b[3~");4},_=>0
                };
                // Compact only acknowledged bytes. Never discard a partial write when another key arrives.
                let t=&mut self.ssh_terminal;
                if t.sent>0 {t.secret.copy_within(t.sent..t.length,0);t.length-=t.sent;t.sent=0;}
                if t.length+n<=t.secret.len(){t.secret[t.length..t.length+n].copy_from_slice(&bytes[..n]);t.length+=n;t.sending=t.length>0;}
                else {crate::ui::command_terminal::mutate(|s|s.notice=b"SSH input busy: wait for the remote host");}
            }
            _ => (),
        }
        true
    }
    // ------------------------=
    // FUNC: ssh_terminal_cancel
    // DESC: Drops remote authority and erases private input without adding it to local history.
    // ------------------=
    pub(super) fn ssh_terminal_cancel(&mut self) {
        if self.current_session==self.ssh_terminal.session&&!matches!(self.mode,ConsoleMode::Locked|ConsoleMode::Authentication)&&crate::ui::command_terminal::inspect(|s|s.remote){
            crate::ui::terminal_vt::inspect(|t|{for row in 0..t.rows{let mut bytes=[0;320];for x in 0..t.columns{bytes[x]=t.cells[row][x].byte;}let n=bytes[..t.columns].iter().rposition(|b|*b!=b' ').map_or(0,|n|n+1);if n>0{crate::ui::command_terminal::mutate(|s|s.line(&bytes[..n]));}}});
        }
        ssh_client::cancel(self.ssh_terminal.session); self.ssh_terminal.erase();
        crate::ui::command_terminal::mutate(|s|s.remote=false);crate::ui::terminal_vt::mutate(|t|t.reset());
        self.ssh_terminal.active = false; self.ssh_terminal.shown = None; self.reset_input();
    }
    // ------------------------=
    // FUNC: ssh_terminal_poll
    // DESC: Displays bounded remote output and authentication transitions using existing terminal styling.
    // ------------------=
    pub(super) fn ssh_terminal_poll(&mut self) {
        if !self.ssh_terminal.active { return; }
        if self.current_session != self.ssh_terminal.session || matches!(self.mode, ConsoleMode::Locked | ConsoleMode::Authentication) {
            self.ssh_terminal_cancel(); return;
        }
        let Some(state) = ssh_client::state(self.current_session) else {
            self.ssh_terminal_cancel(); self.output.write_line(b"SSH disconnected or timed out."); self.redraw(); return;
        };
        self.publish_command_terminal();
        let _capture=crate::ui::command_terminal::Capture::new(true);
        let mut changed = self.ssh_terminal.shown != Some(state);
        if changed {
            self.ssh_terminal.shown = Some(state);
            match state {
                State::HostKey(key) => {
                    self.output.write_line(&infinity_ssh::management::fingerprint(&key));
                    self.output.write_line(b"Verify this fingerprint with the server administrator. Trust for this connection? Y / N");
                }
                State::Password => self.output.write_line(b"Remote account password (private; Escape cancels):"),
                State::Connected => {self.reset_input();crate::ui::terminal_vt::mutate(|t|t.reset());crate::ui::command_terminal::mutate(|s|{s.remote=true;s.changed();});},
                State::Exited(code) => { self.output.write_number(b"Remote exit status: ", code as u64); }
                State::Failed(_) => self.output.write_line(b"SSH authentication or connection failed."),
                _ => (),
            }
        }
        let (cols,rows)=crate::ui::command_terminal::inspect(|s|(s.columns,s.rows));
        let _=ssh_client::resize(self.current_session,cols as u32,rows as u32);
        crate::ui::terminal_vt::mutate(|t|t.resize(cols,rows));
        if state==State::Connected {crate::ui::terminal_vt::mutate(|t|{if !t.reply().is_empty(){if let Ok(n)=ssh_client::write(self.current_session,t.reply()){t.consume_reply(n);}}});}
        if state == State::Connected && self.ssh_terminal.sending {
            let from = self.ssh_terminal.sent; let end = self.ssh_terminal.length;
            if let Ok(n) = ssh_client::write(self.current_session, &self.ssh_terminal.secret[from..end]) {
                self.ssh_terminal.sent += n;
                if self.ssh_terminal.sent == end { self.ssh_terminal.erase(); self.reset_input(); changed = true; }
            }
        }
        let mut bytes = [0; 512];
        for stderr in [false, true] {
            if let Ok(n) = ssh_client::read(self.current_session, &mut bytes, stderr) {
                if n > 0 { crate::ui::terminal_vt::mutate(|t|t.feed(&bytes[..n]));crate::ui::command_terminal::mutate(|s|s.changed()); changed = true; }
            }
        }
        if matches!(state, State::Exited(_) | State::Failed(_)) { self.ssh_terminal_cancel(); }
        if changed { self.redraw(); }
    }
}

#[cfg(not(any(target_arch="aarch64",target_arch="x86_64")))]
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: ssh_terminal_input
    // DESC: Leaves input with the ordinary terminal on architectures without the native network adapter.
    // ------------------=
    pub(super) fn ssh_terminal_input(&mut self, _: ConsoleKey) -> bool { false }
    // ------------------------=
    // FUNC: ssh_terminal_poll
    // DESC: Performs no network work on unsupported targets.
    // ------------------=
    pub(super) fn ssh_terminal_poll(&mut self) {}
}
