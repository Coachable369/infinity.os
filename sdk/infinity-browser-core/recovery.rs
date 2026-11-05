//! Restart boundary for a single-owner component whose entire mutable image and
//! heap are private. No component reference may escape into the supervisor.
#[cfg(target_os = "none")]
#[path = "../servo-runtime-primitives/context.rs"]
mod context;
#[cfg(target_os = "none")]
use core::ptr;

#[cfg(target_os = "none")]
#[repr(C, align(16))]
pub struct Boundary<const N: usize> {
    root: context::Context,
    abandoned: context::Context,
    stack: [u8; N],
    result: u32,
    running: bool,
}
#[cfg(target_os = "none")]
impl<const N: usize> Boundary<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves a stationary supervisor boundary and private component entry stack.
    // ------------------=
    pub const fn new() -> Self {
        Self { root: context::Context::empty(), abandoned: context::Context::empty(),
            stack: [0; N], result: 0, running: false }
    }
    // ------------------------=
    // FUNC: enter
    // DESC: Enters a fresh component stack and returns only after that generation is permanently abandoned.
    // ------------------=
    /// Caller owns this stationary boundary on one CPU. Entry must terminate via
    /// leave, never unwind, and never retain supervisor references or spawn work
    /// outside its private cooperative runtime. An old stack is never resumed.
    pub unsafe fn enter(this: *mut Self, entry: extern "C" fn(usize) -> !, argument: usize) -> Option<u32> {
        if (*this).running { return None; }
        (*this).stack.fill(0);
        if !(*this).abandoned.initialize_argument(&mut (*this).stack, entry, argument) { return None; }
        (*this).result = 0;
        (*this).running = true;
        context::switch(ptr::addr_of_mut!((*this).root), ptr::addr_of!((*this).abandoned));
        (*this).running = false;
        Some((*this).result)
    }
    // ------------------------=
    // FUNC: leave
    // DESC: Discards the entire component call stack without destructors and resumes its saved supervisor.
    // ------------------=
    /// Only the current component or its cooperative child may call this. Its
    /// private heap and complete mutable image must be reset before reentry.
    pub unsafe fn leave(this: *mut Self, result: u32) -> ! {
        if (*this).running {
            (*this).result = result;
            context::switch(ptr::addr_of_mut!((*this).abandoned), ptr::addr_of!((*this).root));
        }
        loop { core::hint::spin_loop(); }
    }
}

pub struct Image<const N: usize> { bytes: [u8; N], length: Option<usize> }
impl<const N: usize> Image<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves a bounded pristine initialized-image checkpoint outside the component.
    // ------------------=
    pub const fn new() -> Self { Self { bytes: [0; N], length: None } }
    // ------------------------=
    // FUNC: prepare
    // DESC: Captures initial relocated data once and restores it plus zeroed private BSS and heap on subsequent entries.
    // ------------------=
    /// Caller must have exclusive ownership of all three disjoint ranges, with
    /// every prior component continuation abandoned and external IO drained.
    pub fn prepare(&mut self, data: &mut [u8], bss: &mut [u8], heap: &mut [u8]) -> bool {
        if data.len() > N || self.length.is_some_and(|length| length != data.len()) { return false; }
        if self.length.is_some() {
            data.copy_from_slice(&self.bytes[..data.len()]);
            bss.fill(0);
            heap.fill(0);
        } else {
            self.bytes[..data.len()].copy_from_slice(data);
            self.length = Some(data.len());
        }
        true
    }
}

#[cfg(test)] mod tests {
    use super::*;
    // ------------------------=
    // FUNC: abandon
    // DESC: Simulates a fatal callback from the component entry stack without returning into it.
    // ------------------=
    #[cfg(target_os = "none")]
    extern "C" fn abandon(pointer: usize) -> ! { unsafe { Boundary::leave(pointer as *mut Boundary<65536>, 4) } }
    // ------------------------=
    // FUNC: repeated_private_image_reclamation
    // DESC: Requires repeated abandoned-stack recovery and restoration of private bytes without changing neighboring state.
    // ------------------=
    #[test]
    fn repeated_private_image_reclamation() {
        #[cfg(target_os = "none")]
        let mut boundary = Boundary::<65536>::new();
        let mut image = Image::<16>::new();
        let mut data = [1, 2, 3, 4]; let mut bss = [0; 8]; let mut heap = [0; 32];
        assert!(image.prepare(&mut data, &mut bss, &mut heap));
        for _ in 0..12 {
            data.fill(9); bss.fill(8); heap.fill(7);
            #[cfg(target_os = "none")] {
                let pointer = &mut boundary as *mut _;
                assert_eq!(unsafe { Boundary::enter(pointer, abandon, pointer as usize) }, Some(4));
            }
            assert!(image.prepare(&mut data, &mut bss, &mut heap));
            assert_eq!(data, [1, 2, 3, 4]); assert_eq!(bss, [0; 8]); assert_eq!(heap, [0; 32]);
        }
        assert!(!image.prepare(&mut [0; 17], &mut bss, &mut heap));
        assert!(!image.prepare(&mut [0; 3], &mut bss, &mut heap));
    }
}
