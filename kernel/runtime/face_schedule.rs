//! Bounded face-worker dispatch batching; input is serviced between batches.
// ------------------------=
// FUNC: batch
// DESC: Amortizes short syscalls with both elapsed-time and call-count limits, stopping immediately at terminal state.
// ------------------=
pub fn batch<T>(mut clock:impl FnMut()->u64,mut step:impl FnMut(u64)->Option<T>)->Option<T> {
    let start=clock();
    for _ in 0..32 {
        let now=clock();
        if now<start || now-start>=4_000_000 {break;}
        if let Some(result)=step(now) {return Some(result);}
    }
    None
}
// ------------------------=
// FUNC: reclaim
// DESC: Batches bounded 64-KiB private-memory wipes, yielding within four milliseconds instead of one repaint per chunk.
// ------------------=
pub fn reclaim<T>(mut clock:impl FnMut()->u64,mut step:impl FnMut(u64)->Option<T>)->Option<T> {
    let start=clock();
    for _ in 0..128 {
        let now=clock();
        if now<start || now-start>=4_000_000 {break;}
        if let Some(result)=step(now) {return Some(result);}
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    // ------------------------=
    // FUNC: computation_yields_to_input
    // DESC: A two-millisecond native slice cannot monopolize the event loop.
    // ------------------=
    #[test]
    fn computation_yields_to_input() {
        let now=Cell::new(0);let mut visits=0;
        let result:Option<()>=batch(||now.get(),|_|{visits+=1;now.set(now.get()+2_000_000);None});
        assert_eq!(result,None);assert_eq!(visits,2);
    }
    // ------------------------=
    // FUNC: short_calls_are_bounded_and_terminal_stops
    // DESC: Fast traps get amortized, a frozen clock is bounded, and terminal tasks are never reentered.
    // ------------------=
    #[test]
    fn short_calls_are_bounded_and_terminal_stops() {
        let mut visits=0;
        let result:Option<()>=batch(||10,|_|{visits+=1;None});
        assert_eq!(result,None);assert_eq!(visits,32);
        let mut visits=0;
        assert_eq!(batch(||10,|_|{visits+=1;Some(7)}),Some(7));assert_eq!(visits,1);
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    // ------------------------=
    // FUNC: cleanup_is_batched_and_time_bounded
    // DESC: Verifies 128-MiB cleanup needs 16 visits rather than 2048, while yielding at the four-millisecond budget.
    // ------------------=
    #[test]
    fn cleanup_is_batched_and_time_bounded() {
        let mut chunks=2048usize;let mut visits=0;
        loop {visits+=1;if reclaim(||0, |_|{chunks-=1;(chunks==0).then_some(())}).is_some(){break;}}
        assert_eq!(visits,16);
        let time=std::cell::Cell::new(0u64);let mut work=0;
        assert_eq!(reclaim(||time.get(), |_|{work+=1;time.set(time.get()+1_000_000);None::<()>}),None);
        assert_eq!(work,4);
    }
}
