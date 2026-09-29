#![allow(dead_code)]
pub use infinity_http as http_transport;
#[path = "../kernel/core/boot_info.rs"]
mod boot_info;
#[path = "../kernel/drivers/https.rs"]
pub(crate) mod https;
#[path = "../kernel/drivers/browser_network.rs"]
mod browser_network;
mod drivers {
    pub(crate) use crate::https;
}
#[path = "../kernel/core/geturl.rs"]
mod geturl_command;
const LINE_CAPACITY: usize = 128;
struct ConsoleOutput {
    lines: Vec<Vec<u8>>,
}
impl ConsoleOutput {
    // ------------------------=
    // FUNC: write_line
    // DESC: Captures presentation separately; command acceptance checks typed exit state instead of prose.
    // ------------------=
    fn write_line(&mut self, bytes: &[u8]) {
        self.lines.push(bytes.to_vec());
    }
}
struct ConsoleRuntime {
    current_user: runtime::identity::StableId,
    current_session: runtime::identity::StableId,
    output: ConsoleOutput,
    redraws: usize,
}
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: redraw
    // DESC: Records presentation requests without introducing framebuffer dependencies into command lifecycle tests.
    // ------------------=
    fn redraw(&mut self) {
        self.redraws += 1;
    }
}
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
    runtime::poll_node_transport(0);
    assert_eq!(
        https::preferred_dns(
            Some(IpAddress::V4([40, 192, 249, 101])),
            Some(IpAddress::V4([192, 168, 1, 254])),
            Some([10, 0, 2, 2]),
        ),
        Some([192, 168, 1, 254]),
    );
    assert_eq!(
        https::preferred_dns(
            Some(IpAddress::V4([1, 1, 1, 1])),
            Some(IpAddress::V4([8, 8, 8, 8])),
            Some([203, 0, 113, 1]),
        ),
        Some([1, 1, 1, 1]),
    );
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
    assert_eq!(https::get_browser(owner,caps[0],caps[1],caps[2],caps[3],
        "example.test",0,"/"),Err(https::Failure::Invalid));
    assert_eq!(https::get_browser(owner,0,caps[1],caps[2],caps[3],
        "example.test",443,"/"),Err(https::Failure::Denied));
    // The larger future must fit its bounded task slot, retain exclusive owner
    // semantics and cancel without leaving the shared network actor occupied.
    let first=https::get_browser(owner,caps[0],caps[1],caps[2],caps[3],"example.test",443,"/").unwrap();
    assert_eq!(https::get_browser(owner,caps[0],caps[1],caps[2],caps[3],
        "example.test",443,"/image.png"),Err(https::Failure::Busy));
    assert_eq!(https::cancel(Identity([44;16])),Err(https::Failure::Denied));
    https::cancel_browser(owner,first).unwrap();
    assert!(matches!(https::take_browser(owner,first),Ok(Some(Err(https::Failure::Cancelled)))));
    let second=https::get_browser(owner,caps[0],caps[1],caps[2],caps[3],"example.test",443,"/").unwrap();
    assert_ne!(first,second);
    assert_eq!(https::cancel_browser(owner,first),Err(https::Failure::Denied));
    assert!(matches!(https::take_browser(owner,first),Err(https::Failure::Denied)));
    assert!(matches!(https::take_browser(owner,second),Ok(None)));
    https::cancel_browser(owner,second).unwrap();
    assert!(matches!(https::take_browser(owner,second),Ok(Some(Err(https::Failure::Cancelled)))));
    unsafe {
        use browser_network as bridge;
        let mut reply=bridge::abi::Response {status:0,headers:core::ptr::null(),headers_length:0,
            body:core::ptr::null(),body_length:0};
        assert!(bridge::configure(owner,[0;4]));
        // Browser leases are refreshed from the active session policy. Deny
        // through that policy, not through lease IDs that renewal replaces.
        let prior=runtime::with_runtime(|r| {
            let prior=*r.network.profiles.active().unwrap();
            let mut denied=prior;denied.id=0;denied.protected=false;
            denied.kind=runtime::network::profile::ProfileKind::Custom;
            denied.internet_allowed=false;
            let id=r.network.profiles.create(denied).unwrap();
            let staged=r.network.profiles.stage(id).unwrap();
            r.network.profiles.commit(staged).unwrap();prior
        }).unwrap();
        let denied=bridge::begin(b"https://example.test/");assert_ne!(denied,0);
        bridge::pump();assert_eq!(bridge::poll(denied,&mut reply),2);
        bridge::cancel(denied);bridge::pump();
        runtime::with_runtime(|r|r.network.profiles.commit(prior).unwrap());
        assert!(bridge::configure(owner,caps));
        let mut ids=[0;16];
        for id in &mut ids {*id=bridge::begin(b"file:///private");assert_ne!(*id,0);}
        assert_eq!(bridge::begin(b"https://example.test/"),0);
        assert!(!bridge::configure(owner,caps));
        bridge::pump();
        for id in ids {assert_eq!(bridge::poll(id,&mut reply),2);bridge::cancel(id);}
        bridge::pump();
        let old=bridge::begin(b"https://example.test/");bridge::pump();
        assert_eq!(bridge::poll(old,&mut reply),0);
        bridge::cancel(old);bridge::pump();assert_eq!(bridge::poll(old,&mut reply),2);
        let current=bridge::begin(b"https://example.test/");assert_ne!(old,current);
        bridge::cancel(old);bridge::pump();assert_eq!(bridge::poll(current,&mut reply),0);
        bridge::cancel(current);bridge::pump();assert_eq!(bridge::poll(current,&mut reply),2);
        let active=bridge::begin(b"https://example.test/");bridge::pump();
        let pending=bridge::begin(b"https://example.test/next");
        bridge::cancel_all();
        assert_eq!(bridge::poll(active,&mut reply),2);
        assert_eq!(bridge::poll(pending,&mut reply),2);
        bridge::pump();assert!(bridge::configure(owner,caps));
    }
    let session = runtime::with_runtime(|r| r.identity.session_nth(0).unwrap()).unwrap();
    let mut command_console = ConsoleRuntime {
        current_user: session.user,
        current_session: session.id,
        output: ConsoleOutput { lines: Vec::new() },
        redraws: 0,
    };
    use core::sync::atomic::Ordering;
    let count = runtime::with_runtime(|r| r.capabilities.count()).unwrap();
    geturl_command::authorize(&mut command_console, false);
    assert_eq!(
        runtime::with_runtime(|r| r.capabilities.count()).unwrap(),
        count
    );
    command_console.current_user = runtime::identity::StableId([99; 16]);
    geturl_command::authorize(&mut command_console, true);
    assert_eq!(
        runtime::with_runtime(|r| r.capabilities.count()).unwrap(),
        count
    );
    command_console.current_user = session.user;
    assert!(geturl_command::execute(
        &mut command_console,
        b"geturl --location https://example.test/"
    ));
    assert_eq!(
        geturl_command::INFINITY_GETURL_EXIT.load(Ordering::Acquire),
        2
    );
    assert!(geturl_command::execute(
        &mut command_console,
        b"geturl --help"
    ));
    assert_eq!(
        geturl_command::INFINITY_GETURL_EXIT.load(Ordering::Acquire),
        0
    );
    assert!(geturl_command::execute(
        &mut command_console,
        b"geturl -s https://example.test/"
    ));
    assert_eq!(
        geturl_command::INFINITY_GETURL_EXIT.load(Ordering::Acquire),
        u32::MAX
    );
    assert_eq!(command_console.redraws, 0);
    https::cancel(owner).unwrap();
    geturl_command::poll(&mut command_console);
    assert_eq!(
        geturl_command::INFINITY_GETURL_EXIT.load(Ordering::Acquire),
        42
    );
    assert_eq!(command_console.redraws, 1);
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
    runtime::poll_node_transport(0);
    runtime::with_runtime(|r| {
        r.start_all(0);
    });
    geturl_command::authorize(&mut command_console, true);
    caps[1] = runtime::with_runtime(|r| {
        (0..r.capabilities.count())
            .filter_map(|i| r.capabilities.nth(i))
            .find(|c| {
                r.capabilities
                    .validate(c.id, owner, C::NetworkSend, 0, 1, 0, 0)
                    .is_ok()
            })
            .unwrap()
            .id
    })
    .unwrap();
    runtime::with_runtime(|r| {
        assert_eq!(r.capabilities.get(caps[1]).unwrap().expires_at, Some(60));
        assert!(r
            .capabilities
            .validate(caps[1], owner, C::NetworkSend, 0, 1, 0, 60)
            .is_err());
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
