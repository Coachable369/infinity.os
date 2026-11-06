//! Stable priority policy for the bounded, serialized native transport queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority { Document, Stylesheet, Critical, Font, Other }

// ------------------------=
// FUNC: dispatch_index
// DESC: Keeps parsing dependencies first but gives visual resources a turn after three script/API dispatches.
// ------------------=
pub fn dispatch_index(queue:impl Iterator<Item=(bool,Priority)>,critical_streak:u8,last_visual:Priority)->usize {
    let mut queue=queue.enumerate();
    let Some((_,(active,first)))=queue.next() else {return 0;};
    if active || first!=Priority::Critical || critical_streak<3 {return 0;}
    let preferred=if last_visual==Priority::Font {Priority::Other} else {Priority::Font};
    let mut fallback=None;
    for (index,(active,priority)) in queue {
        if active || priority<=Priority::Critical {continue;}
        if priority==preferred {return index;}
        fallback.get_or_insert(index);
    }
    fallback.unwrap_or(0)
}

// ------------------------=
// FUNC: dispatched
// DESC: Tracks bounded script bursts independently of transfer chunk count or engine pump frequency.
// ------------------=
pub fn dispatched(streak:u8,priority:Priority)->u8 {
    if priority==Priority::Critical {streak.saturating_add(1)} else {0}
}

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
    // FUNC: visual_resources_progress_during_script_backlog
    // DESC: Dispatches a populated queue while adding scripts and verifies fonts and images cannot be starved.
    // ------------------=
    #[test]
    fn visual_resources_progress_during_script_backlog() {
        let mut queue=std::vec![Priority::Critical;100];
        queue.extend([Priority::Font,Priority::Other,Priority::Other]);
        let mut streak=0;
        let mut last_visual=Priority::Other;
        let mut visual=std::vec::Vec::new();
        for turn in 0..12 {
            let at=dispatch_index(queue.iter().map(|&p|(false,p)),streak,last_visual);
            let priority=queue.remove(at);
            streak=dispatched(streak,priority);
            if priority>Priority::Critical {visual.push((turn,priority));last_visual=priority;}
            let at=insertion_index(queue.iter().map(|&p|(false,p)),Priority::Critical);
            queue.insert(at,Priority::Critical);
        }
        assert_eq!(visual,[(3,Priority::Font),(7,Priority::Other),(11,Priority::Other)]);
        assert_eq!(dispatch_index([(true,Priority::Critical),(false,Priority::Other)].into_iter(),3,Priority::Other),0);
        assert_eq!(dispatch_index([(false,Priority::Stylesheet),(false,Priority::Other)].into_iter(),3,Priority::Other),0);
        assert_eq!(dispatch_index(core::iter::empty(),3,Priority::Other),0);
        let mut queue=std::vec![Priority::Critical;100];
        queue.extend([Priority::Font;100]);
        queue.push(Priority::Other);
        let at=dispatch_index(queue.iter().map(|&p|(false,p)),3,Priority::Font);
        assert_eq!(queue.remove(at),Priority::Other);
    }
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
