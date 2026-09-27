//! Bounded native TLS bookkeeping. The scheduler owns per-thread values and locking.
#![no_std]
pub mod arena;
pub mod wait;
#[cfg(all(target_os = "none", any(target_arch = "aarch64", target_arch = "x86_64")))]
pub mod context;
#[cfg(all(target_os = "none", any(target_arch = "aarch64", target_arch = "x86_64")))]
pub mod executor;
#[cfg(all(feature = "native-abi", target_os = "none"))]
pub mod native;
#[cfg(all(feature = "native-abi", target_os = "none"))]
pub mod network;
#[cfg(all(feature = "native-abi", target_os = "none"))]
pub mod dns;
#[cfg(all(feature = "c-allocator-abi", target_os = "none"))]
mod c_allocator;

pub type Destructor = unsafe extern "C" fn(*mut u8);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Full, InvalidKey }
#[derive(Clone, Copy)]
struct Slot { generation: usize, active: bool, destructor: Option<Destructor> }
#[derive(Clone, Copy)]
struct Value { generation: usize, pointer: usize }
pub struct Keys<const N: usize> { slots: [Slot; N] }
pub struct Values<const N: usize> { slots: [Value; N] }
pub struct Teardown { index: usize, round: usize }
impl Teardown {
    // ------------------------=
    // FUNC: new
    // DESC: Starts a bounded four-pass TLS cleanup for one exiting execution thread.
    // ------------------=
    pub const fn new() -> Self { Self { index: 0, round: 0 } }
    // ------------------------=
    // FUNC: next
    // DESC: Clears before yielding callbacks for execution outside locks; bounds destructor re-entry.
    // ------------------=
    pub fn next<const N: usize>(&mut self, keys: &Keys<N>, values: &mut Values<N>)
        -> Option<(Destructor, usize)> {
        while self.round < 4 {
            while self.index < N {
                let index = self.index;
                self.index += 1;
                if let Some(callback) = keys.take_destructor(values, index) { return Some(callback); }
            }
            self.round += 1;
            self.index = 0;
        }
        // No value, including a destructor-free value, survives thread exit.
        *values = Values::new();
        None
    }
}
impl<const N: usize> Values<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates independent zeroed values for exactly one execution thread.
    // ------------------=
    pub const fn new() -> Self { Self { slots: [Value { generation: 0, pointer: 0 }; N] } }
}
impl<const N: usize> Keys<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes a process-owned bounded key registry with no allocated keys.
    // ------------------=
    pub const fn new() -> Self {
        Self { slots: [Slot { generation: 0, active: false, destructor: None }; N] }
    }
    // ------------------------=
    // FUNC: create
    // DESC: Allocates a nonzero generation-tagged key; exhausted identities are never reused.
    // ------------------=
    pub fn create(&mut self, destructor: Option<Destructor>) -> Result<usize, Error> {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.active { continue; }
            let Some(generation) = slot.generation.checked_add(1) else { continue; };
            let Some(key) = generation.checked_mul(N).and_then(|base| base.checked_add(index)) else { continue; };
            *slot = Slot { generation, active: true, destructor };
            return Ok(key);
        }
        Err(Error::Full)
    }
    // ------------------------=
    // FUNC: lookup
    // DESC: Validates identity and generation before any per-thread storage access.
    // ------------------=
    fn lookup(&self, key: usize) -> Result<usize, Error> {
        if N == 0 || key < N { return Err(Error::InvalidKey); }
        let index = key % N;
        let slot = &self.slots[index];
        if !slot.active || slot.generation != key / N { return Err(Error::InvalidKey); }
        Ok(index)
    }
    // ------------------------=
    // FUNC: destroy
    // DESC: Revokes a key without invoking destructors or exposing values through slot reuse.
    // ------------------=
    pub fn destroy(&mut self, key: usize) -> Result<(), Error> {
        let index = self.lookup(key)?;
        self.slots[index].active = false;
        self.slots[index].destructor = None;
        Ok(())
    }
    // ------------------------=
    // FUNC: get
    // DESC: Reads only the provided thread's current-generation value.
    // ------------------=
    pub fn get(&self, values: &Values<N>, key: usize) -> Result<usize, Error> {
        let index = self.lookup(key)?;
        let value = values.slots[index];
        Ok(if value.generation == self.slots[index].generation { value.pointer } else { 0 })
    }
    // ------------------------=
    // FUNC: set
    // DESC: Associates a pointer-sized value with the owning thread and current key generation.
    // ------------------=
    pub fn set(&self, values: &mut Values<N>, key: usize, pointer: usize) -> Result<(), Error> {
        let index = self.lookup(key)?;
        values.slots[index] = Value { generation: self.slots[index].generation, pointer };
        Ok(())
    }
    // ------------------------=
    // FUNC: take_destructor
    // DESC: Clears one value before returning its callback; caller invokes outside the registry lock.
    // ------------------=
    pub fn take_destructor(&self, values: &mut Values<N>, index: usize) -> Option<(Destructor, usize)> {
        let slot = self.slots.get(index)?;
        let value = &mut values.slots[index];
        if !slot.active || value.generation != slot.generation || value.pointer == 0 { return None; }
        let callback = slot.destructor?;
        let pointer = value.pointer;
        value.pointer = 0;
        Some((callback, pointer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};
    static CALLBACKS: AtomicUsize = AtomicUsize::new(0);
    // ------------------------=
    // FUNC: counting_destructor
    // DESC: Records actual callback execution without dereferencing a synthetic address.
    // ------------------=
    unsafe extern "C" fn counting_destructor(_: *mut u8) { CALLBACKS.fetch_add(1, Ordering::SeqCst); }
    // ------------------------=
    // FUNC: teardown_executes_callbacks_and_bounds_repopulation
    // DESC: Executes teardown callbacks and proves perpetual reinsertion terminates after four rounds.
    // ------------------=
    #[test]
    fn teardown_executes_callbacks_and_bounds_repopulation() {
        CALLBACKS.store(0, Ordering::SeqCst);
        let mut keys = Keys::<2>::new();
        let mut values = Values::new();
        let key = keys.create(Some(counting_destructor)).unwrap();
        let plain = keys.create(None).unwrap();
        keys.set(&mut values, key, 1).unwrap();
        keys.set(&mut values, plain, 9).unwrap();
        let mut teardown = Teardown::new();
        while let Some((callback, pointer)) = teardown.next(&keys, &mut values) {
            assert_eq!(keys.get(&values, key), Ok(0));
            unsafe { callback(pointer as *mut u8); }
            keys.set(&mut values, key, 1).unwrap();
        }
        assert_eq!(CALLBACKS.load(Ordering::SeqCst), 4);
        assert_eq!(keys.get(&values, key), Ok(0));
        assert_eq!(keys.get(&values, plain), Ok(0));
        assert!(teardown.next(&keys, &mut values).is_none());
        assert!(Teardown::new().next(&Keys::<0>::new(), &mut Values::new()).is_none());
    }
    // ------------------------=
    // FUNC: destructor
    // DESC: Provides a nonexecuted callback identity for registry ownership tests.
    // ------------------=
    unsafe extern "C" fn destructor(_: *mut u8) {}
    // ------------------------=
    // FUNC: thread_values_are_independent_and_reused_keys_are_clean
    // DESC: Verifies separation, capacity, revocation and stale-generation rejection.
    // ------------------=
    #[test]
    fn thread_values_are_independent_and_reused_keys_are_clean() {
        let mut keys = Keys::<1>::new();
        let mut a = Values::new();
        let mut b = Values::new();
        let first = keys.create(None).unwrap();
        keys.set(&mut a, first, 11).unwrap();
        keys.set(&mut b, first, 22).unwrap();
        assert_eq!(keys.get(&a, first), Ok(11));
        assert_eq!(keys.get(&b, first), Ok(22));
        assert_eq!(keys.create(None), Err(Error::Full));
        keys.destroy(first).unwrap();
        let second = keys.create(None).unwrap();
        assert_ne!(first, second);
        assert_eq!(keys.get(&a, second), Ok(0));
        assert_eq!(keys.get(&b, second), Ok(0));
        assert_eq!(keys.set(&mut a, first, 99), Err(Error::InvalidKey));
        assert_eq!(Keys::<0>::new().create(None), Err(Error::Full));
    }
    // ------------------------=
    // FUNC: destructor_values_clear_before_callback_and_can_be_repopulated
    // DESC: Ensures callbacks can reenter TLS and receive a new bounded teardown pass.
    // ------------------=
    #[test]
    fn destructor_values_clear_before_callback_and_can_be_repopulated() {
        let mut keys = Keys::<2>::new();
        let mut values = Values::new();
        let key = keys.create(Some(destructor)).unwrap();
        keys.set(&mut values, key, 42).unwrap();
        assert_eq!(keys.take_destructor(&mut values, key % 2).unwrap().1, 42);
        assert_eq!(keys.get(&values, key), Ok(0));
        assert!(keys.take_destructor(&mut values, key % 2).is_none());
        keys.set(&mut values, key, 43).unwrap();
        assert_eq!(keys.take_destructor(&mut values, key % 2).unwrap().1, 43);
    }
}
