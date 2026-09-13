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
    let (mut ar, mut at, mut br, mut bt) = ([0; 1024], [0; 1024], [0; 1024], [0; 1024]);
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
    assert!(lost_syn && corrupted && backpressure);
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
