//! Human rendering only. Authority, state and mutations belong to native IOP.
use super::*;
use crate::runtime::{console_language::OperationNode, iop::storage_protocol::{Operation, StorageOperationV1}};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: execute_pool_node
    // DESC: Converts validated Console arguments into the shared native storage request; output text is never parsed as state or fed to another service.
    // ------------------=
    pub(super) fn execute_pool_node(&mut self, node: &OperationNode<'_>) -> bool {
        if node_argument(node,b"durable")==Some(b"true".as_slice()) {
            self.output.write_line(b"DURABLE POOL APPROVAL: exact configured peers and operations, survives reboot until revoked.");
        }
        if node.schema.action==b"repair" {
            if node_argument(node,b"confirm")!=Some(b"true".as_slice()){self.output.write_line(b"Explicit repair confirmation is required.");return true;}
            let Some(request)=pool_request(node)else{return false};let Some(peer)=node_argument(node,b"peer").and_then(|p|parse_node_id(p.strip_prefix(b"node:").unwrap_or(p)))else{return false};
            let Ok(input)=crate::runtime::node_client::begin_capability_input(self.current_user,self.current_session)else{return true;};
            let result=crate::runtime::storage_metadata_repair::begin(self.current_user,self.current_session,request.object,peer);
            crate::runtime::with_runtime(|r|{let _=r.ui.trusted.release_secure_input(input);});
            match result{Ok(id)=>self.output.write_number(b"Authorized repair pending: ",id),Err(e)=>self.output.write_number(b"Repair denied: ",e as u64)}return true;
        }
        if node.schema.action==b"metadata-authority"||node.schema.action==b"share" {
            if node_argument(node,b"confirm")!=Some(b"true".as_slice()){self.output.write_line(b"Explicit metadata delegation confirmation is required.");return true;}
            let Ok(input)=crate::runtime::node_client::begin_capability_input(self.current_user,self.current_session)else{return true;};
            let result=if node.schema.action==b"share" {
                let durable=node_argument(node,b"durable")==Some(b"true".as_slice());
                if durable {self.output.write_line(b"Until revoked: approved readers and repair writers retain this shared object delegation across reboot.");}
                pool_request(node).and_then(|p|node_argument(node,b"path").map(|path|(p.object,path))).ok_or(crate::runtime::iop::remote::RemoteError::MalformedRequest).and_then(|(object,path)|if durable {crate::runtime::storage_metadata::share_durable(self.current_user,self.current_session,object,path)}else{crate::runtime::storage_metadata::share(self.current_user,self.current_session,object,path)})
            }else{
                let peer=node_argument(node,b"peer").and_then(|p|parse_node_id(p.strip_prefix(b"node:").unwrap_or(p)));
                let grant=node_argument(node,b"grant").and_then(parse_u64_decimal);let lease=pool_approval_lease(node);
                match(peer,grant,lease){(Some(peer),Some(grant),Some(lease))=>crate::runtime::storage_metadata::configure(self.current_user,self.current_session,peer,grant,lease).map(|_|0),_=>Err(crate::runtime::iop::remote::RemoteError::MalformedRequest)}
            };
            crate::runtime::with_runtime(|r|{let _=r.ui.trusted.release_secure_input(input);});
            match result{Ok(id)=>self.output.write_number(b"Metadata operation admitted: ",id),Err(e)=>self.output.write_number(b"Metadata operation denied: ",e as u64)}return true;
        }
        if node.schema.action==b"retire-authority"{
            if node_argument(node,b"confirm")!=Some(b"true".as_slice()){self.output.write_line(b"Explicit confirmation is required to permit recipient replica retirement.");return true;}
            let peer=node_argument(node,b"peer").and_then(|p|parse_node_id(p.strip_prefix(b"node:").unwrap_or(p)));
            let grant=node_argument(node,b"grant").and_then(parse_u64_decimal);let lease=pool_approval_lease(node);
            let (Some(peer),Some(grant),Some(lease))=(peer,grant,lease)else{return false;};
            let Ok(input)=crate::runtime::node_client::begin_capability_input(self.current_user,self.current_session)else{return true;};
            let result=crate::runtime::storage_coordinator::retire_authority(self.current_user,self.current_session,peer,grant,lease);
            crate::runtime::with_runtime(|r|{let _=r.ui.trusted.release_secure_input(input);});
            match result{Ok(())=>self.output.write_line(b"Recipient retirement approval persisted."),Err(e)=>self.output.write_number(b"Retirement authority rejected: ",e as u64)}return true;
        }
        if node.schema.action==b"fixture"{
            if node_argument(node,b"confirm")!=Some(b"true".as_slice()){self.output.write_line(b"This creates synthetic QA content. Explicit confirm=true is required.");return true;}
            let Some(length)=node_argument(node,b"length").and_then(parse_u64_decimal).and_then(|n|u32::try_from(n).ok())else{return false;};
            let Some(seed)=node_argument(node,b"seed").and_then(parse_u64_decimal)else{return false;};
            let policy=match node_argument(node,b"policy"){Some(b"temporary")=>1,Some(b"protected")=>2,Some(b"critical")=>3,_=>return false};
            match crate::runtime::storage_fixture::start(self.current_user,self.current_session,length,seed,policy){
                Ok(())=>self.output.write_line(b"Synthetic QA upload scheduled through the native Pool service."),
                Err(error)=>self.output.write_number(b"QA upload rejected: ",error as u64)}return true;
        }
        if node.schema.action == b"participate" {
            if node_argument(node,b"confirm")!=Some(b"true".as_slice()) {
                self.output.write_line(b"No participation changed. This enables automatic replica transfers under the named peer grants for the stated lease. Use confirm=true to approve.");return true;
            }
            let peer=node_argument(node,b"peer").and_then(|p|parse_node_id(p.strip_prefix(b"node:").unwrap_or(p)));
            let lease=pool_approval_lease(node);
            let mut grants=[0;5];
            for (i,name) in [b"begin".as_slice(),b"chunk",b"commit",b"inspect",b"read"].iter().enumerate(){
                let Some(value)=node_argument(node,name).and_then(parse_u64_decimal)else{return false;};grants[i]=value;
            }
            let (Some(peer),Some(duration))=(peer,lease)else{return false;};
            let Ok(input)=crate::runtime::node_client::begin_capability_input(self.current_user,self.current_session)else{
                self.output.write_line(b"Trusted participation consent is unavailable.");return true;
            };
            let result=crate::runtime::storage_coordinator::participate(self.current_user,self.current_session,peer,grants,duration);
            crate::runtime::with_runtime(|r|{let _=r.ui.trusted.release_secure_input(input);});
            match result{Ok(())=>self.output.write_line(b"Pool participation committed. Automatic replication uses the approved peer grants and lease."),
                Err(error)=>self.output.write_number(b"Participation rejected: ",error as u64)}
            return true;
        }
        if node.schema.operation == crate::runtime::iop::OperationId::ResourceAdvertise {
            let peer = node_argument(node, b"peer").and_then(|p| parse_node_id(p.strip_prefix(b"node:").unwrap_or(p)));
            let grant = node_argument(node, b"grant").and_then(parse_u64_decimal);
            let (Some(peer), Some(grant)) = (peer, grant) else { return false; };
            let durable=node_argument(node,b"durable")==Some(b"true".as_slice());
            if durable && node_argument(node,b"confirm")!=Some(b"true".as_slice()){self.output.write_line(b"Publication until revoked survives reboot; confirm=true is required.");return true;}
            let result=if durable {crate::runtime::storage_coordinator::advertise_durable(self.current_user,self.current_session,peer,grant)}else{crate::runtime::storage_coordinator::advertise(self.current_user,self.current_session,peer,grant)};
            match result {
                Ok(()) => if durable {self.output.write_line(b"Resource publication approved until revoked; every send requires the exact peer authority and fresh trusted session.");}else{self.output.write_line(b"Resource publication approval persisted for up to one hour. Every renewal requires the peer grant and trusted session.");},
                Err(error) => self.output.write_number(b"Resource publication rejected: ", error as u64),
            }
            return true;
        }
        if node.schema.action == b"result" {
            let Some(id) = node_argument(node, b"request").and_then(parse_u64_decimal) else {
                self.output.write_line(b"An exact request identifier is required."); return true;
            };
            if id>>62==3{match crate::runtime::storage_metadata::take(self.current_user,self.current_session,id){Ok(Some(p))=>self.render_pool_response(p),Ok(None)=>self.output.write_line(b"Metadata quorum operation pending."),Err(e)=>self.output.write_number(b"Metadata operation failed: ",e as u64)}return true;}
            if id&(1u64<<61)!=0 {match crate::runtime::storage_metadata_repair::take(self.current_user,self.current_session,id){Ok(Some(generation))=>self.output.write_number(b"Repair generation published: ",generation),Ok(None)=>self.output.write_line(b"Repair pending."),Err(e)=>self.output.write_number(b"Repair failed: ",e as u64)}return true;}
            if id&(1u64<<63)!=0{
                match crate::runtime::storage_coordinator::take_read(self.current_user,self.current_session,id){
                    Ok(Some(response))=>self.render_pool_response(response),Ok(None)=>self.output.write_line(b"Object read pending."),
                    Err(error)=>self.output.write_number(b"Object read failed: ",error as u64)}return true;
            }
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
if matches!(request.operation,Operation::ObjectRead|Operation::ObjectInspect|Operation::PoolInspect)&&(crate::runtime::storage_metadata::bound(request.object)||crate::runtime::storage_metadata::warming()){
            let mut request=request;request.value&=!crate::runtime::storage_coordinator::REMOTE_VERIFIED;
            match crate::runtime::storage_metadata::read(self.current_user,self.current_session,request){Ok(id)=>self.output.write_number(b"Shared object quorum read pending: ",id),Err(e)=>self.output.write_number(b"Shared read denied: ",e as u64)}return true;
        }
        if matches!(request.operation,Operation::ObjectUpdate|Operation::ObjectSetPolicy|Operation::ObjectDelete|Operation::ObjectCopy)&&crate::runtime::storage_metadata::bound(request.object){
            match crate::runtime::storage_metadata::mutate(self.current_user,self.current_session,request){Ok(id)=>self.output.write_number(b"Shared mutation quorum pending: ",id),Err(e)=>self.output.write_number(b"Shared mutation denied: ",e as u64)}return true;
        }
        if request.operation == Operation::ObjectRead && request.value & crate::runtime::storage_coordinator::REMOTE_VERIFIED != 0 {
            if node_argument(node, b"peer").is_some() || node_argument(node, b"grant").is_some() {
                self.output.write_line(b"Remote source selection is owned by the Pool coordinator; omit peer and grant.");
                return true;
            }
            match crate::runtime::storage_coordinator::submit_read(self.current_user, self.current_session, request) {
                Ok(id) => self.output.write_number(b"Object read pending: ", id),
                Err(error) => self.output.write_number(b"Object read admission failed: ", error as u64),
            }
            return true;
        }
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
            Err(_) if request.operation==Operation::ObjectRead=>{
                match crate::runtime::storage_coordinator::submit_read(self.current_user,self.current_session,request){
                    Ok(id)=>self.output.write_number(b"Object read pending: ",id),
                    Err(error)=>self.output.write_number(b"Object read admission failed: ",error as u64)}
            },
            Err(_) => self.output.write_line(b"Pool operation rejected or unavailable. Refresh the version and generation before retrying."),
        }
        true
    }

    // ------------------------=
    // FUNC: render_pool_response
    // DESC: Renders the same typed native completion for local and authenticated remote Pool operations without a second storage database.
    // ------------------=
    fn render_pool_response(&mut self, response: StorageOperationV1) {
                if matches!(response.operation,Operation::PoolUploadBegin|Operation::PoolUploadAppend|Operation::PoolUploadCommit|Operation::PoolUploadAbort){
                    if response.length>=16{self.output.write_hex(b"Upload / committed ObjectId: ",&response.data[..16]);}
                    self.output.write_number(b"Committed: ",response.value);
                    self.output.write_number(b"Completed bytes: ",response.offset);
                }
                if matches!(response.operation, Operation::ObjectCopy | Operation::ObjectCreate | Operation::ObjectUpdate | Operation::ObjectSetPolicy)
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
                if response.operation == Operation::PoolInspect {
                    if response.object != [0;16] && response.length == 64 {
                        self.output.write_hex(b"ObjectId: ", &response.object);
                        self.output.write_line(if response.data[49] >= response.data[48] {
                            b"Protection: HEALTHY"
                        } else if response.data[49] == 0 {
                            b"Protection: OFFLINE"
                        } else { b"Protection: DEGRADED" });
                        self.output.write_number(b"Desired replicas: ", response.data[48] as u64);
                        self.output.write_number(b"Observed verified: ", response.data[49] as u64);
                        self.output.write_number(b"Observed offline: ", response.data[50] as u64);
                        self.output.write_number(b"Stale replicas: ", response.data[51] as u64);
                        self.output.write_number(b"Corrupt replicas: ", response.data[52] as u64);
                        self.output.write_number(b"Under-protected: ", response.data[53] as u64);
                    } else { self.output.write_number(b"Owned objects: ", response.value); }
                }
    }
}

// ------------------------=
// FUNC: pool_approval_lease
// DESC: Requires explicit durable consent without a contradictory finite lease; the sentinel represents until revoked, never an uptime deadline.
// ------------------=
fn pool_approval_lease(node:&OperationNode<'_>)->Option<u64>{
    if node_argument(node,b"durable")==Some(b"true".as_slice()){
        if node_argument(node,b"lease").is_some()||node_argument(node,b"confirm")!=Some(b"true".as_slice()){None}else{Some(u64::MAX)}
    }else{node_argument(node,b"lease").and_then(parse_u64_decimal)}
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
        if value.starts_with(b"/") {p.object=crate::runtime::storage_metadata::lookup(value)?;}else{
        if value.len() != 32 { return None; }
        for (index, pair) in value.chunks_exact(2).enumerate() {
            p.object[index] = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
        }
        if p.object == [0; 16] { return None; }
        }
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
    if operation == Operation::PoolInspect && p.object != [0;16] { p.value = 1; }
    if operation == Operation::ObjectRead {
        p.value = node_argument(node, b"length").and_then(parse_u64_decimal).filter(|n| (1..=64).contains(n))?;
        match node_argument(node, b"source") {
            None | Some(b"local") => {},
            Some(b"remote") => p.value |= crate::runtime::storage_coordinator::REMOTE_VERIFIED,
            _ => return None,
        }
    }
    if operation==Operation::ObjectDelete && node_argument(node,b"confirm")!=Some(b"true".as_slice()){return None;}
    if operation==Operation::PoolUploadBegin{
        let length=u32::try_from(node_argument(node,b"length").and_then(parse_u64_decimal)?).ok()?;
        p.data[..4].copy_from_slice(&length.to_le_bytes());
        let hash=node_argument(node,b"hash")?;if hash.len()!=64{return None;}
        for(i,pair)in hash.chunks_exact(2).enumerate(){p.data[4+i]=(hex_digit(pair[0])?<<4)|hex_digit(pair[1])?;}
        p.data[36]=match node_argument(node,b"policy")?{b"temporary"=>1,b"protected"=>2,b"critical"=>3,_=>return None};
        p.data[37..45].copy_from_slice(&node_argument(node,b"nonce").and_then(parse_u64_decimal)?.to_le_bytes());p.length=45;
    }
    if operation==Operation::PoolUploadAppend{
        let offset=u32::try_from(p.offset).ok()?;p.offset=0;p.data[..4].copy_from_slice(&offset.to_le_bytes());
        let hex=node_argument(node,b"hex")?;if hex.is_empty()||hex.len()>120||hex.len()%2!=0{return None;}
        for(i,pair)in hex.chunks_exact(2).enumerate(){p.data[4+i]=(hex_digit(pair[0])?<<4)|hex_digit(pair[1])?;}
        p.length=(4+hex.len()/2)as u16;
    }
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
