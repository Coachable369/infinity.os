pub type Key = usize;
type Destructor = unsafe extern "C" fn(*mut u8);
unsafe extern "C" {
    fn infinity_std_tls_create(dtor: Option<Destructor>, key: *mut usize) -> i32;
    fn infinity_std_tls_destroy(key: usize);
    fn infinity_std_tls_get(key: usize) -> *mut u8;
    fn infinity_std_tls_set(key: usize, value: *mut u8) -> i32;
}
// ------------------------=
// FUNC: create
// DESC: Allocates a process-wide nonzero key with per-thread values and destructor.
// ------------------=
pub fn create(dtor: Option<Destructor>) -> Key {
    let mut key = 0;
    assert_eq!(unsafe { infinity_std_tls_create(dtor, &mut key) }, 0);
    assert_ne!(key, 0);
    key
}
// ------------------------=
// FUNC: destroy
// DESC: Releases a TLS key after lazy initialization loses its race.
// ------------------=
pub unsafe fn destroy(key: Key) { unsafe { infinity_std_tls_destroy(key) } }
// ------------------------=
// FUNC: get
// DESC: Reads only the current execution thread's value.
// ------------------=
pub unsafe fn get(key: Key) -> *mut u8 { unsafe { infinity_std_tls_get(key) } }
// ------------------------=
// FUNC: set
// DESC: Stores a current-thread value and fails closed if native storage rejects it.
// ------------------=
pub unsafe fn set(key: Key, value: *mut u8) {
    assert_eq!(unsafe { infinity_std_tls_set(key, value) }, 0);
}
