//! Compile gate only; passing does not prove Servo or installed-system support.
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

thread_local! { static SLOT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }

// ------------------------=
// FUNC: runtime_roundtrip
// DESC: Exercises independent TLS, thread wakeup/join, allocation, and monotonic time.
// ------------------=
pub fn runtime_roundtrip() {
    let started = Instant::now();
    SLOT.with(|slot| slot.set(7));
    let state = Arc::new((Mutex::new(None), Condvar::new()));
    let worker_state = Arc::clone(&state);
    let worker = std::thread::spawn(move || {
        SLOT.with(|slot| {
            assert_eq!(slot.get(), 0);
            slot.set(11);
        });
        let values: Vec<u32> = (0..128).collect();
        let (lock, changed) = &*worker_state;
        *lock.lock().unwrap() = Some(values.iter().sum::<u32>());
        changed.notify_one();
    });
    let (lock, changed) = &*state;
    let result = changed.wait_while(lock.lock().unwrap(), |value| value.is_none()).unwrap();
    assert_eq!(*result, Some(8128));
    drop(result);
    worker.join().unwrap();
    SLOT.with(|slot| assert_eq!(slot.get(), 7));
    assert!(Instant::now() >= started);
}
