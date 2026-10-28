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
