//! Resolver ABI contract test. Injected answers test the boundary, not wire DNS.
use infinity_servo_runtime_primitives::dns::{self, Provider};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::net::ToSocketAddrs;
static ACTIVE: AtomicU64 = AtomicU64::new(0);
static NEXT: AtomicU64 = AtomicU64::new(1);
static POLLS: AtomicUsize = AtomicUsize::new(0);
static CANCELLED: AtomicUsize = AtomicUsize::new(0);
static MODE: AtomicUsize = AtomicUsize::new(0);
static BUSY_STARTS: AtomicUsize = AtomicUsize::new(0);
// ------------------------=
// FUNC: begin
// DESC: Admits only the explicit fixture name and prevents overlapping transactions.
// ------------------=
fn begin(host: &str) -> Result<u64, i32> {
    if host != "example.test" { return Err(13); }
    if BUSY_STARTS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        return Err(11);
    }
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    ACTIVE.compare_exchange(0, id, Ordering::Relaxed, Ordering::Relaxed).map_err(|_| 11)?;
    POLLS.store(0, Ordering::Relaxed);
    Ok(id)
}
// ------------------------=
// FUNC: poll
// DESC: Exercises pending, revoked, and complete states without pretending to send DNS packets.
// ------------------=
fn poll(id: u64) -> Result<[u8; 4], i32> {
    if ACTIVE.load(Ordering::Relaxed) != id { return Err(22); }
    if MODE.load(Ordering::Relaxed) == 1 { return Err(13); }
    if MODE.load(Ordering::Relaxed) == 2 { return Err(11); }
    if POLLS.fetch_add(1, Ordering::Relaxed) == 0 { return Err(11); }
    Ok([192, 0, 2, 9])
}
// ------------------------=
// FUNC: cancel
// DESC: Releases only the live identity and records cleanup for all terminal states.
// ------------------=
fn cancel(id: u64) {
    if ACTIVE.compare_exchange(id, 0, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        CANCELLED.fetch_add(1, Ordering::Relaxed);
    }
}
// ------------------------=
// FUNC: run
// DESC: Verifies missing authority, validation, cooperative completion, port preservation and cleanup.
// ------------------=
pub fn run() {
    assert!(("example.test", 443).to_socket_addrs().is_err());
    let provider = std::boxed::Box::leak(std::boxed::Box::new(Provider {
        cpu: super::std_probe::cpu, owner: super::std_probe::cpu(), begin, poll, cancel,
    }));
    assert!(dns::install(provider));
    assert!(!dns::install(provider));
    for invalid in ["", "a..b", "-a.test", "a-.test", "host/path", "a\0b", "é.test"] {
        assert!((invalid, 443).to_socket_addrs().is_err());
        assert_eq!(ACTIVE.load(Ordering::Relaxed), 0);
    }
    let mut results = ("example.test.", 443).to_socket_addrs().unwrap();
    assert_eq!(results.next(), Some(([192, 0, 2, 9], 443).into()));
    assert_eq!(results.next(), None);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 1);
    MODE.store(1, Ordering::Relaxed);
    assert!(("example.test", 80).to_socket_addrs().is_err());
    assert_eq!(ACTIVE.load(Ordering::Relaxed), 0);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 2);
    MODE.store(0, Ordering::Relaxed);
    BUSY_STARTS.store(2, Ordering::Relaxed);
    assert_eq!(("example.test", 80).to_socket_addrs().unwrap().next(), Some(([192, 0, 2, 9], 80).into()));
    assert_eq!(BUSY_STARTS.load(Ordering::Relaxed), 0);
    let mut output = [0xa5; 4];
    assert_eq!(unsafe { dns::infinity_dns_poll(0, output.as_mut_ptr()) }, 22);
    assert_eq!(unsafe { dns::infinity_dns_poll(u64::MAX, output.as_mut_ptr()) }, 22);
    assert_eq!(output, [0xa5; 4]);
    MODE.store(2, Ordering::Relaxed);
    let started = std::time::Instant::now();
    assert_eq!(("example.test", 80).to_socket_addrs().unwrap_err().kind(), std::io::ErrorKind::TimedOut);
    assert!(started.elapsed() >= std::time::Duration::from_secs(5));
    assert_eq!(ACTIVE.load(Ordering::Relaxed), 0);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 4);
    MODE.store(0, Ordering::Relaxed);
    assert_eq!(("example.test", 443).to_socket_addrs().unwrap().next(), Some(([192, 0, 2, 9], 443).into()));
}
