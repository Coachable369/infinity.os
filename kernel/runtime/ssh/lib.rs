//! Bounded SSH transport only. Authentication never creates an OS session.
#![no_std]

use sunset::{ChanFail, Event, PubKey, Runner, ServEvent, Server, SignKey};
pub mod hostkey;
pub mod management;

pub const PACKET_BUFFER: usize = 35_000;
pub const MAX_CONNECTIONS: usize = 4;
pub const MAX_AUTH_ATTEMPTS: u8 = 8;
pub const HANDSHAKE_MS: u64 = 30_000;
pub const AUTHENTICATED_MS: u64 = 300_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Configuration {
    pub enabled: bool,
    pub port: u16,
    pub max_connections: u8,
}
impl Default for Configuration {
    // ------------------------=
    // FUNC: default
    // DESC: Keeps inbound SSH disabled until explicitly provisioned locally.
    // ------------------=
    fn default() -> Self { Self { enabled: false, port: 22, max_connections: 2 } }
}
impl Configuration {
    // ------------------------=
    // FUNC: validate
    // DESC: Rejects invalid ports and connection quotas before allocating network state.
    // ------------------=
    pub fn validate(self) -> Result<Self, Failure> {
        // The existing native outbound actors reserve 49152..=65535.
        if self.port == 0 || self.port >= 49152 || self.max_connections == 0 || self.max_connections as usize > MAX_CONNECTIONS {
            Err(Failure::Configuration)
        } else { Ok(self) }
    }
}

/// An identity reference, not a capability, desktop session, or authorization token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Principal { pub user: [u8; 16], pub public_key: [u8; 32] }

pub trait IdentityAuthority {
    // ------------------------=
    // FUNC: authorized_key
    // DESC: Resolves a currently active native user and registered Ed25519 key without granting capabilities.
    // ------------------=
    fn authorized_key(&self, username: &[u8], key: &[u8; 32]) -> Option<Principal>;
    // ------------------------=
    // FUNC: still_authorized
    // DESC: Rechecks account and key revocation throughout an authenticated connection.
    // ------------------=
    fn still_authorized(&self, principal: Principal) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Failure {
    Configuration, EntropyUnavailable, HostKeyUnavailable, Protocol, Deadline,
    AuthenticationLimit, Revoked, Disconnected, Capacity, PermissionDenied, NetworkUnavailable,
    UnsupportedAlgorithm, InvalidSignature, Integrity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Handshake, Authenticated, Closed(Failure) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Diagnostic { Progress, Idle, KeyOffered, AuthenticationRejected, Authenticated, ChannelDenied, ChannelOpened, ConsoleRequest }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleRequest {
    Shell,
    Exec { bytes: [u8; 512], length: usize },
    Pty { columns: u32, rows: u32 },
    Resize { columns: u32, rows: u32 },
    Interrupt,
}

pub struct Transport<'a> {
    runner: Runner<'a, Server>,
    phase: Phase,
    deadline: u64,
    candidate: Option<Principal>,
    principal: Option<Principal>,
    attempts: u8,
    console_authorized: bool,
    channel: Option<sunset::ChanHandle>,
    channel_used: bool,
    console_started: bool,
    console_pty: bool,
    request: Option<ConsoleRequest>,
}
impl<'a> Transport<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Borrows fixed packet buffers and starts an absolute handshake deadline.
    // ------------------=
    pub fn new(input: &'a mut [u8], output: &'a mut [u8], now: u64) -> Result<Self, Failure> {
        if input.len() != PACKET_BUFFER || output.len() != PACKET_BUFFER {
            return Err(Failure::Configuration);
        }
        input.fill(0);
        output.fill(0);
        Ok(Self { runner: Runner::new_server(input, output), phase: Phase::Handshake,
            deadline: now.saturating_add(HANDSHAKE_MS), candidate: None, principal: None, attempts: 0,
            console_authorized: false, channel: None, channel_used: false, console_started: false, console_pty: false, request: None })
    }
    // ------------------------=
    // FUNC: authorize_console
    // DESC: Enables one native session channel only after the service has created a restricted authenticated identity session.
    // ------------------=
    pub fn authorize_console(&mut self) -> Result<(), Failure> {
        if self.phase != Phase::Authenticated || self.principal.is_none() { return Err(Failure::PermissionDenied); }
        self.console_authorized = true; Ok(())
    }
    // ------------------------=
    // FUNC: take_console_request
    // DESC: Transfers one bounded request to the native Console without retaining untrusted borrowed packet data.
    // ------------------=
    pub fn take_console_request(&mut self) -> Option<ConsoleRequest> { self.request.take() }
    // ------------------------=
    // FUNC: read_console
    // DESC: Reads only admitted channel input with normal SSH window backpressure and live identity checks.
    // ------------------=
    pub fn read_console(&mut self, bytes: &mut [u8], authority: &impl IdentityAuthority, now: u64) -> Result<usize, Failure> {
        self.check(authority, now)?;
        let Some(channel) = self.channel.as_ref() else { return Ok(0); };
        match self.runner.read_channel(channel, sunset::ChanData::Normal, bytes) {
            Ok(n) => Ok(n), Err(sunset::Error::ChannelEOF) => Ok(0), Err(error) => Err(protocol_failure(error)),
        }
    }
    // ------------------------=
    // FUNC: write_console
    // DESC: Keeps stdout and stderr separate and returns only the prefix accepted by SSH's bounded channel window.
    // ------------------=
    pub fn write_console(&mut self, bytes: &[u8], stderr: bool, authority: &impl IdentityAuthority, now: u64) -> Result<usize, Failure> {
        self.check(authority, now)?;
        let channel = self.channel.as_ref().ok_or(Failure::Disconnected)?;
        self.runner.write_channel(channel, if stderr { sunset::ChanData::Stderr } else { sunset::ChanData::Normal }, bytes).map_err(protocol_failure)
    }
    // ------------------------=
    // FUNC: console_eof
    // DESC: Reports channel EOF independently of transport and unrelated connections.
    // ------------------=
    pub fn console_eof(&self) -> bool { self.channel.as_ref().is_some_and(|channel| self.runner.is_channel_eof(channel) || self.runner.is_channel_closed(channel)) }
    // ------------------------=
    // FUNC: finish_console
    // DESC: Sends actual exit status before releasing the application channel; retries while ciphertext or channel windows are blocked.
    // ------------------=
    pub fn finish_console(&mut self, status: u32) -> Result<bool, Failure> {
        let Some(channel) = self.channel.as_ref() else { return Ok(true); };
        if self.runner.is_output_pending() || self.runner.write_channel_ready(channel, sunset::ChanData::Normal).map_err(protocol_failure)? == Some(0) { return Ok(false); }
        self.runner.server_exit_status(channel, status).map_err(protocol_failure)?;
        self.runner.finish_channel_output(channel).map_err(protocol_failure)?;
        let channel = self.channel.take().unwrap(); self.runner.channel_done(channel).map_err(protocol_failure)?;
        Ok(true)
    }
    // ------------------------=
    // FUNC: phase
    // DESC: Reports typed transport state without exposing packets or secret material.
    // ------------------=
    pub fn phase(&self) -> Phase { self.phase }
    // ------------------------=
    // FUNC: principal
    // DESC: Exposes identity only after Sunset has verified the signed authentication request.
    // ------------------=
    pub fn principal(&self) -> Option<Principal> { self.principal }
    // ------------------------=
    // FUNC: close
    // DESC: Makes errors terminal and closes both transport directions before slot recycling.
    // ------------------=
    pub fn close(&mut self, failure: Failure) {
        self.phase = Phase::Closed(failure);
        self.candidate = None;
        self.principal = None;
        self.runner.close_input();
        self.runner.close_output();
    }
    // ------------------------=
    // FUNC: check
    // DESC: Enforces absolute deadlines and live native key authorization on every IO operation.
    // ------------------=
    fn check(&mut self, authority: &impl IdentityAuthority, now: u64) -> Result<(), Failure> {
        if let Phase::Closed(failure) = self.phase { return Err(failure); }
        let failure = if now >= self.deadline { Some(Failure::Deadline) }
            else if self.principal.or(self.candidate).is_some_and(|p| !authority.still_authorized(p)) { Some(Failure::Revoked) }
            else { None };
        if let Some(failure) = failure { self.close(failure); return Err(failure); }
        Ok(())
    }
    // ------------------------=
    // FUNC: input
    // DESC: Accepts only the prefix consumed by the SSH parser; callers retain the remainder under backpressure.
    // ------------------=
    pub fn input(&mut self, bytes: &[u8], authority: &impl IdentityAuthority, now: u64) -> Result<usize, Failure> {
        self.check(authority, now)?;
        match self.runner.input(bytes) {
            Ok(length) => Ok(length),
            Err(error) => { let failure = protocol_failure(error); self.close(failure); Err(failure) }
        }
    }
    // ------------------------=
    // FUNC: output
    // DESC: Borrows pending ciphertext without discarding it when the native TCP buffer is full.
    // ------------------=
    pub fn output(&mut self, authority: &impl IdentityAuthority, now: u64) -> Result<&[u8], Failure> {
        self.check(authority, now)?;
        Ok(self.runner.output_buf())
    }
    // ------------------------=
    // FUNC: consume_output
    // DESC: Retires only bytes actually accepted by TCP, rejecting invalid acknowledgement lengths.
    // ------------------=
    pub fn consume_output(&mut self, length: usize) -> Result<(), Failure> {
        if matches!(self.phase, Phase::Closed(_)) || length > self.runner.output_buf().len() {
            self.close(Failure::Protocol);
            return Err(Failure::Protocol);
        }
        self.runner.consume_output(length);
        Ok(())
    }
    // ------------------------=
    // FUNC: progress
    // DESC: Processes one protocol event; application channels require explicit native Console authorization and calls are bounded per tick.
    // ------------------=
    pub fn progress(&mut self, authority: &impl IdentityAuthority, host_key: &SignKey, now: u64) -> Result<Diagnostic, Failure> {
        self.check(authority, now)?;
        // Drain each response before processing another packet. This bounds
        // queued work even when an authenticated peer refuses to read replies.
        if self.runner.is_output_pending() { return Ok(Diagnostic::Idle); }
        let result = self.event(authority, host_key, now);
        if let Err(failure) = result { self.close(failure); }
        result
    }
    // ------------------------=
    // FUNC: event
    // DESC: Handles Sunset decisions without password login, anonymous success, or native session creation.
    // ------------------=
    fn event(&mut self, authority: &impl IdentityAuthority, host_key: &SignKey, now: u64) -> Result<Diagnostic, Failure> {
        let protocol = protocol_failure;
        match self.runner.progress().map_err(protocol)? {
            Event::None => Ok(Diagnostic::Idle),
            Event::Progressed | Event::Serv(ServEvent::PollAgain) => Ok(Diagnostic::Progress),
            Event::Serv(ServEvent::Hostkeys(event)) => {
                event.hostkeys(&[host_key]).map_err(protocol)?;
                Ok(Diagnostic::Progress)
            }
            Event::Serv(ServEvent::FirstAuth(mut event)) => {
                event.set_auth_methods(false, true).map_err(protocol)?;
                event.reject().map_err(protocol)?;
                Ok(Diagnostic::Progress)
            }
            Event::Serv(ServEvent::PasswordAuth(event)) => {
                event.reject().map_err(protocol)?;
                self.attempts = self.attempts.saturating_add(1);
                if self.attempts >= MAX_AUTH_ATTEMPTS { return Err(Failure::AuthenticationLimit); }
                Ok(Diagnostic::AuthenticationRejected)
            }
            Event::Serv(ServEvent::PubkeyAuth(event)) => {
                self.attempts = self.attempts.saturating_add(1);
                if self.attempts > MAX_AUTH_ATTEMPTS { return Err(Failure::AuthenticationLimit); }
                self.candidate = match (event.username(), event.pubkey()) {
                    (Ok(name), Ok(PubKey::Ed25519(key))) => authority.authorized_key(name.as_bytes(), &key.key.0),
                    _ => None,
                };
                if self.candidate.is_some() {
                    event.allow().map_err(protocol)?;
                    Ok(Diagnostic::KeyOffered)
                } else {
                    event.reject().map_err(protocol)?;
                    Ok(Diagnostic::AuthenticationRejected)
                }
            }
            Event::Serv(ServEvent::Authenticated) => {
                let principal = self.candidate.filter(|p| authority.still_authorized(*p)).ok_or(Failure::Revoked)?;
                self.principal = Some(principal);
                self.phase = Phase::Authenticated;
                self.deadline = now.saturating_add(AUTHENTICATED_MS);
                Ok(Diagnostic::Authenticated)
            }
            Event::Serv(ServEvent::OpenSession(event)) => {
                if self.console_authorized && self.principal.is_some() && !self.channel_used {
                    self.channel = Some(event.accept().map_err(protocol)?); self.channel_used = true;
                    Ok(Diagnostic::ChannelOpened)
                } else {
                    event.reject(ChanFail::SSH_OPEN_ADMINISTRATIVELY_PROHIBITED).map_err(protocol)?;
                    Ok(Diagnostic::ChannelDenied)
                }
            }
            Event::Serv(ServEvent::SessionShell(event)) => {
                if self.console_authorized && !self.console_started && self.request.is_none()
                    && self.channel.as_ref().is_some_and(|channel| channel.num() == event.channel()) {
                    self.request = Some(ConsoleRequest::Shell); self.console_started = true;
                    event.succeed().map_err(protocol)?; Ok(Diagnostic::ConsoleRequest)
                } else { event.fail().map_err(protocol)?; Ok(Diagnostic::ChannelDenied) }
            }
            Event::Serv(ServEvent::SessionExec(event)) => {
                let command = event.command().map_err(protocol)?;
                if self.console_authorized && !self.console_started && self.request.is_none()
                    && self.channel.as_ref().is_some_and(|channel| channel.num() == event.channel())
                    && !command.is_empty() && command.len() <= 512 && !command.as_bytes().contains(&0) {
                    let mut bytes = [0; 512]; bytes[..command.len()].copy_from_slice(command.as_bytes());
                    self.request = Some(ConsoleRequest::Exec { bytes, length: command.len() }); self.console_started = true;
                    event.succeed().map_err(protocol)?; Ok(Diagnostic::ConsoleRequest)
                } else { event.fail().map_err(protocol)?; Ok(Diagnostic::ChannelDenied) }
            }
            Event::Serv(ServEvent::SessionSubsystem(event)) => { event.fail().map_err(protocol)?; Ok(Diagnostic::ChannelDenied) }
            Event::Serv(ServEvent::SessionPty(event)) => {
                let size = event.terminal_size().ok().filter(|(columns, rows)| (0..=1000).contains(columns) && (0..=1000).contains(rows));
                if self.console_authorized && !self.console_started && !self.console_pty && self.request.is_none()
                    && self.channel.as_ref().is_some_and(|channel| channel.num() == event.channel()) && size.is_some() {
                    let (columns, rows) = size.unwrap(); self.console_pty = true;
                    self.request = Some(ConsoleRequest::Pty { columns, rows });
                    event.succeed().map_err(protocol)?; Ok(Diagnostic::ConsoleRequest)
                } else { event.fail().map_err(protocol)?; Ok(Diagnostic::ChannelDenied) }
            }
            Event::Serv(ServEvent::SessionControl(event)) => {
                let control = event.terminal_control().ok();
                let request = match control {
                    Some(sunset::event::TerminalControl::Resize { columns, rows })
                        if self.console_pty && (0..=1000).contains(&columns) && (0..=1000).contains(&rows) => Some(ConsoleRequest::Resize { columns, rows }),
                    Some(sunset::event::TerminalControl::Interrupt) if self.console_started => Some(ConsoleRequest::Interrupt),
                    _ => None,
                };
                if self.console_authorized && self.request.is_none() && request.is_some()
                    && self.channel.as_ref().is_some_and(|channel| channel.num() == event.channel()) {
                    self.request = request; event.succeed().map_err(protocol)?; Ok(Diagnostic::ConsoleRequest)
                } else { event.fail().map_err(protocol)?; Ok(Diagnostic::ChannelDenied) }
            }
            Event::Serv(ServEvent::SessionEnv(event)) => { event.fail().map_err(protocol)?; Ok(Diagnostic::ChannelDenied) }
            Event::Serv(ServEvent::Defunct) => Err(Failure::Disconnected),
            Event::Cli(_) => Err(Failure::Protocol),
        }
    }
}

pub use sunset::{ed25519_dalek, SignKey as HostKey};

// ------------------------=
// FUNC: protocol_failure
// DESC: Maps typed library failures into stable non-secret diagnostic categories without parsing message text.
// ------------------=
fn protocol_failure(error: sunset::Error) -> Failure {
    match error {
        sunset::Error::EntropyUnavailable => Failure::EntropyUnavailable,
        sunset::Error::AlgoNoMatch { .. } | sunset::Error::UnknownMethod { .. } => Failure::UnsupportedAlgorithm,
        sunset::Error::BadSig => Failure::InvalidSignature,
        sunset::Error::BadDecrypt => Failure::Integrity,
        sunset::Error::SessionEOF => Failure::Disconnected,
        _ => Failure::Protocol,
    }
}

#[cfg(test)]
mod tests;
