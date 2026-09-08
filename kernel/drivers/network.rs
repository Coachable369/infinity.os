//! Bounded native NIC service pump. No rendering work or packet-by-packet IEF.
#[cfg(target_arch = "x86_64")]
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_arch = "x86_64")]
static BUSY: AtomicBool = AtomicBool::new(false);
#[cfg(target_arch = "x86_64")]
static mut NIC: Option<super::e1000::E1000> = None;
#[cfg(target_arch = "x86_64")]
static mut NEXT_POLL: u64 = 0;

// ------------------------=
// FUNC: initialize
// DESC: Binds the reference native NIC after restored network configuration is available.
// ------------------=
pub fn initialize() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        if BUSY.swap(true, Ordering::Acquire) { return; }
        if (&*(&raw const NIC)).is_none() {
            NIC = super::e1000::E1000::initialize();
        }
        BUSY.store(false, Ordering::Release);
    }
}

// ------------------------=
// FUNC: poll
// DESC: Services at most four RX and four TX descriptors once per millisecond; never spins or waits for hardware.
// ------------------=
pub fn poll() {
    #[cfg(target_arch = "x86_64")]
    {
        if BUSY.swap(true, Ordering::Acquire) { return; }
        unsafe {
            let now = crate::ui::performance::monotonic_ns().or_else(|| (&*(&raw const NIC)).as_ref().and_then(|nic| nic.reference_clock_ns()));
            let Some(now) = now else { BUSY.store(false, Ordering::Release); return; };
            if now >= NEXT_POLL {
                NEXT_POLL = now.saturating_add(1_000_000);
                if let Some(nic) = (&mut *(&raw mut NIC)).as_mut() {
                    pump(nic, now / 1_000_000_000);
                    crate::runtime::poll_node_transport(now / 1_000_000_000);
                }
            }
        }
        BUSY.store(false, Ordering::Release);
    }
}

#[cfg(target_arch = "x86_64")]
// ------------------------=
// FUNC: pump
// DESC: Moves bounded NIC frames through the shared NetworkRuntime using only configured IPv4 and observed link state.
// ------------------=
fn pump(nic: &mut super::e1000::E1000, now: u64) {
    use crate::runtime::network::types::*;
    crate::runtime::with_runtime(|runtime| {
        let network = &mut runtime.network;
        let link = nic.link_up();
        let _ = network.register_firmware_device(FirmwareNetworkDevice {
            firmware_handle: 0, device_id: 0x100e8086, hardware_address: Some(nic.mac),
            link_state: if link { LinkState::Up } else { LinkState::Down },
            maximum_frame_size: 1500, can_receive: true, can_transmit: true,
        });
        let _ = network.interfaces.bind_native_device(2, NetworkDevice {
            device_id: 0x100e8086, driver_id: 0x45313030, link_type: LinkType::Ethernet,
            hardware_address: Some(nic.mac), maximum_frame_size: 1500,
            link_state: if link { LinkState::Up } else { LinkState::Down },
            can_receive: true, can_transmit: true, offload_capabilities: 0,
            operational_state: if link { OperationalState::Ready } else { OperationalState::Offline }, error_code: 0,
        });
        let enabled = link && network.interfaces.interface(2).map(|i| i.enabled).unwrap_or(false)
            && network.profiles.active().map(|p| p.interfaces_enabled).unwrap_or(false);
        let address = if enabled {
            (0..network.interfaces.address_count()).filter_map(|i| network.interfaces.address_nth(i))
                .find_map(|a| match a.address {
                    IpAddress::V4(ip) if a.interface_id == 2 && a.state == AddressState::Preferred && a.valid_until.map(|end| now < end).unwrap_or(true) => Some(ip), _ => None,
                }).unwrap_or([0; 4])
        } else { [0; 4] };
        network.wire.configure(nic.mac, address);
        network.connections.bind_native_address(if address == [0; 4] { None } else { Some(address) });
        let mut frame = [0u8; super::e1000::MAX_FRAME];
        for _ in 0..4 {
            let Some(length) = nic.receive(&mut frame) else { break; };
            if length != 0 { let _ = network.wire.ingest(&frame[..length], now); }
        }
        for _ in 0..4 {
            let Some(packet) = network.wire.receive_datagram() else { break; };
            let _ = network.connections.deliver_datagram(packet, &mut network.policy, &runtime.capabilities, now);
        }
        for _ in 0..4 {
            let Some(frame) = network.wire.peek_transmit() else { break; };
            if !nic.transmit(&frame.bytes[..frame.length]) { break; }
            network.wire.complete_transmit();
        }
        let (rx, tx, drops) = nic.statistics();
        let _ = network.interfaces.update_native_counters(2, rx, tx, drops.saturating_add(network.wire.rejected_frames));
    });
}
