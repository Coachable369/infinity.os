#[path = "../kernel/ui/disk_view.rs"]
mod disk_view;
// ------------------------=
// FUNC: main
// DESC: Exercises partition-map gaps, exact boundaries, physical ordering and wide extents.
// ------------------=
fn main() {
    let mut view=disk_view::DiskView::new();
    assert_eq!(view.partition_at(0),None);
    view.push(b"second",1024,8,[0;16],true);
    view.push(b"first",512,2,[0;16],false);
    assert_eq!(view.partition_at(1023),None);
    assert_eq!(view.partition_at(1024),Some(1));
    assert_eq!(view.partition_at(1535),Some(1));
    assert_eq!(view.partition_at(1536),None);
    assert_eq!(view.partition_at(4096),Some(0));
    assert_eq!(view.partition_at(5120),None);
    view.push(b"large",u64::MAX,u64::MAX,[0;16],false);
    assert_eq!(view.partition_at(u128::from(u64::MAX)*512),Some(2));
}
