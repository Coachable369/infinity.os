//! Four-frame adapter between the native NIC pump and TCP. No allocation or waiting.
use smoltcp::{
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    time::Instant,
};
const MTU: usize = 1514;
const DEPTH: usize = 4;
#[derive(Clone, Copy)]
struct Frame {
    bytes: [u8; MTU],
    length: usize,
}
struct Queue {
    frames: [Frame; DEPTH],
    start: usize,
    count: usize,
}
impl Queue {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves fixed NIC queue storage without borrowing the global framebuffer or heap.
    // ------------------=
    const fn new() -> Self {
        Self {
            frames: [Frame {
                bytes: [0; MTU],
                length: 0,
            }; DEPTH],
            start: 0,
            count: 0,
        }
    }
    // ------------------------=
    // FUNC: pop
    // DESC: Removes an accepted packet from a FIFO ring.
    // ------------------=
    fn pop(&mut self) -> Option<Frame> {
        if self.count == 0 {
            return None;
        }
        let frame = self.frames[self.start];
        self.start = (self.start + 1) % DEPTH;
        self.count -= 1;
        Some(frame)
    }
}
pub struct EthernetQueue {
    rx: Queue,
    tx: Queue,
}
impl EthernetQueue {
    // ------------------------=
    // FUNC: new
    // DESC: Creates bounded ingress and egress rings for a single authorized network service.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            rx: Queue::new(),
            tx: Queue::new(),
        }
    }
    // ------------------------=
    // FUNC: ingest
    // DESC: Copies a frame only if there is capacity; malformed lengths and overflow never overwrite queued data.
    // ------------------=
    pub fn ingest(&mut self, bytes: &[u8]) -> bool {
        if !(14..=MTU).contains(&bytes.len()) || self.rx.count == DEPTH {
            return false;
        }
        let frame = &mut self.rx.frames[(self.rx.start + self.rx.count) % DEPTH];
        frame.bytes[..bytes.len()].copy_from_slice(bytes);
        frame.length = bytes.len();
        self.rx.count += 1;
        true
    }
    // ------------------------=
    // FUNC: pending
    // DESC: Borrows the oldest transmitted frame until the driver acknowledges successful NIC submission.
    // ------------------=
    pub fn pending(&self) -> Option<&[u8]> {
        if self.tx.count == 0 {
            return None;
        }
        let frame = &self.tx.frames[self.tx.start];
        Some(&frame.bytes[..frame.length])
    }
    // ------------------------=
    // FUNC: transmitted
    // DESC: Retires one frame only after the NIC has accepted it.
    // ------------------=
    pub fn transmitted(&mut self) {
        self.tx.pop();
    }
}
pub struct Receive(Frame);
pub struct Transmit<'a>(&'a mut Queue);
impl RxToken for Receive {
    // ------------------------=
    // FUNC: consume
    // DESC: Hands one bounded frame to the TCP parser.
    // ------------------=
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        f(&self.0.bytes[..self.0.length])
    }
}
impl TxToken for Transmit<'_> {
    // ------------------------=
    // FUNC: consume
    // DESC: Serializes directly into a reserved frame slot with no temporary allocation.
    // ------------------=
    fn consume<R, F>(self, length: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let frame = &mut self.0.frames[(self.0.start + self.0.count) % DEPTH];
        let result = f(&mut frame.bytes[..length]);
        frame.length = length;
        self.0.count += 1;
        result
    }
}
impl Device for EthernetQueue {
    type RxToken<'a> = Receive;
    type TxToken<'a> = Transmit<'a>;
    // ------------------------=
    // FUNC: receive
    // DESC: Preserves ingress until space exists for any required protocol response.
    // ------------------=
    fn receive(&mut self, _: Instant) -> Option<(Receive, Transmit<'_>)> {
        if self.tx.count == DEPTH {
            return None;
        }
        self.rx
            .pop()
            .map(|frame| (Receive(frame), Transmit(&mut self.tx)))
    }
    // ------------------------=
    // FUNC: transmit
    // DESC: Reports NIC queue pressure instead of spinning or discarding unsubmitted packets.
    // ------------------=
    fn transmit(&mut self, _: Instant) -> Option<Transmit<'_>> {
        if self.tx.count == DEPTH {
            None
        } else {
            Some(Transmit(&mut self.tx))
        }
    }
    // ------------------------=
    // FUNC: capabilities
    // DESC: Advertises Ethernet MTU and software checksum verification with a four-frame burst limit.
    // ------------------=
    fn capabilities(&self) -> DeviceCapabilities {
        let mut capabilities = DeviceCapabilities::default();
        capabilities.medium = Medium::Ethernet;
        capabilities.max_transmission_unit = MTU;
        capabilities.max_burst_size = Some(DEPTH);
        capabilities
    }
}
