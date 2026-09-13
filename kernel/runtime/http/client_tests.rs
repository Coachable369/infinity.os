use super::*;
use core::{
    cell::Cell,
    task::{Context, Waker},
};
use rand_core::SeedableRng;
use std::rc::Rc;

#[derive(Default)]
struct Observations {
    time: Cell<i64>,
    allowed: Cell<bool>,
    rx: Cell<usize>,
    tx: Cell<usize>,
}
struct TestLink(Rc<Observations>);
impl Link for TestLink {
    // ------------------------=
    // FUNC: register_waker
    // DESC: Uses the test's manually stepped clock instead of creating self-wake work.
    // ------------------=
    fn register_waker(&mut self, _: &Waker) {}
    // ------------------------=
    // FUNC: now
    // DESC: Supplies a manually advanced monotonic clock.
    // ------------------=
    fn now(&self) -> Instant {
        Instant::from_millis(self.0.time.get())
    }
    // ------------------------=
    // FUNC: allowed
    // DESC: Models immediate authority revocation between scheduler polls.
    // ------------------=
    fn allowed(&mut self, _: [u8; 4], _: u16) -> bool {
        self.0.allowed.get()
    }
    // ------------------------=
    // FUNC: receive
    // DESC: Counts attempted ingress without supplying a response.
    // ------------------=
    fn receive(&mut self, _: &mut [u8; 1514]) -> Option<usize> {
        self.0.rx.set(self.0.rx.get() + 1);
        None
    }
    // ------------------------=
    // FUNC: transmit
    // DESC: Counts NIC submissions and models available hardware capacity.
    // ------------------=
    fn transmit(&mut self, _: &[u8]) -> bool {
        self.0.tx.set(self.0.tx.get() + 1);
        true
    }
}

// ------------------------=
// FUNC: cancellation_case
// DESC: Exercises the real client future through pending connect followed by denial or deadline expiry.
// ------------------=
fn cancellation_case(timeout: bool, initial_denial: bool, destination: Destination) {
    let state = Rc::new(Observations::default());
    state.allowed.set(!initial_denial);
    let config = Configuration {
        mac: [2, 0, 0, 0, 0, 1],
        address: [10, 0, 0, 1],
        prefix: 24,
        gateway: None,
        dns_server: [10, 0, 0, 2],
        local_port: 49152,
        deadline: Instant::from_millis(100),
    };
    let mut read = [0; 16640];
    let mut write = [0; 2048];
    let mut request = [0; 1024];
    let mut response = [0; 4096];
    let mut future = core::pin::pin!(get(
        TestLink(state.clone()),
        config,
        destination,
        rand_chacha::ChaCha20Rng::from_seed([1; 32]),
        crate::tls::system_roots(),
        1,
        "example.test",
        443,
        "/",
        https::Buffers {
            read_record: &mut read,
            write_record: &mut write,
            request: &mut request,
            response: &mut response
        }
    ));
    let mut context = Context::from_waker(Waker::noop());
    if !initial_denial {
        assert!(future.as_mut().poll(&mut context).is_pending());
        assert_eq!(state.tx.get(), 0);
        if timeout {
            state.time.set(100);
        } else {
            state.allowed.set(false);
        }
    }
    let result = future.as_mut().poll(&mut context);
    if timeout {
        assert!(matches!(result, Poll::Ready(Err(Error::Timeout))));
    } else {
        assert!(matches!(result, Poll::Ready(Err(Error::Denied))));
    }
    assert_eq!(
        state.tx.get(),
        0,
        "queued packets must not escape after revocation"
    );
}

#[test]
// ------------------------=
// FUNC: client_revocation_prevents_pending_tcp_and_dns_packets
// DESC: Verifies no queued ARP/TCP/DNS packet is submitted after authority is revoked.
// ------------------=
fn client_revocation_prevents_pending_tcp_and_dns_packets() {
    cancellation_case(false, false, Destination::Address([10, 0, 0, 2]));
    cancellation_case(false, false, Destination::Resolve);
}

#[test]
// ------------------------=
// FUNC: client_deadline_cancels_pending_connect_and_resolution
// DESC: Verifies one hard deadline covers DNS as well as connection setup.
// ------------------=
fn client_deadline_cancels_pending_connect_and_resolution() {
    cancellation_case(true, false, Destination::Address([10, 0, 0, 2]));
    cancellation_case(true, false, Destination::Resolve);
}

#[test]
// ------------------------=
// FUNC: client_initial_denial_emits_no_packets
// DESC: Rejects unauthorized DNS and direct destinations before attempting network traffic.
// ------------------=
fn client_initial_denial_emits_no_packets() {
    cancellation_case(false, true, Destination::Address([10, 0, 0, 2]));
    cancellation_case(false, true, Destination::Resolve);
}
