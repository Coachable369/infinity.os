//! Read-only privileged-debugger snapshot of actual installed UI/service state.
//! There is no command channel, authority grant, or test-only state mutation.
use super::ConsoleRuntime;

#[used]
#[no_mangle]
static mut INFINITY_DIAGNOSTIC_SNAPSHOT: [u64; 512] = [0; 512];

// ------------------------=
// FUNC: words
// DESC: Packs public binary identifiers into fixed little-endian diagnostic slots.
// ------------------=
fn words(target: &mut [u64], bytes: &[u8]) {
    for (slot, chunk) in target.iter_mut().zip(bytes.chunks_exact(8)) {
        *slot = u64::from_le_bytes(chunk.try_into().unwrap());
    }
}

// ------------------------=
// FUNC: publish
// DESC: Publishes a bounded once-per-clock snapshot for a debugger already authorized to read kernel memory; no credentials or traffic keys are copied.
// ------------------=
pub(super) fn publish(console: &ConsoleRuntime) {
    let mut data = [0u64; 512];
    data[0] = 0x494e464449414731; data[1] = 1;
    data[3] = (!console.system.live_profile) as u64;
    data[4] = console.mode as u64; data[5] = console.installer_step as u64;
    data[6] = console.installer_focus as u64; data[7] = console.system_step as u64;
    data[8] = console.system_focus as u64;
    data[9] = (!console.current_user.is_zero()) as u64 | ((!console.current_session.is_zero()) as u64) << 1 | (console.settings_editing as u64) << 2 | (console.onboarding_validation_error as u64) << 3;
    data[11] = console.system.framebuffer_width as u64; data[12] = console.system.framebuffer_height as u64;
    data[10] = crate::runtime::node_client::clock().unwrap_or(u64::MAX);
    crate::runtime::with_runtime(|runtime| {
        if let Some(local) = runtime.nodes.local_id() { words(&mut data[16..20], &local.0); }
        data[20] = runtime.nodes.control_version(); data[21] = runtime.node_projection.version;
        data[22] = runtime.node_projection.stale as u64; data[23] = runtime.node_projection.gaps;
        data[24] = runtime.nodes.discovered_nodes().iter().flatten().count() as u64;
        data[25] = runtime.nodes.discovered_nodes().iter().flatten().filter(|node| matches!(node.trust, crate::runtime::node::types::TrustState::Trusted | crate::runtime::node::types::TrustState::Restricted)).count() as u64;
        data[26] = runtime.nodes.sessions().iter().flatten().filter(|session| session.state == crate::runtime::node::types::SessionState::Established).count() as u64;
        data[29] = runtime.nodes.configured_links().iter().flatten().count() as u64;
        data[30] = runtime.nodes.audit_records().iter().flatten().count() as u64;
        if let Some(peer) = console.selected_node_id {
            words(&mut data[32..36], &peer.0);
            if let Some(verification) = runtime.nodes.local_id().and_then(|local| runtime.node_transport.trust.verification(local, peer)) {
                data[36] = verification.pairing; data[37] = verification.code as u64;
                data[38] = verification.expires; data[39] = verification.state as u64;
                words(&mut data[40..44], &verification.fingerprint); words(&mut data[44..48], &verification.transaction);
            }
        }
        data[48] = runtime.network.connections.count() as u64;
        data[49] = runtime.capabilities.count() as u64;
        for (index, error) in runtime.node_links.errors.iter().enumerate() {
            data[50 + index] = error.map(|value| value as u64 + 1).unwrap_or(0);
        }
        for (index, node) in runtime.node_projection.nodes.iter().take(runtime.node_projection.node_count).enumerate() { words(&mut data[128 + index * 16..144 + index * 16], node); }
        for (index, domain) in runtime.node_projection.domains.iter().take(runtime.node_projection.domain_count).enumerate() { words(&mut data[384 + index * 13..397 + index * 13], domain); data[27] += domain[80] as u64; data[28] += (u64::from_le_bytes(domain[72..80].try_into().unwrap()) != 0) as u64; }
    });
    unsafe {
        let pointer = (&raw mut INFINITY_DIAGNOSTIC_SNAPSHOT).cast::<u64>();
        let generation = core::ptr::read_volatile(pointer.add(2)).wrapping_add(2) & !1;
        core::ptr::write_volatile(pointer.add(2), generation | 1);
        data[2] = generation; data[511] = generation;
        for index in 0..512 { if index != 2 { core::ptr::write_volatile(pointer.add(index), data[index]); } }
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        core::ptr::write_volatile(pointer.add(2), generation);
    }
}
