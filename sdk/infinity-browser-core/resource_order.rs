//! Stable priority policy for the bounded, serialized native transport queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority { Document, Stylesheet, Critical, Font, Other }

// ------------------------=
// FUNC: classify
// DESC: Uses the engine's Fetch destination metadata to prioritize parsing and layout dependencies.
// ------------------=
pub fn classify(document:bool,destination:&str)->Priority {
    if document {return Priority::Document;}
    match destination {
        "document"|"iframe"|"frame"=>Priority::Document,
        "style"=>Priority::Stylesheet,
        "script"|""=>Priority::Critical,
        "font"=>Priority::Font,
        _=>Priority::Other,
    }
}

// ------------------------=
// FUNC: insertion_index
// DESC: Preserves active transfers and equal-priority FIFO order while advancing higher-priority queued work.
// ------------------=
pub fn insertion_index(mut queue:impl ExactSizeIterator<Item=(bool,Priority)>,priority:Priority)->usize {
    let length=queue.len();
    queue.position(|(active,current)|!active && current>priority).unwrap_or(length)
}

// ------------------------=
// FUNC: eviction_index
// DESC: Selects the least important queued request so parsing dependencies survive resource-heavy pages.
// ------------------=
pub fn eviction_index(queue:impl Iterator<Item=(bool,Priority)>,incoming:Priority)->Option<usize> {
    queue.enumerate().filter_map(|(index,(active,current))|
        (!active && current>incoming).then_some(index)).last()
}

// ------------------------=
// FUNC: retry_pre_header
// DESC: Retries bounded idempotent requests that fail before any response metadata reaches the engine.
// ------------------=
pub fn retry_pre_header(method:&str,failures:u8)->bool {
    matches!(method,"GET"|"HEAD"|"OPTIONS") && failures<2
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    // ------------------------=
    // FUNC: dispatch_order_preserves_active_work_and_fifo
    // DESC: Applies actual queue insertions and verifies document/critical precedence without replacing in-flight work.
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
        assert_eq!(classify(false,""),Priority::Critical);
        assert_eq!(eviction_index(queue.iter().map(|&(active,p,_)|(active,p)),Priority::Critical),Some(6));
        assert_eq!(eviction_index(queue.iter().map(|&(active,p,_)|(active,p)),Priority::Other),None);
        assert!(retry_pre_header("GET",0));
        assert!(retry_pre_header("GET",1));
        assert!(!retry_pre_header("GET",2));
        assert!(!retry_pre_header("POST",0));
    }

    // ------------------------=
    // FUNC: embedded_documents_and_styles_bypass_script_backlog
    // DESC: Keeps challenge frames and render-blocking CSS ahead of queued application scripts without preempting live work.
    // ------------------=
    #[test]
    fn embedded_documents_and_styles_bypass_script_backlog() {
        let mut queue=std::vec![(true,Priority::Critical,0)];
        for id in 1..30 {queue.push((false,classify(false,"script"),id));}
        for (id,destination) in [(30,"style"),(31,"iframe")] {
            let priority=classify(false,destination);
            let at=insertion_index(queue.iter().map(|&(active,p,_)|(active,p)),priority);
            queue.insert(at,(false,priority,id));
        }
        assert_eq!(queue.iter().take(3).map(|x|x.2).collect::<std::vec::Vec<_>>(),[0,31,30]);
        assert_eq!(queue.len(),32);
    }
}
