use infinity_http::{
    body,
    https::{self, Buffers},
    response::Body,
    tls::CertificateVerifier,
};
use rand_core::SeedableRng;
use rcgen::{
    BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};

// ------------------------=
// FUNC: certificates
// DESC: Creates test-only roots and leaf certificates without shipping private keys or test trust into production.
// ------------------=
fn certificates() -> (rcgen::Certificate, rcgen::Certificate, KeyPair) {
    let mut ca = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca.key_usages = vec![KeyUsagePurpose::KeyCertSign];
    let key = KeyPair::generate().unwrap();
    let root = ca.self_signed(&key).unwrap();
    let issuer = Issuer::new(ca, key);
    let mut leaf = CertificateParams::new(vec!["localhost".into()]).unwrap();
    leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let key = KeyPair::generate().unwrap();
    (root, leaf.signed_by(&key, &issuer).unwrap(), key)
}

#[test]
// ------------------------=
// FUNC: certificate_validation_rejects_wrong_name_time_root_and_signature
// DESC: Exercises actual X509 signatures and rejects invalid peer authentication rather than matching diagnostics.
// ------------------=
fn certificate_validation_rejects_wrong_name_time_root_and_signature() {
    let (root, leaf, _) = certificates();
    let roots = [webpki::anchor_from_trusted_cert(root.der()).unwrap()];
    assert!(CertificateVerifier::new(&roots, "localhost", 1_800_000_000)
        .unwrap()
        .verify_chain(leaf.der(), &[])
        .is_ok());
    assert!(
        CertificateVerifier::new(&roots, "wrong.example", 1_800_000_000)
            .unwrap()
            .verify_chain(leaf.der(), &[])
            .is_err()
    );
    assert!(CertificateVerifier::new(&roots, "localhost", 1)
        .unwrap()
        .verify_chain(leaf.der(), &[])
        .is_err());
    assert!(
        CertificateVerifier::new(&roots, "localhost", 100_000_000_000)
            .unwrap()
            .verify_chain(leaf.der(), &[])
            .is_err()
    );
    assert!(CertificateVerifier::new(&roots, "localhost", 0).is_err());
    let (other, _, _) = certificates();
    let other_roots = [webpki::anchor_from_trusted_cert(other.der()).unwrap()];
    assert!(
        CertificateVerifier::new(&other_roots, "localhost", 1_800_000_000)
            .unwrap()
            .verify_chain(leaf.der(), &[])
            .is_err()
    );
    let mut damaged = leaf.der().to_vec();
    let end = damaged.len() - 1;
    damaged[end] ^= 1;
    assert!(CertificateVerifier::new(&roots, "localhost", 1_800_000_000)
        .unwrap()
        .verify_chain(&damaged, &[])
        .is_err());
}

struct TestStream(TcpStream);
impl embedded_io_async::ErrorType for TestStream {
    type Error = embedded_io_async::ErrorKind;
}
impl embedded_io_async::Read for TestStream {
    // ------------------------=
    // FUNC: read
    // DESC: Adapts a timeout-bounded host test socket; production uses the native transport instead.
    // ------------------=
    async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Self::Error> {
        self.0
            .read(bytes)
            .map_err(|_| embedded_io_async::ErrorKind::Other)
    }
}
impl embedded_io_async::Write for TestStream {
    // ------------------------=
    // FUNC: write
    // DESC: Sends test ciphertext through the host fixture only.
    // ------------------=
    async fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        self.0
            .write(bytes)
            .map_err(|_| embedded_io_async::ErrorKind::Other)
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Flushes the test socket without changing production transport behavior.
    // ------------------=
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
struct Noop;
impl Wake for Noop {
    // ------------------------=
    // FUNC: wake
    // DESC: Supplies a test waker for the synchronously completing socket fixture.
    // ------------------=
    fn wake(self: Arc<Self>) {}
}

#[test]
// ------------------------=
// FUNC: authenticated_tls13_get_returns_complete_chunked_body
// DESC: Interoperates with an independent TLS server and verifies decrypted, decoded response bytes.
// ------------------=
fn authenticated_tls13_get_returns_complete_chunked_body() {
    authenticated_exchange(Some(8192));
    authenticated_exchange(Some(1));
    authenticated_exchange(None);
}

// ------------------------=
// FUNC: authenticated_exchange
// DESC: Checks header retention, insufficient capacity, and legacy body-only behavior against real TLS records.
// ------------------=
fn authenticated_exchange(header_capacity: Option<usize>) {
    let (root, leaf, key) = certificates();
    let provider = rustls::crypto::ring::default_provider();
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone()],
            rustls::pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        )
        .unwrap();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut stream = rustls::StreamOwned::new(
            rustls::ServerConnection::new(Arc::new(config)).unwrap(),
            socket,
        );
        let mut request = [0; 512];
        let count = stream.read(&mut request).unwrap();
        assert!(count > 0);
        stream.write_all(b"HTTP/1.1 103 Early Hints\r\nLink: </ignored>\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n").unwrap();
        stream.flush().unwrap();
    });
    let stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let roots = [webpki::anchor_from_trusted_cert(root.der()).unwrap()];
    let (mut read, mut write, mut request) =
        ([0; 16640], [0; 4096], [0; 512]);
    // Exercise the actual browser buffer contract, including the HTTPS layer's
    // capacity guard, against independent authenticated TLS records.
    let mut response = vec![0; https::MAX_RESPONSE_BODY];
    let mut headers = [0; 8192];
    let result = {
        let mut future = std::pin::pin!(https::get_with_headers(
            TestStream(stream),
            rand_chacha::ChaCha20Rng::from_seed([7; 32]),
            &roots,
            1_800_000_000,
            "localhost",
            "/",
            Buffers {
                read_record: &mut read,
                write_record: &mut write,
                request: &mut request,
                response: &mut response
            },
            header_capacity.map(|capacity| &mut headers[..capacity])
        ));
        let waker = Waker::from(Arc::new(Noop));
        let mut context = Context::from_waker(&waker);
        let Poll::Ready(result) = std::future::Future::poll(future.as_mut(), &mut context) else {
            panic!("fixture unexpectedly pending");
        };
        result
    };
    worker.join().unwrap();
    if header_capacity == Some(1) {
        assert!(matches!(result, Err(https::Error::Capacity)));
        assert!(headers.iter().all(|byte| *byte == 0));
        return;
    }
    let result = result.unwrap();
    assert_eq!(result.status, 200);
    assert_eq!(result.body_bytes, 11);
    assert_eq!(&response[..result.body_bytes], b"hello world");
    if header_capacity.is_some() {
        let head = infinity_http::response::Headers::parse(&headers[..result.header_bytes]).unwrap();
        assert_eq!(head.values("content-type").collect::<Vec<_>>(), vec![b"text/plain".as_slice()]);
        assert_eq!(head.values("SET-cookie").collect::<Vec<_>>(), vec![b"a=1".as_slice(), b"b=2".as_slice()]);
        assert_eq!(head.values("link").count(), 0);
    } else {
        assert_eq!(result.header_bytes, 0);
    }
}

#[test]
// ------------------------=
// FUNC: chunked_fragmentation_preserves_incomplete_input
// DESC: Verifies every partial chunk boundary, malformed terminators and premature EOF.
// ------------------=
fn chunked_fragmentation_preserves_incomplete_input() {
    let data = b"3\r\nabc\r\n0\r\n\r\n";
    for split in 0..data.len() {
        let mut bytes = data[..split].to_vec();
        assert_eq!(body::complete(&mut bytes, Body::Chunked, false), Ok(None));
        assert_eq!(bytes, data[..split]);
        assert_eq!(
            body::complete(&mut bytes, Body::Chunked, true),
            Err(body::Error::Truncated)
        );
    }
    let mut bytes = *data;
    assert_eq!(
        body::complete(&mut bytes, Body::Chunked, false),
        Ok(Some(3))
    );
    assert_eq!(&bytes[..3], b"abc");
    let mut malformed = *b"1\r\naXX";
    assert_eq!(
        body::complete(&mut malformed, Body::Chunked, false),
        Err(body::Error::Invalid)
    );
}
