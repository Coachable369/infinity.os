//! UI-thread widget placement, bounded damage, and architecture-neutral persistence.
use super::geometry::{Point, Rect};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub positions: [[u16; 2]; 2],
    pub weather_position: [u16;2],
    pub menu_widget: Option<usize>,
    pub menu_hover: Option<usize>,
    pub visible: u8,
    pub transparency: u32,
    pub adjusting: bool,
    pub transparency_menu: bool,
    pub visibility_menu: bool,
    pub trash_position: Option<[u16;2]>,
    pub drag: Option<(usize, i32, i32)>,
    pub menu: Option<Point>,
}
impl State {
    // ------------------------=
    // FUNC: material_alpha
    // DESC: Adjusts only the widget background while keeping text and control contrast intact.
    // ------------------=
    pub fn material_alpha(self, id:usize, base:u8)->u8 {
        (u32::from(base)*(100-self.percent(id))/100) as u8
    }
    // ------------------------=
    // FUNC: percent
    // DESC: Reads the legacy quarter steps and additional precision from the session layout.
    // ------------------=
    pub fn percent(self,id:usize)->u32 {
        (25*((self.transparency>>(2*id))&3)+((self.transparency>>(6+5*id))&31)).min(100)
    }
    // ------------------------=
    // FUNC: set_percent
    // DESC: Stores a continuous slider selection while preserving the other widgets.
    // ------------------=
    pub fn set_percent(&mut self,id:usize,percent:u32){
        let value=percent.min(100);let quarters=(value/25).min(3);
        let (lo,hi)=(2*id,6+5*id);
        self.transparency=(self.transparency & !((3<<lo)|(31<<hi)))|(quarters<<lo)|((value-25*quarters)<<hi);
    }
    // ------------------------=
    // FUNC: shown
    // DESC: Keeps the overview off a standalone browser desktop without changing saved widget preferences.
    // ------------------=
    pub fn shown(self, id: usize, browser_visible: bool) -> bool {
        id < 3 && self.visible & (1 << id) != 0 && !(id == 0 && browser_visible)
    }
    // ------------------------=
    // FUNC: new
    // DESC: Keeps legacy right-column placement until a user moves a widget.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            positions: [[0; 2]; 2],
            weather_position: [0;2], menu_widget: None, menu_hover: None,
            visible: 2, adjusting:false, transparency: 0, transparency_menu: false, visibility_menu: false,
            trash_position: None,
            drag: None,
            menu: None,
        }
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Packs only persistent placement and visibility into reserved session bytes.
    // ------------------=
    pub fn encode(self) -> u64 {
        let mut value = (1u64 << 63) | ((self.visible as u64 & 3) << 40);
        for i in 0..4 {
            value |= (self.positions[i / 2][i % 2].min(1001) as u64) << (i * 10);
        }
        if let Some([x,y])=self.trash_position {value |= (1u64<<62)|((x.min(1000) as u64)<<42)|((y.min(1000) as u64)<<52);}
        if self.visible & 4 != 0 { !value } else { value }
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Restores layout while dropping transient drag and menu capture; zero is legacy default.
    // ------------------=
    pub fn decode(value: u64) -> Self {
        let mut state = Self::new();
        if value == 0 {return state;}
        let weather = value >> 63 == 0;
        let value = if weather { !value } else { value };
        if value & (1u64<<62)!=0 {state.trash_position=Some([((value>>42)&1023).min(1000) as u16,((value>>52)&1023).min(1000) as u16]);}
        state.visible = ((value >> 40) & 3) as u8 | ((weather as u8) << 2);
        for i in 0..4 {
            state.positions[i / 2][i % 2] = (((value >> (i * 10)) & 1023) as u16).min(1001);
        }
        state
    }
    // ------------------------=
    // FUNC: position_trash
    // DESC: Resolves the normalized personal placement against the current display bounds.
    // ------------------=
    pub fn position_trash(self,mut tile:Rect,width:usize,height:usize)->Rect {
        if let Some([x,y])=self.trash_position {
            tile.x=(width*x as usize/1000).min(width.saturating_sub(tile.width as usize)) as i32;
            tile.y=(height*y as usize/1000).clamp((height*7/100).min(height.saturating_sub(tile.height as usize)),height.saturating_sub(tile.height as usize)) as i32;
        }
        tile
    }
    // ------------------------=
    // FUNC: rect
    // DESC: Shares clamped physical widget geometry between painting, hit testing and damage.
    // ------------------=
    pub fn rect(
        self,
        id: usize,
        width: usize,
        height: usize,
        scale: usize,
        minimized: bool,
    ) -> Rect {
        if id==2 {
            let w=super::desktop_template::widget_width(width,height) as usize;let h=w/2;
            let [x,y]=self.weather_position;
            let preferred=Rect{x:if x==0{width/10}else{((x-1)as usize*width/1000).min(width.saturating_sub(w))}as i32,
                y:if y==0{height.saturating_sub(h+105*scale)}else{((y-1)as usize*height/1000).min(height.saturating_sub(h))}as i32,width:w as u32,height:h as u32};
            if self.visible&7!=7 || self.drag.is_some(){return preferred}
            let obstacles=[self.rect(0,width,height,scale,false),self.rect(1,width,height,scale,false)];
            if obstacles.iter().all(|r|!r.intersects(preferred)){return preferred}
            let gap=(12*scale) as i32;
            let xmax=width.saturating_sub(w) as i32;
            let ymin=(height*7/100) as i32;
            let ymax=(height.saturating_sub(h+82*scale) as i32).max(ymin);
            let mut best=None;
            for x in [preferred.x,0,xmax,obstacles[0].x-w as i32-gap,obstacles[1].x-w as i32-gap,obstacles[0].right()+gap,obstacles[1].right()+gap] {
                for y in [preferred.y,ymin,ymax,obstacles[0].bottom()+gap,obstacles[1].bottom()+gap,obstacles[0].y-h as i32-gap,obstacles[1].y-h as i32-gap] {
                    let candidate=Rect{x:x.clamp(0,xmax),y:y.clamp(ymin,ymax),..preferred};
                    let distance=(candidate.x-preferred.x).abs()+(candidate.y-preferred.y).abs();
                    if obstacles.iter().all(|r|!r.intersects(candidate)) && best.is_none_or(|(d,_)|distance<d){best=Some((distance,candidate));}
                }
            }
            return best.map_or(preferred,|(_,rect)|rect);
        }
        let top = height * 7 / 100;
        let overview_height = (330 * scale).min(height * 30 / 100);
        let default_y = if id == 0 || self.visible & 1 == 0 {
            top
        } else {
            top + overview_height + 20 * scale
        };
        let bottom = height.saturating_sub(102 * scale);
        let h = if id == 0 {
            overview_height
        } else if minimized {
            50 * scale
        } else {
            bottom.saturating_sub(default_y)
        };
        let authored=super::desktop_template::bounds((id+2) as u8,2,width,height).map(|mut rect| {
            if id == 0 && self.positions[0]==[0,0] && self.visible & 2 != 0 {
                if let Some(ai)=super::desktop_template::bounds(3,2,width,height) {
                    if rect.intersects(ai) {
                        rect.x=(ai.x-rect.width as i32-12*scale as i32).max(0);
                    }
                }
            }
            if id == 1 && self.visible & 1 == 0 {
                if let Some(overview) = super::desktop_template::bounds(2,2,width,height) {
                    rect.y = overview.y;
                }
            }
            rect
        });
        // Authoring owns the default geometry. Work-area snapping constrains
        // user drags, not saved designs (which may intentionally overlap a dock).
        if self.positions[id] == [0, 0] && !(minimized && id == 1) {
            if let Some(rect) = authored { return rect; }
        }
        let w=authored.map(|r|r.width as usize).unwrap_or(width*22/100);
        let h=if minimized&&id==1 {h}else{authored.map(|r|r.height as usize).unwrap_or(h)};
        let default_y=authored.map(|r|r.y.max(0) as usize).unwrap_or(default_y);
        let [x, y] = self.positions[id];
        let x = if x == 0 {
            authored.map(|r|r.x.max(0) as usize).unwrap_or(width*68/100)
        } else {
            (x as usize - 1) * width / 1000
        };
        let y = if y == 0 {
            default_y
        } else {
            (y as usize - 1) * height / 1000
        };
        Rect {
            x: x.min(width.saturating_sub(w + 8 * scale)) as i32,
            y: y.max(top).min(bottom.saturating_sub(h).max(top)) as i32,
            width: w as u32,
            height: h as u32,
        }
    }
    // ------------------------=
    // FUNC: move_pointer
    // DESC: Tracks a captured header and snaps the final drop to work-area and adjacent-widget edges.
    // ------------------=
    pub fn move_pointer(
        &mut self,
        point: Point,
        held: bool,
        width: usize,
        height: usize,
        scale: usize,
        minimized: bool,
    ) {
        self.move_with_neighbours(point,held,width,height,scale,minimized,&[]);
    }
    // ------------------------=
    // FUNC: move_with_neighbours
    // DESC: Includes visible application-authored widgets in the built-in drop targets.
    // ------------------=
    pub fn move_with_neighbours(&mut self,point:Point,held:bool,width:usize,height:usize,scale:usize,minimized:bool,extra:&[Rect]) {
        let Some((id, grab_x, grab_y)) = self.drag else {
            return;
        };
        let bounds = self.rect(id, width, height, scale, minimized && id == 1);
        let mut x = point.x - grab_x;
        let mut y = point.y - grab_y;
        let left = (8 * scale) as i32;
        let right = width.saturating_sub(bounds.width as usize + 8 * scale) as i32;
        let top = (height * 7 / 100) as i32;
        let bottom = height
            .saturating_sub(102 * scale + bounds.height as usize)
            .max(top as usize) as i32;
        if !held {

            for edge in [left, right] {
                if (x - edge).abs() <= (20 * scale) as i32 {
                    x = edge;
                }
            }
            for edge in [top, bottom] {
                if (y - edge).abs() <= (20 * scale) as i32 {
                    y = edge;
                }
            }
            let builtins=(0..3).filter(|&other|other!=id && self.visible&(1<<other)!=0)
                .map(|other|self.rect(other,width,height,scale,minimized && other==1));
            let snap=stack_point(Point{x,y},bounds,top,bottom,scale,builtins.chain(extra.iter().copied()));
            x=snap.x;
            y=snap.y;
            self.drag = None;
        }
        let position = [
            (x.clamp(left.min(right), right).max(0) as usize * 1000 / width.max(1) + 1) as u16,
            (y.clamp(top, bottom).max(0) as usize * 1000 / height.max(1) + 1) as u16,
        ];
        if id==2{self.weather_position=position;}else{self.positions[id]=position;}
    }
    // ------------------------=
    // FUNC: menu_rect
    // DESC: Keeps the widget chooser inside the display at the click position.
    // ------------------=
    pub fn menu_rect(self, width: usize, height: usize, scale: usize) -> Option<Rect> {
        self.menu.map(|p| {
            let w = (260 * scale).min(width);
            let rows=if self.transparency_menu {3} else if self.visibility_menu {5} else if self.menu_widget.is_some(){4}else{2};
            let h = ((rows*36+8)*scale).min(height);
            Rect {
                x: p.x.clamp(0, width.saturating_sub(w) as i32),
                y: p.y.clamp(0, height.saturating_sub(h) as i32),
                width: w as u32,
                height: h as u32,
            }
        })
    }
}
static mut STATE: State = State::new();
static mut PAINTED: State = State::new();
// ------------------------=
// FUNC: stack_point
// DESC: Selects the nearest reachable vertical stack and aligns its leading edge in one pointer operation.
// ------------------=
pub fn stack_point(point:Point,bounds:Rect,top:i32,bottom:i32,scale:usize,targets:impl Iterator<Item=Rect>)->Point {
    let (mut x,mut y)=(point.x,point.y);
    let gap=(12*scale) as i32;
    let reach=(26*scale) as i32;
    let mut nearest=None;
    for other in targets.filter(|r|r.width!=0 && r.height!=0) {
        if y<other.bottom() && y+bounds.height as i32>other.y {
            for side in [other.right()+gap,other.x-bounds.width as i32-gap] {
                if (x-side).abs()<=gap {x=side;}
            }
            if (y-other.y).abs()<=gap && (x-other.x).abs()<=bounds.width as i32+2*gap {y=other.y;}
        }
        if (point.x-other.x).abs()>reach*2 {continue;}
        for edge in [other.bottom()+gap,other.y-bounds.height as i32-gap] {
            let distance=(point.y-edge).abs();
            if (top..=bottom).contains(&edge) && distance<=reach && nearest.is_none_or(|(d,_,_)|distance<d) {
                nearest=Some((distance,other.x,edge));
            }
        }
    }
    nearest.map_or(Point{x,y},|(_,x,y)|Point{x,y})
}

// ------------------------=
// FUNC: current
// DESC: Reads the UI-thread presentation snapshot.
// ------------------=
pub fn current() -> State {
    unsafe { *(&raw const STATE) }
}
// ------------------------=
// FUNC: publish
// DESC: Updates widget presentation without service calls or allocations.
// ------------------=
pub fn publish(state: State) {
    unsafe {
        *(&raw mut STATE) = state;
    }
}
// ------------------------=
// FUNC: take_damage
// DESC: Reports only changed widgets and chooser coverage, leaving ordinary pointer movement damage-free.
// ------------------=
pub fn take_damage(width: usize, height: usize, scale: usize) -> Option<Rect> {
    let old = unsafe { *(&raw const PAINTED) };
    let new = current();
    unsafe {
        *(&raw mut PAINTED) = new;
    }
    let mut damage: Option<Rect> = None;
    if old.trash_position!=new.trash_position {
        let size=(height/21).max(44);
        let tile=Rect{x:width.saturating_sub(size+24+12*scale) as i32,y:height.saturating_sub(size+32+16*scale) as i32,width:(size+24) as u32,height:(size+32) as u32};
        damage=Some(old.position_trash(tile,width,height).union(new.position_trash(tile,width,height)));
    }
    for id in 0..3 {
        if old.rect(id,width,height,scale,false) != new.rect(id,width,height,scale,false) || (old.visible ^ new.visible) & (1 << id) != 0
            || old.percent(id)!=new.percent(id) {
            for state in [old, new] {
                if state.visible & (1 << id) != 0 {
                    let rect = state.rect(id, width, height, scale, false);
                    damage = Some(damage.map_or(rect, |d| d.union(rect)));
                }
            }
        }
    }
    if old.menu_hover!=new.menu_hover || old.transparency!=new.transparency || old.menu != new.menu || old.transparency_menu != new.transparency_menu
        || old.visibility_menu != new.visibility_menu || old.menu_widget != new.menu_widget {
        for state in [old, new] {
            if let Some(rect) = state.menu_rect(width, height, scale) {
                damage = Some(damage.map_or(rect, |d| d.union(rect)));
            }
        }
    }
    damage.map(|mut rect| {
        let outset=(12*scale) as u32;
        rect.x-=outset as i32;
        rect.y-=outset as i32;
        rect.width=rect.width.saturating_add(2*outset);
        rect.height=rect.height.saturating_add(3*outset);
        rect.intersection(Rect {
            x: 0,
            y: 0,
            width: width as u32,
            height: height as u32,
        })
    })
}
