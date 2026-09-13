//! Egress confinement for a dedicated DNS/HTTPS Ethernet client.

// ------------------------=
// FUNC: local_port
// DESC: Allows only endpoint-bound IPv4 TCP/UDP or ARP for its next hop, returning zero for ARP and the actual source port otherwise.
// ------------------=
pub fn local_port(
    frame: &[u8],
    mac: [u8; 6],
    source: [u8; 4],
    destination: [u8; 4],
    next_hop: [u8; 4],
    port: u16,
) -> Option<u16> {
    if frame.len() < 14 || frame[6..12] != mac {
        return None;
    }
    match &frame[12..14] {
        [8, 6] => {
            if frame.len() < 42
                || frame[14..20] != [0, 1, 8, 0, 6, 4]
                || !matches!(&frame[20..22], [0, 1] | [0, 2])
                || frame[22..28] != mac
                || frame[28..32] != source
                || frame[38..42] != next_hop
            {
                return None;
            }
            Some(0)
        }
        [8, 0] => {
            if frame.len() < 34 || frame[14] >> 4 != 4 {
                return None;
            }
            let header = (frame[14] as usize & 15) * 4;
            let length = u16::from_be_bytes([frame[16], frame[17]]) as usize;
            let fragment = u16::from_be_bytes([frame[20], frame[21]]);
            let protocol = if port == 53 { 17 } else { 6 };
            let minimum = if port == 53 { 8 } else { 20 };
            if header < 20
                || length < header + minimum
                || frame.len() < 14 + length
                || fragment & 0x3fff != 0
                || frame[23] != protocol
                || frame[26..30] != source
                || frame[30..34] != destination
            {
                return None;
            }
            let at = 14 + header;
            if u16::from_be_bytes([frame[at + 2], frame[at + 3]]) != port {
                return None;
            }
            let local = u16::from_be_bytes([frame[at], frame[at + 1]]);
            (local != 0).then_some(local)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: confines_automatic_stack_output
    // DESC: Rejects unrelated stack-generated traffic, alternate endpoints, fragments and truncated packets before NIC submission.
    // ------------------=
    #[test]
    fn confines_automatic_stack_output() {
        let mac = [2, 0, 0, 0, 0, 1];
        let source = [10, 0, 0, 1];
        let remote = [1, 2, 3, 4];
        let mut frame = [0; 54];
        frame[6..12].copy_from_slice(&mac);
        frame[12..14].copy_from_slice(&[8, 0]);
        frame[14] = 0x45;
        frame[17] = 40;
        frame[23] = 6;
        frame[26..30].copy_from_slice(&source);
        frame[30..34].copy_from_slice(&remote);
        frame[34..36].copy_from_slice(&50000u16.to_be_bytes());
        frame[36..38].copy_from_slice(&443u16.to_be_bytes());
        assert_eq!(
            local_port(&frame, mac, source, remote, remote, 443),
            Some(50000)
        );
        for length in 0..54 {
            assert_eq!(
                local_port(&frame[..length], mac, source, remote, remote, 443),
                None
            );
        }
        for offset in [6, 12, 20, 23, 26, 30, 36] {
            let mut changed = frame;
            changed[offset] ^= 1;
            assert_eq!(local_port(&changed, mac, source, remote, remote, 443), None);
        }
        assert_eq!(local_port(&frame, mac, source, remote, remote, 53), None);
    }
}
