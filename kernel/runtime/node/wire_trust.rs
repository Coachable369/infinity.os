//! Phase 9A: bounded, signed pairing and explicit two-party approval before sessions.
//! This module never dispatches IOP or grants application authority.
use super::transport::{endpoint_bytes, LinkSnapshot};
use super::{map_crypto_error, types::*, NodeCrypto, NodeRuntime};
use crate::runtime::network::types::Packet;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

const MAGIC: &[u8; 8] = b"IN9A0001";
const HEADER: usize = 112;
const OFFER: usize = 155;
const MAX_TRANSACTIONS: usize = 4;
const HISTORY: usize = 16;
const LEASE: u64 = 120;
const RETRIES: u8 = 12;
const MAX_DATA: usize = 192;
const INIT: u8 = 1;
const RESPONSE: u8 = 2;
const PROOF: u8 = 3;
const ACK: u8 = 4;
const CONFIRM: u8 = 5;
const CANCEL: u8 = 6;
const SESSION_INIT: u8 = 7;
const SESSION_RESPONSE: u8 = 8;
const SESSION_FINISH: u8 = 9;
const SESSION_ACK: u8 = 10;
const DATA: u8 = 11;
const CLOSE: u8 = 12;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WireState {
    AwaitResponse,
    AwaitProof,
    AwaitAck,
    PendingVerification,
    LocallyConfirmed,
    RemotelyConfirmed,
    Confirmed,
    SessionResponse,
    SessionFinish,
    SessionAck,
    Established,
    Cancelled,
    Expired,
    Failed,
    Closed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Verification {
    pub transaction: [u8; 32],
    pub local: NodeId,
    pub peer: NodeId,
    pub fingerprint: [u8; 32],
    pub code: u32,
    pub pairing: u64,
    pub scope: u64,
    pub expires: u64,
    pub protocol: u16,
    pub state: WireState,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReceivedData {
    pub peer: NodeId,
    pub reference: [u8; 16],
    pub length: usize,
    pub bytes: [u8; MAX_DATA],
}

#[derive(Clone)]
struct Transaction {
    id: [u8; 32],
    link: LinkSnapshot,
    stage: WireState,
    local_offer: [u8; OFFER],
    remote_offer: [u8; OFFER],
    secret: [u8; 32],
    digest: [u8; 32],
    paired: [u8; 32],
    init_hash: [u8; 32],
    pairing: u64,
    local_confirmed: bool,
    remote_confirmed: bool,
    expires: u64,
    session: Option<u64>,
    pending: Option<Packet>,
    next_send: u64,
    attempts: u8,
    ended_at: u64,
    end_site: u32,
}

#[derive(Clone)]
pub struct WireTrust {
    pub persist_pairing: fn(&[u8; NODE_STATE_BYTES]) -> bool,
    seed: Option<[u8; 32]>,
    counter: u64,
    transactions: [Option<Transaction>; MAX_TRANSACTIONS],
    retired: [[u8; 32]; HISTORY],
    retired_count: usize,
    received: [Option<ReceivedData>; 8],
    pub rejected: u64,
    pub last_error: Option<NodeError>,
}

impl WireTrust {
    // ------------------------=
    // FUNC: lifecycle
    // DESC: Exposes only public transaction state for lifecycle diagnostics, including terminal transactions hidden from the confirmation UI.
    // ------------------=
    pub fn lifecycle(&self, peer: NodeId) -> Option<([u8; 32], WireState, u64, u8, u64, u32)> {
        self.transactions.iter().flatten().find(|t| t.link.peer == Some(peer)).map(|t| (t.id, t.stage, t.expires, t.local_confirmed as u8 | ((t.remote_confirmed as u8) << 1), t.ended_at, t.end_site))
    }
    // ------------------------=
    // FUNC: transaction_for_pairing
    // DESC: Resolves an exact local pairing handle to its authenticated wire transaction without exposing secret state.
    // ------------------=
    pub fn transaction_for_pairing(&self, handle: u64) -> Option<[u8; 32]> {
        self.transactions.iter().flatten().find(|transaction| transaction.pairing == handle && handle != 0).map(|transaction| transaction.id)
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates bounded transactions without identity material or network authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            persist_pairing: reject_unconfigured_persistence,
            seed: None,
            counter: 0,
            transactions: [const { None }; MAX_TRANSACTIONS],
            retired: [[0; 32]; HISTORY],
            retired_count: 0,
            received: [None; 8],
            rejected: 0,
            last_error: None,
        }
    }

    // ------------------------=
    // FUNC: initialize
    // DESC: Domain-separates boot-fresh entropy for nonrepeating transaction and agreement material.
    // ------------------=
    pub fn initialize(&mut self, entropy: &[u8; 32]) {
        self.seed = Some(hash(&[b"InfinityOS wire trust entropy v1", entropy]));
    }

    // ------------------------=
    // FUNC: fresh
    // DESC: Derives fresh secret material with a checked counter and never persists it.
    // ------------------=
    fn fresh(&mut self) -> Result<[u8; 32], NodeError> {
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or(NodeError::ResourceLimit)?;
        Ok(hash(&[
            b"InfinityOS wire trust fresh v1",
            &self.seed.ok_or(NodeError::EntropyUnavailable)?,
            &self.counter.to_le_bytes(),
        ]))
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Starts an explicit selected-peer pairing or fresh reconnect; no confirmation is inferred.
    // ------------------=
    pub fn begin(
        &mut self,
        nodes: &mut NodeRuntime,
        link: LinkSnapshot,
        scope: u64,
        reconnect: bool,
        now: u64,
    ) -> Result<[u8; 32], NodeError> {
        let peer = checked_peer(nodes, link.peer.ok_or(NodeError::UnknownNode)?, now)?;
        let index = if let Some(index) = self.transactions.iter().position(|slot| {
            slot.as_ref()
                .map(|t| t.link.peer == Some(peer.id))
                .unwrap_or(false)
        }) {
            let t = self.transactions[index].as_ref().unwrap();
            if reconnect {
                if !matches!(
                    t.stage,
                    WireState::Closed
                        | WireState::Confirmed
                        | WireState::Failed
                        | WireState::Expired
                ) || t.paired == [0; 32]
                {
                    return Err(NodeError::UnsupportedState);
                }
                if peer.trust != TrustState::Trusted {
                    return Err(NodeError::NotTrusted);
                }
            } else if !matches!(
                t.stage,
                WireState::Cancelled | WireState::Expired | WireState::Failed
            ) || !matches!(peer.trust, TrustState::Untrusted | TrustState::Discovered)
            {
                return Err(NodeError::UnsupportedState);
            }
            self.retire(t.id)?;
            index
        } else {
            if reconnect && nodes.paired_digest(peer.id).is_none() {
                return Err(NodeError::NotTrusted);
            }
            self.transactions
                .iter()
                .position(Option::is_none)
                .ok_or(NodeError::ResourceLimit)?
        };
        let paired = if reconnect {
            self.transactions[index]
                .as_ref()
                .map(|t| t.paired)
                .or_else(|| nodes.paired_digest(peer.id))
                .unwrap_or([0; 32])
        } else {
            [0; 32]
        };
        let pairing = if reconnect {
            self.transactions[index].as_ref().map(|transaction| transaction.pairing).unwrap_or(0)
        } else {
            nodes.begin_pairing(peer.id, now)?.id
        };
        let entropy = self.fresh()?;
        let id = hash(&[b"InfinityOS wire transaction v1", &entropy]);
        let (offer, secret) = offer(nodes, link, scope, paired, entropy)?;
        let kind = if reconnect { SESSION_INIT } else { INIT };
        let packet = signed(nodes, kind, peer.id, id, &offer)?;
        let init_hash = hash(&[&packet.bytes[..packet.length as usize - 64]]);
        self.transactions[index] = Some(Transaction {
            id,
            link,
            stage: if reconnect {
                WireState::SessionResponse
            } else {
                WireState::AwaitResponse
            },
            local_offer: offer,
            remote_offer: [0; OFFER],
            secret,
            digest: [0; 32],
            paired,
            init_hash,
            pairing,
            local_confirmed: reconnect,
            remote_confirmed: reconnect,
            expires: now.saturating_add(LEASE),
            session: None,
            pending: Some(packet),
            next_send: now,
            attempts: 0,
            ended_at: 0,
            end_site: 0,
        });
        nodes.record(0xda01, peer.id, now, correlation(id), kind);
        Ok(id)
    }

    // ------------------------=
    // FUNC: verification
    // DESC: Exposes only independently authenticated public verification material, never a secret or automatic approval.
    // ------------------=
    pub fn verification(&self, local: NodeId, peer: NodeId) -> Option<Verification> {
        let t = self
            .transactions
            .iter()
            .flatten()
            .find(|t| t.link.peer == Some(peer))?;
        if !matches!(
            t.stage,
            WireState::PendingVerification
                | WireState::LocallyConfirmed
                | WireState::RemotelyConfirmed
                | WireState::Confirmed
        ) {
            return None;
        }
        Some(Verification {
            transaction: t.id,
            local,
            peer,
            fingerprint: t.digest,
            code: verification_code(t.digest),
            pairing: t.pairing,
            scope: u64::from_le_bytes(t.local_offer[115..123].try_into().ok()?),
            expires: t.expires,
            protocol: 1,
            state: t.stage,
        })
    }

    // ------------------------=
    // FUNC: confirm
    // DESC: Requires an external supplied code and explicit approval before signing a local confirmation.
    // ------------------=
    pub fn confirm(
        &mut self,
        nodes: &mut NodeRuntime,
        id: [u8; 32],
        code: u32,
        approved: bool,
        now: u64,
    ) -> Result<(), NodeError> {
        let t = self
            .transactions
            .iter_mut()
            .flatten()
            .find(|t| t.id == id)
            .ok_or(NodeError::PairingNotFound)?;
        check_pending(nodes, t, now)?;
        if !approved {
            return Err(NodeError::HumanApprovalRequired);
        }
        if !matches!(
            t.stage,
            WireState::PendingVerification | WireState::RemotelyConfirmed
        ) && !(t.stage == WireState::LocallyConfirmed && t.remote_confirmed) {
            return Err(NodeError::UnsupportedState);
        }
        if code != verification_code(t.digest) {
            nodes.record(0xda03, t.link.peer.unwrap(), now, correlation(id), 0);
            return Err(NodeError::VerificationMismatch);
        }
        t.local_confirmed = true;
        queue(
            t,
            signed(nodes, CONFIRM, t.link.peer.unwrap(), id, &t.digest)?,
            now,
        );
        commit_confirmation(nodes, t, now, self.persist_pairing)
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Commits cancellation locally before a bounded authenticated best-effort peer notification.
    // ------------------=
    pub fn cancel(
        &mut self,
        nodes: &mut NodeRuntime,
        id: [u8; 32],
        now: u64,
    ) -> Result<(), NodeError> {
        let t = self
            .transactions
            .iter_mut()
            .flatten()
            .find(|t| t.id == id)
            .ok_or(NodeError::PairingNotFound)?;
        check_pending(nodes, t, now)?;
        let packet = signed(nodes, CANCEL, t.link.peer.unwrap(), id, &t.digest)?;
        terminate(nodes, t, WireState::Cancelled, now);
        queue(t, packet, now);
        t.expires = now.saturating_add(6);
        nodes.record(0xda02, t.link.peer.unwrap(), now, correlation(id), 0);
        Ok(())
    }

    // ------------------------=
    // FUNC: session
    // DESC: Returns a locally established handle only after the authenticated finish acknowledgement.
    // ------------------=
    pub fn session(&self, peer: NodeId) -> Option<u64> {
        self.transactions
            .iter()
            .flatten()
            .find(|t| t.link.peer == Some(peer) && t.stage == WireState::Established)
            .and_then(|t| t.session)
    }

    // ------------------------=
    // FUNC: send_data
    // DESC: Queues bounded AEAD-protected bytes or an authenticated close and closes local keys immediately on close.
    // ------------------=
    pub fn send_data(
        &mut self,
        nodes: &mut NodeRuntime,
        peer: NodeId,
        bytes: &[u8],
        close: bool,
        now: u64,
    ) -> Result<(), NodeError> {
        if bytes.len() > MAX_DATA || (close && !bytes.is_empty()) {
            return Err(NodeError::ResourceLimit);
        }
        let t = self
            .transactions
            .iter_mut()
            .flatten()
            .find(|t| t.link.peer == Some(peer) && t.stage == WireState::Established)
            .ok_or(NodeError::SessionNotFound)?;
        if t.pending.is_some() {
            return Err(NodeError::ResourceLimit);
        }
        let handle = t.session.ok_or(NodeError::SessionNotFound)?;
        let session = nodes
            .sessions
            .iter()
            .flatten()
            .find(|s| s.id == handle)
            .ok_or(NodeError::SessionNotFound)?;
        let mut packet = header(
            nodes.local_id().ok_or(NodeError::EntropyUnavailable)?,
            peer,
            t.id,
            if close { CLOSE } else { DATA },
        );
        packet.bytes[112..128].copy_from_slice(&session.protocol_reference);
        packet.bytes[128..136]
            .copy_from_slice(&session.send_sequence.saturating_add(1).to_le_bytes());
        packet.bytes[136..138].copy_from_slice(&(bytes.len() as u16).to_le_bytes());
        let mut payload = [0; MAX_DATA];
        payload[..bytes.len()].copy_from_slice(bytes);
        let (_, tag) = nodes.protect(
            handle,
            &packet.bytes[..138],
            &mut payload[..bytes.len()],
            now,
        )?;
        packet.bytes[138..138 + bytes.len()].copy_from_slice(&payload[..bytes.len()]);
        packet.bytes[138 + bytes.len()..154 + bytes.len()].copy_from_slice(&tag);
        packet.length = (154 + bytes.len()) as u16;
        queue(t, packet, now);
        if close {
            nodes.close_session(handle, now, correlation(t.id))?;
            nodes.record(0xda06, peer, now, correlation(t.id), 0);
            t.stage = WireState::Closed;
            t.expires = now.saturating_add(6);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: receive_data
    // DESC: Removes one authenticated application payload from the bounded receive queue.
    // ------------------=
    pub fn receive_data(&mut self) -> Option<ReceivedData> {
        let value = self.received[0].take()?;
        self.received.rotate_left(1);
        self.received[7] = None;
        Some(value)
    }

    // ------------------------=
    // FUNC: receive_protocol
    // DESC: Demultiplexes one authenticated protocol payload without consuming unrelated application data.
    // ------------------=
    pub fn receive_protocol(&mut self, prefix: &[u8]) -> Option<ReceivedData> {
        let index = self.received.iter().position(|slot| slot.as_ref().map(|v| v.bytes[..v.length].starts_with(prefix)).unwrap_or(false))?;
        let value = self.received[index].take();
        self.received[index..].rotate_left(1);
        self.received[7] = None;
        value
    }

    // ------------------------=
    // FUNC: outgoing
    // DESC: Returns at most one due frame for a selected connected endpoint without a blocking retry loop.
    // ------------------=
    pub fn outgoing(&mut self, connection: u32, now: u64) -> Option<Packet> {
        let t = self
            .transactions
            .iter_mut()
            .flatten()
            .find(|t| t.link.connection == connection)?;
        if now >= t.expires || now < t.next_send || t.attempts >= RETRIES {
            return None;
        }
        let packet = t.pending?;
        t.next_send = now.saturating_add(2);
        t.attempts += 1;
        Some(packet)
    }

    // ------------------------=
    // FUNC: sent
    // DESC: Removes single-send data after UDP admission; signed negotiation retains bounded retransmission state.
    // ------------------=
    pub fn sent(&mut self, connection: u32) {
        if let Some(t) = self
            .transactions
            .iter_mut()
            .flatten()
            .find(|t| t.link.connection == connection)
        {
            if t.pending
                .as_ref()
                .map(|p| matches!(p.bytes[8], DATA | ACK | SESSION_ACK))
                .unwrap_or(false)
            {
                t.pending = None;
            }
        }
    }

    // ------------------------=
    // FUNC: tick
    // DESC: Expires bounded negotiations and invalidates live sessions on peer loss or current trust loss.
    // ------------------=
    pub fn tick(&mut self, nodes: &mut NodeRuntime, now: u64) {
        for t in self.transactions.iter_mut().flatten() {
            if matches!(
                t.stage,
                WireState::Cancelled | WireState::Expired | WireState::Failed | WireState::Closed
            ) {
                if now >= t.expires {
                    t.pending = None;
                }
                continue;
            }
            if now >= t.expires {
                terminate(nodes, t, WireState::Expired, now);
                continue;
            }
            if let Err(error) = checked_peer(nodes, t.link.peer.unwrap(), now) {
                self.last_error = Some(error);
                terminate(nodes, t, WireState::Failed, now);
            } else if let Some(handle) = t.session {
                if !nodes
                    .sessions
                    .iter()
                    .flatten()
                    .any(|s| s.id == handle && s.state == SessionState::Established)
                {
                    terminate(nodes, t, WireState::Closed, now);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: disconnect
    // DESC: Invalidates pairing and session material immediately when the owning endpoint or link disappears.
    // ------------------=
    pub fn disconnect(&mut self, nodes: &mut NodeRuntime, connection: u32, now: u64) {
        for t in self
            .transactions
            .iter_mut()
            .flatten()
            .filter(|t| t.link.connection == connection)
        {
            if !matches!(
                t.stage,
                WireState::Failed | WireState::Cancelled | WireState::Closed | WireState::Expired
            ) {
                terminate(nodes, t, WireState::Failed, now);
            }
        }
    }

    // ------------------------=
    // FUNC: ingest
    // DESC: Validates and routes one production wire message, auditing bounded failures without logging payloads or secrets.
    // ------------------=
    pub fn ingest(
        &mut self,
        nodes: &mut NodeRuntime,
        link: LinkSnapshot,
        bytes: &[u8],
        now: u64,
    ) -> Result<(), NodeError> {
        let result = self.process(nodes, link, bytes, now);
        if let Err(error) = result {
            self.rejected = self.rejected.saturating_add(1);
            self.last_error = Some(error);
            nodes.record(0xda05, link.peer.unwrap_or_default(), now, 0, error as u8);
            if bytes.len() >= HEADER
                && matches!(
                    error,
                    NodeError::SignatureInvalid
                        | NodeError::IdentityMismatch
                        | NodeError::UnsupportedVersion
                )
            {
                for t in self
                    .transactions
                    .iter_mut()
                    .flatten()
                    .filter(|t| t.link.connection == link.connection && bytes[80..112] == t.id)
                {
                    if matches!(
                        t.stage,
                        WireState::AwaitResponse
                            | WireState::AwaitProof
                            | WireState::AwaitAck
                            | WireState::SessionResponse
                            | WireState::SessionFinish
                            | WireState::SessionAck
                    ) {
                        terminate(nodes, t, WireState::Failed, now);
                    }
                }
            }
        }
        result
    }

    // ------------------------=
    // FUNC: process
    // DESC: Enforces signed transcript progression and authenticated session data without remote runtime shortcuts.
    // ------------------=
    fn process(
        &mut self,
        nodes: &mut NodeRuntime,
        link: LinkSnapshot,
        bytes: &[u8],
        now: u64,
    ) -> Result<(), NodeError> {
        if bytes.len() < HEADER
            || bytes.len() > 512
            || &bytes[..8] != MAGIC
            || bytes[9..16] != [1, 0, 0, 0, 0, 0, 0]
        {
            return Err(NodeError::InvalidAdvertisement);
        }
        let peer = checked_peer(nodes, link.peer.ok_or(NodeError::UnknownNode)?, now)?;
        if bytes[16..48] != peer.id.0
            || bytes[48..80] != nodes.local_id().ok_or(NodeError::EntropyUnavailable)?.0
        {
            return Err(NodeError::IdentityMismatch);
        }
        let kind = bytes[8];
        let id: [u8; 32] = bytes[80..112]
            .try_into()
            .map_err(|_| NodeError::InvalidAdvertisement)?;
        if matches!(kind, DATA | CLOSE) {
            return self.encrypted(nodes, peer.id, id, kind, bytes, now);
        }
        if bytes.len() < HEADER + 64 {
            return Err(NodeError::InvalidAdvertisement);
        }
        let end = bytes.len() - 64;
        let signature: [u8; 64] = bytes[end..]
            .try_into()
            .map_err(|_| NodeError::InvalidAdvertisement)?;
        NodeCrypto::verify(&peer.public_key, &bytes[..end], &signature)
            .map_err(map_crypto_error)?;
        let body = &bytes[HEADER..end];
        if matches!(kind, INIT | SESSION_INIT) {
            if body.len() != OFFER {
                return Err(NodeError::InvalidAdvertisement);
            }
            validate_offer(&peer, link, body)?;
            if self.retired[..self.retired_count].contains(&id) {
                return Err(NodeError::ReplayDetected);
            }
            if let Some(t) = self.transactions.iter_mut().flatten().find(|t| t.id == id) {
                if t.init_hash != hash(&[&bytes[..end]]) {
                    return Err(NodeError::IdentityMismatch);
                }
                if matches!(t.stage, WireState::AwaitProof | WireState::SessionFinish) {
                    t.next_send = now;
                    return Ok(());
                }
                return Err(NodeError::ReplayDetected);
            }
            let index = if let Some(index) = self.transactions.iter().position(|slot| {
                slot.as_ref()
                    .map(|t| t.link.peer == Some(peer.id))
                    .unwrap_or(false)
            }) {
                let t = self.transactions[index].as_ref().unwrap();
                if kind == SESSION_INIT {
                    if peer.trust != TrustState::Trusted
                        || !matches!(
                            t.stage,
                            WireState::Confirmed
                                | WireState::Closed
                                | WireState::Failed
                                | WireState::Established
                                | WireState::Expired
                        )
                        || body[123..155] != t.paired
                    {
                        return Err(NodeError::UnsupportedState);
                    }
                } else if !matches!(peer.trust, TrustState::Untrusted | TrustState::Discovered)
                    || !matches!(
                        t.stage,
                        WireState::Cancelled | WireState::Expired | WireState::Failed
                    )
                {
                    return Err(NodeError::UnsupportedState);
                }
                let old_id = t.id;
                let old_session = t.session;
                self.retire(old_id)?;
                if let Some(handle) = old_session {
                    let _ = nodes.close_session(handle, now, correlation(old_id));
                }
                index
            } else {
                if kind == SESSION_INIT && (peer.trust != TrustState::Trusted || nodes.paired_digest(peer.id).map(|digest| body[123..155] != digest).unwrap_or(true)) {
                    return Err(NodeError::NotTrusted);
                }
                self.transactions
                    .iter()
                    .position(Option::is_none)
                    .ok_or(NodeError::ResourceLimit)?
            };
            let paired: [u8; 32] = body[123..155]
                .try_into()
                .map_err(|_| NodeError::InvalidAdvertisement)?;
            if kind == INIT && paired != [0; 32] {
                return Err(NodeError::InvalidAdvertisement);
            }
            let scope = u64::from_le_bytes(
                body[115..123]
                    .try_into()
                    .map_err(|_| NodeError::InvalidAdvertisement)?,
            );
            let pairing = if kind == INIT {
                nodes.begin_pairing(peer.id, now)?.id
            } else {
                self.transactions[index].as_ref().map(|transaction| transaction.pairing).unwrap_or(0)
            };
            let entropy = self.fresh()?;
            let (local_offer, secret) = offer(nodes, link, scope, paired, entropy)?;
            let mut remote_offer = [0; OFFER];
            remote_offer.copy_from_slice(body);
            let digest = transcript(
                nodes.local_id().unwrap(),
                peer.id,
                id,
                kind,
                &local_offer,
                &remote_offer,
            );
            let init_hash = hash(&[&bytes[..end]]);
            let mut response = [0; OFFER + 32];
            response[..OFFER].copy_from_slice(&local_offer);
            response[OFFER..].copy_from_slice(&init_hash);
            let packet = signed(
                nodes,
                if kind == INIT {
                    RESPONSE
                } else {
                    SESSION_RESPONSE
                },
                peer.id,
                id,
                &response,
            )?;
            self.transactions[index] = Some(Transaction {
                id,
                link,
                stage: if kind == INIT {
                    WireState::AwaitProof
                } else {
                    WireState::SessionFinish
                },
                local_offer,
                remote_offer,
                secret,
                digest,
                paired,
                init_hash,
                pairing,
                local_confirmed: kind != INIT,
                remote_confirmed: kind != INIT,
                expires: now.saturating_add(LEASE),
                session: None,
                pending: Some(packet),
                next_send: now,
                attempts: 0,
                ended_at: 0,
                end_site: 0,
            });
            nodes.record(0xda01, peer.id, now, correlation(id), kind);
            return Ok(());
        }
        let t = self
            .transactions
            .iter_mut()
            .flatten()
            .find(|t| {
                t.id == id && t.link.connection == link.connection && t.link.peer == Some(peer.id)
            })
            .ok_or(NodeError::PairingNotFound)?;
        if now >= t.expires {
            terminate(nodes, t, WireState::Expired, now);
            return Err(NodeError::PairingExpired);
        }
        if matches!(
            t.stage,
            WireState::Cancelled | WireState::Expired | WireState::Failed | WireState::Closed
        ) {
            return Err(NodeError::UnsupportedState);
        }
        match kind {
            RESPONSE | SESSION_RESPONSE => {
                if body.len() != OFFER + 32 || body[OFFER..] != t.init_hash {
                    return Err(NodeError::IdentityMismatch);
                }
                validate_offer(&peer, link, &body[..OFFER])?;
                if body[115..155] != t.local_offer[115..155] {
                    return Err(NodeError::IdentityMismatch);
                }
                if !matches!(
                    (kind, t.stage),
                    (RESPONSE, WireState::AwaitResponse)
                        | (SESSION_RESPONSE, WireState::SessionResponse)
                ) {
                    return Err(NodeError::ReplayDetected);
                }
                t.remote_offer.copy_from_slice(&body[..OFFER]);
                t.digest = transcript(
                    nodes.local_id().unwrap(),
                    peer.id,
                    id,
                    if kind == RESPONSE { INIT } else { SESSION_INIT },
                    &t.local_offer,
                    &t.remote_offer,
                );
                if kind == RESPONSE {
                    validate_agreement(nodes, t)?;
                }
                let proof = signed(
                    nodes,
                    if kind == RESPONSE {
                        PROOF
                    } else {
                        SESSION_FINISH
                    },
                    peer.id,
                    id,
                    &t.digest,
                )?;
                if kind == SESSION_RESPONSE {
                    open(nodes, t, now)?;
                }
                queue(t, proof, now);
                t.stage = if kind == RESPONSE {
                    WireState::AwaitAck
                } else {
                    WireState::SessionAck
                };
            }
            PROOF | SESSION_FINISH => {
                if body != t.digest {
                    return Err(NodeError::IdentityMismatch);
                }
                let expected = if kind == PROOF {
                    WireState::AwaitProof
                } else {
                    WireState::SessionFinish
                };
                if t.stage != expected {
                    if matches!(
                        (kind, t.stage),
                        (PROOF, WireState::PendingVerification)
                            | (SESSION_FINISH, WireState::Established)
                    ) {
                        queue(
                            t,
                            signed(
                                nodes,
                                if kind == PROOF { ACK } else { SESSION_ACK },
                                peer.id,
                                id,
                                &t.digest,
                            )?,
                            now,
                        );
                        return Ok(());
                    }
                    return Err(NodeError::ReplayDetected);
                }
                if kind == PROOF {
                    validate_agreement(nodes, t)?;
                }
                let ack = signed(
                    nodes,
                    if kind == PROOF { ACK } else { SESSION_ACK },
                    peer.id,
                    id,
                    &t.digest,
                )?;
                if kind == PROOF {
                    verification_ready(nodes, t)?;
                } else {
                    open(nodes, t, now)?;
                    t.stage = WireState::Established;
                    t.expires = now.saturating_add(super::SESSION_LEASE_TICKS);
                }
                queue(t, ack, now);
            }
            ACK | SESSION_ACK => {
                if body != t.digest
                    || !matches!(
                        (kind, t.stage),
                        (ACK, WireState::AwaitAck) | (SESSION_ACK, WireState::SessionAck)
                    )
                {
                    return Err(NodeError::ReplayDetected);
                }
                t.pending = None;
                if kind == ACK {
                    verification_ready(nodes, t)?;
                } else {
                    t.stage = WireState::Established;
                    t.expires = now.saturating_add(super::SESSION_LEASE_TICKS);
                }
            }
            CONFIRM => {
                check_pending(nodes, t, now)?;
                if body != t.digest
                    || !matches!(
                        t.stage,
                        WireState::PendingVerification
                            | WireState::LocallyConfirmed
                            | WireState::RemotelyConfirmed
                            | WireState::Confirmed
                    )
                {
                    return Err(NodeError::VerificationMismatch);
                }
                t.remote_confirmed = true;
                // Authenticated consent proves peer liveness even if the local
                // durable write fails. Keep the transaction retryable within its
                // original lease; this does not grant trust before persistence.
                if let Some(live) = nodes.discovered.iter_mut().flatten().find(|p| p.id == peer.id) {
                    live.last_seen = now;
                }
                commit_confirmation(nodes, t, now, self.persist_pairing)?;
            }
            CANCEL => {
                if body.len() != 32 {
                    return Err(NodeError::InvalidAdvertisement);
                }
                terminate(nodes, t, WireState::Cancelled, now);
                nodes.record(0xda02, peer.id, now, correlation(id), 0);
            }
            _ => return Err(NodeError::UnsupportedVersion),
        }
        if let Some(peer) = nodes
            .discovered
            .iter_mut()
            .flatten()
            .find(|p| p.id == peer.id)
        {
            peer.last_seen = now;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: encrypted
    // DESC: Authenticates session identity, sequence, direction and payload before exposing data or processing close.
    // ------------------=
    fn encrypted(
        &mut self,
        nodes: &mut NodeRuntime,
        peer: NodeId,
        id: [u8; 32],
        kind: u8,
        bytes: &[u8],
        now: u64,
    ) -> Result<(), NodeError> {
        if bytes.len() < 154 {
            return Err(NodeError::InvalidAdvertisement);
        }
        let length = u16::from_le_bytes([bytes[136], bytes[137]]) as usize;
        if length > MAX_DATA || bytes.len() != 154 + length || (kind == CLOSE && length != 0) {
            return Err(NodeError::InvalidAdvertisement);
        }
        let t = self
            .transactions
            .iter_mut()
            .flatten()
            .find(|t| t.id == id && t.link.peer == Some(peer) && t.stage == WireState::Established)
            .ok_or(NodeError::SessionNotFound)?;
        let handle = t.session.ok_or(NodeError::SessionNotFound)?;
        let session = nodes
            .sessions
            .iter()
            .flatten()
            .find(|s| s.id == handle)
            .ok_or(NodeError::SessionNotFound)?;
        if bytes[112..128] != session.protocol_reference {
            return Err(NodeError::SessionNotFound);
        }
        let reference = session.protocol_reference;
        let sequence = u64::from_le_bytes(
            bytes[128..136]
                .try_into()
                .map_err(|_| NodeError::InvalidAdvertisement)?,
        );
        let mut payload = [0; MAX_DATA];
        payload[..length].copy_from_slice(&bytes[138..138 + length]);
        let tag: [u8; 16] = bytes[138 + length..]
            .try_into()
            .map_err(|_| NodeError::InvalidAdvertisement)?;
        if kind == DATA && self.received.iter().all(Option::is_some) {
            return Err(NodeError::ResourceLimit);
        }
        nodes.unprotect(
            handle,
            sequence,
            &bytes[..138],
            &mut payload[..length],
            &tag,
            now,
        )?;
        if kind == CLOSE {
            nodes.close_session(handle, now, correlation(id))?;
            nodes.record(0xda06, peer, now, correlation(id), 0);
            t.stage = WireState::Closed;
            t.pending = None;
        } else {
            let slot = self
                .received
                .iter()
                .position(Option::is_none)
                .ok_or(NodeError::ResourceLimit)?;
            self.received[slot] = Some(ReceivedData {
                peer,
                reference,
                length,
                bytes: payload,
            });
            t.pending = None;
        }
        if let Some(peer) = nodes.discovered.iter_mut().flatten().find(|p| p.id == peer) {
            peer.last_seen = now;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: retire
    // DESC: Keeps bounded terminal transaction identifiers without evicting replay protection during this boot.
    // ------------------=
    fn retire(&mut self, id: [u8; 32]) -> Result<(), NodeError> {
        if self.retired_count == HISTORY {
            return Err(NodeError::ResourceLimit);
        }
        self.retired[self.retired_count] = id;
        self.retired_count += 1;
        Ok(())
    }
}

// ------------------------=
// FUNC: accepts
// DESC: Identifies the phase-9A binary envelope without interpreting human text.
// ------------------=
pub fn accepts(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && &bytes[..8] == MAGIC
}
// ------------------------=
// FUNC: hash
// DESC: Hashes a fixed ordered field sequence with SHA-256.
// ------------------=
fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for part in parts {
        h.update(part);
    }
    h.finalize().into()
}
// ------------------------=
// FUNC: correlation
// DESC: Derives an audit correlation from the public transaction identity.
// ------------------=
fn correlation(id: [u8; 32]) -> u64 {
    u64::from_le_bytes(id[..8].try_into().unwrap())
}
// ------------------------=
// FUNC: verification_code
// DESC: Computes the public short authentication string independently from the shared authenticated digest.
// ------------------=
fn verification_code(digest: [u8; 32]) -> u32 {
    u32::from_le_bytes(digest[..4].try_into().unwrap()) % 1_000_000
}
// ------------------------=
// FUNC: checked_peer
// DESC: Revalidates current peer identity, reachability, compatibility and block/revoke state.
// ------------------=
fn checked_peer(nodes: &NodeRuntime, id: NodeId, now: u64) -> Result<NodeDescriptor, NodeError> {
    let peer = nodes
        .discovered
        .iter()
        .flatten()
        .find(|p| p.id == id)
        .copied()
        .ok_or(NodeError::UnknownNode)?;
    if matches!(peer.trust, TrustState::Blocked | TrustState::Revoked) {
        return Err(NodeError::Blocked);
    }
    if now.saturating_sub(peer.last_seen) > super::DISCOVERY_LEASE_TICKS {
        return Err(NodeError::SessionExpired);
    }
    if peer.protocol_min > 1 || peer.protocol_max < 1 {
        return Err(NodeError::UnsupportedVersion);
    }
    Ok(peer)
}
// ------------------------=
// FUNC: header
// DESC: Encodes a canonical architecture-neutral source, destination and transaction envelope.
// ------------------=
fn header(local: NodeId, peer: NodeId, id: [u8; 32], kind: u8) -> Packet {
    let mut p = Packet {
        bytes: [0; 512],
        length: HEADER as u16,
    };
    p.bytes[..8].copy_from_slice(MAGIC);
    p.bytes[8] = kind;
    p.bytes[9] = 1;
    p.bytes[16..48].copy_from_slice(&local.0);
    p.bytes[48..80].copy_from_slice(&peer.0);
    p.bytes[80..112].copy_from_slice(&id);
    p
}
// ------------------------=
// FUNC: signed
// DESC: Signs the complete bounded envelope and body using the protected long-term identity credential.
// ------------------=
fn signed(
    nodes: &NodeRuntime,
    kind: u8,
    peer: NodeId,
    id: [u8; 32],
    body: &[u8],
) -> Result<Packet, NodeError> {
    if HEADER + body.len() + 64 > 512 {
        return Err(NodeError::ResourceLimit);
    }
    let mut p = header(
        nodes.local_id().ok_or(NodeError::EntropyUnavailable)?,
        peer,
        id,
        kind,
    );
    let end = HEADER + body.len();
    p.bytes[HEADER..end].copy_from_slice(body);
    let signature = nodes
        .crypto
        .sign(
            nodes.key_ref.ok_or(NodeError::EntropyUnavailable)?,
            &p.bytes[..end],
        )
        .map_err(map_crypto_error)?;
    p.bytes[end..end + 64].copy_from_slice(&signature);
    p.length = (end + 64) as u16;
    Ok(p)
}
// ------------------------=
// FUNC: offer
// DESC: Creates fresh agreement and nonce fields bound to identity, reachable endpoint, requested scope and prior pairing.
// ------------------=
fn offer(
    nodes: &NodeRuntime,
    link: LinkSnapshot,
    scope: u64,
    paired: [u8; 32],
    entropy: [u8; 32],
) -> Result<([u8; OFFER], [u8; 32]), NodeError> {
    let (secret, public) = NodeCrypto::agreement_keypair(&entropy);
    let mut out = [0; OFFER];
    out[..32].copy_from_slice(&nodes.crypto.public_identity().map_err(map_crypto_error)?);
    out[32..64].copy_from_slice(&public);
    out[64..96].copy_from_slice(&hash(&[b"InfinityOS agreement nonce v1", &entropy]));
    out[96..115].copy_from_slice(&endpoint_bytes(link.local));
    out[115..123].copy_from_slice(&scope.to_le_bytes());
    out[123..].copy_from_slice(&paired);
    Ok((out, secret))
}
// ------------------------=
// FUNC: validate_offer
// DESC: Prevents signed negotiation from changing the discovered peer key or connected endpoint.
// ------------------=
fn validate_offer(peer: &NodeDescriptor, link: LinkSnapshot, body: &[u8]) -> Result<(), NodeError> {
    if body.len() != OFFER
        || body[..32] != peer.public_key
        || body[96..115] != endpoint_bytes(link.remote)
    {
        return Err(NodeError::IdentityMismatch);
    }
    Ok(())
}
// ------------------------=
// FUNC: transcript
// DESC: Canonically binds ordered identities and both complete offers, including ephemerals, nonces, endpoints and scope.
// ------------------=
fn transcript(
    local: NodeId,
    peer: NodeId,
    id: [u8; 32],
    kind: u8,
    a: &[u8; OFFER],
    b: &[u8; OFFER],
) -> [u8; 32] {
    let (lo, hi, first, second) = if local.0 < peer.0 {
        (local, peer, a, b)
    } else {
        (peer, local, b, a)
    };
    hash(&[
        b"InfinityOS authenticated wire transcript v1",
        &[kind, 1],
        &id,
        &lo.0,
        &hi.0,
        first,
        second,
    ])
}
// ------------------------=
// FUNC: validate_agreement
// DESC: Rejects non-contributory agreement before verification; transient derived material is immediately zeroized.
// ------------------=
fn validate_agreement(nodes: &NodeRuntime, t: &Transaction) -> Result<(), NodeError> {
    let public: [u8; 32] = t.remote_offer[32..64]
        .try_into()
        .map_err(|_| NodeError::InvalidAdvertisement)?;
    let (mut tx, mut rx, _) = NodeCrypto::derive_duplex_keys(
        &t.secret,
        &public,
        &nodes.local_id().unwrap().0,
        &t.link.peer.unwrap().0,
        &t.digest,
    )
    .map_err(map_crypto_error)?;
    tx.zeroize();
    rx.zeroize();
    Ok(())
}
// ------------------------=
// FUNC: verification_ready
// DESC: Publishes independent verification only after mutual identity proofs; pairing ephemeral secrets are discarded.
// ------------------=
fn verification_ready(nodes: &mut NodeRuntime, t: &mut Transaction) -> Result<(), NodeError> {
    let p = nodes
        .pairings
        .iter_mut()
        .flatten()
        .find(|p| p.id == t.pairing && p.state == PairingState::AwaitingConfirmation)
        .ok_or(NodeError::PairingNotFound)?;
    p.verification_code = verification_code(t.digest);
    t.secret.zeroize();
    t.stage = WireState::PendingVerification;
    Ok(())
}
// ------------------------=
// FUNC: check_pending
// DESC: Rechecks terminal, expiry and superseding trust state before an operator decision or remote confirmation.
// ------------------=
fn check_pending(nodes: &NodeRuntime, t: &Transaction, now: u64) -> Result<(), NodeError> {
    if now >= t.expires {
        return Err(NodeError::PairingExpired);
    }
    checked_peer(nodes, t.link.peer.unwrap(), now)?;
    if matches!(
        t.stage,
        WireState::Cancelled | WireState::Expired | WireState::Failed | WireState::Closed
    ) {
        return Err(NodeError::UnsupportedState);
    }
    Ok(())
}
// ------------------------=
// FUNC: commit_confirmation
// DESC: Commits trust only when both authenticated remote consent and explicit local consent exist.
// ------------------=
fn commit_confirmation(
    nodes: &mut NodeRuntime,
    t: &mut Transaction,
    now: u64,
    persist: fn(&[u8; NODE_STATE_BYTES]) -> bool,
) -> Result<(), NodeError> {
    if t.local_confirmed && t.remote_confirmed {
        if t.stage != WireState::Confirmed {
            nodes.commit_wire_pairing(
                t.pairing,
                verification_code(t.digest),
                t.digest,
                now,
                correlation(t.id),
                persist,
            )?;
        }
        t.paired = t.digest;
        t.stage = WireState::Confirmed;
    } else {
        t.stage = if t.local_confirmed {
            WireState::LocallyConfirmed
        } else {
            WireState::RemotelyConfirmed
        };
    }
    Ok(())
}

// ------------------------=
// FUNC: reject_unconfigured_persistence
// DESC: Fails closed until the host service explicitly supplies its durable pairing writer.
// ------------------=
fn reject_unconfigured_persistence(_: &[u8; NODE_STATE_BYTES]) -> bool { false }
// ------------------------=
// FUNC: queue
// DESC: Replaces one bounded retransmission slot with a new authenticated protocol step.
// ------------------=
fn queue(t: &mut Transaction, packet: Packet, now: u64) {
    t.pending = Some(packet);
    t.next_send = now;
    t.attempts = 0;
}
// ------------------------=
// FUNC: open
// DESC: Calls the existing duplex session mechanism only with authenticated transcript and current committed trust.
// ------------------=
fn open(nodes: &mut NodeRuntime, t: &mut Transaction, now: u64) -> Result<(), NodeError> {
    if t.paired == [0; 32] || !t.local_confirmed || !t.remote_confirmed {
        return Err(NodeError::NotTrusted);
    }
    let public: [u8; 32] = t.remote_offer[32..64]
        .try_into()
        .map_err(|_| NodeError::InvalidAdvertisement)?;
    let handle = nodes.open_session(
        t.link.peer.unwrap(),
        &t.secret,
        &public,
        &t.digest,
        now,
        correlation(t.id),
    )?;
    t.secret.zeroize();
    t.session = Some(handle);
    Ok(())
}
// ------------------------=
// FUNC: terminate
// DESC: Clears pending trust and ephemeral material and zeroizes any established traffic keys on terminal failure.
// ------------------=
#[track_caller]
fn terminate(nodes: &mut NodeRuntime, t: &mut Transaction, state: WireState, now: u64) {
    t.ended_at = now;
    t.end_site = core::panic::Location::caller().line();
    t.secret.zeroize();
    t.pending = None;
    if let Some(handle) = t.session {
        let _ = nodes.close_session(handle, now, correlation(t.id));
    } else {
        let _ = nodes.cancel_pairing(t.pairing);
    }
    t.stage = state;
    if matches!(state, WireState::Failed | WireState::Expired) {
        nodes.record(
            0xda04,
            t.link.peer.unwrap_or_default(),
            now,
            correlation(t.id),
            state as u8,
        );
    }
}

impl Drop for Transaction {
    // ------------------------=
    // FUNC: drop
    // DESC: Wipes any outstanding ephemeral agreement secret when a bounded transaction slot is replaced or destroyed.
    // ------------------=
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}
impl Drop for WireTrust {
    // ------------------------=
    // FUNC: drop
    // DESC: Wipes boot-scoped derivation entropy when this service instance is destroyed.
    // ------------------=
    fn drop(&mut self) {
        if let Some(seed) = self.seed.as_mut() {
            seed.zeroize();
        }
    }
}
