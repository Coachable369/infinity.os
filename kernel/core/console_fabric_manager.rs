//! Private native Fabric Manager bridge; all authorization remains in Fabric management.
use crate::runtime::{fabric::management, identity::StableId, console_execution::Bytes,
    native_memory::{copy_to_user, copy_from_user, PageTable}};
use core::fmt::Write;
pub const PATH: &[u8] = b"/System/infinity-fabric.py";
pub const SOURCE: &[u8] = include_bytes!("../../runtime/install/System/infinity-fabric.py");

// ------------------------=
// FUNC: dispatch
// DESC: Exposes bounded cluster inspection and revision-checked local contribution policy to the immutable signed-in app.
// ------------------=
pub fn dispatch<T: PageTable>(allowed: bool, owner: [u8;16], session: [u8;16], tables: &mut T,
    operation: u64, args: [u64;6]) -> Option<i64> {
    if operation != 0x12f9 && operation != 0x12fa { return None; }
    if !allowed { return Some(-13); }
    let user = StableId(owner); let session = StableId(session);
    if operation == 0x12fa { return Some(node_dispatch(user, session, tables, args)); }
    if args[0] == 3 {
        let Ok(epoch) = u32::try_from(args[2]) else { return Some(-22); };
        return Some(match management::cancel_job(user, session, args[1], epoch) {
            Ok(()) => 0,
            Err(crate::runtime::compute::ComputeError::AccessDenied) => -13,
            Err(crate::runtime::compute::ComputeError::StaleResult) => -11,
            Err(_) => -5,
        });
    }
    if args[0] == 1 {
        if args[2] > 1 || args[4] >> 32 != 0 { return Some(-22); }
        let limits = crate::runtime::compute::ProviderComputeLimits {
            enabled: args[2] != 0, memory_bytes: args[3], cpu_units: (args[4] & 65535) as u16,
            concurrent_tasks: (args[4] >> 16) as u16 };
        return Some(match management::update_provider(user, session, limits, args[1]) {
            Ok(_) => 0,
            Err(crate::runtime::compute::ComputeError::AccessDenied) => -13,
            Err(_) => -5,
        });
    }
    if args[0] == 2 {
        let jobs = match management::inspect_jobs(user, session) {
            Ok(value) => value, Err(_) => return Some(-13),
        };
        let mut out = Bytes::<16384>::new();
        out.write(b"[");
        let mut comma = false;
        for job in jobs.iter().flatten() {
            if comma { out.write(b","); } comma = true;
            let _ = write!(out, "{{\"id\":{},\"epoch\":{},\"state\":{},\"cpu\":{},\"memory\":{},\"ticks\":{},\"node\":\"",
                job.task_id, job.epoch, job.state as u8, job.accounting.reserved_cpu_units,
                job.accounting.reserved_memory_bytes, job.accounting.used_cpu_ticks);
            for byte in job.node.0 { let _ = write!(out, "{:02x}", byte); }
            out.write(b"\"}");
        }
        out.write(b"]");
        if out.truncated() { return Some(-28); }
        return Some(if copy_to_user(tables, args[5], out.bytes()).is_ok() { out.bytes().len() as i64 } else { -14 });
    }
    if args[0] != 0 { return Some(-22); }
    let cluster = match management::inspect_cluster(user, session) { Ok(value) => value, Err(_) => return Some(-13) };
    let mut out = Bytes::<16384>::new();
    let _ = write!(out, "{{\"observed_at\":{},\"nodes\":[", cluster.capacity.observed_at);
    let mut comma = false;
    for node in cluster.nodes.iter().flatten() {
        if comma { out.write(b","); } comma = true;
        out.write(b"{\"id\":\"");
        for byte in node.id.0 { let _ = write!(out, "{:02x}", byte); }
        let _ = write!(out, "\",\"local\":{},\"reachability\":\"{:?}\",\"trust\":\"{:?}\",\"last_seen\":{},\"resources\":[",
            node.local, node.reachability, node.trust, node.last_seen);
        for (i, category) in node.categories.iter().take(3).enumerate() {
            if i != 0 { out.write(b","); }
            let _ = write!(out, "{{\"known\":{},\"usable\":{},\"capacity\":{},\"available\":{},\"provider_reserved\":{},\"consumer_reserved\":{}}}",
                category.known_resources, category.usable_resources, category.capacity, category.available, category.provider_reserved, category.consumer_reserved);
        }
        out.write(b"]}");
    }
    out.write(b"],\"totals\":[");
    for (i, category) in cluster.capacity.categories.iter().take(3).enumerate() {
        if i != 0 { out.write(b","); }
        let available = category.local.available as u128 + category.remote.available as u128;
        let _ = write!(out, "{}", available);
    }
    out.write(b"],\"provider\":");
    if let Ok(provider) = management::inspect_provider(user, session) {
        let _ = write!(out, "{{\"enabled\":{},\"revision\":{},\"memory\":{},\"cpu\":{},\"tasks\":{},\"active\":{},\"allocated_memory\":{}}}",
            provider.limits.enabled, provider.revision, provider.limits.memory_bytes, provider.limits.cpu_units,
            provider.limits.concurrent_tasks, provider.active_tasks, provider.allocated_memory_bytes);
    } else { out.write(b"null"); }
    out.write(b"}");
    if out.truncated() { return Some(-28); }
    Some(if copy_to_user(tables, args[5], out.bytes()).is_ok() { out.bytes().len() as i64 } else { -14 })
}

// ------------------------=
// FUNC: node_dispatch
// DESC: Brokers explicit Fabric pairing decisions through the existing authenticated native node authority.
// ------------------=
fn node_dispatch<T: PageTable>(user: StableId, session: StableId, tables: &mut T, args: [u64;6]) -> i64 {
    use crate::runtime::{with_runtime, node_client, iop::{OperationId, NodeOperationV1, NODE_OPERATION_HUMAN_APPROVED}};
    if management::inspect_provider(user, session).is_err() { return -13; }
    let mut id = [0u8;32];
    if copy_from_user(tables, args[1], &mut id).is_err() { return -14; }
    let Some(now) = node_client::clock() else { return -11; };
    let peer = crate::runtime::node::types::NodeId(id);
    let prepared = with_runtime(|runtime| {
        let node = runtime.nodes.discovered_nodes().iter().flatten().find(|node| node.id == peer)?;
        let verification = runtime.node_transport.trust.verification(runtime.nodes.local_id()?, peer);
        Some((*node, verification))
    }).flatten();
    let Some((node, verification)) = prepared else { return -2; };
    if args[0] == 0 {
        let mut out = Bytes::<16384>::new();
        if let Some(v) = verification.filter(|v| now < v.expires) {
            let _ = write!(out, "{{\"handle\":{},\"code\":{},\"expires\":{},\"state\":\"{:?}\",\"fingerprint\":\"", v.pairing, v.code, v.expires, v.state);
            for byte in v.fingerprint { let _ = write!(out, "{:02x}", byte); }
            out.write(b"\"}");
        } else { out.write(b"null"); }
        return if copy_to_user(tables,args[5],out.bytes()).is_ok() { out.bytes().len() as i64 } else { -14 };
    }
    let mut request = NodeOperationV1 { node_id:id, handle:0, scope:node.policy.scope,
        lease_deadline:node.policy.expires_at, operation:0, rights:0, value:0, flags:0, schema_version:1 };
    let operation = match args[0] {
        1 => OperationId::NodePairBegin,
        2 | 3 => {
            let Some(v) = verification.filter(|v| v.pairing == args[2] && now < v.expires) else { return -11; };
            request.handle=v.pairing; request.scope=v.scope; request.lease_deadline=0;
            if args[0]==2 {
                if args[3]>999999 { return -22; }
                request.value=args[3] as u32; request.flags=NODE_OPERATION_HUMAN_APPROVED;
                OperationId::NodePairConfirm
            } else { OperationId::NodePairCancel }
        }
        4 => OperationId::NodeSessionOpen,
        5 => OperationId::NodeJoin,
        6 => {
            if ![1,2,10].contains(&args[2]) || args[3]>1 { return -22; }
            request.flags=args[2] as u32; request.value=args[3] as u32;
            OperationId::NodePolicyUpdate
        }
        _ => return -22,
    };
    request.operation=operation.machine_id();
    let lease = if args[0]==2 {
        let Ok(lease)=node_client::begin_pairing_input(user,session) else { return -13; };
        let valid=with_runtime(|runtime| runtime.ui.trusted.authorize_trusted_window(lease).is_ok()).unwrap_or(false);
        if !valid { with_runtime(|runtime| { let _=runtime.ui.trusted.release_secure_input(lease); }); return -13; }
        Some(lease)
    } else { None };
    let result=node_client::submit(user,session,operation,request);
    if let Some(lease)=lease { with_runtime(|runtime| { let _=runtime.ui.trusted.release_secure_input(lease); }); }
    match result { Ok(_)=>0, Err(crate::runtime::iop::IopError::AccessDenied)=>-13, Err(_)=>-5 }
}
