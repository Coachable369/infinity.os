//! Trusted engineering UART channel: exposes public diagnostics and requires external decisions.
use crate::{
    fixture::Fixture,
    runtime::{
        network::types::Packet,
        node::types::{NodeError, NodeId},
    },
};
const PREFIX: [u8; 4] = [0x91, 0x9a, 0x4f, 0x50];
pub struct Operator {
    input: [u8; 512],
    used: usize,
    needed: usize,
}
impl Operator {
    // ------------------------=
    // FUNC: ready
    // DESC: Announces guest initialization through a binary operator record before accepting external commands.
    // ------------------=
    pub fn ready() {
        for byte in PREFIX {
            send(byte);
        }
        for byte in [255, 0, 0, 0] {
            send(byte);
        }
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates a fixed-size binary operator command parser.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            input: [0; 512],
            used: 0,
            needed: 0,
        }
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Consumes at most sixteen UART bytes per call; no guest-side approval or protocol peer substitution occurs.
    // ------------------=
    pub fn poll(
        &mut self,
        f: &mut Fixture,
        now: u64,
        tx: &Option<Packet>,
        rx: &Option<Packet>,
        saved: &mut Option<Packet>,
    ) {
        for _ in 0..16 {
            if unsafe { read(0x3fd) } & 1 == 0 {
                break;
            }
            let byte = unsafe { read(0x3f8) };
            if self.used < 4 && byte != PREFIX[self.used] {
                self.used = 0;
                continue;
            }
            self.input[self.used] = byte;
            self.used += 1;
            if self.used == 7 {
                self.needed = 7 + u16::from_le_bytes([self.input[5], self.input[6]]) as usize;
                if self.needed > 512 {
                    self.used = 0;
                    self.needed = 0;
                }
            }
            if self.needed != 0 && self.used == self.needed {
                let command = self.input[4];
                let mut output = [0; 512];
                let result = execute(
                    f,
                    command,
                    &self.input[7..self.used],
                    now,
                    tx,
                    rx,
                    saved,
                    &mut output,
                );
                let (status, length) = match result {
                    Ok(length) => (0, length),
                    Err(error) => {
                        output[0] = error as u8;
                        (1, 1)
                    }
                };
                for byte in PREFIX {
                    send(byte);
                }
                send(command);
                send(status);
                for byte in (length as u16).to_le_bytes() {
                    send(byte);
                }
                for byte in &output[..length] {
                    send(*byte);
                }
                self.used = 0;
                self.needed = 0;
            }
        }
    }
}
// ------------------------=
// FUNC: read
// DESC: Reads one x86 UART register in the native engineering guest.
// ------------------=
unsafe fn read(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!("in al, dx",in("dx")port,out("al")value,options(nomem,nostack));
    value
}
// ------------------------=
// FUNC: send
// DESC: Emits a diagnostic byte with a bounded hardware-ready wait.
// ------------------=
fn send(value: u8) {
    for _ in 0..100000 {
        if unsafe { read(0x3fd) } & 0x20 != 0 {
            unsafe {
                core::arch::asm!("out dx, al",in("dx")0x3f8u16,in("al")value,options(nomem,nostack));
            }
            return;
        }
    }
}
// ------------------------=
// FUNC: node
// DESC: Decodes an explicit full peer NodeId supplied by the external operator.
// ------------------=
fn node(bytes: &[u8]) -> Result<NodeId, NodeError> {
    Ok(NodeId(
        bytes
            .get(..32)
            .ok_or(NodeError::InvalidAdvertisement)?
            .try_into()
            .map_err(|_| NodeError::InvalidAdvertisement)?,
    ))
}
// ------------------------=
// FUNC: execute
// DESC: Executes typed operator commands against this guest only; all peer negotiation and data use the production wire adapter.
// ------------------=
fn execute(
    f: &mut Fixture,
    command: u8,
    input: &[u8],
    now: u64,
    tx: &Option<Packet>,
    rx: &Option<Packet>,
    saved: &mut Option<Packet>,
    out: &mut [u8; 512],
) -> Result<usize, NodeError> {
    match command {
        0 => {
            out[..32].copy_from_slice(&f.nodes.local_id().ok_or(NodeError::EntropyUnavailable)?.0);
            out[32..40].copy_from_slice(&now.to_le_bytes());
            Ok(40)
        }
        1 => {
            let id = node(input)?;
            let n = f
                .nodes
                .discovered_nodes()
                .iter()
                .flatten()
                .find(|p| p.id == id)
                .ok_or(NodeError::UnknownNode)?;
            out[0] = n.trust as u8;
            out[1] = n.reachability as u8;
            out[2..10].copy_from_slice(&n.last_seen.to_le_bytes());
            let link = f
                .transport
                .inspect(f.connection, f.owner)
                .ok_or(NodeError::UnknownNode)?;
            out[10] = (link.peer == Some(id)) as u8;
            out[11..13].copy_from_slice(&link.local.port.to_le_bytes());
            out[13..15].copy_from_slice(&link.remote.port.to_le_bytes());
            Ok(15)
        }
        2 => {
            if input.len() != 33 {
                return Err(NodeError::InvalidAdvertisement);
            }
            let peer = node(input)?;
            let link = f
                .transport
                .inspect(f.connection, f.owner)
                .filter(|l| l.peer == Some(peer))
                .ok_or(NodeError::UnknownNode)?;
            let id = f
                .transport
                .trust
                .begin(&mut f.nodes, link, 0, input[32] != 0, now)?;
            out[..32].copy_from_slice(&id);
            Ok(32)
        }
        3 => {
            let v = f
                .transport
                .trust
                .verification(f.nodes.local_id().unwrap(), node(input)?)
                .ok_or(NodeError::PairingNotFound)?;
            out[..32].copy_from_slice(&v.transaction);
            out[32..64].copy_from_slice(&v.fingerprint);
            out[64..68].copy_from_slice(&v.code.to_le_bytes());
            out[68..76].copy_from_slice(&v.pairing.to_le_bytes());
            out[76..84].copy_from_slice(&v.expires.to_le_bytes());
            out[84..92].copy_from_slice(&v.scope.to_le_bytes());
            out[92] = v.state as u8;
            out[93..125].copy_from_slice(&v.local.0);
            out[125..157].copy_from_slice(&v.peer.0);
            Ok(157)
        }
        4 => {
            if input.len() != 37 {
                return Err(NodeError::InvalidAdvertisement);
            }
            let id = input[..32]
                .try_into()
                .map_err(|_| NodeError::InvalidAdvertisement)?;
            let code = u32::from_le_bytes(input[32..36].try_into().unwrap());
            f.transport
                .trust
                .confirm(&mut f.nodes, id, code, input[36] == 1, now)?;
            Ok(0)
        }
        5 => {
            let handle = f
                .transport
                .trust
                .session(node(input)?)
                .ok_or(NodeError::SessionNotFound)?;
            let session = f
                .nodes
                .sessions()
                .iter()
                .flatten()
                .find(|s| s.id == handle)
                .ok_or(NodeError::SessionNotFound)?;
            out[..8].copy_from_slice(&handle.to_le_bytes());
            out[8..24].copy_from_slice(&session.protocol_reference);
            out[24..32].copy_from_slice(&session.send_sequence.to_le_bytes());
            out[32..40].copy_from_slice(&session.receive_sequence.to_le_bytes());
            out[40] = session.state as u8;
            Ok(41)
        }
        6 => {
            let peer = node(input)?;
            f.transport
                .trust
                .send_data(&mut f.nodes, peer, &input[32..], false, now)?;
            Ok(0)
        }
        7 => {
            let data = f
                .transport
                .trust
                .receive_data()
                .ok_or(NodeError::ResourceLimit)?;
            out[..32].copy_from_slice(&data.peer.0);
            out[32..48].copy_from_slice(&data.reference);
            out[48..50].copy_from_slice(&(data.length as u16).to_le_bytes());
            out[50..50 + data.length].copy_from_slice(&data.bytes[..data.length]);
            Ok(50 + data.length)
        }
        8 => {
            f.transport
                .trust
                .send_data(&mut f.nodes, node(input)?, &[], true, now)?;
            Ok(0)
        }
        9 => {
            if input.len() != 32 {
                return Err(NodeError::InvalidAdvertisement);
            }
            f.transport
                .trust
                .cancel(&mut f.nodes, input.try_into().unwrap(), now)?;
            Ok(0)
        }
        10 => {
            let mode = *input.first().ok_or(NodeError::InvalidAdvertisement)?;
            let mut packet = if mode == 1 || mode >= 3 {
                (*saved).ok_or(NodeError::SessionNotFound)?
            } else if mode == 2 {
                (*rx).ok_or(NodeError::SessionNotFound)?
            } else {
                (*tx).ok_or(NodeError::SessionNotFound)?
            };
            match mode {
                0 | 1 => {}
                2 => {
                    let mut source = [0; 32];
                    source.copy_from_slice(&packet.bytes[16..48]);
                    let mut destination = [0; 32];
                    destination.copy_from_slice(&packet.bytes[48..80]);
                    packet.bytes[16..48].copy_from_slice(&destination);
                    packet.bytes[48..80].copy_from_slice(&source);
                }
                3 => packet.bytes[packet.length as usize - 1] ^= 1,
                4 => packet.bytes[138] ^= 1,
                5 => packet.bytes[112] ^= 1,
                6 => packet.bytes[128] = packet.bytes[128].wrapping_add(20),
                _ => return Err(NodeError::UnsupportedVersion),
            }
            if matches!(mode, 2 | 3 | 4) {
                packet.bytes[128] = packet.bytes[128].wrapping_add(20);
            }
            let cap = (0..f.capabilities.count())
                .filter_map(|i| f.capabilities.nth(i))
                .find(|c| {
                    c.kind == crate::runtime::capability::CapabilityType::NetworkSend
                        && c.target == f.connection as u64
                })
                .map(|c| c.id)
                .ok_or(NodeError::CapabilityDenied)?;
            f.network
                .send_datagram(
                    f.owner,
                    cap,
                    f.connection,
                    &packet.bytes[..packet.length as usize],
                    now,
                    now + 1,
                    &f.capabilities,
                )
                .map_err(|_| NodeError::ResourceLimit)?;
            Ok(0)
        }
        11 => {
            let p = f.nodes.begin_pairing(node(input)?, now)?;
            f.nodes.cancel_pairing(p.id)?;
            Ok(0)
        }
        12 => {
            *saved = *tx;
            Ok(0)
        }
        13 => {
            out[..8].copy_from_slice(&f.transport.trust.rejected.to_le_bytes());
            out[8] = f.transport.trust.last_error.map(|e| e as u8).unwrap_or(255);
            out[9..17].copy_from_slice(&f.transport.rejected_packets.to_le_bytes());
            Ok(17)
        }
        14 => crate::exit(0x10),
        15 => {
            let types = [
                0xda01, 0xda02, 0xda03, 0xda04, 0xda05, 0xda06, 0xd001, 0xd003,
            ];
            let mut mask = 0u64;
            for record in f.nodes.audit_records().iter().flatten() {
                if let Some(index) = types.iter().position(|kind| *kind == record.event_type) {
                    mask |= 1 << index;
                }
            }
            out[..8].copy_from_slice(&mask.to_le_bytes());
            Ok(8)
        }
        _ => Err(NodeError::UnsupportedVersion),
    }
}
