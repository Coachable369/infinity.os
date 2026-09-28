// ------------------------=
// FUNC: adjacent_index
// DESC: Shares active-adjacent insertion between the native shell and Servo owner thread.
// ------------------=
pub fn adjacent_index(ids: impl Iterator<Item=u32>, active:u32)->usize {
    let mut count=0;
    for id in ids {count+=1;if id==active {return count;}}
    count
}

#[cfg(test)]
mod tests {
    // ------------------------=
    // FUNC: new_tab_is_adjacent_without_reordering_existing_tabs
    // DESC: Verifies empty, first, middle, last and stale selection insertion positions.
    // ------------------=
    #[test]
    fn new_tab_is_adjacent_without_reordering_existing_tabs() {
        assert_eq!(super::adjacent_index([].into_iter(),0),0);
        for (active,expected) in [(10,1),(20,2),(30,3),(99,3)] {
            assert_eq!(super::adjacent_index([10,20,30].into_iter(),active),expected);
        }
    }
}
