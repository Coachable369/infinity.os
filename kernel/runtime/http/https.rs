//! Single bounded HTTP/1.1 GET over strictly authenticated TLS 1.3.
use crate::{
    body, request, response,
    tls::{CertificateVerifier, Provider},
};
use embedded_io_async::{Read, Write};
use embedded_tls::{Aes128GcmSha256, TlsConfig, TlsConnection, TlsContext, TlsError};
use rustls_pki_types::TrustAnchor;

#[derive(Debug)]
pub enum Error {
    Tls(TlsError),
    Request(request::Error),
    Header(response::Error),
    Body(body::Error),
    Capacity,
    Truncated,
    Protocol,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body_bytes: usize,
}
pub struct Buffers<'a> {
    pub read_record: &'a mut [u8],
    pub write_record: &'a mut [u8],
    pub request: &'a mut [u8],
    pub response: &'a mut [u8],
}

// ------------------------=
// FUNC: get
// DESC: Sends a bounded GET only after chain, hostname and handshake authentication and returns one complete decoded body.
// ------------------=
/// The stream must enforce cancellation, capability revocation and a deadline.
/// Caller must provide fresh cryptographic entropy, trusted time and installed roots.
/// Dropping this future requires cancelling/discarding the underlying connection.
pub async fn get<S: Read + Write, R: rand_core::CryptoRngCore>(
    stream: S,
    rng: R,
    roots: &[TrustAnchor<'_>],
    unix_seconds: u64,
    host: &str,
    path: &str,
    buffers: Buffers<'_>,
) -> Result<Response, Error> {
    if buffers.read_record.len() < 16640
        || buffers.write_record.len() < 2048
        || buffers.response.len() > 1024 * 1024
    {
        return Err(Error::Capacity);
    }
    let length = request::get(host, path, buffers.request).map_err(Error::Request)?;
    let verifier = CertificateVerifier::new(roots, host, unix_seconds).map_err(Error::Tls)?;
    let config = TlsConfig::new()
        .with_server_name(host)
        .with_alpn(&[b"http/1.1"]);
    let mut connection =
        TlsConnection::<_, Aes128GcmSha256>::new(stream, buffers.read_record, buffers.write_record);
    connection
        .open(TlsContext::new(&config, Provider { rng, verifier }))
        .await
        .map_err(Error::Tls)?;
    let mut sent = 0;
    while sent < length {
        let count = connection
            .write(&buffers.request[sent..length])
            .await
            .map_err(Error::Tls)?;
        if count == 0 {
            return Err(Error::Truncated);
        }
        sent += count;
    }
    connection.flush().await.map_err(Error::Tls)?;
    let mut used = 0;
    let mut head = None;
    let mut interim = 0;
    loop {
        if used == buffers.response.len() {
            return Err(Error::Capacity);
        }
        let count = match connection.read(&mut buffers.response[used..]).await {
            Ok(count) => count,
            Err(TlsError::ConnectionClosed) => 0,
            Err(error) => return Err(Error::Tls(error)),
        };
        used += count;
        if head.is_none() {
            loop {
                let Some(parsed) =
                    response::parse(&buffers.response[..used], false).map_err(Error::Header)?
                else {
                    break;
                };
                if parsed.status < 200 {
                    interim += 1;
                    if interim > 8 || parsed.status == 101 {
                        return Err(Error::Protocol);
                    }
                    buffers.response.copy_within(parsed.bytes..used, 0);
                    used -= parsed.bytes;
                } else {
                    head = Some(parsed);
                    break;
                }
            }
        }
        if let Some(head) = head {
            if let Some(length) = body::complete(
                &mut buffers.response[head.bytes..used],
                head.body,
                count == 0,
            )
            .map_err(Error::Body)?
            {
                buffers
                    .response
                    .copy_within(head.bytes..head.bytes + length, 0);
                return Ok(Response {
                    status: head.status,
                    body_bytes: length,
                });
            }
        }
        if count == 0 {
            return Err(Error::Truncated);
        }
    }
}
