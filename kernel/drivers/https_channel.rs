//! Stateful socket commands hosted by the existing BSP network actor.
use super::*;
use http::channel::{Command, Control, Failure as ChannelFailure, IO_BYTES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SocketResult {
    pub state: u64, pub count: usize, pub address: [u8; 4], pub error: Option<Failure>,
}
struct Mailbox {
    ticket: Option<Ticket>, command: Option<Command>, result: SocketResult,
    bytes: [u8; IO_BYTES], length: usize,
}
impl Mailbox {
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no live channel or retained private transport data.
    // ------------------=
    const fn new() -> Self {
        Self { ticket: None, command: None, result: SocketResult { state: 0, count: 0, address: [0; 4], error: None },
            bytes: [0; IO_BYTES], length: 0 }
    }
}
static mut MAILBOX: Mailbox = Mailbox::new();

// ------------------------=
// FUNC: clear
// DESC: Erases private socket command/response bytes after the actor has destroyed its suspended future.
// ------------------=
pub(super) unsafe fn clear() { MAILBOX = Mailbox::new(); }

struct SocketControl(Ticket);
impl Control for SocketControl {
    // ------------------------=
    // FUNC: resolved
    // DESC: Retains the actual peer address so getpeername never substitutes an unresolved label.
    // ------------------=
    fn resolved(&mut self, address: [u8; 4]) { unsafe {
        let mailbox = &mut *(&raw mut MAILBOX);
        if mailbox.ticket == Some(self.0) { mailbox.result.address = address; }
    }}
    // ------------------------=
    // FUNC: connected
    // DESC: Reports the actual native TCP establishment under the owning actor's lock.
    // ------------------=
    fn connected(&mut self) { unsafe {
        let mailbox = &mut *(&raw mut MAILBOX);
        if mailbox.ticket == Some(self.0) { mailbox.result.state = 2; }
    }}
    // ------------------------=
    // FUNC: command
    // DESC: Transfers one complete bounded command and its copied bytes without exposing native NIC or kernel pointers.
    // ------------------=
    fn command(&mut self, _: &mut Context<'_>, out: &mut [u8]) -> Poll<Command> { unsafe {
        let mailbox = &mut *(&raw mut MAILBOX);
        if mailbox.ticket != Some(self.0) { return Poll::Ready(Command::Close); }
        let Some(command) = mailbox.command.take() else { return Poll::Pending; };
        if let Command::Write(count) = command {
            if count > out.len() || count != mailbox.length { return Poll::Ready(Command::Close); }
            out[..count].copy_from_slice(&mailbox.bytes[..count]);
        }
        mailbox.bytes.fill(0); mailbox.length = 0;
        Poll::Ready(command)
    }}
    // ------------------------=
    // FUNC: complete
    // DESC: Publishes real socket byte counts and certificate failure classifications for the exact channel generation.
    // ------------------=
    fn complete(&mut self, result: Result<usize, ChannelFailure>, incoming: &[u8]) { unsafe {
        let mailbox = &mut *(&raw mut MAILBOX);
        if mailbox.ticket != Some(self.0) { return; }
        mailbox.bytes.fill(0);
        mailbox.length = incoming.len().min(IO_BYTES);
        mailbox.bytes[..mailbox.length].copy_from_slice(&incoming[..mailbox.length]);
        mailbox.result.state = 4;
        match result {
            Ok(count) => { mailbox.result.count = count; mailbox.result.error = None; }
            Err(error) => { mailbox.result.count = 0; mailbox.result.error = Some(match error {
                ChannelFailure::Certificate => Failure::Certificate,
                ChannelFailure::Invalid | ChannelFailure::AlreadyEncrypted => Failure::Invalid,
                _ => Failure::Transport,
            }); }
        }
    }}
}

// ------------------------=
// FUNC: failure
// DESC: Keeps certificate, permission, timeout and DNS failures distinct when a native connection future terminates.
// ------------------=
fn failure(error: http::client::Error) -> Failure {
    match error {
        http::client::Error::Denied => Failure::Denied,
        http::client::Error::Timeout => Failure::Timeout,
        http::client::Error::Transport(http::transport::Error::Timeout) => Failure::Timeout,
        http::client::Error::Transport(http::transport::Error::ResolutionFailed) => Failure::Resolution,
        http::client::Error::Https(error) if error.certificate_rejected() => Failure::Certificate,
        _ => Failure::Transport,
    }
}

// ------------------------=
// FUNC: socket_begin
// DESC: Opens a real DNS query or endpoint-scoped TCP connection on the shared actor without bypassing native leases or creating a second scheduler.
// ------------------=
pub(crate) fn socket_begin(owner: SecurityIdentity, caps: [CapabilityId; 4], host: &[u8],
    address: Option<[u8; 4]>, port: u16, resolve_only: bool) -> Result<Ticket, Failure> {
    let Some(_guard) = lock() else { return Err(Failure::Busy); };
    if host.is_empty() || host.len() > 253 || port == 0 || core::str::from_utf8(host).is_err()
        || host.iter().any(|b| !b.is_ascii_alphanumeric() && !b".-".contains(b)) { return Err(Failure::Invalid); }
    unsafe {
        if OWNER.is_some() { return Err(Failure::Busy); }
        let ticket = Ticket(NEXT_TICKET.checked_add(1).ok_or(Failure::Unavailable)?);
        let (mut config, auth, seed, time) = prepare_connection(owner, caps[0], caps[1], caps[2], caps[3], false)?;
        if !resolve_only {
            config.deadline = Instant::from_millis(CLOCK.saturating_add(3_600_000).min(i64::MAX as u64) as i64);
        }
        let mut name = [0; 253]; name[..host.len()].copy_from_slice(host); let length = host.len();
        let future = async move {
            let host = core::str::from_utf8(&name[..length]).map_err(|_| Failure::Invalid)?;
            let literal = host.parse::<core::net::Ipv4Addr>().ok().map(|ip| ip.octets());
            let destination = address.or(literal).map(Destination::Address).unwrap_or(Destination::Resolve);
            let result = if resolve_only {
                http::client::connect(ServiceLink(auth), config, destination,
                    u64::from_le_bytes(seed[..8].try_into().unwrap()), host, port, http::client::ResolveAddress).await
                    .map(|address| { let mailbox = &mut *(&raw mut MAILBOX);
                        mailbox.result.address = address; mailbox.result.state = 7; })
            } else {
                let conversation = http::channel::Channel { control: SocketControl(ticket),
                    rng: Some(http::rand_chacha::ChaCha20Rng::from_seed(seed)),
                    roots: http::tls::system_roots(), unix_seconds: time };
                http::client::connect(ServiceLink(auth), config, destination,
                    u64::from_le_bytes(seed[..8].try_into().unwrap()), host, port, conversation).await
                    .map(|_| { (&mut *(&raw mut MAILBOX)).result.state = 5; })
            };
            if let Err(error) = result {
                let error = failure(error); let mailbox = &mut *(&raw mut MAILBOX);
                mailbox.bytes.fill(0); mailbox.length = 0; mailbox.result.state = 6; mailbox.result.error = Some(error);
                return Err(error);
            }
            Err(Failure::Cancelled) // No HTTP response loan; typed channel result is retained separately.
        };
        Pin::new_unchecked(&mut *(&raw mut TASK)).start(future).map_err(|_| Failure::Capacity)?;
        clear(); MAILBOX.ticket = Some(ticket); MAILBOX.result.state = 1;
        RX = Ring::new(); TX = Ring::new(); AUTH = Some(auth); OWNER = Some(owner);
        ENDPOINT = (auth.dns, 53);
        NEXT_TICKET = ticket.0; ACTIVE_TICKET = Some(ticket); RESULT = None;
        Ok(ticket)
    }
}

// ------------------------=
// FUNC: admitted
// DESC: Rechecks actor identity, ticket and original endpoint/profile leases before every socket read, write or result copy.
// ------------------=
unsafe fn admitted(owner: SecurityIdentity, ticket: Ticket) -> Result<(), Failure> {
    if OWNER != Some(owner) || ACTIVE_TICKET != Some(ticket) || MAILBOX.ticket != Some(ticket) { return Err(Failure::Denied); }
    let auth = AUTH.ok_or(Failure::Denied)?;
    let (address, port) = if ENDPOINT.1 == 0 { (auth.dns, 53) } else { ENDPOINT };
    if !authorized(auth, address, port, CLOCK / 1000) { return Err(Failure::Denied); }
    Ok(())
}

// ------------------------=
// FUNC: socket_command
// DESC: Queues only one bounded real socket operation and copies user-independent bytes before returning to guest execution.
// ------------------=
pub(crate) fn socket_command(owner: SecurityIdentity, ticket: Ticket, command: Command, data: &[u8]) -> Result<(), Failure> {
    let Some(_guard) = lock() else { return Err(Failure::Busy); };
    unsafe {
        admitted(owner, ticket)?;
        let mailbox = &mut *(&raw mut MAILBOX);
        if !matches!(mailbox.result.state, 2 | 4) || mailbox.command.is_some() { return Err(Failure::Busy); }
        if data.len() > IO_BYTES { return Err(Failure::Capacity); }
        match command {
            Command::Read(count) if count > IO_BYTES || !data.is_empty() => return Err(Failure::Capacity),
            Command::Write(count) if count != data.len() || count > IO_BYTES => return Err(Failure::Capacity),
            Command::StartTls { length, .. } if !(1..=253).contains(&length) => return Err(Failure::Invalid),
            Command::StartTls { .. } | Command::Close if !data.is_empty() => return Err(Failure::Invalid),
            _ => {}
        }
        mailbox.bytes.fill(0); mailbox.bytes[..data.len()].copy_from_slice(data); mailbox.length = data.len();
        mailbox.command = Some(command); mailbox.result.state = 3; mailbox.result.count = 0; mailbox.result.error = None;
        Ok(())
    }
}

// ------------------------=
// FUNC: socket_result
// DESC: Returns typed progress or terminal failure without interpreting human-readable logs.
// ------------------=
pub(crate) fn socket_result(owner: SecurityIdentity, ticket: Ticket) -> Result<SocketResult, Failure> {
    let Some(_guard) = lock() else { return Err(Failure::Busy); };
    unsafe {
        admitted(owner, ticket)?;
        if let Some(Err(error)) = (&*(&raw const RESULT)).as_ref() {
            if !matches!(MAILBOX.result.state, 5 | 7) || *error != Failure::Cancelled { return Err(*error); }
        }
        Ok(MAILBOX.result)
    }
}

// ------------------------=
// FUNC: socket_read
// DESC: Copies only completed decrypted channel bytes while authority remains valid, preserving bounded short-read semantics.
// ------------------=
pub(crate) fn socket_read(owner: SecurityIdentity, ticket: Ticket, out: &mut [u8]) -> Result<usize, Failure> {
    let Some(_guard) = lock() else { return Err(Failure::Busy); };
    unsafe {
        admitted(owner, ticket)?;
        let mailbox = &mut *(&raw mut MAILBOX);
        if mailbox.result.state != 4 || mailbox.result.error.is_some() { return Err(Failure::Invalid); }
        if out.len() < mailbox.length { return Err(Failure::Capacity); }
        out[..mailbox.length].copy_from_slice(&mailbox.bytes[..mailbox.length]);
        Ok(mailbox.length)
    }
}
