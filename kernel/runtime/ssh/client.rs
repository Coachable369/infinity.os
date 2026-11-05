//! Native bounded SSH client. Human decisions suspend progress rather than blocking the NIC.
use sunset::{Runner, Client, Event, CliEvent, PubKey, ChanHandle, ChanData};
use crate::{Failure, PACKET_BUFFER, HANDSHAKE_MS, protocol_failure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State { Connecting, HostKey([u8; 32]), Password, Opening, Connected, Exited(u32), Failed(Failure) }

pub struct Connection<'a> {
    runner: Runner<'a, Client>,
    state: State,
    username: [u8; 128],
    username_len: usize,
    channel: Option<ChanHandle>,
    deadline: u64,
    password_attempts: u8,
    columns:u32,rows:u32,
}

impl<'a> Connection<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Starts a client with fixed buffers and no stored password or automatic host trust.
    // ------------------=
    pub fn new(input: &'a mut [u8], output: &'a mut [u8], username: &str, now: u64) -> Result<Self, Failure> {
        if input.len() != PACKET_BUFFER || output.len() != PACKET_BUFFER || username.is_empty()
            || username.len() > 128 || username.bytes().any(|b| b < 32 || b == 127) { return Err(Failure::Configuration); }
        input.fill(0); output.fill(0);
        let mut name = [0; 128]; name[..username.len()].copy_from_slice(username.as_bytes());
        Ok(Self { runner: Runner::new_client(input, output), state: State::Connecting,
            username: name, username_len: username.len(), channel: None,
            deadline: now.saturating_add(HANDSHAKE_MS), password_attempts: 0,columns:80,rows:24 })
    }
    // ------------------------=
    // FUNC: state
    // DESC: Returns structured progress without exposing authentication secrets.
    // ------------------=
    pub fn state(&self) -> State { self.state }
    // ------------------------=
    // FUNC: input
    // DESC: Consumes only the encrypted TCP prefix accepted by the protocol engine.
    // ------------------=
    pub fn input(&mut self, bytes: &[u8]) -> Result<usize, Failure> { self.runner.input(bytes).map_err(protocol_failure) }
    // ------------------------=
    // FUNC: output
    // DESC: Exposes pending encrypted bytes until the socket confirms consumption.
    // ------------------=
    pub fn output(&mut self) -> &[u8] { self.runner.output_buf() }
    // ------------------------=
    // FUNC: consume_output
    // DESC: Preserves partial-write backpressure and rejects invalid acknowledgments.
    // ------------------=
    pub fn consume_output(&mut self, count: usize) -> Result<(), Failure> {
        if count > self.runner.output_buf().len() { return Err(Failure::Protocol); }
        self.runner.consume_output(count); Ok(())
    }
    // ------------------------=
    // FUNC: confirm_host
    // DESC: Requires the exact displayed host key before resuming authentication.
    // ------------------=
    pub fn confirm_host(&mut self, key: [u8; 32], now: u64) -> Result<(), Failure> {
        if self.state != State::HostKey(key) { return Err(Failure::PermissionDenied); }
        self.runner.answer_hostkey(true).map_err(protocol_failure)?;
        self.state = State::Connecting; self.deadline = now.saturating_add(HANDSHAKE_MS); Ok(())
    }
    // ------------------------=
    // FUNC: password
    // DESC: Encrypts a supplied secret only after host verification and never retains a plaintext copy.
    // ------------------=
    pub fn password(&mut self, password: &str, now: u64) -> Result<(), Failure> {
        if self.state != State::Password || password.is_empty() || password.len() > 1024 { return Err(Failure::PermissionDenied); }
        self.runner.answer_password(password).map_err(protocol_failure)?;
        self.state = State::Connecting; self.deadline = now.saturating_add(HANDSHAKE_MS); Ok(())
    }
    // ------------------------=
    // FUNC: progress
    // DESC: Advances one bounded protocol event while leaving host and password prompts to the local UI.
    // ------------------=
    pub fn progress(&mut self, now: u64) -> Result<State, Failure> {
        if matches!(self.state, State::Failed(_) | State::Exited(_)) { return Ok(self.state); }
        let result = self.step(now);
        if let Err(error) = result { self.state = State::Failed(error); }
        result.map(|_| self.state)
    }
    // ------------------------=
    // FUNC: step
    // DESC: Processes encrypted authentication and one shell channel with bounded retry and prompt deadlines.
    // ------------------=
    fn step(&mut self, now: u64) -> Result<(), Failure> {
        if self.state != State::Connected && now >= self.deadline { return Err(Failure::Deadline); }
        if matches!(self.state, State::HostKey(_) | State::Password) || !self.runner.output_buf().is_empty() { return Ok(()); }
        let mut open = false;
        match self.runner.progress().map_err(protocol_failure)? {
            Event::Cli(CliEvent::Hostkey(event)) => {
                let PubKey::Ed25519(key) = event.hostkey().map_err(protocol_failure)? else { return Err(Failure::UnsupportedAlgorithm); };
                self.state = State::HostKey(key.key.0); self.deadline = now.saturating_add(120_000);
            }
            Event::Cli(CliEvent::Username(event)) => event.username(core::str::from_utf8(&self.username[..self.username_len]).map_err(|_| Failure::Configuration)?).map_err(protocol_failure)?,
            Event::Cli(CliEvent::Pubkey(event)) => event.skip().map_err(protocol_failure)?,
            Event::Cli(CliEvent::Password(_)) => {
                self.password_attempts += 1;
                if self.password_attempts > 3 { return Err(Failure::AuthenticationLimit); }
                self.state = State::Password; self.deadline = now.saturating_add(120_000);
            }
            Event::Cli(CliEvent::Authenticated) => { open = true; self.state = State::Opening; }
            Event::Cli(CliEvent::SessionOpened(mut event)) => {
                event.pty(sunset::Pty{term:"vt100".try_into().map_err(|_|Failure::Configuration)?,cols:self.columns,rows:self.rows,width:0,height:0,modes:Default::default()}).map_err(protocol_failure)?;
                event.shell().map_err(protocol_failure)?; self.state = State::Connected;
            }
            Event::Cli(CliEvent::SessionExit(sunset::CliSessionExit::Status(code))) => self.state = State::Exited(code),
            Event::Cli(CliEvent::SessionExit(_)) => self.state = State::Exited(255),
            Event::Cli(CliEvent::Defunct) => return Err(Failure::Disconnected),
            Event::None | Event::Progressed | Event::Cli(CliEvent::PollAgain | CliEvent::Banner(_)) => (),
            _ => return Err(Failure::Protocol),
        }
        if open { self.channel = Some(self.runner.open_client_session().map_err(protocol_failure)?); }
        Ok(())
    }
    // ------------------------=
    // FUNC: read
    // DESC: Drains remote output while keeping stderr distinguishable from stdout.
    // ------------------=
    pub fn read(&mut self, bytes: &mut [u8], stderr: bool) -> Result<usize, Failure> {
        let Some(channel) = self.channel.as_ref() else { return Ok(0); };
        match self.runner.read_channel(channel, if stderr { ChanData::Stderr } else { ChanData::Normal }, bytes) {
            Ok(n) => Ok(n), Err(sunset::Error::ChannelEOF) => Ok(0), Err(e) => Err(protocol_failure(e)),
        }
    }
    // ------------------------=
    // FUNC: write
    // DESC: Forwards bounded terminal input only after successful authentication.
    // ------------------=
    pub fn write(&mut self, bytes: &[u8]) -> Result<usize, Failure> {
        if self.state != State::Connected { return Err(Failure::PermissionDenied); }
        self.runner.write_channel(self.channel.as_ref().ok_or(Failure::Disconnected)?, ChanData::Normal, bytes).map_err(protocol_failure)
    }
}

impl Connection<'_> {
    // ------------------------=
    // FUNC: resize
    // DESC: Requests the measured PTY size initially and sends window-change notifications after login.
    // ------------------=
    pub fn resize(&mut self,columns:u32,rows:u32)->Result<(),Failure>{
        let columns=columns.clamp(1,320);let rows=rows.clamp(1,96);
        if self.columns==columns&&self.rows==rows{return Ok(());}
        if self.state==State::Connected{self.runner.term_window_change(self.channel.as_ref().ok_or(Failure::Disconnected)?,&sunset::packets::WinChange{cols:columns,rows,width:0,height:0}).map_err(protocol_failure)?;}
        self.columns=columns;self.rows=rows;Ok(())
    }
}
