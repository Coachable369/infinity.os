//! Read-only privileged-debugger snapshot of actual installed UI/service state.
//! There is no command channel, authority grant, or test-only state mutation.
use super::ConsoleRuntime;

#[used]
#[no_mangle]
static mut INFINITY_DIAGNOSTIC_SNAPSHOT: [u64; 512] = [0; 512];
#[used]
#[no_mangle]
static mut INFINITY_POOL_DIAGNOSTIC_SNAPSHOT: [u64; 256] = [0; 256];
#[used]
#[no_mangle]
static mut INFINITY_SETTINGS_DIAGNOSTIC_SNAPSHOT: [u64; 128] = [0; 128];

// ------------------------=
// FUNC: publish_settings
// DESC: Exposes non-secret live Settings geometry and disclosure state to read-only behavioral verification.
// ------------------=
fn publish_settings(console: &ConsoleRuntime, generation: u64) {
    let mut data = [0u64; 128];
    data[0] = 0x494e465345545331;
    data[1] = 1;
    data[2] = generation;
    data[127] = generation;
    if console.mode == super::ConsoleMode::Settings {
        let layout = crate::ui::system_layout::SystemLayout::new(
            console.system.framebuffer_width, console.system.framebuffer_height);
        let state = console.settings_window;
        let section = console.system_focus;
        let window = layout.settings_window_geometry_for_section(state, section);
        data[3] = 1;
        data[4] = section as u64;
        data[5] = state.expanded_row.map(|i| i as u64 + 1).unwrap_or(0);
        data[6] = state.scroll_offset as u64;
        data[7] = window.maximum_scroll as u64;
        data[8] = state.row_count as u64;
        for (at, rect) in [(9, window.window), (13, window.viewport), (17, window.scrollbar_track), (21, window.scrollbar_thumb)] {
            data[at..at+4].copy_from_slice(&[rect.x as u64, rect.y as u64, rect.width as u64, rect.height as u64]);
        }
        for index in 0..state.row_count.min(8) {
            let row = layout.settings_row_geometry_for_section(state, index, section);
            let arrow = layout.settings_row_element_geometry(state, section, index,
                crate::ui::installer_template::InstallerTemplateRole::SettingsDisclosure).unwrap_or(row.summary);
            let at = 25 + index * 8;
            data[at..at+8].copy_from_slice(&[row.summary.x as u64, row.summary.y as u64,
                row.summary.width as u64, row.summary.height as u64, arrow.x as u64,
                arrow.y as u64, arrow.width as u64, arrow.height as u64]);
            if state.expanded_row == Some(index) {
                data[111..115].copy_from_slice(&[row.detail.x as u64, row.detail.y as u64,
                    row.detail.width as u64, row.detail.height as u64]);
            }
        }
        for index in 0..11 {
            let nav = layout.settings_section_geometry_for_section(state, index, section);
            data[89 + index * 2] = (nav.x + nav.width as i32 / 2) as u64;
            data[90 + index * 2] = (nav.y + nav.height as i32 / 2) as u64;
        }
    }
    unsafe {
        let pointer = (&raw mut INFINITY_SETTINGS_DIAGNOSTIC_SNAPSHOT).cast::<u64>();
        core::ptr::write_volatile(pointer.add(2), generation | 1);
        for index in 0..128 { if index != 2 { core::ptr::write_volatile(pointer.add(index), data[index]); } }
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        core::ptr::write_volatile(pointer.add(2), generation);
    }
}

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
// DESC: Publishes bounded clock/input-completion state for an authorized debugger; no credentials or traffic keys are copied.
// ------------------=
pub(super) fn publish(console: &ConsoleRuntime) {
    let mut data = [0u64; 512];
    let mut pool = [0u64;256]; pool[0]=0x494e46504f4f4c31;pool[1]=1;
    data[0] = 0x494e464449414731; data[1] = 1;
    data[3] = (!console.system.live_profile) as u64;
    data[4] = console.mode as u64; data[5] = console.installer_step as u64;
    data[6] = console.installer_focus as u64; data[7] = console.system_step as u64;
    data[8] = console.system_focus as u64;
    data[9] = (!console.current_user.is_zero()) as u64 | ((!console.current_session.is_zero()) as u64) << 1 | (console.settings_editing as u64) << 2 | (console.onboarding_validation_error as u64) << 3;
    data[11] = console.system.framebuffer_width as u64; data[12] = console.system.framebuffer_height as u64;
    data[13] = console.pointer_x as u64; data[14] = console.pointer_y as u64;
    data[15] = console.pointer_buttons as u64;
    if console.mode == super::ConsoleMode::Settings {
        let rect=crate::ui::system_layout::SystemLayout::new(console.system.framebuffer_width as usize,console.system.framebuffer_height as usize)
            .settings_window_geometry_for_section(console.settings_window,console.system_focus).window;
        pool[246]=rect.x as u64;pool[247]=rect.y as u64;
        pool[248]=rect.width as u64;pool[249]=rect.height as u64;
        pool[250]=console.settings_window_dragging as u64;
    }
    data[10] = crate::runtime::node_client::clock().unwrap_or(u64::MAX);
    // Non-secret editor acceptance only. Zero means unavailable; otherwise this
    // is length plus one, never buffer contents. Authentication is excluded.
    if matches!(console.mode, super::ConsoleMode::Console | super::ConsoleMode::Repair | super::ConsoleMode::AppLauncher)
        || (console.mode == super::ConsoleMode::Desktop && console.command_window.visible)
        || (console.mode == super::ConsoleMode::Settings && console.system_focus == 6 && console.settings_editing)
    {
        data[71] = console.command_length as u64 + 1;
    }
    if console.mode == super::ConsoleMode::Settings && console.system_focus == 7 && console.settings_editing {
        data[54] = console.command_length as u64;
        data[55] = console.node_input_lease.map(|lease| lease.expires_at).unwrap_or(0);
    }
    crate::runtime::with_runtime(|runtime| {
        let fixture=runtime.storage_fixture.observation;
        pool[3]=fixture.phase as u64;pool[4]=fixture.offset as u64;pool[5]=fixture.length as u64;
        words(&mut pool[6..8],&fixture.object);words(&mut pool[8..12],&fixture.hash);
        pool[12]=fixture.version;pool[13]=fixture.generation;
        pool[14]=runtime.storage_coordinator.completed;
        pool[15]=runtime.storage_coordinator.last_error.map(|e|e as u64+1).unwrap_or(0);
        pool[16]=runtime.storage_coordinator.last_read;
        pool[17]=runtime.storage_view.revision;
        let view=runtime.storage_view.snapshot;pool[18]=view.count as u64;pool[19]=view.selected as u64;
        pool[20]=view.capacity;pool[21]=view.eligible;pool[22]=view.reserved;pool[23]=view.nodes as u64;
        pool[24]=view.ready as u64;pool[25]=view.failed as u64;
        let transfer=runtime.storage_coordinator.active_transfer();
        pool[26]=transfer.0 as u64;pool[27]=transfer.1;pool[28]=transfer.2;
        pool[29]=transfer.5;pool[31]=fixture.error.map(|e|e as u64+1).unwrap_or(0);
        pool[30]=runtime.storage_coordinator.completed_read;
        pool[251]=runtime.storage_coordinator.read_error.map(|e|e as u64+1).unwrap_or(0);
        pool[252]=runtime.storage_metadata.last_request;
        pool[253]=runtime.storage_metadata.completed_request;
        pool[254]=runtime.storage_metadata.last_error.map(|e|e as u64+1).unwrap_or(0);
        let pool_events=runtime.storage_view.event_diagnostics();
        data[507]=pool_events.0 as u64;data[508]=pool_events.1;data[509]=pool_events.2;data[510]=pool_events.3;
        words(&mut pool[240..242],&transfer.3);words(&mut pool[242..246],&transfer.4);
        for (index,object) in view.objects.iter().enumerate().filter_map(|(i,o)|o.map(|o|(i,o))) {
            let at=32+index*16;words(&mut pool[at..at+2],&object.id);
            pool[at+2]=object.version;pool[at+3]=object.generation;pool[at+4]=object.bytes;
            pool[at+5]=object.desired as u64;pool[at+6]=object.verified as u64;pool[at+7]=object.offline as u64;
            pool[at+8]=object.stale as u64;pool[at+9]=object.corrupt as u64;pool[at+10]=object.healing as u64;
        }
        // Selected-object placement projection: read-only identities, no payloads.
        for (index, placement) in view.placements.iter().enumerate().filter_map(|(i,p)|p.map(|p|(i,p))) {
            let at=160+index*10;
            words(&mut pool[at..at+4],&placement.node);
            words(&mut pool[at+4..at+6],&placement.resource);
            words(&mut pool[at+6..at+8],&placement.device);
            pool[at+8]=placement.version;pool[at+9]=placement.state as u64;
        }
        if let Some(local) = runtime.nodes.local_id() { words(&mut data[16..20], &local.0); }
        data[20] = runtime.nodes.control_version(); data[21] = runtime.node_projection.version;
        data[22] = runtime.node_projection.stale as u64; data[23] = runtime.node_projection.gaps;
        data[24] = runtime.nodes.discovered_nodes().iter().flatten().count() as u64;
        data[25] = runtime.nodes.discovered_nodes().iter().flatten().filter(|node| matches!(node.trust, crate::runtime::node::types::TrustState::Trusted | crate::runtime::node::types::TrustState::Restricted)).count() as u64;
        data[26] = runtime.nodes.sessions().iter().flatten().filter(|session| session.state == crate::runtime::node::types::SessionState::Established).count() as u64;
        data[29] = runtime.nodes.configured_links().iter().flatten().count() as u64;
        data[30] = runtime.nodes.audit_records().iter().flatten().count() as u64;
        data[68] = runtime.node_transport.poll_calls;
        data[69] = runtime.node_transport.serviced_links;
        data[70] = runtime.node_transport.received_packets;
        data[72] = runtime.node_operator.last_submitted;
        data[91] = runtime.node_operator.last_detail_length as u64;
        words(&mut data[92..108], &runtime.node_operator.last_detail);
        data[108] = runtime.services.inspect(crate::runtime::service::SERVICE_REPLICA_STORAGE)
            .map(|service| service.state as u64 + 1).unwrap_or(0);
        data[109] = runtime.fabric_resources.entries().iter().flatten().count() as u64;
        data[489] = runtime.storage_operator.last_submitted;
        data[494] = runtime.storage_advertiser.completed.saturating_add(runtime.storage_coordinator.publication_completed());
        data[495] = runtime.storage_coordinator.publication_last_error().or(runtime.storage_advertiser.last_error).map(|e| e as u64).unwrap_or(0);
        data[496] = runtime.fabric_resources.entries().iter().flatten().filter(|r| r.online).count() as u64;
        if let Some(resource) = runtime.fabric_resources.entries().iter().flatten().next() {
            words(&mut data[497..499], &resource.id.0);
            words(&mut data[499..501], &resource.device);
            data[501] = resource.capacity; data[502] = resource.available;
            data[503] = resource.reserved; data[504] = resource.generation;
            data[505] = resource.sequence; data[506] = resource.expires;
        }
        if let Some(result) = runtime.storage_operator.last_completion {
            data[490] = result.request_id; data[491] = result.correlation_id; data[492] = result.causation_id;
            data[493] = result.result.err().map(|e| e as u64 + 2).unwrap_or(1);
        }
        if let Some(observation) = runtime.storage_last_observation {
            if let Ok(bytes) = observation.encode() { data[110] = 1; words(&mut data[111..128], &bytes); }
        }
        if let Some(completion) = runtime.node_operator.last_completion {
            data[73] = completion.request_id;
            data[74] = completion.correlation_id;
            data[75] = completion.causation_id;
            match completion.result {
                Ok(response) => { data[76] = 1; words(&mut data[77..87], &response.encode()); }
                Err(error) => data[76] = error as u64 + 1,
            }
        }
        if let Some(grant) = runtime.nodes.remote_grants().iter().flatten().max_by_key(|grant| grant.id) {
            data[87] = grant.id;
            data[88] = grant.revoked as u64;
            data[89] = grant.expires_at;
            data[90] = grant.operation as u64;
        }
        if let Some(peer) = console.selected_node_id {
            words(&mut data[32..36], &peer.0);
            if let Some((transaction, stage, expires, approvals, ended, site)) = runtime.node_transport.trust.lifecycle(peer) {
                data[56] = stage as u64 + 1; data[57] = approvals as u64; data[58] = expires;
                data[62] = ended; data[63] = site as u64; words(&mut data[64..68], &transaction);
            }
            data[59] = runtime.nodes.discovered_nodes().iter().flatten().find(|node| node.id == peer).map(|node| node.last_seen).unwrap_or(0);
            data[60] = runtime.node_transport.trust.last_error.map(|error| error as u64 + 1).unwrap_or(0);
            data[61] = runtime.node_transport.last_error.map(|error| error as u64 + 1).unwrap_or(0);
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
        publish_settings(console, generation);
        core::ptr::write_volatile(pointer.add(2), generation | 1);
        data[2] = generation; data[511] = generation;
        for index in 0..512 { if index != 2 { core::ptr::write_volatile(pointer.add(index), data[index]); } }
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        core::ptr::write_volatile(pointer.add(2), generation);
        let pool_pointer=(&raw mut INFINITY_POOL_DIAGNOSTIC_SNAPSHOT).cast::<u64>();
        core::ptr::write_volatile(pool_pointer.add(2),generation|1);pool[2]=generation;pool[255]=generation;
        for index in 0..256 {if index!=2 {core::ptr::write_volatile(pool_pointer.add(index),pool[index]);}}
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        core::ptr::write_volatile(pool_pointer.add(2),generation);
    }
}
