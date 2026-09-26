use super::*;
use smoltcp::phy::{DeviceCapabilities, Medium, RxToken, TxToken};
use std::{collections::VecDeque, vec, vec::Vec};

#[derive(Default)]
struct Ethernet {
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
}
struct Receive(Vec<u8>);
struct Transmit<'a>(&'a mut VecDeque<Vec<u8>>);

struct WakeCount(core::sync::atomic::AtomicUsize);
impl std::task::Wake for WakeCount {
    // ------------------------=
    // FUNC: wake
    // DESC: Counts actual executor notifications independently of network pump calls.
    // ------------------=
    fn wake(self: std::sync::Arc<Self>) {
        self.0.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    }
}

#[test]
// ------------------------=
// FUNC: async_readiness_wakes_only_for_progress_and_terminal_events
// DESC: Exercises native TCP handshake, idle polls, data, full send queues and cancellation without host sockets.
// ------------------=
fn async_readiness_wakes_only_for_progress_and_terminal_events() {
    async_readiness_case(true);
    async_readiness_case(false);
}

// ------------------------=
// FUNC: async_readiness_case
// DESC: Runs identical packet-based readiness checks with cancellation or clean FIN termination.
// ------------------=
fn async_readiness_case(cancel: bool) {
    use core::{future::Future, task::{Context, Poll}, sync::atomic::Ordering};
    use embedded_io_async::{Read, Write};
    use std::{sync::Arc, task::Waker};
    let (mut a, mut b) = (Ethernet::default(), Ethernet::default());
    let (mut ar, mut at, mut br, mut bt) = ([0; 1024], [0; 1024], [0; 1024], [0; 1024]);
    let (mut sa, mut sb) = ([SocketStorage::EMPTY], [SocketStorage::EMPTY]);
    let mut client = Transport::new(&mut a, [2,0,0,0,0,1], [10,0,0,1], 24, None,
        1, Instant::from_millis(0), &mut sa, &mut ar, &mut at).unwrap();
    let mut server = Transport::new(&mut b, [2,0,0,0,0,2], [10,0,0,2], 24, None,
        2, Instant::from_millis(0), &mut sb, &mut br, &mut bt).unwrap();
    server.sockets.get_mut::<Socket>(server.handle).listen(80).unwrap();
    client.connect([10,0,0,2], 80, 49152, Instant::from_millis(0), Instant::from_millis(10000)).unwrap();
    assert_eq!(client.readiness(), Readiness { readable: false, writable: false });
    for tick in 0..100 {
        client.poll(&mut a, Instant::from_millis(tick));
        b.rx.extend(a.tx.drain(..));
        server.poll(&mut b, Instant::from_millis(tick));
        a.rx.extend(b.tx.drain(..));
    }
    assert_eq!(client.state(), State::Established);
    assert_eq!(client.readiness(), Readiness { readable: false, writable: true });
    let mut session = crate::async_stream::Session::new(client);
    let mut stream = session.stream();
    let pump = stream.session();
    let wakes = Arc::new(WakeCount(core::sync::atomic::AtomicUsize::new(0)));
    let waker = Waker::from(wakes.clone());
    let mut cx = Context::from_waker(&waker);
    let mut output = [0; 4];
    {
        let mut read = core::pin::pin!(stream.read(&mut output));
        assert!(read.as_mut().poll(&mut cx).is_pending());
        for tick in 100..200 { pump.poll(&mut a, Instant::from_millis(tick)); }
        assert_eq!(wakes.0.load(Ordering::SeqCst), 0);
        assert_eq!(server.send(&[9, 8, 7, 6]), Ok(4));
        server.poll(&mut b, Instant::from_millis(201));
        a.rx.extend(b.tx.drain(..));
        pump.poll(&mut a, Instant::from_millis(202));
        assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
        pump.poll(&mut a, Instant::from_millis(203));
        assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
        assert!(matches!(read.as_mut().poll(&mut cx), Poll::Ready(Ok(4))));
    }
    assert_eq!(output, [9,8,7,6]);
    {
        let bytes = [42; 1024];
        let mut write = core::pin::pin!(stream.write(&bytes));
        assert!(matches!(write.as_mut().poll(&mut cx), Poll::Ready(Ok(1024))));
    }
    {
        let mut write = core::pin::pin!(stream.write(&[5]));
        assert!(write.as_mut().poll(&mut cx).is_pending());
        for tick in 204..210 { pump.poll(&mut a, Instant::from_millis(tick)); }
        assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
        for tick in 210..300 {
            b.rx.extend(a.tx.drain(..));
            server.poll(&mut b, Instant::from_millis(tick));
            a.rx.extend(b.tx.drain(..));
            pump.poll(&mut a, Instant::from_millis(tick));
        }
        assert_eq!(wakes.0.load(Ordering::SeqCst), 2);
        assert!(matches!(write.as_mut().poll(&mut cx), Poll::Ready(Ok(1))));
    }
    {
        let mut read = core::pin::pin!(stream.read(&mut output));
        assert!(read.as_mut().poll(&mut cx).is_pending());
        if cancel {
            pump.cancel();
        } else {
            server.close();
            for tick in 300..400 {
                b.rx.extend(a.tx.drain(..));
                server.poll(&mut b, Instant::from_millis(tick));
                a.rx.extend(b.tx.drain(..));
                pump.poll(&mut a, Instant::from_millis(tick));
            }
        }
        assert_eq!(wakes.0.load(Ordering::SeqCst), 3);
        if cancel {
            assert!(matches!(read.as_mut().poll(&mut cx), Poll::Ready(Err(_))));
        } else {
            assert!(matches!(read.as_mut().poll(&mut cx), Poll::Ready(Ok(0))));
        }
        pump.cancel();
        assert_eq!(wakes.0.load(Ordering::SeqCst), 3);
    }
}

// ------------------------=
// FUNC: dns_answer
// DESC: Builds a checksummed DNS A response from the actual emitted query, optionally with a mismatched transaction ID.
// ------------------=
fn dns_answer(query: &[u8], wrong_id: bool, wrong_server: bool) -> Vec<u8> {
    use smoltcp::wire::{
        EthernetFrame, EthernetProtocol, IpProtocol, Ipv4Packet, Ipv4Repr, UdpPacket, UdpRepr,
    };
    let ethernet = EthernetFrame::new_checked(query).unwrap();
    let ip = Ipv4Packet::new_checked(ethernet.payload()).unwrap();
    let udp = UdpPacket::new_checked(ip.payload()).unwrap();
    let mut answer = udp.payload().to_vec();
    if wrong_id {
        answer[0] ^= 1;
    }
    answer[2..4].copy_from_slice(&0x8180u16.to_be_bytes());
    answer[6..8].copy_from_slice(&1u16.to_be_bytes());
    answer.extend_from_slice(&[0xc0, 0x0c, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 192, 0, 2, 42]);
    let mut bytes = vec![0; 14 + 20 + 8 + answer.len()];
    let mut frame = EthernetFrame::new_unchecked(&mut bytes[..]);
    frame.set_src_addr(ethernet.dst_addr());
    frame.set_dst_addr(ethernet.src_addr());
    frame.set_ethertype(EthernetProtocol::Ipv4);
    let repr = Ipv4Repr {
        src_addr: if wrong_server {
            Ipv4Address::new(10, 0, 0, 99)
        } else {
            ip.dst_addr()
        },
        dst_addr: ip.src_addr(),
        next_header: IpProtocol::Udp,
        payload_len: 8 + answer.len(),
        hop_limit: 64,
    };
    let checksums = smoltcp::phy::ChecksumCapabilities::default();
    let mut packet = Ipv4Packet::new_unchecked(frame.payload_mut());
    repr.emit(&mut packet, &checksums);
    let mut response = UdpPacket::new_unchecked(packet.payload_mut());
    UdpRepr {
        src_port: 53,
        dst_port: udp.src_port(),
    }
    .emit(
        &mut response,
        &repr.src_addr.into(),
        &repr.dst_addr.into(),
        answer.len(),
        |payload| payload.copy_from_slice(&answer),
        &checksums,
    );
    bytes
}

#[test]
// ------------------------=
// FUNC: wire_dns_resolves_and_rejects_wrong_transaction
// DESC: Exchanges real ARP and DNS frames, rejects a mismatched response, consumes an A answer, then tests timeout and cancellation slot reuse.
// ------------------=
fn wire_dns_resolves_and_rejects_wrong_transaction() {
    let (mut a, mut b) = (Ethernet::default(), Ethernet::default());
    let (mut ar, mut at, mut br, mut bt) = ([0; 1024], [0; 1024], [0; 1024], [0; 1024]);
    let mut sa = [SocketStorage::EMPTY, SocketStorage::EMPTY];
    let mut sb = [SocketStorage::EMPTY];
    let mut queries = [None];
    let now = Instant::from_millis(0);
    let mut client = Transport::new(
        &mut a,
        [2, 0, 0, 0, 0, 1],
        [10, 0, 0, 1],
        24,
        None,
        42,
        now,
        &mut sa,
        &mut ar,
        &mut at,
    )
    .unwrap();
    let mut peer = Transport::new(
        &mut b,
        [2, 0, 0, 0, 0, 2],
        [10, 0, 0, 2],
        24,
        None,
        43,
        now,
        &mut sb,
        &mut br,
        &mut bt,
    )
    .unwrap();
    client.enable_dns([10, 0, 0, 2], &mut queries).unwrap();
    client
        .resolve("example.test", now, Instant::from_millis(2000))
        .unwrap();
    assert_eq!(client.resolved_address(), Err(Error::WouldBlock));
    assert_eq!(
        client.resolve("second.test", now, Instant::from_millis(2000)),
        Err(Error::Busy)
    );
    let mut request = None;
    for tick in 0..100 {
        let now = Instant::from_millis(tick);
        client.poll(&mut a, now);
        while let Some(frame) = a.tx.pop_front() {
            if frame[12..14] == [8, 0] && frame[23] == 17 {
                request = Some(frame);
            } else {
                b.rx.push_back(frame);
            }
        }
        peer.poll(&mut b, now);
        a.rx.extend(b.tx.drain(..));
        if request.is_some() {
            break;
        }
    }
    let request = request.expect("a DNS datagram must reach the NIC");
    a.rx.push_back(dns_answer(&request, false, true));
    client.poll(&mut a, Instant::from_millis(99));
    assert_eq!(client.resolved_address(), Err(Error::WouldBlock));
    a.rx.push_back(dns_answer(&request, true, false));
    client.poll(&mut a, Instant::from_millis(100));
    assert_eq!(client.resolved_address(), Err(Error::WouldBlock));
    a.rx.push_back(dns_answer(&request, false, false));
    client.poll(&mut a, Instant::from_millis(101));
    assert_eq!(client.resolved_address(), Ok([192, 0, 2, 42]));
    assert_eq!(client.resolved_address(), Err(Error::Configuration));
    client
        .resolve("timeout.test", now, Instant::from_millis(200))
        .unwrap();
    client.poll(&mut a, Instant::from_millis(200));
    assert_eq!(client.resolved_address(), Err(Error::Timeout));
    client
        .resolve("cancel.test", now, Instant::from_millis(2000))
        .unwrap();
    client.cancel();
    assert_eq!(client.resolved_address(), Err(Error::Cancelled));
    assert_eq!(
        client.resolve("retry.test", now, Instant::from_millis(2000)),
        Ok(())
    );
}
impl RxToken for Receive {
    // ------------------------=
    // FUNC: consume
    // DESC: Supplies a complete synthetic Ethernet frame to the real protocol implementation.
    // ------------------=
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        f(&self.0)
    }
}
impl TxToken for Transmit<'_> {
    // ------------------------=
    // FUNC: consume
    // DESC: Captures actual emitted Ethernet bytes without using host networking.
    // ------------------=
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut bytes = vec![0; len];
        let result = f(&mut bytes);
        self.0.push_back(bytes);
        result
    }
}
impl Device for Ethernet {
    type RxToken<'a> = Receive;
    type TxToken<'a> = Transmit<'a>;
    // ------------------------=
    // FUNC: receive
    // DESC: Dequeues a test frame while retaining response capacity.
    // ------------------=
    fn receive(&mut self, _: Instant) -> Option<(Receive, Transmit<'_>)> {
        self.rx
            .pop_front()
            .map(|frame| (Receive(frame), Transmit(&mut self.tx)))
    }
    // ------------------------=
    // FUNC: transmit
    // DESC: Models a bounded NIC transmit queue and exposes backpressure.
    // ------------------=
    fn transmit(&mut self, _: Instant) -> Option<Transmit<'_>> {
        if self.tx.len() < 4 {
            Some(Transmit(&mut self.tx))
        } else {
            None
        }
    }
    // ------------------------=
    // FUNC: capabilities
    // DESC: Enables real software checksums on standard Ethernet frames.
    // ------------------=
    fn capabilities(&self) -> DeviceCapabilities {
        let mut c = DeviceCapabilities::default();
        c.medium = Medium::Ethernet;
        c.max_transmission_unit = 1514;
        c.max_burst_size = Some(4);
        c
    }
}

#[test]
// ------------------------=
// FUNC: reliable_stream_survives_loss_and_backpressure
// DESC: Exchanges binary data through two real TCP stacks after losing a SYN and corrupting a data frame.
// ------------------=
fn reliable_stream_survives_loss_and_backpressure() {
    let (mut a, mut b) = (Ethernet::default(), Ethernet::default());
    let (mut ar, mut at, mut br, mut bt) = ([0; 4096], [0; 4096], [0; 4096], [0; 4096]);
    let (mut sa, mut sb) = ([SocketStorage::EMPTY], [SocketStorage::EMPTY]);
    let mut client = Transport::new(
        &mut a,
        [2, 0, 0, 0, 0, 1],
        [10, 0, 0, 1],
        24,
        None,
        1,
        Instant::from_millis(0),
        &mut sa,
        &mut ar,
        &mut at,
    )
    .unwrap();
    let mut server = Transport::new(
        &mut b,
        [2, 0, 0, 0, 0, 2],
        [10, 0, 0, 2],
        24,
        None,
        2,
        Instant::from_millis(0),
        &mut sb,
        &mut br,
        &mut bt,
    )
    .unwrap();
    server
        .sockets
        .get_mut::<Socket>(server.handle)
        .listen(80)
        .unwrap();
    client
        .connect(
            [10, 0, 0, 2],
            80,
            49152,
            Instant::from_millis(0),
            Instant::from_millis(30000),
        )
        .unwrap();
    let payload: Vec<u8> = (0..7000).map(|i| (i % 251) as u8).collect();
    let mut sent = 0;
    let mut received = Vec::new();
    let mut lost_syn = false;
    let mut corrupted = false;
    let mut backpressure = false;
    let mut held: Option<Vec<u8>> = None;
    let mut reordered = false;
    let mut duplicated = false;
    for tick in 0..1500 {
        let now = Instant::from_millis(tick * 20);
        client.poll(&mut a, now);
        server.poll(&mut b, now);
        while let Some(mut frame) = a.tx.pop_front() {
            if frame.len() >= 54 && frame[23] == 6 {
                if frame[47] & 2 != 0 && !lost_syn {
                    lost_syn = true;
                    continue;
                }
                let end = 14 + u16::from_be_bytes([frame[16], frame[17]]) as usize;
                let header = 34 + ((frame[46] >> 4) as usize * 4);
                if end > header && !corrupted {
                    frame[header] ^= 1;
                    corrupted = true;
                } else if end > header && !reordered {
                    if let Some(delayed) = held.take() {
                        assert_ne!(&delayed[38..42], &frame[38..42]);
                        b.rx.push_back(frame.clone());
                        b.rx.push_back(frame);
                        b.rx.push_back(delayed);
                        reordered = true;
                        duplicated = true;
                        continue;
                    }
                    held = Some(frame);
                    continue;
                }
            }
            b.rx.push_back(frame);
        }
        while let Some(frame) = b.tx.pop_front() {
            a.rx.push_back(frame);
        }
        if client.state() == State::Established && sent < payload.len() {
            match client.send(&payload[sent..]) {
                Ok(n) => sent += n,
                Err(Error::WouldBlock) => backpressure = true,
                other => panic!("{other:?}"),
            }
        }
        if tick % 5 == 0 {
            let mut out = [0; 173];
            if let Ok(n) = server.receive(&mut out) {
                received.extend_from_slice(&out[..n]);
            }
        }
        if received.len() == payload.len() {
            break;
        }
    }
    assert!(lost_syn && corrupted && backpressure && reordered && duplicated);
    assert_eq!(received, payload);
    client.cancel();
    assert_eq!(client.send(b"forbidden"), Err(Error::Cancelled));
    assert_eq!(client.receive(&mut [0; 1]), Err(Error::Cancelled));
    assert_eq!(client.state(), State::Closed);
}

#[test]
// ------------------------=
// FUNC: unreachable_peer_hits_hard_deadline
// DESC: Verifies a nonresponsive endpoint cannot hold a transaction indefinitely.
// ------------------=
fn unreachable_peer_hits_hard_deadline() {
    let mut nic = Ethernet::default();
    let (mut rx, mut tx) = ([0; 128], [0; 128]);
    let mut storage = [SocketStorage::EMPTY];
    let mut stream = Transport::new(
        &mut nic,
        [2, 0, 0, 0, 0, 1],
        [10, 0, 0, 1],
        24,
        None,
        8,
        Instant::from_millis(0),
        &mut storage,
        &mut rx,
        &mut tx,
    )
    .unwrap();
    stream
        .connect(
            [10, 0, 0, 2],
            80,
            49152,
            Instant::from_millis(0),
            Instant::from_millis(100),
        )
        .unwrap();
    stream.poll(&mut nic, Instant::from_millis(100));
    assert_eq!(stream.state(), State::Closed);
    assert_eq!(stream.send(b"x"), Err(Error::Timeout));
}

#[test]
// ------------------------=
// FUNC: handshake_reset_and_half_close_are_distinct
// DESC: Exchanges actual TCP frames to distinguish pending, refused, reset and clean half-close outcomes.
// ------------------=
fn handshake_reset_and_half_close_are_distinct() {
    for outcome in 0..3 {
        let reset = outcome == 1;
        let (mut a, mut b) = (Ethernet::default(), Ethernet::default());
        let (mut ar, mut at, mut br, mut bt) = ([0; 1024], [0; 1024], [0; 1024], [0; 1024]);
        let (mut sa, mut sb) = ([SocketStorage::EMPTY], [SocketStorage::EMPTY]);
        let zero = Instant::from_millis(0);
        let mut client = Transport::new(
            &mut a,
            [2, 0, 0, 0, 0, 1],
            [10, 0, 0, 1],
            24,
            None,
            1,
            zero,
            &mut sa,
            &mut ar,
            &mut at,
        )
        .unwrap();
        let mut server = Transport::new(
            &mut b,
            [2, 0, 0, 0, 0, 2],
            [10, 0, 0, 2],
            24,
            None,
            2,
            zero,
            &mut sb,
            &mut br,
            &mut bt,
        )
        .unwrap();
        assert_eq!(client.receive(&mut [0; 1]), Err(Error::Disconnected));
        if outcome != 2 {
            server
                .sockets
                .get_mut::<Socket>(server.handle)
                .listen(80)
                .unwrap();
        }
        client
            .connect([10, 0, 0, 2], 80, 49152, zero, Instant::from_millis(5000))
            .unwrap();
        assert_eq!(client.receive(&mut [0; 1]), Err(Error::WouldBlock));
        assert_eq!(client.send(b"early"), Err(Error::WouldBlock));
        for tick in 0..100 {
            let now = Instant::from_millis(tick);
            client.poll(&mut a, now);
            b.rx.extend(a.tx.drain(..));
            server.poll(&mut b, now);
            a.rx.extend(b.tx.drain(..));
        }
        if outcome == 2 {
            assert_eq!(client.state(), State::Closed);
            assert_eq!(client.receive(&mut [0; 1]), Err(Error::Disconnected));
            continue;
        }
        assert_eq!(client.state(), State::Established);
        assert_eq!(server.state(), State::Established);
        if reset {
            server.cancel();
        } else {
            assert_eq!(server.send(&[4, 0, 255, 7]), Ok(4));
            server.close();
        }
        for tick in 100..300 {
            let now = Instant::from_millis(tick);
            server.poll(&mut b, now);
            a.rx.extend(b.tx.drain(..));
            client.poll(&mut a, now);
            b.rx.extend(a.tx.drain(..));
        }
        let mut bytes = [0; 16];
        if reset {
            assert_eq!(client.receive(&mut bytes), Err(Error::Disconnected));
            assert_eq!(client.send(b"no"), Err(Error::Disconnected));
        } else {
            assert_eq!(client.receive(&mut bytes), Ok(4));
            assert_eq!(&bytes[..4], &[4, 0, 255, 7]);
            assert_eq!(client.receive(&mut bytes), Ok(0));
            // A peer's FIN shuts down only its sending direction.
            assert_eq!(client.send(b"reply"), Ok(5));
        }
    }
}
