//! Authored desktop geometry shared by Studio, rendering, damage and input.
use super::{geometry::{Point,Rect}, installer_template::{InstallerTemplate,InstallerTemplateElement,InstallerTemplateRole as Role}, installer_layout::scale_template_rect};
pub const BYTES: &[u8] = include_bytes!("../../assets/boot/desktop-screens.iuit");
use core::{cell::UnsafeCell,sync::atomic::{AtomicU8,Ordering}};
struct Validated {state:AtomicU8,value:UnsafeCell<Option<InstallerTemplate<'static>>>}
// Only the initializer writes; acquire readers observe immutable generation data.
unsafe impl Sync for Validated {}
static VALIDATED:Validated=Validated{state:AtomicU8::new(0),value:UnsafeCell::new(None)};

// ------------------------=
// FUNC: template
// DESC: Reads the same bounded desktop document in the guest and isolated Studio preview.
// ------------------=
pub fn template() -> Option<InstallerTemplate<'static>> {
    #[cfg(studio_native_preview)]
    let bytes=crate::studio_native_render::desktop_template_bytes().unwrap_or(BYTES);
    #[cfg(not(studio_native_preview))]
    let bytes=BYTES;
    if VALIDATED.state.load(Ordering::Acquire)==2 {return unsafe{*VALIDATED.value.get()};}
    if VALIDATED.state.compare_exchange(0,1,Ordering::Acquire,Ordering::Relaxed).is_err(){
        // A concurrent initializer is not a missing/invalid desktop document.
        // Publish once; other readers must observe that same parsed generation.
        while VALIDATED.state.load(Ordering::Acquire)!=2 {core::hint::spin_loop();}
        return unsafe{*VALIDATED.value.get()};
    }
    let value=InstallerTemplate::parse_desktop(bytes).ok();
    unsafe{*VALIDATED.value.get()=value;}
    VALIDATED.state.store(2,Ordering::Release);
    value
}

// ------------------------=
// FUNC: menu
// DESC: Resolves the optional authored Infinity menu from the same live/installed template used for desktop paint.
// ------------------=
pub fn menu()->Option<super::installer_template::navigation::Menu<'static>> {template()?.navigation()}

// ------------------------=
// FUNC: element
// DESC: Resolves a stable native role independently of editable labels, identifiers and stacking order.
// ------------------=
pub fn element(scene:u8,role:u8)->Option<InstallerTemplateElement<'static>> {
    let t=template()?;
    (0..t.element_count(scene)?).filter_map(|i|t.element_at(scene,i)).find(|e|e.role==role)
}

// ------------------------=
// FUNC: dock_scene
// DESC: Locates the native-bound dock without reserving IDs already used by user-authored menu screens.
// ------------------=
pub fn dock_scene() -> Option<u8> {
    bound_scene(Role::DesktopDock as u8)
}

// ------------------------=
// FUNC: bound_scene
// DESC: Finds an authored native surface by binding rather than a mutable scene index or title.
// ------------------=
pub fn bound_scene(role:u8) -> Option<u8> {
    let t = template()?;
    (4..=32).find(|&scene| (0..t.element_count(scene).unwrap_or(0))
        .filter_map(|index| t.element_at(scene, index)).any(|e| e.role == role))
}

// ------------------------=
// FUNC: dock_hit
// DESC: Resolves visible dock bindings in authored stacking order, sharing image bounds with native painting.
// ------------------=
pub fn dock_hit(point: Point, width: usize, height: usize) -> Option<usize> {
    let scene = dock_scene()?;
    let t = template()?;
    dock_hit_in(t,scene,point,width,height)
}

// ------------------------=
// FUNC: scene_bounds
// DESC: Includes all visible authored layers in damage restoration when a user moves dock items outside their original panel.
// ------------------=
pub fn scene_bounds(scene:u8,width:usize,height:usize)->Rect {
    let Some(t)=template() else{return Rect::default();};
    scene_bounds_in(t,scene,width,height)
}

// ------------------------=
// FUNC: scene_bounds_in
// DESC: Computes nonempty authored damage without accidentally including the screen origin in the union.
// ------------------=
pub fn scene_bounds_in(t:InstallerTemplate<'_>,scene:u8,width:usize,height:usize)->Rect {
    (0..t.element_count(scene).unwrap_or(0)).filter_map(|i|t.element_at(scene,i))
        .filter(|e|!e.hidden && e.opacity>0 && e.role!=Role::SceneBackground as u8)
        .map(|e|mapped(e,width,height,None)).reduce(|area,r|area.union(r)).unwrap_or_default()
}

// ------------------------=
// FUNC: dock_hit_in
// DESC: Hit-tests a validated authored dock in either installed runtime or a Studio-produced acceptance fixture.
// ------------------=
pub fn dock_hit_in(t: InstallerTemplate<'_>, scene:u8, point:Point, width:usize, height:usize)->Option<usize> {
    (0..t.element_count(scene)?).rev().filter_map(|index| t.layer_at(scene, index))
        .find(|e| !e.hidden && e.opacity > 0 && (73..=80).contains(&e.role)
            && mapped(*e, width, height, None).contains(point)).map(|e| (e.role - 73) as usize)
}

// ------------------------=
// FUNC: bounds
// DESC: Shares authored geometry between painting and hit targets, excluding hidden and transparent controls.
// ------------------=
pub fn bounds(scene:u8,role:u8,width:usize,height:usize)->Option<Rect> {
    let e=element(scene,role)?;
    if e.hidden||e.opacity==0{return None;}
    Some(mapped(e,width,height,None))
}

// ------------------------=
// FUNC: mapped
// DESC: Translates a widget's authored child positions with its user-dragged panel without changing saved layout.
// ------------------=
pub fn mapped(e:InstallerTemplateElement<'_>,width:usize,height:usize,translation:Option<(i32,i32)>)->Rect {
    let r=scale_template_rect(e.frame,width,height);
    let (dx,dy)=translation.unwrap_or((0,0));
    Rect{x:(r.left as i32).saturating_add(dx),y:(r.top as i32).saturating_add(dy),width:r.width as u32,height:r.height as u32}
}

// ------------------------=
// FUNC: navigation_hit
// DESC: Dispatches only explicit native roles in topmost visible order; decoration cannot acquire executable authority.
// ------------------=
pub fn navigation_hit(point:Point,width:usize,height:usize)->Option<u8> {
    let t=template()?;
    (0..t.element_count(1)?).rev().filter_map(|i|t.layer_at(1,i))
        .find(|e|!e.hidden&&e.opacity>0&&(49..=59).contains(&e.role)&&e.role!=Role::DesktopPrivacy as u8
            &&mapped(*e,width,height,None).contains(point)).map(|e|e.role)
}
