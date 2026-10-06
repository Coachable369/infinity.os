//! Single bounded HTTP/1.1 GET over strictly authenticated TLS 1.3.
use crate::{
    body, request, response,
    tls::{CertificateVerifier, Provider},
};
use embedded_io_async::{Read, Write};
use embedded_tls::{Aes128GcmSha256, TlsConfig, TlsConnection, TlsContext, TlsError};
use rustls_pki_types::TrustAnchor;

pub const MAX_RESPONSE_BODY: usize = 4 * 1024 * 1024;

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
impl Error {
    // ------------------------=
    // FUNC: certificate_rejected
    // DESC: Distinguishes authenticated certificate validation failure from generic transport failures.
    // ------------------=
    pub fn certificate_rejected(&self) -> bool {
        matches!(self, Self::Tls(TlsError::InvalidCertificate))
    }
}

#[cfg(test)]
mod tests {
    // ------------------------=
    // FUNC: certificate_failure_is_not_a_transport_failure
    // DESC: Verifies the typed classification used by native browser negative-certificate acceptance.
    // ------------------=
    #[test]
    fn certificate_failure_is_not_a_transport_failure() {
        assert!(super::Error::Tls(super::TlsError::InvalidCertificate).certificate_rejected());
        assert!(!super::Error::Protocol.certificate_rejected());
        assert!(!super::Error::Truncated.certificate_rejected());
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body_bytes: usize,
    /// Bytes copied into the optional caller-owned header buffer.
    pub header_bytes: usize,
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
    get_with_headers(stream, rng, roots, unix_seconds, host, path, buffers, None).await
}

// ------------------------=
// FUNC: get_with_headers
// DESC: Preserves the final authenticated response head in bounded caller storage before decoding its body.
// ------------------=
pub async fn get_with_headers<S: Read + Write, R: rand_core::CryptoRngCore>(
    stream: S,
    rng: R,
    roots: &[TrustAnchor<'_>],
    unix_seconds: u64,
    host: &str,
    path: &str,
    buffers: Buffers<'_>,
    headers: Option<&mut [u8]>,
) -> Result<Response, Error> {
    get_with_request_headers(stream, rng, roots, unix_seconds, host, path, &[], buffers, headers).await
}

// ------------------------=
// FUNC: get_with_request_headers
// DESC: Sends validated browser fields over the authenticated native TLS connection.
// ------------------=
pub async fn get_with_request_headers<S: Read + Write, R: rand_core::CryptoRngCore>(
    stream: S, rng: R, roots: &[TrustAnchor<'_>], unix_seconds: u64,
    host: &str, path: &str, request_headers: &[u8], buffers: Buffers<'_>,
    headers: Option<&mut [u8]>,
) -> Result<Response, Error> {
    get_streaming(stream, rng, roots, unix_seconds, host, path, request_headers, buffers, headers, |_, _| true).await
}

// ------------------------=
// FUNC: get_streaming
// DESC: Publishes authenticated headers and validated payload incrementally, waiting cooperatively on consumer backpressure.
// ------------------=
pub async fn get_streaming<S: Read + Write, R: rand_core::CryptoRngCore>(
    stream: S, rng: R, roots: &[TrustAnchor<'_>], unix_seconds: u64,
    host: &str, path: &str, request_headers: &[u8], buffers: Buffers<'_>,
    headers: Option<&mut [u8]>, progress: impl FnMut(Option<&[u8]>, &[u8]) -> bool,
) -> Result<Response, Error> {
    request_streaming(stream,rng,roots,unix_seconds,host,path,"GET",request_headers,&[],buffers,headers,progress).await
}

// ------------------------=
// FUNC: request_streaming
// DESC: Sends bounded binary request bodies with native framing and streams only authenticated response payload.
// ------------------=
pub async fn request_streaming<S: Read + Write, R: rand_core::CryptoRngCore>(
    stream:S,rng:R,roots:&[TrustAnchor<'_>],unix_seconds:u64,
    host:&str,path:&str,method:&str,request_headers:&[u8],request_body:&[u8],buffers:Buffers<'_>,
    mut headers:Option<&mut[u8]>,mut progress:impl FnMut(Option<&[u8]>,&[u8])->bool,
) -> Result<Response, Error> {
    if buffers.read_record.len() < 16640
        || buffers.write_record.len() < 2048
        || buffers.response.len() > MAX_RESPONSE_BODY
    {
        return Err(Error::Capacity);
    }
    let length = request::head(host,path,method,request_headers,request_body.len(),buffers.request).map_err(Error::Request)?;
    let verifier = CertificateVerifier::new(roots, host, unix_seconds).map_err(Error::Tls)?;
    let config = TlsConfig::new()
        .enable_rsa_signatures()
        .with_server_name(host)
        .with_alpn(&[b"http/1.1"]);
    let mut connection =
        TlsConnection::<_, Aes128GcmSha256>::new(stream, buffers.read_record, buffers.write_record);
    connection
        .open(TlsContext::new(&config, Provider { rng, verifier }))
        .await
        .map_err(Error::Tls)?;
    for part in [&buffers.request[..length],request_body] {
      let mut sent = 0;
      while sent < part.len() {
        let count = connection
            .write(&part[sent..])
            .await
            .map_err(Error::Tls)?;
        if count == 0 {
            return Err(Error::Truncated);
        }
        sent += count;
      }
    }
    connection.flush().await.map_err(Error::Tls)?;
    let mut used = 0;
    let mut head = None;
    let mut interim = 0;
    let mut header_bytes = 0;
    let mut cursor = body::Cursor::default();
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
                    response::parse(&buffers.response[..used], method=="HEAD").map_err(Error::Header)?
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
                    if let Some(target) = headers.as_deref_mut() {
                        if target.len() < parsed.bytes {
                            return Err(Error::Capacity);
                        }
                        target[..parsed.bytes].copy_from_slice(&buffers.response[..parsed.bytes]);
                        header_bytes = parsed.bytes;
                    }
                    head = Some(parsed);
                    core::future::poll_fn(|_| if progress(Some(&buffers.response[..parsed.bytes]), &[]) {
                        core::task::Poll::Ready(())
                    } else { core::task::Poll::Pending }).await;
                    break;
                }
            }
        }
        if let Some(head) = head {
            while let Some(chunk) = cursor.next(&buffers.response[head.bytes..used], head.body).map_err(Error::Body)? {
                core::future::poll_fn(|_| if progress(None, chunk) {
                    core::task::Poll::Ready(())
                } else { core::task::Poll::Pending }).await;
            }
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
                    header_bytes,
                });
            }
        }
        if count == 0 {
            return Err(Error::Truncated);
        }
    }
}
