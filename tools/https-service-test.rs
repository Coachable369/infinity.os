#![allow(dead_code)]
pub use infinity_http as http_transport;
#[path = "../kernel/core/boot_info.rs"]
mod boot_info;
#[path = "../kernel/drivers/https.rs"]
mod https;
#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "../kernel/ui/mod.rs"]
mod ui;
// ------------------------=
// FUNC: output_text
// DESC: Suppresses diagnostics; acceptance uses typed state and packet counts instead.
// ------------------=
fn output_text(_: &[u8]) {}
mod console {
    // ------------------------=
    // FUNC: certificate_time
    // DESC: Supplies explicit fixture time only in this host lifecycle test.
    // ------------------=
    pub fn certificate_time(_: u64) -> Option<u64> {
        Some(1_800_000_000)
    }
}
mod e1000 {
    pub struct E1000(pub usize);
    impl E1000 {
        // ------------------------=
        // FUNC: transmit
        // DESC: Counts submitted frame bytes without using a host network stack.
        // ------------------=
        pub fn transmit(&mut self, bytes: &[u8]) -> bool {
            self.0 += bytes.len();
            true
        }
    }
}
// ------------------------=
// FUNC: main
// DESC: Exercises the production actor with real Runtime capabilities, policy, sessions, queued traffic and cancellation.
// ------------------=
fn main() {
    use runtime::{
        capability::CapabilityType as C, execution::SecurityIdentity as Identity, network::types::*,
    };
    runtime::initialize(false);
    let (owner, mut caps) = runtime::with_runtime(|r| {
        r.identity.create_machine(b"test", 3, 1, 0).unwrap();
        let user = r.identity.create_user(b"tester", b"Tester", 0).unwrap();
        r.identity
            .create_password(user.id, b"fixture-password", 0)
            .unwrap();
        let session = r
            .identity
            .create_session(user.id, b"fixture-password", 0)
            .unwrap();
        let owner = Identity(session.id.0);
        r.network
            .interfaces
            .add_interface(NetworkInterface {
                id: 2,
                device: NetworkDevice {
                    device_id: 1,
                    driver_id: 1,
                    link_type: LinkType::Ethernet,
                    hardware_address: Some([2, 0, 0, 0, 0, 1]),
                    maximum_frame_size: 1500,
                    link_state: LinkState::Up,
                    can_receive: true,
                    can_transmit: true,
                    offload_capabilities: 0,
                    operational_state: OperationalState::Ready,
                    error_code: 0,
                },
                enabled: true,
                rx_packets: 0,
                tx_packets: 0,
                rx_drops: 0,
                tx_drops: 0,
            })
            .unwrap();
        r.network
            .interfaces
            .add_address(
                2,
                IpAddress::V4([10, 0, 0, 1]),
                24,
                AddressScope::Private,
                AddressSource::TestFixture,
                None,
                None,
            )
            .unwrap();
        r.network
            .interfaces
            .add_route(
                IpAddress::V4([0; 4]),
                0,
                Some(IpAddress::V4([10, 0, 0, 2])),
                2,
                1,
                RouteSource::TestFixture,
                None,
            )
            .unwrap();
        r.network
            .resolver
            .set_server(0, Some(IpAddress::V4([10, 0, 0, 2])))
            .unwrap();
        r.network
            .interfaces
            .add_route(
                IpAddress::V4([10, 0, 0, 0]),
                24,
                None,
                2,
                0,
                RouteSource::TestFixture,
                None,
            )
            .unwrap();
        r.network.policy.set_default_action(PolicyAction::Allow);
        let mut caps = [0; 4];
        for (i, kind) in [
            C::NetworkConnect,
            C::NetworkSend,
            C::NetworkReceive,
            C::NetworkResolve,
        ]
        .into_iter()
        .enumerate()
        {
            caps[i] = r
                .capabilities
                .grant(kind, 0, 1, 0, Identity([9; 16]), owner, None, 0)
                .unwrap();
        }
        (owner, caps)
    })
    .unwrap();
    let mut boot: boot_info::BootInfo = unsafe { core::mem::zeroed() };
    boot.firmware_entropy = [23; 32];
    boot.firmware_entropy_valid = 1;
    https::initialize(&boot);
    assert_eq!(
        https::get(
            owner,
            caps[0],
            caps[1],
            caps[2],
            caps[3],
            "example.test",
            "/"
        ),
        Ok(())
    );
    assert!(https::get(
        owner,
        caps[0],
        caps[1],
        caps[2],
        caps[3],
        "example.test",
        "/"
    )
    .is_err());
    assert!(https::take(Identity([44; 16])).is_err());
    let mut nic = e1000::E1000(0);
    for now in 0..4 {
        https::poll(&mut nic, now);
    }
    assert!(nic.0 > 0);
    let before = nic.0;
    runtime::with_runtime(|r| r.capabilities.revoke(caps[1]).unwrap());
    https::poll(&mut nic, 5);
    assert_eq!(nic.0, before);
    assert!(matches!(https::take(owner), Ok(Some(Err(_)))));
    assert!(matches!(
        https::get(
            owner,
            caps[0],
            caps[1],
            caps[2],
            caps[3],
            "example.test",
            "/"
        ),
        Err(https::Failure::Denied)
    ));
    caps[1] = runtime::with_runtime(|r| {
        r.capabilities
            .grant(C::NetworkSend, 0, 1, 0, Identity([9; 16]), owner, None, 0)
            .unwrap()
    })
    .unwrap();
    https::get(
        owner,
        caps[0],
        caps[1],
        caps[2],
        caps[3],
        "example.test",
        "/",
    )
    .unwrap();
    runtime::with_runtime(|r| {
        r.network
            .resolver
            .set_server(0, Some(IpAddress::V4([10, 0, 0, 3])))
            .unwrap()
    });
    https::poll(&mut nic, 6);
    assert_eq!(nic.0, before);
    assert!(matches!(https::take(owner), Ok(Some(Err(_)))));
    runtime::with_runtime(|r| {
        r.network
            .resolver
            .set_server(0, Some(IpAddress::V4([10, 0, 0, 2])))
            .unwrap()
    });
    https::get(
        owner,
        caps[0],
        caps[1],
        caps[2],
        caps[3],
        "example.test",
        "/",
    )
    .unwrap();
    assert_eq!(
        https::cancel(Identity([44; 16])),
        Err(https::Failure::Denied)
    );
    https::cancel(owner).unwrap();
    assert!(matches!(
        https::take(owner),
        Ok(Some(Err(https::Failure::Cancelled)))
    ));
    https::get(
        owner,
        caps[0],
        caps[1],
        caps[2],
        caps[3],
        "example.test",
        "/",
    )
    .unwrap();
    let replacement = runtime::with_runtime(|r| {
        r.network
            .interfaces
            .add_route(
                IpAddress::V4([0; 4]),
                0,
                Some(IpAddress::V4([10, 0, 0, 4])),
                2,
                0,
                RouteSource::TestFixture,
                None,
            )
            .unwrap()
    })
    .unwrap();
    https::poll(&mut nic, 7);
    assert_eq!(nic.0, before);
    assert!(matches!(https::take(owner), Ok(Some(Err(_)))));
    runtime::with_runtime(|r| r.network.interfaces.remove_route(replacement).unwrap());
    https::get(
        owner,
        caps[0],
        caps[1],
        caps[2],
        caps[3],
        "example.test",
        "/",
    )
    .unwrap();
    runtime::with_runtime(|r| {
        let s = (0..runtime::identity::MAX_SESSIONS)
            .filter_map(|i| r.identity.session_nth(i))
            .find(|s| s.id.0 == owner.0)
            .unwrap();
        r.identity.lock_session(s.id, s.user).unwrap();
    });
    https::poll(&mut nic, 8);
    assert_eq!(nic.0, before);
    assert!(https::take(owner).is_err());
    println!("HTTPS actor lifecycle passed");
}
