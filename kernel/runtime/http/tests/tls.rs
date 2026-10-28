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

struct ChannelControl {
    commands: std::collections::VecDeque<infinity_http::channel::Command>,
    results: std::rc::Rc<std::cell::RefCell<Vec<(Result<usize, infinity_http::channel::Failure>, Vec<u8>)>>>,
    connected: std::rc::Rc<std::cell::Cell<bool>>,
}
impl infinity_http::channel::Control for ChannelControl {
    // ------------------------=
    // FUNC: connected
    // DESC: Records actual entry into the connected native conversation.
    // ------------------=
    fn connected(&mut self) { self.connected.set(true); }
    // ------------------------=
    // FUNC: command
    // DESC: Supplies typed socket operations and exact outgoing application bytes without parsing protocol prose.
    // ------------------=
    fn command(&mut self, _: &mut Context<'_>, out: &mut [u8]) -> Poll<infinity_http::channel::Command> {
        let command = self.commands.pop_front().expect("unexpected socket operation");
        if let infinity_http::channel::Command::Write(4) = command { out[..4].copy_from_slice(&[1, 0, 255, 2]); }
        Poll::Ready(command)
    }
    // ------------------------=
    // FUNC: complete
    // DESC: Retains structured transport outcomes and decrypted byte values for assertions.
    // ------------------=
    fn complete(&mut self, result: Result<usize, infinity_http::channel::Failure>, data: &[u8]) {
        self.results.borrow_mut().push((result, data.to_vec()));
    }
}

// ------------------------=
// FUNC: channel_exchange
// DESC: Exercises the real persistent TLS socket conversation against an independent TLS server and verifies invalid peers never receive application bytes.
// ------------------=
fn channel_exchange(wrong_host: bool, wrong_root: bool) {
    use infinity_http::{channel::{Channel, Command, Failure}, client::Conversation};
    use std::future::Future;
    let (root, leaf, key) = certificates();
    let (other_root, _, _) = certificates();
    let server = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13]).unwrap().with_no_client_auth()
        .with_single_cert(vec![leaf.der().clone()], rustls::pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()).into()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut stream = rustls::StreamOwned::new(rustls::ServerConnection::new(Arc::new(server)).unwrap(), socket);
        let mut bytes = [0; 4];
        let accepted = stream.read_exact(&mut bytes).is_ok();
        if accepted {
            assert_eq!(bytes, [1, 0, 255, 2]);
            stream.write_all(&[9, 8, 0, 7]).unwrap(); stream.flush().unwrap();
        }
        accepted
    });
    let roots = [webpki::anchor_from_trusted_cert(if wrong_root { other_root.der() } else { root.der() }).unwrap()];
    let results = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let connected = std::rc::Rc::new(std::cell::Cell::new(false));
    let host: &[u8] = if wrong_host { b"wrong.example" } else { b"localhost" };
    let mut hostname = [0; 253]; hostname[..host.len()].copy_from_slice(host);
    let commands = [Command::StartTls { hostname, length: host.len(), trust: infinity_http::trust::Policy::System }, Command::Write(4), Command::Read(4), Command::Close];
    let mut channel = Channel { control: ChannelControl { commands: commands.into(), results: results.clone(), connected: connected.clone() },
        rng: Some(rand_chacha::ChaCha20Rng::from_seed([7; 32])), roots: &roots, unix_seconds: 1_800_000_000 };
    let stream = TcpStream::connect(address).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    stream.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
    let outcome = {
        let mut future = std::pin::pin!(channel.exchange(TestStream(stream)));
        let waker = Waker::from(Arc::new(Noop));
        let mut context = Context::from_waker(&waker);
        match future.as_mut().poll(&mut context) { Poll::Ready(result) => result, Poll::Pending => panic!("synchronous test transport unexpectedly pending") }
    };
    assert!(connected.get());
    if wrong_host || wrong_root {
        assert!(outcome.is_err());
        assert_eq!(*results.borrow(), vec![(Err(Failure::Certificate), vec![])]);
        assert!(!worker.join().unwrap());
    } else {
        assert!(outcome.is_ok());
        assert_eq!(*results.borrow(), vec![(Ok(0), vec![]), (Ok(4), vec![]), (Ok(4), vec![9, 8, 0, 7]), (Ok(0), vec![])]);
        assert!(worker.join().unwrap());
    }
}

#[test]
// ------------------------=
// FUNC: persistent_native_channel_authenticates_and_exchanges_binary_bytes
// DESC: Verifies independent socket operations retain one cryptographically authenticated connection.
// ------------------=
fn persistent_native_channel_authenticates_and_exchanges_binary_bytes() { channel_exchange(false, false); }

#[test]
// ------------------------=
// FUNC: persistent_native_channel_rejects_hostname_and_root_mismatch
// DESC: Proves certificate failure prevents later queued application writes from executing.
// ------------------=
fn persistent_native_channel_rejects_hostname_and_root_mismatch() {
    channel_exchange(true, false); channel_exchange(false, true);
}

#[test]
// ------------------------=
// FUNC: authenticated_tls13_get_returns_complete_chunked_body
// DESC: Interoperates with an independent TLS server and verifies decrypted, decoded response bytes.
// ------------------=
fn authenticated_tls13_get_returns_complete_chunked_body() {
    authenticated_exchange(Some(8192), false,"GET",&[]);
    authenticated_exchange(Some(1), false,"GET",&[]);
    authenticated_exchange(None, false,"GET",&[]);
}

#[test]
// ------------------------=
// FUNC: authenticated_prefix_arrives_before_server_sends_final_bytes
// DESC: The independent TLS server withholds the suffix until the real client delivers its first payload under consumer backpressure.
// ------------------=
fn authenticated_prefix_arrives_before_server_sends_final_bytes() {
    authenticated_exchange(Some(8192), true,"GET",&[]);
}

#[test]
// ------------------------=
// FUNC: authenticated_post_preserves_binary_body_and_native_length
// DESC: Checks real TLS POST bytes and response streaming against an independent HTTP parser and server.
// ------------------=
fn authenticated_post_preserves_binary_body_and_native_length() {
    authenticated_exchange(Some(8192),true,"POST",&[0,13,10,255,128,65]);
}

#[test]
// ------------------------=
// FUNC: browser_headers_allow_authenticated_eof_framed_response
// DESC: Verifies a browser request receives every byte when response framing depends on TLS closure.
// ------------------=
fn browser_headers_allow_authenticated_eof_framed_response() {
    authenticated_exchange_mode(Some(8192), false, "GET", &[], true);
}

// ------------------------=
// FUNC: authenticated_exchange
// DESC: Checks header retention, insufficient capacity, and legacy body-only behavior against real TLS records.
// ------------------=
fn authenticated_exchange(header_capacity: Option<usize>, streaming: bool,method:&'static str,payload:&'static[u8]) {
    authenticated_exchange_mode(header_capacity, streaming, method, payload, false);
}

// ------------------------=
// FUNC: authenticated_exchange_mode
// DESC: Exercises chunked and close-delimited bodies against an independent authenticated TLS server.
// ------------------=
fn authenticated_exchange_mode(header_capacity: Option<usize>, streaming: bool,method:&'static str,payload:&'static[u8],eof:bool) {
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
    let (prefix_tx,prefix_rx)=std::sync::mpsc::channel();
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
        let mut count=0;
        loop {
            let received=stream.read(&mut request[count..]).unwrap();
            assert!(received>0);count+=received;
            let mut fields=[httparse::EMPTY_HEADER;16];
            let mut parsed=httparse::Request::new(&mut fields);
            if let httparse::Status::Complete(head)=parsed.parse(&request[..count]).unwrap() {
                assert_eq!(parsed.method,Some(method));
                assert_eq!(parsed.headers.iter().find(|h|h.name.eq_ignore_ascii_case("connection")).unwrap().value,b"close");
                assert_eq!(parsed.headers.iter().find(|h|h.name.eq_ignore_ascii_case("cookie")).unwrap().value,b"sid=123");
                if !payload.is_empty() {
                    let length=parsed.headers.iter().find(|h|h.name.eq_ignore_ascii_case("content-length")).unwrap().value;
                    assert_eq!(std::str::from_utf8(length).unwrap().parse::<usize>().unwrap(),payload.len());
                }
                if count<head+payload.len() {continue;}
                assert_eq!(&request[head..count],payload);
                break;
            }
        }
        if eof {
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nConnection: close\r\n\r\nhello world").unwrap();
            stream.flush().unwrap();
            stream.conn.send_close_notify();
        } else if streaming {
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n").unwrap();
            stream.flush().unwrap();
            assert_eq!(prefix_rx.recv_timeout(Duration::from_secs(5)).unwrap(), b"hello");
            stream.write_all(b"6\r\n world\r\n0\r\n\r\n").unwrap();
        } else {
            stream.write_all(b"HTTP/1.1 103 Early Hints\r\nLink: </ignored>\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n").unwrap();
        }
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
    let mut chunks=Vec::new();
    let mut statuses=Vec::new();
    let mut blocked=false;
    let mut waits=0;
    let result = {
        let mut future = std::pin::pin!(https::request_streaming(
            TestStream(stream),
            rand_chacha::ChaCha20Rng::from_seed([7; 32]),
            &roots,
            1_800_000_000,
            "localhost",
            "/",
            method,
            b"Cookie: sid=123\r\n",
            payload,
            Buffers {
                read_record: &mut read,
                write_record: &mut write,
                request: &mut request,
                response: &mut response
            },
            header_capacity.map(|capacity| &mut headers[..capacity]),
            |head, bytes| {
                if !streaming {return true;}
                if !blocked {blocked=true;return false;}
                blocked=false;
                if let Some(head)=head {
                    statuses.push(infinity_http::response::parse(head,false).unwrap().unwrap().status);
                } else {
                    if chunks.is_empty() {prefix_tx.send(bytes.to_vec()).unwrap();}
                    chunks.extend_from_slice(bytes);
                }
                true
            }
        ));
        let waker = Waker::from(Arc::new(Noop));
        let mut context = Context::from_waker(&waker);
        loop {
            match std::future::Future::poll(future.as_mut(), &mut context) {
                Poll::Ready(result)=>break result,
                Poll::Pending=>{waits+=1;assert!(streaming && waits<=8);},
            }
        }
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
    if streaming {assert_eq!(statuses,vec![200]);assert_eq!(chunks,b"hello world");assert_eq!(waits,3);}
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
