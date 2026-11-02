//! Native layout recursion budget. No allocation, cached worker identity, or yielding.
//! This covers selected construction edges, not arbitrary stack writes or large frames.
#![allow(unsafe_code)]

pub(crate) const RESERVE: usize = 512 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BudgetError {
    Unavailable,
    InvalidBounds,
    OutsideStack,
    Insufficient,
}

unsafe extern "C" {
    fn infinity_std_stack_bounds(low: *mut usize, high: *mut usize) -> i32;
}

// ------------------------=
// FUNC: stack_pointer
// DESC: Samples the executing native stack without a call, allocation, or fabricated local-address estimate.
// ------------------=
#[inline(always)]
fn stack_pointer() -> usize {
    let sp: usize;
    #[cfg(target_arch = "aarch64")]
    unsafe { core::arch::asm!("mov {}, sp", out(reg) sp, options(nomem, nostack, preserves_flags)); }
    #[cfg(target_arch = "x86_64")]
    unsafe { core::arch::asm!("mov {}, rsp", out(reg) sp, options(nomem, nostack, preserves_flags)); }
    sp
}

// ------------------------=
// FUNC: budget
// DESC: Validates the entire lease before subtracting and preserves a fixed emergency reserve below SP.
// ------------------=
fn budget(low: usize, high: usize, sp: usize) -> Result<(), BudgetError> {
    if low >= high { return Err(BudgetError::InvalidBounds); }
    if sp < low || sp > high { return Err(BudgetError::OutsideStack); }
    if sp - low < RESERVE { return Err(BudgetError::Insufficient); }
    Ok(())
}

// ------------------------=
// FUNC: check_current
// DESC: Resolves the current executor lease on every check, including after Rayon moves work to another stack.
// ------------------=
#[inline(never)]
pub(crate) fn check_current() -> Result<(), BudgetError> {
    let mut low = 0;
    let mut high = 0;
    // SAFETY: Both outputs are live locals; the owner-local provider never yields.
    if unsafe { infinity_std_stack_bounds(&mut low, &mut high) } != 0 {
        return Err(BudgetError::Unavailable);
    }
    budget(low, high, stack_pointer())
}

// ------------------------=
// FUNC: require
// DESC: Stops at a checked call edge; the native panic hook quarantines the engine without unwinding or resuming layout.
// ------------------=
#[inline(never)]
pub(crate) fn require() {
    if check_current().is_err() {
        // Static payload: no formatting or heap work before the installed fatal hook.
        panic!("Native layout stack budget exhausted");
    }
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;
    use std::cell::Cell;

    std::thread_local! {
        static REMAINING: Cell<Option<usize>> = const { Cell::new(None) };
    }

    // ------------------------=
    // FUNC: infinity_std_stack_bounds
    // DESC: Supplies per-test-thread leases around the real SP without writing or exhausting stack memory.
    // ------------------=
    #[no_mangle]
    unsafe extern "C" fn infinity_std_stack_bounds(low: *mut usize, high: *mut usize) -> i32 {
        let Some(remaining) = REMAINING.get() else { return 95; };
        let sp = stack_pointer();
        unsafe { low.write(sp - remaining); high.write(sp + RESERVE); }
        0
    }

    // ------------------------=
    // FUNC: exact_budget_boundaries_and_invalid_leases
    // DESC: Checks typed outcomes at the reserve boundary, outside the lease, and near address-space limits.
    // ------------------=
    #[test]
    fn exact_budget_boundaries_and_invalid_leases() {
        let low = 4096;
        let high = low + 2 * RESERVE;
        assert_eq!(budget(low, high, low + RESERVE), Ok(()));
        assert_eq!(budget(low, high, low + RESERVE - 1), Err(BudgetError::Insufficient));
        assert_eq!(budget(low, high, low - 1), Err(BudgetError::OutsideStack));
        assert_eq!(budget(low, high, high + 1), Err(BudgetError::OutsideStack));
        assert_eq!(budget(high, low, high), Err(BudgetError::InvalidBounds));
        assert_eq!(budget(low, low, low), Err(BudgetError::InvalidBounds));
        assert_eq!(budget(usize::MAX - RESERVE, usize::MAX, usize::MAX), Ok(()));
        assert_eq!(budget(usize::MAX - 10, usize::MAX, usize::MAX), Err(BudgetError::Insufficient));
    }

    // ------------------------=
    // FUNC: checks_refresh_identity_and_revocation
    // DESC: Exercises the real ABI call and SP reader, including lease changes and an unavailable owner.
    // ------------------=
    #[test]
    fn checks_refresh_identity_and_revocation() {
        REMAINING.set(Some(2 * RESERVE));
        assert_eq!(check_current(), Ok(()));
        std::thread::spawn(|| {
            assert_eq!(check_current(), Err(BudgetError::Unavailable));
            REMAINING.set(Some(16 * 1024));
            assert_eq!(check_current(), Err(BudgetError::Insufficient));
        }).join().unwrap();
        assert_eq!(check_current(), Ok(()));
        REMAINING.set(None);
        assert_eq!(check_current(), Err(BudgetError::Unavailable));
    }

    // ------------------------=
    // FUNC: rejected_small_stack_never_enters_callee
    // DESC: Runs the production gate on a small host stack with bounded mocked capacity, without causing overflow.
    // ------------------=
    #[test]
    fn rejected_small_stack_never_enters_callee() {
        std::thread::Builder::new().stack_size(64 * 1024).spawn(|| {
            REMAINING.set(Some(16 * 1024));
            let entered = Cell::new(false);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                require();
                entered.set(true);
            }));
            assert!(result.is_err());
            assert!(!entered.get());
        }).unwrap().join().unwrap();
    }

    // ------------------------=
    // FUNC: admitted_stack_enters_callee
    // DESC: Verifies that sufficient capacity preserves normal execution through the same gate.
    // ------------------=
    #[test]
    fn admitted_stack_enters_callee() {
        REMAINING.set(Some(2 * RESERVE));
        require();
        assert_eq!(check_current(), Ok(()));
    }
}
