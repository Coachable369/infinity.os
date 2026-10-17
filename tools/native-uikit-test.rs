#[path = "../kernel/ui/native_uikit.rs"]
mod native_uikit;

// ------------------------=
// FUNC: element
// DESC: Creates one valid test element without bypassing the production state API.
// ------------------=
fn element(id:u32,kind:native_uikit::ElementKind,x:u32,y:u32)->native_uikit::Element {
    let mut label=[0u8;native_uikit::LABEL_CAPACITY];label[..3].copy_from_slice(b"Add");
    native_uikit::Element{id,kind,x,y,width:90,height:48,label,label_length:3}
}

// ------------------------=
// FUNC: main
// DESC: Verifies declarative layout, stable updates, hit testing, event order, bounds, and lifecycle behavior.
// ------------------=
fn main() {
    native_uikit::close();
    assert!(!native_uikit::open(100,420,b"Too narrow"));
    assert!(native_uikit::open(480,420,b"Calculator"));
    let opened=unsafe{native_uikit::INFINITY_NATIVE_UI_SNAPSHOT};
    assert_eq!(opened[3..6],[1,480,420]);
    assert!(native_uikit::upsert(element(7,native_uikit::ElementKind::Button,20,100)));
    let mut updated=element(7,native_uikit::ElementKind::Button,30,110);
    updated.label[..5].copy_from_slice(b"Clear");updated.label_length=5;
    assert!(native_uikit::upsert(updated));
    assert!(!native_uikit::upsert(element(8,native_uikit::ElementKind::Button,450,400)));
    let snapshot=native_uikit::presentation();
    assert!(snapshot.active);assert_eq!(snapshot.elements.iter().flatten().count(),1);
    let published=unsafe{native_uikit::INFINITY_NATIVE_UI_SNAPSHOT};
    assert_eq!(published[6],1);assert_eq!(published[7],3);
    assert!(!native_uikit::activate(20,100));
    assert!(native_uikit::activate(35,115));
    let activated=unsafe{native_uikit::INFINITY_NATIVE_UI_SNAPSHOT};
    assert_eq!(activated[8..10],[1,7]);
    assert_eq!(native_uikit::poll(),Some(native_uikit::Event{kind:1,id:7,value:0}));
    assert!(native_uikit::request_close());
    assert_eq!(native_uikit::poll(),Some(native_uikit::Event{kind:2,id:0,value:0}));
    native_uikit::close();
    assert!(native_uikit::open(640,640,b"Chess"));
    for id in 1..=native_uikit::MAX_ELEMENTS as u32 {
        let x=((id-1)%8)*64;let y=((id-1)/8)*48;
        assert!(native_uikit::upsert(element(id,native_uikit::ElementKind::Button,x,y)));
    }
    assert_eq!(native_uikit::presentation().elements.iter().flatten().count(),96);
    assert!(!native_uikit::upsert(element(97,native_uikit::ElementKind::Button,0,590)));
    native_uikit::close();assert!(!native_uikit::presentation().active);
    assert_eq!(unsafe{native_uikit::INFINITY_NATIVE_UI_SNAPSHOT}[3],0);
}
