//! Bounded Ethernet/ARP/IPv4/UDP mechanism beneath the native Network Service.
//! No authority is granted here: callers must authorize endpoints before send.
pub const FRAME_BYTES: usize = 1514;
pub const DATAGRAM_BYTES: usize = 512;
const DEPTH: usize = 8;
const NEIGHBORS: usize = 16;
const LEASE: u64 = 60;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WireError { InvalidFrame, Unsupported, QueueFull, AddressUnresolved, Unconfigured }
#[derive(Clone, Copy)]
pub struct Frame { pub bytes: [u8; FRAME_BYTES], pub length: usize }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Datagram { pub source: [u8; 4], pub destination: [u8; 4], pub source_port: u16, pub destination_port: u16, pub bytes: [u8; DATAGRAM_BYTES], pub length: usize }
#[derive(Clone, Copy)]
struct Neighbor { address: [u8; 4], mac: Option<[u8; 6]>, expires: u64 }
pub struct WireNetwork {
    mac: [u8; 6], address: [u8; 4],
    neighbors: [Option<Neighbor>; NEIGHBORS],
    tx: [Option<Frame>; DEPTH], rx: [Option<Datagram>; DEPTH],
    pub accepted_frames: u64, pub rejected_frames: u64,
}

// ------------------------=
// FUNC: checksum
// DESC: Computes the Internet checksum, including an odd trailing byte.
// ------------------=
pub fn checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    for pair in bytes.chunks(2) { sum += (u32::from(pair[0]) << 8) | u32::from(*pair.get(1).unwrap_or(&0)); }
    while sum >> 16 != 0 { sum = (sum & 0xffff) + (sum >> 16); }
    !(sum as u16)
}
// ------------------------=
// FUNC: word
// DESC: Decodes a network-order word after the enclosing packet's bounds were checked.
// ------------------=
fn word(bytes: &[u8], at: usize) -> u16 { u16::from_be_bytes([bytes[at], bytes[at + 1]]) }
// ------------------------=
// FUNC: put
// DESC: Encodes a word into a statically bounded protocol header.
// ------------------=
fn put(bytes: &mut [u8], at: usize, value: u16) { bytes[at..at+2].copy_from_slice(&value.to_be_bytes()); }

impl WireNetwork {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates fixed-capacity packet queues with no configured address or ambient transport authority.
    // ------------------=
    pub const fn new() -> Self { Self { mac: [0; 6], address: [0; 4], neighbors: [None; NEIGHBORS], tx: [None; DEPTH], rx: [None; DEPTH], accepted_frames: 0, rejected_frames: 0 } }
    // ------------------------=
    // FUNC: configure
    // DESC: Clears stale frames and neighbors whenever interface identity or address changes.
    // ------------------=
    pub fn configure(&mut self, mac: [u8; 6], address: [u8; 4]) {
        if self.mac != mac || self.address != address {
            self.mac = mac; self.address = address;
            self.neighbors.fill(None); self.tx.fill(None); self.rx.fill(None);
        }
    }
    // ------------------------=
    // FUNC: enqueue
    // DESC: Enqueues a frame without overwriting older pending packets.
    // ------------------=
    fn enqueue(&mut self, frame: Frame) -> Result<(), WireError> {
        let slot = self.tx.iter_mut().find(|slot| slot.is_none()).ok_or(WireError::QueueFull)?;
        *slot = Some(frame); Ok(())
    }
    // ------------------------=
    // FUNC: peek_transmit
    // DESC: Exposes the oldest frame until hardware accepts ownership.
    // ------------------=
    pub fn peek_transmit(&self) -> Option<&Frame> { self.tx[0].as_ref() }
    // ------------------------=
    // FUNC: complete_transmit
    // DESC: Removes a frame only after successful submission to the NIC.
    // ------------------=
    pub fn complete_transmit(&mut self) { self.tx.rotate_left(1); self.tx[DEPTH-1] = None; }
    // ------------------------=
    // FUNC: receive_datagram
    // DESC: Returns the oldest validated UDP datagram from the bounded ingress queue.
    // ------------------=
    pub fn receive_datagram(&mut self) -> Option<Datagram> { let packet = self.rx[0].take()?; self.rx.rotate_left(1); self.rx[DEPTH-1] = None; Some(packet) }
    // ------------------------=
    // FUNC: ethernet
    // DESC: Builds an Ethernet II header in a fixed-size frame.
    // ------------------=
    fn ethernet(&self, destination: [u8; 6], protocol: u16) -> Frame {
        let mut frame = Frame { bytes: [0; FRAME_BYTES], length: 60 };
        frame.bytes[..6].copy_from_slice(&destination); frame.bytes[6..12].copy_from_slice(&self.mac);
        put(&mut frame.bytes, 12, protocol); frame
    }
    // ------------------------=
    // FUNC: arp
    // DESC: Builds a bounded Ethernet/IPv4 ARP request or response.
    // ------------------=
    fn arp(&self, operation: u16, target: [u8; 4], mac: [u8; 6]) -> Frame {
        let mut frame = self.ethernet(if operation == 1 { [255; 6] } else { mac }, 0x806);
        let bytes = &mut frame.bytes;
        put(bytes, 14, 1); put(bytes, 16, 0x800); bytes[18] = 6; bytes[19] = 4; put(bytes, 20, operation);
        bytes[22..28].copy_from_slice(&self.mac); bytes[28..32].copy_from_slice(&self.address);
        bytes[32..38].copy_from_slice(&mac); bytes[38..42].copy_from_slice(&target); frame
    }
    // ------------------------=
    // FUNC: send
    // DESC: Queues one authorized UDP datagram or starts bounded next-hop ARP resolution without waiting.
    // ------------------=
    pub fn send(&mut self, destination: [u8; 4], next_hop: [u8; 4], source_port: u16, destination_port: u16, payload: &[u8], now: u64) -> Result<(), WireError> {
        if self.address == [0; 4] { return Err(WireError::Unconfigured); }
        if payload.len() > DATAGRAM_BYTES || source_port == 0 || destination_port == 0 { return Err(WireError::InvalidFrame); }
        let mac = if destination == [255; 4] { [255; 6] } else {
            for slot in &mut self.neighbors { if slot.map(|n| now >= n.expires).unwrap_or(false) { *slot = None; } }
            match self.neighbors.iter().flatten().find(|n| n.address == next_hop) {
                Some(Neighbor { mac: Some(mac), .. }) => *mac,
                Some(_) => return Err(WireError::AddressUnresolved),
                None => {
                    let index = self.neighbors.iter().position(Option::is_none).ok_or(WireError::QueueFull)?;
                    self.enqueue(self.arp(1, next_hop, [0; 6]))?;
                    self.neighbors[index] = Some(Neighbor { address: next_hop, mac: None, expires: now.saturating_add(2) });
                    return Err(WireError::AddressUnresolved);
                }
            }
        };
        let mut frame = self.ethernet(mac, 0x800);
        let b = &mut frame.bytes;
        b[14] = 0x45; put(b, 16, (28 + payload.len()) as u16); put(b, 20, 0x4000); b[22] = 64; b[23] = 17;
        b[26..30].copy_from_slice(&self.address); b[30..34].copy_from_slice(&destination);
        let sum = checksum(&b[14..34]); put(b, 24, sum);
        put(b, 34, source_port); put(b, 36, destination_port); put(b, 38, (8 + payload.len()) as u16);
        b[42..42+payload.len()].copy_from_slice(payload);
        // IPv4 permits an absent UDP checksum; ingress still validates one when present.
        frame.length = (42 + payload.len()).max(60);
        self.enqueue(frame)
    }
    // ------------------------=
    // FUNC: ingest
    // DESC: Validates wire input before any indexing and records actual acceptance/rejection counters.
    // ------------------=
    pub fn ingest(&mut self, bytes: &[u8], now: u64) -> Result<(), WireError> {
        let result = self.parse(bytes, now);
        if result.is_ok() { self.accepted_frames = self.accepted_frames.saturating_add(1); }
        else { self.rejected_frames = self.rejected_frames.saturating_add(1); }
        result
    }
    // ------------------------=
    // FUNC: parse
    // DESC: Parses Ethernet destination, ARP or unfragmented IPv4/UDP with bounded payloads.
    // ------------------=
    fn parse(&mut self, b: &[u8], now: u64) -> Result<(), WireError> {
        if !(14..=FRAME_BYTES).contains(&b.len()) || self.address == [0; 4] { return Err(WireError::InvalidFrame); }
        if b[..6] != self.mac && b[..6] != [255; 6] { return Err(WireError::InvalidFrame); }
        if b[6] & 1 != 0 || b[6..12] == [0; 6] { return Err(WireError::InvalidFrame); }
        match word(b, 12) {
            0x806 => {
                if b.len() < 42 || word(b, 14) != 1 || word(b, 16) != 0x800 || b[18..20] != [6, 4] || b[22..28] != b[6..12] || b[38..42] != self.address { return Err(WireError::InvalidFrame); }
                let operation = word(b, 20);
                let sender: [u8; 4] = b[28..32].try_into().unwrap();
                let mac: [u8; 6] = b[22..28].try_into().unwrap();
                if operation == 1 { self.enqueue(self.arp(2, sender, mac))?; }
                else if operation != 2 || b[32..38] != self.mac { return Err(WireError::InvalidFrame); }
                // Learn only solicited addresses; discovery never creates authority.
                if let Some(slot) = self.neighbors.iter_mut().flatten().find(|n| n.address == sender && now < n.expires) { slot.mac = Some(mac); slot.expires = now.saturating_add(LEASE); }
                Ok(())
            }
            0x800 => {
                if b.len() < 34 || b[14] >> 4 != 4 { return Err(WireError::InvalidFrame); }
                let header = usize::from(b[14] & 15) * 4;
                let total = usize::from(word(b, 16));
                if header < 20 || total < header + 8 || 14 + total > b.len() || checksum(&b[14..14+header]) != 0 || word(b, 20) & 0x3fff != 0 || b[22] == 0 { return Err(WireError::InvalidFrame); }
                if b[23] != 17 { return Err(WireError::Unsupported); }
                if b[30..34] != self.address && b[30..34] != [255; 4] { return Err(WireError::InvalidFrame); }
                let udp = 14 + header;
                let length = usize::from(word(b, udp+4));
                if length < 8 || length != total-header || length-8 > DATAGRAM_BYTES || word(b, udp+2) == 0 { return Err(WireError::InvalidFrame); }
                if word(b, udp+6) != 0 {
                    let mut check = [0u8; 12+8+DATAGRAM_BYTES];
                    check[..8].copy_from_slice(&b[26..34]); check[9] = 17; put(&mut check, 10, length as u16);
                    check[12..12+length].copy_from_slice(&b[udp..udp+length]);
                    if checksum(&check[..12+length]) != 0 { return Err(WireError::InvalidFrame); }
                }
                let slot = self.rx.iter_mut().find(|slot| slot.is_none()).ok_or(WireError::QueueFull)?;
                let mut packet = Datagram { source: b[26..30].try_into().unwrap(), destination: b[30..34].try_into().unwrap(), source_port: word(b, udp), destination_port: word(b, udp+2), bytes: [0; DATAGRAM_BYTES], length: length-8 };
                packet.bytes[..packet.length].copy_from_slice(&b[udp+8..udp+length]); *slot = Some(packet); Ok(())
            }
            _ => Err(WireError::Unsupported),
        }
    }
}
