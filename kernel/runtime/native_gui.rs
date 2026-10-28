//! Trusted, bounded native GUI authority for one authenticated application.
//! The host supplies identity/context; requests cannot choose either authority.
use super::{capability::{CapabilityId, CapabilityType}, execution::SecurityIdentity,
    native_memory::{NativeMemory, PageTable}, native_surface::NativeSurfaces, InfinityRuntime};
use crate::ui::{geometry::{Point, Rect}, window::{ContextId, SurfaceId, WindowId, Window, WindowState}};

const MAX_GRANTS: usize = 96;
const MAX_LEASES: usize = 24;

/// Last presented native footprints, independent of ordinary console content.
/// Pixel-only publications need no backdrop reconstruction; moves, resize,
/// withdrawal, destruction and stacking changes do.
pub struct SceneDamage { previous: [Option<(WindowId, Rect)>; 8] }

impl SceneDamage {
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no previously presented native windows.
    // ------------------=
    pub const fn new() -> Self { Self { previous: [None; 8] } }

    // ------------------------=
    // FUNC: update
    // DESC: Returns clipped old/new chrome-and-client damage, including destroyed windows and edge grab indicators, without invalidating unchanged pixel-only updates.
    // ------------------=
    pub fn update(&mut self, windows: &[Option<Window>; 8], display: Rect, scale: usize) -> Option<Rect> {
        let mut current = [None; 8];
        let mut damage: Option<Rect> = None;
        for (index, window) in windows.iter().enumerate() {
            if let Some(window) = window.filter(|w| w.state != WindowState::Withdrawn) {
                let title = title_bounds(window, scale);
                let bounds = if window.visible { window.bounds.union(title) } else { title };
                if bounds.width > 0 && bounds.height > 0 {
                    let pad = (3 * scale.max(1)) as i32;
                    let footprint = Rect { x: bounds.x.saturating_sub(pad), y: bounds.y.saturating_sub(pad),
                        width: bounds.width.saturating_add(2 * pad as u32), height: bounds.height.saturating_add(2 * pad as u32) }
                        .intersection(display);
                    if footprint.width > 0 && footprint.height > 0 { current[index] = Some((window.id, footprint)); }
                }
            }
            if current[index] != self.previous[index] {
                for (_, rect) in [current[index], self.previous[index]].into_iter().flatten() {
                    damage = Some(damage.map_or(rect, |old| old.union(rect)));
                }
            }
        }
        self.previous = current;
        damage
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChromePart { Title, Control(usize), Resize { left: bool, right: bool, top: bool, bottom: bool } }
#[derive(Clone, Copy)]
struct ChromeDrag { window: WindowId, part: ChromePart, start: Point, bounds: Rect }

// ------------------------=
// FUNC: title_bounds
// DESC: Places shared OS chrome outside native client pixels; minimized windows reside only in the tray.
// ------------------=
pub fn title_bounds(window: Window, scale: usize) -> Rect {
    let height = if matches!(window.state,WindowState::Fullscreen|WindowState::Withdrawn|WindowState::Minimized) { 0 } else { 44 * scale.max(1) as u32 };
    Rect { x: window.bounds.x, y: window.bounds.y.saturating_sub(height as i32), width: window.bounds.width, height }
}

// ------------------------=
// FUNC: client_work_area
// DESC: Reserves top bar, recoverable title and dock space while allowing clients to move beyond either side.
// ------------------=
pub fn client_work_area(display: Rect, scale: usize) -> Rect {
    let s = scale.max(1) as u32;
    let top = (88 * s).min(display.height / 3);
    Rect { x: display.x, y: display.y + top as i32, width: display.width,
        height: display.height.saturating_sub(top).saturating_sub((76 * s).min(display.height / 3)) }
}

// ------------------------=
// FUNC: chrome_hit
// DESC: Shares exact authored controls and exterior side/corner resize hit areas without intercepting client pixels or top-edge drags.
// ------------------=
fn chrome_hit(window: Window, point: Point, scale: usize) -> Option<ChromePart> {
    if matches!(window.state,WindowState::Fullscreen|WindowState::Withdrawn) { return None; }
    let title = title_bounds(window, scale);
    if title.contains(point) {
        if let Some(index) = crate::ui::app_launcher::window_controls::hit(crate::ui::app_launcher::window_controls::bounds(title, title.height, scale), point) {
            return Some(ChromePart::Control(index));
        }
        return Some(ChromePart::Title);
    }
    if !window.visible || window.state != WindowState::Normal { return None; }
    let edge = 6 * scale.max(1) as i32;
    let b = window.bounds;
    if point.x < b.x - edge || point.x >= b.right() + edge || point.y < title.y - edge || point.y >= b.bottom() + edge { return None; }
    let left = point.x < b.x;
    let right = point.x >= b.right();
    let top = (left || right) && point.y < title.y + edge;
    let bottom = point.y >= b.bottom();
    if left || right || bottom { Some(ChromePart::Resize { left, right, top, bottom }) } else { None }
}

pub struct Gui {
    pub surfaces: NativeSurfaces,
    owner: Option<(SecurityIdentity, ContextId)>,
    grants: [Option<CapabilityId>; MAX_GRANTS],
    leases: [Option<(u64, CapabilityId)>; MAX_LEASES],
    revision: u64,
    hover: Option<WindowId>,
    chrome_drag: Option<ChromeDrag>,
}

impl Gui {
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no authority, subscriptions, retained pixels or bound application identity.
    // ------------------=
    pub const fn new() -> Self {
        Self { surfaces: NativeSurfaces::new(), owner: None,
            grants: [None; MAX_GRANTS], leases: [None; MAX_LEASES], revision: 0, hover: None, chrome_drag: None }
    }
    // ------------------------=
    // FUNC: revision
    // DESC: Exposes committed mutation revisions so the desktop can request a repaint without polling pixel contents.
    // ------------------=
    pub const fn revision(&self) -> u64 { self.revision }
    // ------------------------=
    // FUNC: context
    // DESC: Returns the trusted context bound to this host, never an application-supplied owner number.
    // ------------------=
    pub const fn context(&self) -> Option<ContextId> {
        match self.owner { Some((_, owner)) => Some(owner), None => None }
    }
    // ------------------------=
    // FUNC: pointer_target
    // DESC: Records the trusted input router's current target and restores the OS pointer when leaving this application's windows.
    // ------------------=
    pub fn pointer_target(&mut self, target: Option<WindowId>) {
        self.hover = target;
        if target.is_none() { crate::ui::text_input::set_pointer_shape(crate::ui::text_input::PointerShape::Default); }
    }
    // ------------------------=
    // FUNC: retire_unused_grants
    // DESC: Reclaims authority for destroyed owned resources after their final event lease ends, preserving queued destroy notifications until read or unsubscribed.
    // ------------------=
    fn retire_unused_grants(&mut self,runtime:&mut InfinityRuntime,caller:SecurityIdentity) {
        for lease in &mut self.leases {
            if lease.is_some_and(|(id,_)|runtime.events.queue_depth(id).is_none()) {*lease=None;}
        }
        for slot in &mut self.grants {
            let Some(id)=*slot else{continue;};
            let Some(cap)=runtime.capabilities.get(id) else{*slot=None;continue;};
            let absent=match cap.kind {
                CapabilityType::SurfacePublish|CapabilityType::SurfaceResize|CapabilityType::SurfaceDestroy=>runtime.ui.surfaces.inspect(SurfaceId(cap.target as u32)).is_none(),
                CapabilityType::WindowManageOwn|CapabilityType::WindowFocus|CapabilityType::WindowCaptureInput=>runtime.ui.windows.inspect(WindowId(cap.target as u32)).is_none(),
                CapabilityType::EventSubscribe=>cap.constraints!=0&&runtime.ui.windows.inspect(WindowId(cap.constraints as u32)).is_none()
                    && !self.leases.iter().flatten().any(|(_,grant)|*grant==id),
                _=>false,
            };
            if absent {let _=runtime.capabilities.revoke(id);let _=runtime.capabilities.retire_leaf(id,caller);*slot=None;}
        }
    }
    // ------------------------=
    // FUNC: desktop_pointer
    // DESC: Handles trusted native window chrome with press/release capture; returns client routing separately so title input never reaches widgets.
    // ------------------=
    pub fn desktop_pointer(&mut self, runtime: &mut InfinityRuntime, point: Point,
        phase: crate::ui::input::PointerPhase, button: u8, display: Rect, scale: usize, now: u64) -> (bool, Option<WindowId>) {
        use crate::ui::input::PointerPhase as P;
        use crate::ui::text_input::{set_pointer_shape, PointerShape as S};
        let Some(owner) = self.context() else { return (false, None); };
        if self.chrome_drag.is_none() {
            if let Some(id)=runtime.ui.windows.captured_pointer().filter(|id|runtime.ui.windows.inspect(*id).is_some_and(|w|w.owner==owner)) {
                self.pointer_target(Some(id)); return (false,Some(id));
            }
        }
        let work = client_work_area(display, scale);
        let mut hit = None;
        for index in (0..runtime.ui.windows.count()).rev() {
            let Some(window) = runtime.ui.windows.ordered_window(index).copied().filter(|w| w.owner == owner) else { continue; };
            if let Some(part) = chrome_hit(window, point, scale) { hit = Some((window, Some(part))); break; }
            if window.visible && window.bounds.contains(point) { hit = Some((window, None)); break; }
        }
        let highlight = hit.and_then(|(w, part)| match part {
            Some(ChromePart::Control(index)) => Some(crate::ui::app_launcher::window_controls::Highlight {
                rect: crate::ui::app_launcher::window_controls::bounds(title_bounds(w,scale), title_bounds(w,scale).height, scale)[index],
                pressed: self.chrome_drag.is_some_and(|d| d.window == w.id && d.part == ChromePart::Control(index)),
            }), _ => None,
        });
        if crate::ui::app_launcher::window_controls::publish(highlight) { self.revision = self.revision.wrapping_add(1); }
        if let Some(drag) = self.chrome_drag {
            if !runtime.ui.windows.inspect(drag.window).is_some_and(|w| w.owner == owner) { self.chrome_drag = None; return (false,None); }
            if phase == P::Move {
                let dx = point.x.saturating_sub(drag.start.x); let dy = point.y.saturating_sub(drag.start.y);
                match drag.part {
                    ChromePart::Title => { let _ = runtime.ui.windows.move_window(owner,drag.window,Point{x:drag.bounds.x.saturating_add(dx),y:drag.bounds.y.saturating_add(dy)},work); }
                    ChromePart::Resize { left, right, top, bottom } => {
                        let window = runtime.ui.windows.inspect(drag.window).unwrap();
                        let width = (drag.bounds.width as i64 + if left {-dx as i64} else if right {dx as i64} else {0})
                            .clamp(window.minimum_width as i64,display.width.max(window.minimum_width) as i64) as u32;
                        let height = (drag.bounds.height as i64 + if top {-dy as i64} else if bottom {dy as i64} else {0})
                            .clamp(window.minimum_height as i64,work.height.max(window.minimum_height) as i64) as u32;
                        if left || top {
                            let _ = runtime.ui.windows.move_window(owner,drag.window,Point {
                                x: if left {drag.bounds.right().saturating_sub(width as i32)} else {drag.bounds.x},
                                y: if top {drag.bounds.bottom().saturating_sub(height as i32)} else {drag.bounds.y},
                            },work);
                        }
                        let _ = runtime.ui.windows.resize_window(owner,drag.window,width,height,work);
                    }
                    ChromePart::Control(_) => {},
                }
                self.revision = self.revision.wrapping_add(1);
                runtime.flush_window_events(now,drag.window.0 as u64,drag.window.0 as u64);
            }
            if matches!(phase,P::Up|P::Cancel) {
                self.chrome_drag = None;
                if phase == P::Up && hit.is_some_and(|(w,part)|w.id==drag.window&&part==Some(drag.part)) {
                    if let ChromePart::Control(index) = drag.part {
                        let current = runtime.ui.windows.inspect(drag.window).unwrap().state;
                        if index == 2 { let _ = runtime.ui.windows.request_close(owner,drag.window); }
                        else {
                            let state = if current != WindowState::Normal {WindowState::Normal}
                                else if index == 0 {WindowState::Minimized} else {WindowState::Maximized};
                            let _ = runtime.ui.windows.set_state(owner,drag.window,state,work,display);
                        }
                    }
                }
                self.revision = self.revision.wrapping_add(1);
                runtime.flush_window_events(now,drag.window.0 as u64,drag.window.0 as u64);
            }
            self.pointer_target(None);
            return (true, Some(drag.window));
        }
        if let Some((window, part)) = hit {
            if phase == P::Down && button == 1 {
                if window.state == WindowState::Minimized && part == Some(ChromePart::Title) {
                    let _ = runtime.ui.windows.set_state(owner,window.id,WindowState::Normal,work,display);
                }
                if runtime.ui.windows.focus(window.id).is_err() {return (true,None);}
                if let Some(part) = part { self.chrome_drag = Some(ChromeDrag{window:window.id,part,start:point,bounds:window.bounds}); }
                self.revision = self.revision.wrapping_add(1);
                runtime.flush_window_events(now,window.id.0 as u64,window.id.0 as u64);
            }
            self.pointer_target(if part.is_none() {Some(window.id)} else {None});
            if let Some(ChromePart::Resize{left,right,top,bottom}) = part {
                set_pointer_shape(if (left&&top)||(right&&bottom) {S::ResizeNorthWestSouthEast}
                    else if (left&&bottom)||(right&&top) {S::ResizeNorthEastSouthWest}
                    else if left||right {S::ResizeHorizontal} else {S::ResizeVertical});
            }
            return (part.is_some(), Some(window.id));
        }
        let target = runtime.ui.windows.pointer_target(point).filter(|id|runtime.ui.windows.inspect(*id).is_some_and(|w|w.owner==owner));
        self.pointer_target(target);
        (false,target)
    }
    // ------------------------=
    // FUNC: request
    // DESC: Grants only exact native surface/window operations on the requesting context's resources, with nondelegable lifetime ownership.
    // ------------------=
    fn request(&mut self, runtime: &mut InfinityRuntime, caller: SecurityIdentity,
        owner: ContextId, category: u64, target: u64) -> i64 {
        self.retire_unused_grants(runtime,caller);
        let id = match u32::try_from(target) { Ok(value) => value, Err(_) => return -3 };
        let surface = || runtime.ui.surfaces.inspect(SurfaceId(id)).is_some_and(|s| s.owner == owner);
        let window = || runtime.ui.windows.inspect(WindowId(id)).is_some_and(|w| w.owner == owner);
        let (kind, scope, constraints) = match category {
            0 if target == 0 => (CapabilityType::SurfaceCreate, super::SERVICE_WINDOW_SERVER as u64, 0),
            1 if target == 0 => (CapabilityType::WindowCreate, super::SERVICE_WINDOW_SERVER as u64, 0),
            2 if surface() => (CapabilityType::SurfacePublish, target, 0),
            3 if surface() => (CapabilityType::SurfaceResize, target, 0),
            4 if surface() => (CapabilityType::SurfaceDestroy, target, 0),
            5 if window() => (CapabilityType::WindowManageOwn, target, 0),
            6 if window() => (CapabilityType::WindowFocus, target, 0),
            7 if window() => (CapabilityType::WindowCaptureInput, target, 0),
            8 if window() => (CapabilityType::EventSubscribe, super::EVENT_NATIVE_INPUT as u64, target),
            9 if window() => (CapabilityType::EventSubscribe, super::EVENT_WINDOW_RESIZED as u64, target),
            10 if window() => (CapabilityType::EventSubscribe, super::EVENT_WINDOW_DESTROYED as u64, target),
            11 if window() => (CapabilityType::EventSubscribe, super::EVENT_WINDOW_CLOSE_REQUESTED as u64, target),
            _ => return -1,
        };
        for grant in self.grants.iter().flatten() {
            if runtime.capabilities.get(*grant).is_some_and(|c| !c.revoked && c.holder == caller
                && c.kind == kind && c.target == scope && c.constraints == constraints) {
                return *grant as i64;
            }
        }
        let Some(slot) = self.grants.iter_mut().find(|s| s.is_none()) else { return -4; };
        match runtime.capabilities.grant(kind, scope, 1, constraints, caller, caller, None, 0) {
            Ok(id) => { *slot = Some(id); id as i64 }, Err(_) => -4,
        }
    }
    // ------------------------=
    // FUNC: dispatch
    // DESC: Binds authenticated desktop-only calls to native services; every mutation still undergoes the existing capability and memory checks.
    // ------------------=
    pub fn dispatch<T: PageTable>(&mut self, runtime: &mut InfinityRuntime, memory: &mut NativeMemory,
        tables: &mut T, caller: SecurityIdentity, owner: ContextId, authorized: bool,
        operation: u64, args: [u64; 6], work: Rect, display: Rect, now: u64) -> Option<i64> {
        if !matches!(operation, 0x1100..=0x1108 | 0x1110..=0x1112 | 0x1120..=0x1124 | 0x1251..=0x1254) { return None; }
        if !authorized || owner.0 == 0 || caller == SecurityIdentity([0;16]) { return Some(-1); }
        if self.owner.is_some_and(|bound| bound != (caller, owner)) { return Some(-1); }
        self.owner = Some((caller, owner));
        let mut args=args;
        if operation==0x1100 {
            if let Ok(y)=i32::try_from(args[3] as i64) {args[3]=y.max(work.y) as i64 as u64;}
        }
        if operation == 0x1254 {
            if args[2..].iter().any(|v| *v != 0) { return Some(-3); }
            let Ok(id) = u32::try_from(args[0]) else { return Some(-3); };
            if self.hover != Some(WindowId(id)) || !runtime.ui.windows.inspect(WindowId(id)).is_some_and(|w|w.owner==owner&&w.visible) { return Some(-1); }
            use crate::ui::text_input::PointerShape as S;
            let shape = match args[1] { 0=>S::Default,1=>S::Text,2=>S::ResizeNorthWestSouthEast,
                3=>S::ResizeNorthEastSouthWest,4=>S::ResizeVertical,5=>S::ResizeHorizontal,_=>return Some(-6) };
            crate::ui::text_input::set_pointer_shape(shape);
            return Some(0);
        }
        if operation == 0x1253 {
            if args[2..].iter().any(|v| *v != 0) { return Some(-3); }
            let Ok(id) = u32::try_from(args[0]) else { return Some(-3); };
            let Some(window) = runtime.ui.windows.inspect(WindowId(id)) else { return Some(-2); };
            if window.owner != owner { return Some(-1); }
            return Some(match args[1] {
                0 => window.bounds.x as i64, 1 => window.bounds.y as i64,
                2 => window.bounds.width as i64, 3 => window.bounds.height as i64,
                4 => window.visible as i64,
                5 => match window.state {WindowState::Normal=>0,WindowState::Minimized=>1,WindowState::Maximized=>2,WindowState::Fullscreen=>3,WindowState::Withdrawn=>4},
                _ => -3,
            });
        }
        if operation == 0x1252 {
            if args[1..].iter().any(|v| *v != 0) { return Some(-3); }
            return Some(match args[0] {
                0 => display.x as i64, 1 => display.y as i64,
                2 => display.width as i64, 3 => display.height as i64,
                4 => work.x as i64, 5 => work.y as i64,
                6 => work.width as i64, 7 => work.height as i64, _ => -3,
            });
        }
        if operation == 0x1251 {
            if args[2..].iter().any(|v| *v != 0) { return Some(-3); }
            return Some(self.request(runtime, caller, owner, args[0], args[1]));
        }
        if operation == 0x1110 && !self.leases.iter().any(Option::is_none) { return Some(-4); }
        if operation == 0x1110 && args[3] > now.saturating_add(3_600_000_000_000) { return Some(-3); }
        // A session can host multiple processes. A same-session token is not
        // sufficient: it must also have been issued by this exact GUI host.
        let owns = if matches!(operation, 0x1111 | 0x1112) {
            self.leases.iter().flatten().any(|(lease,_)|*lease==args[0])
        } else { self.grants.contains(&Some(args[0])) };
        if !owns { return Some(-1); }
        // The legacy desktop presenter currently accepts opaque retained
        // application pixels. Do not promise alpha composition before it has a
        // preserved backdrop/damage contract; Tk uses XRGB surfaces.
        if operation == 0x1120 && args[3] != 0 { return Some(-6); }
        let result = self.surfaces.dispatch(runtime, memory, tables, caller, owner,
            operation, args, work, display, now);
        if result >= 0 {
            if operation == 0x1110 { *self.leases.iter_mut().find(|s| s.is_none()).unwrap() = Some((result as u64,args[0])); }
            if operation == 0x1112 { for slot in &mut self.leases { if slot.is_some_and(|(lease,_)|lease==args[0]) { *slot = None; } } }
            if matches!(operation, 0x1100..=0x1108 | 0x1122..=0x1124) { self.revision = self.revision.wrapping_add(1); }
        }
        Some(result)
    }
    // ------------------------=
    // FUNC: close
    // DESC: Stops input, destroys context-owned windows/pixels and retires nondelegable grants after task execution has stopped.
    // ------------------=
    pub fn close(&mut self, runtime: &mut InfinityRuntime, memory: &mut NativeMemory) {
        if let Some((caller, owner)) = self.owner {
            for lease in &mut self.leases { if let Some((id,_)) = lease.take() { let _ = runtime.events.unsubscribe_owned(caller, id); } }
            self.surfaces.context_failed(runtime, memory, owner);
            for slot in &mut self.grants {
                if let Some(id) = slot.take() {
                    let _ = runtime.capabilities.revoke(id);
                    let _ = runtime.capabilities.retire_leaf(id, caller);
                }
            }
            self.owner = None;
            self.chrome_drag = None;
            self.pointer_target(None);
            self.revision = self.revision.wrapping_add(1);
        }
    }
}
