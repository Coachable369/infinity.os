//! Stable priority policy for the bounded, serialized native transport queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority { Document, Style, Script, Font, Other }

// ------------------------=
// FUNC: classify
// DESC: Uses the engine's Fetch destination metadata to prioritize parsing and layout dependencies.
// ------------------=
pub fn classify(document:bool,destination:&str)->Priority {
    if document {return Priority::Document;}
    match destination {"style"=>Priority::Style,"script"|""=>Priority::Script,"font"=>Priority::Font,_=>Priority::Other}
}

// ------------------------=
// FUNC: insertion_index
// DESC: Preserves active transfers and equal-priority FIFO order while advancing higher-priority queued work.
// ------------------=
pub fn insertion_index(mut queue:impl ExactSizeIterator<Item=(bool,Priority)>,priority:Priority)->usize {
    let length=queue.len();
    queue.position(|(active,current)|!active && current>priority).unwrap_or(length)
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    // ------------------------=
    // FUNC: dispatch_order_preserves_active_work_and_fifo
    // DESC: Applies actual queue insertions and verifies document/style precedence without replacing in-flight work.
    // ------------------=
    #[test]
    fn dispatch_order_preserves_active_work_and_fifo() {
        let mut queue=std::vec![(true,Priority::Other,0)];
        for (id,document,destination) in [(1,false,"image"),(2,false,"font"),(3,false,"script"),(4,false,"style"),(5,true,"document"),(6,false,"style")] {
            let priority=classify(document,destination);
            let at=insertion_index(queue.iter().map(|&(active,p,_)|(active,p)),priority);
            queue.insert(at,(false,priority,id));
        }
        assert_eq!(queue.iter().map(|&(_,_,id)|id).collect::<std::vec::Vec<_>>(),[0,5,4,6,3,2,1]);
        assert_eq!(insertion_index(core::iter::empty(),Priority::Document),0);
        assert_eq!(classify(false,""),Priority::Script);
    }
}
