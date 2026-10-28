//! Stateful native byte-stream conversations for language socket adapters.
//! No HTTP emulation, ambient authority, host descriptors or permissive TLS.
use crate::{client::{Conversation, Error}, https, tls::{CertificateVerifier, Provider}};
use core::{future::poll_fn, task::{Context, Poll}};
use embedded_io_async::{Read, Write};
use embedded_tls::{Aes128GcmSha256, TlsConfig, TlsConnection, TlsContext, TlsError};

pub const IO_BYTES: usize = 16384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Read(usize), Write(usize),
    StartTls { hostname: [u8; 253], length: usize, trust: crate::trust::Policy },
    Close,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure { Invalid, Transport, Certificate, Tls, AlreadyEncrypted }

/// The BSP-owned adapter supplies one generation-fenced command at a time.
/// It copies writes into this conversation's buffer before returning Ready.
pub trait Control {
    // ------------------------=
    // FUNC: resolved
    // DESC: Publishes the actual resolved peer address before connecting, without authorizing it implicitly.
    // ------------------=
    fn resolved(&mut self, _address: [u8; 4]) {}
    // ------------------------=
    // FUNC: connected
    // DESC: Announces actual TCP establishment, never a proposed or queued connection.
    // ------------------=
    fn connected(&mut self);
    // ------------------------=
    // FUNC: command
    // DESC: Yields only a fully copied bounded command from the authenticated channel owner.
    // ------------------=
    fn command(&mut self, context: &mut Context<'_>, outgoing: &mut [u8]) -> Poll<Command>;
    // ------------------------=
    // FUNC: complete
    // DESC: Publishes actual byte counts or typed errors only after the transport or cryptographic operation finishes.
    // ------------------=
    fn complete(&mut self, result: Result<usize, Failure>, incoming: &[u8]);
}

pub struct Channel<'a, C, R> {
    pub control: C,
    pub rng: Option<R>,
    pub roots: &'a [rustls_pki_types::TrustAnchor<'a>],
    pub unix_seconds: u64,
}

impl<C: Control, R: rand_core::CryptoRngCore> Channel<'_, C, R> {
    // ------------------------=
    // FUNC: encrypted
    // DESC: Authenticates the real TLS handshake and then services actual decrypted reads and encrypted writes on the same TCP connection.
    // ------------------=
    async fn encrypted<S: Read + Write>(&mut self, stream: S, hostname: &[u8], policy: crate::trust::Policy) -> Result<(), Error> {
        let host = core::str::from_utf8(hostname).map_err(|_| Error::Denied)?;
        let restricted;
        let roots = match policy {
            crate::trust::Policy::System => self.roots,
            crate::trust::Policy::Certifi20260617 => {
                restricted = crate::trust::certifi_roots(self.roots).map_err(|_| Error::Denied)?;
                restricted.as_slice()
            }
        };
        let verifier = CertificateVerifier::new(roots, host, self.unix_seconds)
            .map_err(|error| Error::Https(https::Error::Tls(error)))?;
        let config = TlsConfig::new().enable_rsa_signatures().with_server_name(host);
        let mut read_record = [0; 16640];
        let mut write_record = [0; 16640];
        let mut connection = TlsConnection::<_, Aes128GcmSha256>::new(stream, &mut read_record, &mut write_record);
        let result = connection.open(TlsContext::new(&config, Provider {
            rng: self.rng.take().ok_or(Error::Denied)?, verifier,
        })).await;
        if let Err(error) = result {
            self.control.complete(Err(if matches!(error, TlsError::InvalidCertificate) {
                Failure::Certificate
            } else { Failure::Tls }), &[]);
            return Err(Error::Https(https::Error::Tls(error)));
        }
        self.control.complete(Ok(0), &[]);
        let mut bytes = [0; IO_BYTES];
        loop {
            match poll_fn(|cx| self.control.command(cx, &mut bytes)).await {
                Command::Read(0) | Command::Write(0) => self.control.complete(Ok(0), &[]),
                Command::Read(count) if count <= IO_BYTES => {
                    match connection.read(&mut bytes[..count]).await {
                        Ok(count) => self.control.complete(Ok(count), &bytes[..count]),
                        Err(TlsError::ConnectionClosed) => self.control.complete(Ok(0), &[]),
                        Err(error) => { self.control.complete(Err(Failure::Tls), &[]);
                            return Err(Error::Https(https::Error::Tls(error))); }
                    }
                }
                Command::Write(count) if count <= IO_BYTES => {
                    let result = connection.write(&bytes[..count]).await;
                    match result {
                        Ok(count) => {
                            connection.flush().await.map_err(|error| Error::Https(https::Error::Tls(error)))?;
                            self.control.complete(Ok(count), &[]);
                        }
                        Err(error) => { self.control.complete(Err(Failure::Tls), &[]);
                            return Err(Error::Https(https::Error::Tls(error))); }
                    }
                }
                Command::Close => {
                    let result = connection.close().await;
                    self.control.complete(result.as_ref().map(|_| 0).map_err(|_| Failure::Tls), &[]);
                    return result.map(|_| ()).map_err(|(_, error)| Error::Https(https::Error::Tls(error)));
                }
                Command::StartTls { .. } => self.control.complete(Err(Failure::AlreadyEncrypted), &[]),
                _ => self.control.complete(Err(Failure::Invalid), &[]),
            }
        }
    }
}

impl<C: Control, R: rand_core::CryptoRngCore> Conversation for Channel<'_, C, R> {
    type Output = ();
    // ------------------------=
    // FUNC: resolved
    // DESC: Makes native endpoint metadata available to socket clients while leaving connection and policy checks in the shared transport.
    // ------------------=
    fn resolved(&mut self, address: [u8; 4]) -> Option<Self::Output> {
        self.control.resolved(address); None
    }
    // ------------------------=
    // FUNC: exchange
    // DESC: Retains an established stream across independent socket operations and upgrades it only through an explicit authenticated TLS handshake.
    // ------------------=
    async fn exchange<S: Read + Write>(&mut self, mut stream: S) -> Result<(), Error> {
        self.control.connected();
        let mut bytes = [0; IO_BYTES];
        loop {
            match poll_fn(|cx| self.control.command(cx, &mut bytes)).await {
                Command::Read(0) | Command::Write(0) => self.control.complete(Ok(0), &[]),
                Command::Read(count) if count <= IO_BYTES => {
                    match stream.read(&mut bytes[..count]).await {
                        Ok(count) => self.control.complete(Ok(count), &bytes[..count]),
                        Err(_) => { self.control.complete(Err(Failure::Transport), &[]);
                            return Err(Error::Transport(crate::transport::Error::Disconnected)); }
                    }
                }
                Command::Write(count) if count <= IO_BYTES => {
                    match stream.write(&bytes[..count]).await {
                        Ok(count) => self.control.complete(Ok(count), &[]),
                        Err(_) => { self.control.complete(Err(Failure::Transport), &[]);
                            return Err(Error::Transport(crate::transport::Error::Disconnected)); }
                    }
                }
                Command::StartTls { hostname, length, trust } if (1..=hostname.len()).contains(&length) => {
                    return self.encrypted(stream, &hostname[..length], trust).await;
                }
                Command::Close => { self.control.complete(Ok(0), &[]); return Ok(()); }
                _ => self.control.complete(Err(Failure::Invalid), &[]),
            }
        }
    }
}
