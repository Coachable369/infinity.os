//! Human rendering only. Authority, state and mutations belong to native IOP.
use super::*;
use crate::runtime::{console_language::OperationNode, iop::storage_protocol::{Operation, StorageOperationV1}};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: execute_pool_node
    // DESC: Converts validated Console arguments into the shared native storage request; output text is never parsed as state or fed to another service.
    // ------------------=
    pub(super) fn execute_pool_node(&mut self, node: &OperationNode<'_>) -> bool {
        if node.schema.operation == crate::runtime::iop::OperationId::ResourceAdvertise {
            let peer = node_argument(node, b"peer").and_then(|p| parse_node_id(p.strip_prefix(b"node:").unwrap_or(p)));
            let grant = node_argument(node, b"grant").and_then(parse_u64_decimal);
            let (Some(peer), Some(grant)) = (peer, grant) else { return false; };
            match crate::runtime::storage_advertiser::start(self.current_user, self.current_session, peer, grant) {
                Ok(()) => self.output.write_line(b"Resource publication scheduled. Logout or failed authority stops renewal."),
                Err(error) => self.output.write_number(b"Resource publication rejected: ", error as u64),
            }
            return true;
        }
        if node.schema.action == b"result" {
            let Some(id) = node_argument(node, b"request").and_then(parse_u64_decimal) else {
                self.output.write_line(b"An exact request identifier is required."); return true;
            };
            match crate::runtime::storage_operator::take_result(self.current_user, self.current_session, id) {
                Ok(Some(result)) => match result.result {
                    Ok(response) => self.render_pool_response(response),
                    Err(error) => self.output.write_number(b"Remote storage error: ", error as u64),
                },
                Ok(None) => self.output.write_line(b"Storage request pending. The desktop remains available."),
                Err(error) => self.output.write_number(b"Result access error: ", error as u64),
            }
            return true;
        }
        let Some(request) = pool_request(node) else {
            self.output.write_line(b"Use a full ObjectId, bounded content, valid generation/version and an explicit policy.");
            return true;
        };
        match (node_argument(node, b"peer"), node_argument(node, b"grant")) {
            (Some(peer), Some(grant)) => {
                let (Some(peer), Some(grant)) = (parse_node_id(peer.strip_prefix(b"node:").unwrap_or(peer)), parse_u64_decimal(grant)) else {
                    self.output.write_line(b"Use a full peer NodeId and its issued grant handle."); return true;
                };
                match crate::runtime::storage_operator::submit(self.current_user, self.current_session, peer, grant, request) {
                    Ok(id) => self.output.write_number(b"Queued storage request: ", id),
                    Err(error) => self.output.write_number(b"Storage admission error: ", error as u64),
                }
                return true;
            },
            (None, None) => {},
            _ => { self.output.write_line(b"Remote storage requires both peer and grant."); return true; },
        }
        match crate::runtime::storage_client::execute(self.current_user, self.current_session, request) {
            Ok(response) => self.render_pool_response(response),
            Err(_) => self.output.write_line(b"Pool operation rejected or unavailable. Refresh the version and generation before retrying."),
        }
        true
    }

    // ------------------------=
    // FUNC: render_pool_response
    // DESC: Renders the same typed native completion for local and authenticated remote Pool operations without a second storage database.
    // ------------------=
    fn render_pool_response(&mut self, response: StorageOperationV1) {
                if matches!(response.operation, Operation::ObjectCopy | Operation::ObjectCreate | Operation::ObjectUpdate | Operation::ObjectSetPolicy | Operation::PoolInspect)
                    && response.length >= 50 {
                    self.output.write_hex(b"ObjectId: ", &response.data[..16]);
                    self.output.write_number(b"Desired independent replicas: ", response.data[48] as u64);
                    self.output.write_line(match response.data[49] { 1 => b"Protection: HEALTHY", 2 => b"Protection: DEGRADED", _ => b"Protection: OFFLINE" });
                } else if response.operation == Operation::ObjectRead {
                    self.output.write_hex(b"Verified bytes: ", &response.data[..response.length as usize]);
                } else if response.operation == Operation::ObjectInspect {
                    self.output.write_hex(b"Manifest page: ", &response.data[..response.length as usize]);
                }
                self.output.write_number(b"Version: ", response.object_version);
                self.output.write_number(b"Generation: ", response.manifest_generation);
                if response.operation == Operation::PoolInspect { self.output.write_number(b"Owned objects: ", response.value); }
    }
}

// ------------------------=
// FUNC: pool_request
// DESC: Validates all numeric and full-width identity arguments before issuing any capability-backed native request.
// ------------------=
fn pool_request(node: &OperationNode<'_>) -> Option<StorageOperationV1> {
    let operation = Operation::decode(node.schema.operation as u32).ok()?;
    let mut p = StorageOperationV1 { operation, object: [0; 16], authority_generation: 1,
        manifest_generation: 0, object_version: 0, offset: 0, scope: 0, value: 0, length: 0, data: [0; 64] };
    if let Some(target) = node.target {
        let value = target.value.strip_prefix(b"object:").unwrap_or(target.value);
        if value.len() != 32 { return None; }
        for (index, pair) in value.chunks_exact(2).enumerate() {
            p.object[index] = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
        }
        if p.object == [0; 16] { return None; }
    }
    for (name, out) in [(b"generation".as_slice(), &mut p.manifest_generation),
        (b"version".as_slice(), &mut p.object_version), (b"offset".as_slice(), &mut p.offset)] {
        if let Some(value) = node_argument(node, name) { *out = parse_u64_decimal(value)?; }
    }
    if matches!(operation, Operation::ObjectCreate | Operation::ObjectSetPolicy) {
        p.value = match node_argument(node, b"policy")? { b"temporary" => 1, b"protected" => 2, b"critical" => 3, _ => return None };
    }
    if operation == Operation::ObjectCreate { p.offset = node_argument(node, b"nonce").and_then(parse_u64_decimal).filter(|n| *n != 0)?; }
    if operation == Operation::ObjectCopy { p.value = node_argument(node, b"nonce").and_then(parse_u64_decimal).filter(|n| *n != 0)?; }
    if matches!(operation, Operation::ObjectCreate | Operation::ObjectUpdate) {
        let content = node_argument(node, b"content").unwrap_or(b"");
        if content.len() > 64 { return None; }
        p.data[..content.len()].copy_from_slice(content); p.length = content.len() as u16;
    }
    if operation == Operation::ObjectInspect { p.value = 64; }
    if operation == Operation::ObjectRead { p.value = node_argument(node, b"length").and_then(parse_u64_decimal).filter(|n| *n <= 64)?; }
    Some(p)
}

// ------------------------=
// FUNC: hex_digit
// DESC: Decodes exactly one hexadecimal nibble without accepting truncation or alternate identities.
// ------------------=
fn hex_digit(value: u8) -> Option<u8> {
    match value { b'0'..=b'9' => Some(value-b'0'), b'a'..=b'f' => Some(value-b'a'+10),
        b'A'..=b'F' => Some(value-b'A'+10), _ => None }
}
