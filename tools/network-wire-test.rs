//! HOST LOGIC TESTS: these tests do not prove a NIC or installed-node exchange.
#[path = "../kernel/runtime/network/wire.rs"]
mod wire;
use wire::{WireNetwork, WireError};

// ------------------------=
// FUNC: transfer
// DESC: Transfers a bounded encoded frame between parser fixtures, never presented as network acceptance.
// ------------------=
fn transfer(source: &mut WireNetwork, destination: &mut WireNetwork, now: u64) {
    let frame = *source.peek_transmit().unwrap();
    destination.ingest(&frame.bytes[..frame.length], now).unwrap();
    source.complete_transmit();
}
// ------------------------=
// FUNC: main
// DESC: Tests byte-level ARP, UDP, truncation, checksums, bounded queues and address-change invalidation.
// ------------------=
fn main() {
    let mut a = WireNetwork::new(); let mut b = WireNetwork::new();
    let ip_a = [10, 42, 0, 1]; let ip_b = [10, 42, 0, 2];
    a.configure([2, 0, 0, 0, 0, 1], ip_a); b.configure([2, 0, 0, 0, 0, 2], ip_b);
    assert_eq!(a.send(ip_b, ip_b, 49152, 49153, &[1, 2, 3], 1), Err(WireError::AddressUnresolved));
    transfer(&mut a, &mut b, 1); transfer(&mut b, &mut a, 1);
    a.send(ip_b, ip_b, 49152, 49153, &[1, 2, 3], 2).unwrap();
    let original = *a.peek_transmit().unwrap();
    for length in 0..45 {
        assert!(b.ingest(&original.bytes[..length], 2).is_err());
    }
    let mut corrupt = original; corrupt.bytes[24] ^= 1;
    assert_eq!(b.ingest(&corrupt.bytes[..corrupt.length], 2), Err(WireError::InvalidFrame));
    // An oversized IP header must reject without an out-of-bounds read.
    corrupt = original; corrupt.bytes[14] = 0x4f;
    assert_eq!(b.ingest(&corrupt.bytes[..34], 2), Err(WireError::InvalidFrame));
    transfer(&mut a, &mut b, 2);
    let packet = b.receive_datagram().unwrap();
    assert_eq!(packet.source, ip_a); assert_eq!(packet.destination, ip_b);
    assert_eq!(packet.destination_port, 49153); assert_eq!(&packet.bytes[..packet.length], &[1, 2, 3]);
    assert_eq!(b.send(ip_a, ip_a, 49153, 49152, &[4, 5], 3), Err(WireError::AddressUnresolved));
    transfer(&mut b, &mut a, 3); transfer(&mut a, &mut b, 3);
    b.send(ip_a, ip_a, 49153, 49152, &[4, 5], 4).unwrap(); transfer(&mut b, &mut a, 4);
    assert_eq!(a.receive_datagram().unwrap().length, 2);
    for _ in 0..8 { b.ingest(&original.bytes[..original.length], 5).unwrap(); }
    assert_eq!(b.ingest(&original.bytes[..original.length], 5), Err(WireError::QueueFull));
    for _ in 0..8 { assert!(b.receive_datagram().is_some()); }
    assert!(b.receive_datagram().is_none());
    for _ in 0..8 { a.send(ip_b, ip_b, 49152, 49153, &[0; 512], 5).unwrap(); }
    assert_eq!(a.send(ip_b, ip_b, 49152, 49153, &[0], 5), Err(WireError::QueueFull));
    assert_eq!(a.send(ip_b, ip_b, 49152, 49153, &[0; 513], 5), Err(WireError::InvalidFrame));
    a.configure([2, 0, 0, 0, 0, 1], [10, 42, 0, 3]);
    assert!(a.peek_transmit().is_none());
    assert_eq!(a.send(ip_b, ip_b, 49152, 49153, &[0], 6), Err(WireError::AddressUnresolved));
    assert_eq!(b.send(ip_a, ip_a, 49153, 49152, &[0], 70), Err(WireError::AddressUnresolved));
    // Arbitrary Ethernet input is rejected or bounded, never panics.
    let mut seed = 1u32;
    let mut bytes = [0u8; 1515];
    for length in 0..=1515 {
        for byte in &mut bytes[..length] { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); *byte = (seed >> 24) as u8; }
        let _ = b.ingest(&bytes[..length], 70);
    }
}
