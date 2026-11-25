//! Private native Fabric Manager bridge; all authorization remains in Fabric management.
use crate::runtime::{fabric::management, identity::StableId, console_execution::Bytes,
    native_memory::{copy_to_user, PageTable}};
use core::fmt::Write;
pub const PATH: &[u8] = b"/System/infinity-fabric.py";
pub const SOURCE: &[u8] = include_bytes!("../../runtime/install/System/infinity-fabric.py");

// ------------------------=
// FUNC: dispatch
// DESC: Exposes bounded cluster inspection and revision-checked local contribution policy to the immutable signed-in app.
// ------------------=
pub fn dispatch<T: PageTable>(allowed: bool, owner: [u8;16], session: [u8;16], tables: &mut T,
    operation: u64, args: [u64;6]) -> Option<i64> {
    if operation != 0x12f9 { return None; }
    if !allowed { return Some(-13); }
    let user = StableId(owner); let session = StableId(session);
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
