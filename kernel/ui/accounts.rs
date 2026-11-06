//! Pure account presentation and hit geometry; authentication stays in the service.
use super::geometry::{Point, Rect};

pub const CAPACITY: usize = 16;
pub type Id = [u8; 16];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text {
    pub bytes: [u8; 48],
    pub len: usize,
    pub cursor: usize,
    pub selected: bool,
}
impl Text {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded editable label.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            bytes: [0; 48],
            len: 0,
            cursor: 0,
            selected: false,
        }
    }
    // ------------------------=
    // FUNC: from_bytes
    // DESC: Copies a service label without borrowing runtime storage.
    // ------------------=
    pub fn from_bytes(value: &[u8]) -> Self {
        let mut text = Self::new();
        text.len = value.len().min(48);
        text.bytes[..text.len].copy_from_slice(&value[..text.len]);
        text.cursor = text.len;
        text
    }
    // ------------------------=
    // FUNC: as_bytes
    // DESC: Returns only the populated label bytes.
    // ------------------=
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
    // ------------------------=
    // FUNC: edit
    // DESC: Applies bounded ASCII editing with insertion and deletion at the caret.
    // ------------------=
    pub fn edit(&mut self, key: Edit) -> bool {
        if self.selected && matches!(key, Edit::Insert(_) | Edit::Backspace | Edit::Delete) {
            *self = Self::new();
            if !matches!(key, Edit::Insert(_)) {
                return true;
            }
        }
        self.selected = false;
        match key {
            Edit::Insert(byte) if (32..=126).contains(&byte) && self.len < 48 => {
                self.bytes
                    .copy_within(self.cursor..self.len, self.cursor + 1);
                self.bytes[self.cursor] = byte;
                self.len += 1;
                self.cursor += 1;
            }
            Edit::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.bytes
                    .copy_within(self.cursor + 1..self.len, self.cursor);
                self.len -= 1;
                self.bytes[self.len] = 0;
            }
            Edit::Delete if self.cursor < self.len => {
                self.bytes
                    .copy_within(self.cursor + 1..self.len, self.cursor);
                self.len -= 1;
                self.bytes[self.len] = 0;
            }
            Edit::Left => self.cursor = self.cursor.saturating_sub(1),
            Edit::Right => self.cursor = (self.cursor + 1).min(self.len),
            Edit::Home => self.cursor = 0,
            Edit::End => self.cursor = self.len,
            _ => return false,
        }
        true
    }
}
#[derive(Clone, Copy)]
pub enum Edit {
    Insert(u8),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Users,
    Groups,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Browse,
    Create,
    Rename,
    Password,
    Delete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feedback {
    None,
    Saved,
    Deleted,
    Invalid,
    PasswordMismatch,
    Denied,
    Conflict,
    Full,
    Unavailable,
    Credential,
    RateLimited,
    Failed,
    TooLong,
    PasswordLength,
    RestartRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    None,
    Users,
    Groups,
    New,
    Back,
    Row(usize),
    Rename,
    Enabled,
    Role,
    RoleOption(bool),
    Password,
    Delete,
    Member(usize),
    Field(usize),
    Save,
    Cancel,
    Confirm,
    ListUp,
    ListDown,
    DetailUp,
    DetailDown,
}
#[derive(Clone, Copy)]
pub struct Entry {
    pub id: Id,
    pub name: Text,
    pub handle: Text,
    pub enabled: bool,
    pub admin: bool,
    pub builtin: bool,
    pub members: u16,
}
impl Entry {
    // ------------------------=
    // FUNC: empty
    // DESC: Supplies an unused directory slot without a synthetic identity.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            id: [0; 16],
            name: Text::new(),
            handle: Text::new(),
            enabled: false,
            admin: false,
            builtin: false,
            members: 0,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Directory {
    pub users: [Entry; CAPACITY],
    pub groups: [Entry; CAPACITY],
    pub users_len: usize,
    pub groups_len: usize,
    pub actor: Id,
    pub administrator: bool,
    pub available: bool,
}
impl Directory {
    // ------------------------=
    // FUNC: empty
    // DESC: Represents unavailable data without claiming authentication or permission.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            users: [Entry::empty(); CAPACITY],
            groups: [Entry::empty(); CAPACITY],
            users_len: 0,
            groups_len: 0,
            actor: [0; 16],
            administrator: false,
            available: false,
        }
    }
    // ------------------------=
    // FUNC: entries
    // DESC: Exposes bounded committed rows for the selected segment.
    // ------------------=
    pub fn entries(&self, tab: Tab) -> &[Entry] {
        match tab {
            Tab::Users => &self.users[..self.users_len.min(CAPACITY)],
            Tab::Groups => &self.groups[..self.groups_len.min(CAPACITY)],
        }
    }
}
#[derive(Clone, Copy)]
pub struct State {
    pub tab: Tab,
    pub selected: Option<Id>,
    pub detail: bool,
    pub role_open: bool,
    pub mode: Mode,
    pub focus: Target,
    pub list_scroll: usize,
    pub detail_scroll: usize,
    pub name: Text,
    pub handle: Text,
    pub password_lengths: [usize; 2],
    pub password_cursors: [usize; 2],
    pub feedback: Feedback,
}
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Opens a neutral directory with no inferred selection or authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            tab: Tab::Users,
            selected: None,
            detail: false,
            role_open: false,
            mode: Mode::Browse,
            focus: Target::Users,
            list_scroll: 0,
            detail_scroll: 0,
            name: Text::new(),
            handle: Text::new(),
            password_lengths: [0; 2],
            password_cursors: [0; 2],
            feedback: Feedback::None,
        }
    }
    // ------------------------=
    // FUNC: selected_index
    // DESC: Resolves selection by stable identity after directory changes.
    // ------------------=
    pub fn selected_index(&self, data: &Directory) -> Option<usize> {
        data.entries(self.tab)
            .iter()
            .position(|e| Some(e.id) == self.selected)
            .or_else(|| if self.selected.is_none() {
                data.entries(self.tab).iter().position(|e|self.tab==Tab::Users && e.id==data.actor)
                    .or_else(||(!data.entries(self.tab).is_empty()).then_some(0))
            } else {None})
    }
    // ------------------------=
    // FUNC: selected_entry
    // DESC: Retrieves a committed selected record without retaining a service borrow.
    // ------------------=
    pub fn selected_entry<'a>(&self, data: &'a Directory) -> Option<&'a Entry> {
        self.selected_index(data)
            .map(|i| &data.entries(self.tab)[i])
    }
    // ------------------------=
    // FUNC: select
    // DESC: Selects a stable record and clears stale form presentation.
    // ------------------=
    pub fn select(&mut self, data: &Directory, index: usize) {
        if let Some(entry) = data.entries(self.tab).get(index) {
            self.cancel();
            self.selected = Some(entry.id);
            self.detail = true;
            self.focus = Target::Row(index);
            self.detail_scroll = 0;
        }
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Clears form labels and all secret-length presentation on cancellation.
    // ------------------=
    pub fn cancel(&mut self) {
        self.role_open = false;
        self.mode = Mode::Browse;
        self.name = Text::new();
        self.handle = Text::new();
        self.password_lengths = [0; 2];
        self.password_cursors = [0; 2];
        self.feedback = Feedback::None;
        self.detail_scroll = 0;
        self.focus = Target::Back;
    }
    // ------------------------=
    // FUNC: begin
    // DESC: Creates an explicit editing transaction from committed values.
    // ------------------=
    pub fn begin(&mut self, mode: Mode, data: &Directory) {
        self.cancel();
        self.mode = mode;
        self.detail = true;
        self.focus = if mode == Mode::Delete {
            Target::Cancel
        } else {
            Target::Field(if mode == Mode::Password { 2 } else { 0 })
        };
        if mode == Mode::Rename {
            if let Some(entry) = self.selected_entry(data) {
                self.name = entry.name;
                self.handle = entry.handle;
            }
        }
    }
    // ------------------------=
    // FUNC: switch
    // DESC: Changes directory segments and drops stale selection and drafts.
    // ------------------=
    pub fn switch(&mut self, tab: Tab) {
        *self = Self { tab, ..Self::new() };
        self.focus = if tab == Tab::Users {
            Target::Users
        } else {
            Target::Groups
        };
    }
}
#[derive(Clone, Copy)]
pub struct View {
    pub state: State,
    pub data: Directory,
}
static mut VIEW: View = View {
    state: State::new(),
    data: Directory::empty(),
};
// ------------------------=
// FUNC: publish
// DESC: Publishes a secret-free snapshot on the serialized console UI lane.
// ------------------=
pub fn publish(view: View) {
    unsafe {
        VIEW = view;
    }
}
// ------------------------=
// FUNC: current
// DESC: Copies the last published account presentation without accessing services in paint.
// ------------------=
pub fn current() -> View {
    unsafe { VIEW }
}

#[derive(Clone, Copy)]
pub struct Control {
    pub target: Target,
    pub bounds: Rect,
    pub clip: Rect,
    pub enabled: bool,
}
impl Control {
    // ------------------------=
    // FUNC: empty
    // DESC: Supplies an unused hit-test slot.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            target: Target::None,
            bounds: Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            clip: Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            enabled: false,
        }
    }
}
pub struct Layout {
    pub controls: [Control; 64],
    pub count: usize,
    pub list: Rect,
    pub details: Rect,
    pub footer: Rect,
    pub title: Rect,
    pub narrow: bool,
    pub list_max: usize,
    pub detail_max: usize,
}
impl Layout {
    // ------------------------=
    // FUNC: add
    // DESC: Registers one stable bounded control in visual navigation order.
    // ------------------=
    fn add(&mut self, target: Target, bounds: Rect, clip: Rect, enabled: bool) {
        if self.count < 64 {
            self.controls[self.count] = Control {
                target,
                bounds,
                clip,
                enabled,
            };
            self.count += 1;
        }
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Uses the exact painted clip and geometry for pointer activation.
    // ------------------=
    pub fn hit(&self, point: Point) -> Option<Target> {
        self.controls[..self.count]
            .iter().rev()
            .find(|c| c.enabled && c.bounds.intersection(c.clip).contains(point))
            .map(|c| c.target)
    }
    // ------------------------=
    // FUNC: tab
    // DESC: Cycles enabled controls and reveals offscreen list or detail focus.
    // ------------------=
    pub fn tab(&self, state: &mut State, backwards: bool) {
        if self.count == 0 {
            return;
        }
        let start = self.controls[..self.count]
            .iter()
            .position(|c| c.target == state.focus)
            .unwrap_or(if backwards { 0 } else { self.count - 1 });
        for step in 1..=self.count {
            let index = if backwards {
                (start + self.count - step) % self.count
            } else {
                (start + step) % self.count
            };
            let c = self.controls[index];
            if !c.enabled {
                continue;
            }
            state.focus = c.target;
            self.reveal(state, c);
            return;
        }
    }
    // ------------------------=
    // FUNC: reveal
    // DESC: Scrolls only the pane containing the newly focused control.
    // ------------------=
    pub fn reveal(&self, state: &mut State, c: Control) {
        let delta = if c.bounds.y < c.clip.y {
            c.bounds.y - c.clip.y
        } else if c.bounds.bottom() > c.clip.bottom() {
            c.bounds.bottom() - c.clip.bottom()
        } else {
            0
        };
        if matches!(c.target, Target::Row(_)) {
            state.list_scroll = (state.list_scroll as i32 + delta).max(0) as usize;
            state.list_scroll = state.list_scroll.min(self.list_max);
        } else if c.clip == self.details {
            state.detail_scroll = (state.detail_scroll as i32 + delta).max(0) as usize;
            state.detail_scroll = state.detail_scroll.min(self.detail_max);
        }
    }
}
// ------------------------=
// FUNC: layout
// DESC: Produces responsive list/detail geometry shared by paint, pointer, focus and tests.
// ------------------=
pub fn layout(viewport: Rect, state: &State, data: &Directory, scale: usize) -> Layout {
    let s = scale.max(1) as u32;
    let gap = 16 * s;
    let bar = 36 * s;
    let footer_h = (if state.feedback==Feedback::None {8*s}else{48*s}).min(viewport.height);
    let actions_h = if state.mode == Mode::Browse {
        0
    } else {
        (44 * s).min(viewport.height.saturating_sub(footer_h))
    };
    let header_h = (88 * s).min(viewport.height.saturating_sub(footer_h + actions_h));
    let body = Rect {
        x: viewport.x,
        y: viewport.y + header_h as i32,
        width: viewport.width,
        height: viewport
            .height
            .saturating_sub(header_h + footer_h + actions_h),
    };
    let narrow = viewport.width < 580 * s;
    let show_detail = (!narrow || state.detail)
        && (state.selected_entry(data).is_some() || state.mode != Mode::Browse);
    let list_width = if narrow {
        body.width
    } else {
        body.width * 39 / 100
    };
    let list = Rect {
        width: if narrow && show_detail { 0 } else { list_width },
        height: body.height.saturating_sub(bar + gap),
        ..body
    };
    let title_top=(16+if narrow{44}else{0})*s;
    let detail_top=((if state.mode==Mode::Browse {180}else{112})+if narrow{44}else{0})*s;
    let detail_top=detail_top.min(viewport.height.saturating_sub(footer_h+actions_h+bar+gap+68*s));
    let details = if show_detail {
        Rect {
            x: if narrow {
                body.x
            } else {
                body.x + (list_width + gap) as i32
            },
            y: viewport.y + detail_top as i32,
            width: if narrow {
                body.width
            } else {
                body.width.saturating_sub(list_width + gap + 16*s)
            },
            height: viewport.height.saturating_sub(detail_top+footer_h+actions_h+bar+gap),
        }
    } else {
        Rect::default()
    };
    let mut out = Layout {
        controls: [Control::empty(); 64],
        count: 0,
        list,
        details,
        footer: Rect {
            x: viewport.x,
            y: viewport.bottom() - footer_h as i32,
            width: viewport.width,
            height: footer_h,
        },
        title: Rect {
            y: viewport.y + title_top as i32,
            height: detail_top.saturating_sub(title_top+32*s),
            ..details
        },
        narrow,
        list_max: 0,
        detail_max: 0,
    };
    // Keep the directory controls with the directory on wide screens.
    let third = if narrow && show_detail { Target::Back } else { Target::New };
    let compact = narrow && show_detail;
    let strip = if narrow { viewport.width } else { list_width };
    let segment = if compact { strip.saturating_sub(2 * gap) / 3 }
        else { strip / 2 };
    for (i, target) in [Target::Users, Target::Groups].into_iter().enumerate() {
        out.add(target, Rect { x: viewport.x + (i as u32 * (segment + if compact {gap}else{0})) as i32,
            y: viewport.y, width: segment, height: bar.min(header_h) }, viewport,
            state.mode == Mode::Browse);
    }
    let toolbar = if compact {
        Rect { x: viewport.x + (2 * (segment + gap)) as i32, y: viewport.y,
            width: segment, height: bar.min(header_h) }
    } else {
        Rect { x: list.x, y: body.bottom() - bar as i32,
            width: (36 * s).min(list.width), height: bar }
    };
    out.add(third, toolbar, viewport, state.mode == Mode::Browse
        && (third != Target::New || data.administrator));
    if list.width > 0 {
        let row_h = 64 * s;
        out.list_max =
            (data.entries(state.tab).len() as u32 * row_h).saturating_sub(list.height) as usize;
        let scroll = state.list_scroll.min(out.list_max);
        for i in 0..data.entries(state.tab).len() {
            out.add(
                Target::Row(i),
                Rect {
                    x: list.x,
                    y: list.y + (i as u32 * row_h) as i32 - scroll as i32,
                    width: list.width,
                    height: 60 * s,
                },
                list,
                state.mode == Mode::Browse,
            );
        }
        let remove = Rect { x: toolbar.right() + gap as i32, y: toolbar.y,
            width: (36*s).min(list.width.saturating_sub(toolbar.width + gap)), height: toolbar.height };
        let selected = state.selected_entry(data);
        out.add(Target::Delete, remove, viewport, state.mode == Mode::Browse && data.administrator &&
            selected.is_some_and(|e|!e.builtin && (state.tab==Tab::Groups || e.id!=data.actor)));
        if out.list_max > 0 {
            let start = remove.right() + gap as i32;
            let step = list.right().saturating_sub(start).max(0) as u32 / 2;
            for (i, target) in [Target::ListUp, Target::ListDown].into_iter().enumerate() {
                out.add(target, Rect { x: start + (i as u32 * step) as i32,
                    y: body.bottom() - bar as i32, width: step, height: bar }, viewport,
                    if i == 0 { scroll > 0 } else { scroll < out.list_max });
            }
        }
    }
    if show_detail {
        let entry = state.selected_entry(data);
        let can_edit = data.administrator
            || entry.is_some_and(|e| state.tab == Tab::Users && e.id == data.actor);
        let protected = entry.is_some_and(|e| e.builtin);
        let mut targets = [Target::None; 40];
        let mut enabled = [false; 40];
        let mut n = 0;
        match state.mode {
            Mode::Create | Mode::Rename | Mode::Password => {
                let first = if state.mode == Mode::Password { 2 } else { 0 };
                let last = if state.mode == Mode::Password
                    || (state.mode == Mode::Create && state.tab == Tab::Users)
                {
                    4
                } else {
                    2
                };
                for i in first..last {
                    targets[n] = Target::Field(i);
                    enabled[n] = true;
                    n += 1;
                }
            }
            Mode::Delete => {}
            Mode::Browse => {
                let count = if state.tab == Tab::Users {
                    data.groups_len
                } else {
                    data.users_len
                };
                for i in 0..count {
                    targets[n] = Target::Member(i);
                    enabled[n] = data.administrator;
                    n += 1;
                }
                for (target, on) in [
                    (
                        Target::Enabled,
                        data.administrator && state.tab == Tab::Users,
                    ),
                    (Target::Rename, can_edit && !protected),
                    (Target::Password, can_edit && state.tab == Tab::Users),
                    (
                        Target::Delete,
                        data.administrator
                            && !protected
                            && (state.tab == Tab::Groups
                                || entry.is_some_and(|e| e.id != data.actor)),
                    ),
                ] {
                    if target == Target::Delete && list.width > 0 { continue; }
                    if state.tab == Tab::Groups
                        && matches!(target, Target::Password | Target::Enabled)
                    {
                        continue;
                    }
                    targets[n] = target;
                    enabled[n] = on;
                    n += 1;
                }
            }
        }
        let row_height = |target| if matches!(target,Target::Field(_)) {68*s} else {44*s};
        let total:u32=targets[..n].iter().map(|&t|row_height(t)).sum();

        out.detail_max=total.saturating_sub(details.height) as usize;
        let mut y=details.y-state.detail_scroll.min(out.detail_max) as i32;
        for i in 0..n {
            let t=targets[i];

            let h=row_height(t);
            let action_width=(details.width.saturating_sub(gap)/2).min(160*s);
            let width=if matches!(t,Target::Rename|Target::Password|Target::Delete) {action_width}
                else {details.width};
            let x=details.x;
            out.add(t,Rect{x,y,width,height:h-4*s},details,enabled[i]);
            y+=h as i32;
        }
        if out.detail_max > 0 {
        for (i, target) in [Target::DetailUp, Target::DetailDown]
            .iter()
            .copied()
            .enumerate()
        {
            out.add(
                target,
                Rect {
                    x: details.x + (i as u32 * (44*s+gap)) as i32,
                    y: body.bottom() - bar as i32,
                    width: (44*s).min(details.width/2),
                    height: bar,
                },
                viewport,
                if i == 0 {
                    state.detail_scroll > 0
                } else {
                    state.detail_scroll < out.detail_max
                },
            );
        }
    }
    }
    if show_detail && state.mode==Mode::Browse && state.tab==Tab::Users && out.title.height>=100*s {
        let role=Rect{x:out.title.x+(96*s)as i32,y:out.title.y+(68*s)as i32,
            width:out.title.width.saturating_sub(96*s).min(220*s),height:32*s};
        out.add(Target::Role,role,viewport,data.administrator);
        if state.role_open {
            for (i,admin) in [true,false].into_iter().enumerate() {
                out.add(Target::RoleOption(admin),Rect{y:role.bottom()+(i as u32*32*s)as i32,..role},viewport,data.administrator);
            }
        }
    }
    if state.mode != Mode::Browse {
        let left = if details.width > 0 {
            details.x
        } else {
            viewport.x
        };
        let width = if details.width > 0 {
            details.width
        } else {
            viewport.width
        };
        for (i, target) in [
            if state.mode == Mode::Delete {
                Target::Confirm
            } else {
                Target::Save
            },
            Target::Cancel,
        ]
        .iter()
        .copied()
        .enumerate()
        {
            out.add(
                target,
                Rect {
                    x: left + (i as u32 * (width / 2 + gap / 2)) as i32,
                    y: viewport.bottom() - (footer_h + actions_h) as i32,
                    width: width.saturating_sub(gap) / 2,
                    height: bar.min(actions_h),
                },
                viewport,
                target == Target::Cancel
                    || data.available && (state.mode != Mode::Delete || data.administrator),
            );
        }
    }
    if state.feedback == Feedback::RestartRequired {
        for control in &mut out.controls[..out.count] {
            control.enabled = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: initial_selection_and_role_menu_share_authorized_geometry
    // DESC: Opens the authenticated account without fixture preselection and routes menu hits above underlying membership controls.
    // ------------------=
    #[test]
    fn initial_selection_and_role_menu_share_authorized_geometry(){
        let mut data=Directory::empty();data.available=true;data.administrator=true;
        data.users_len=2;data.users[0].id=[1;16];data.users[1].id=[2;16];data.actor=[2;16];
        let mut state=State::new();
        assert_eq!(state.selected_index(&data),Some(1));
        let viewport=Rect{x:0,y:0,width:1000,height:650};
        assert!(layout(viewport,&state,&data,1).details.width>0);
        state.role_open=true;
        let geometry=layout(viewport,&state,&data,1);
        for admin in [true,false]{
            let control=geometry.controls[..geometry.count].iter().find(|c|c.target==Target::RoleOption(admin)).unwrap();
            assert_eq!(geometry.hit(Point{x:control.bounds.x+1,y:control.bounds.y+1}),Some(Target::RoleOption(admin)));
        }
        data.administrator=false;
        let geometry=layout(viewport,&state,&data,1);
        assert!(geometry.controls[..geometry.count].iter().filter(|c|matches!(c.target,Target::Role|Target::RoleOption(_))).all(|c|!c.enabled));
    }
    // ------------------------=
    // FUNC: uncertain_commit_disables_every_account_control
    // DESC: Keeps an ambiguous durable result read-only even when a stale directory still contains administrator authority.
    // ------------------=
    #[test]
    fn uncertain_commit_disables_every_account_control() {
        let mut data = Directory::empty();
        data.available = true;
        data.administrator = true;
        let mut state = State::new();
        state.begin(Mode::Create, &data);
        state.feedback = Feedback::RestartRequired;
        let geometry = layout(
            Rect {
                x: 0,
                y: 0,
                width: 600,
                height: 400,
            },
            &state,
            &data,
            1,
        );
        assert!(geometry.controls[..geometry.count]
            .iter()
            .all(|c| !c.enabled));
        let focus = state.focus;
        geometry.tab(&mut state, false);
        assert_eq!(state.focus, focus);
    }
    // ------------------------=
    // FUNC: form_actions_remain_visible_and_text_overflow_is_atomic
    // DESC: Verifies pinned confirmation controls and bounded ordinary-field selection replacement.
    // ------------------=
    #[test]
    fn form_actions_remain_visible_and_text_overflow_is_atomic() {
        let mut data = Directory::empty();
        data.available = true;
        data.administrator = true;
        data.users_len = 1;
        data.users[0].id = [1; 16];
        for mode in [Mode::Create, Mode::Rename, Mode::Password, Mode::Delete] {
            let mut state = State::new();
            state.select(&data, 0);
            state.begin(mode, &data);
            let viewport = Rect {
                x: 10,
                y: 20,
                width: 300,
                height: 260,
            };
            for scroll in [0, 100, 10000] {
                state.detail_scroll = scroll;
                let layout = layout(viewport, &state, &data, 1);
                for target in [
                    Target::Cancel,
                    if mode == Mode::Delete {
                        Target::Confirm
                    } else {
                        Target::Save
                    },
                ] {
                    let control = layout.controls[..layout.count]
                        .iter()
                        .find(|c| c.target == target)
                        .unwrap();
                    assert_eq!(control.bounds.intersection(viewport), control.bounds);
                    assert_eq!(
                        layout.hit(Point {
                            x: control.bounds.x,
                            y: control.bounds.y
                        }),
                        Some(target)
                    );
                    assert!(!control.bounds.intersects(layout.details));
                }
            }
        }
        let mut text = Text::from_bytes(&[b'a'; 48]);
        let original = text;
        assert!(!text.edit(Edit::Insert(b'b')));
        assert_eq!(text, original);
        text.selected = true;
        assert!(text.edit(Edit::Insert(b'z')));
        assert_eq!(text.as_bytes(), b"z");
    }
    // ------------------------=
    // FUNC: selection_survives_reorder_and_cancel_clears_drafts
    // DESC: Verifies identity-based selection and actual draft clearing transitions.
    // ------------------=
    #[test]
    fn selection_survives_reorder_and_cancel_clears_drafts() {
        let mut d = Directory::empty();
        d.users_len = 2;
        d.users[0].id = [1; 16];
        d.users[1].id = [2; 16];
        let mut s = State::new();
        s.select(&d, 1);
        d.users.swap(0, 1);
        assert_eq!(s.selected_index(&d), Some(0));
        s.begin(Mode::Password, &d);
        s.password_lengths = [8, 8];
        s.cancel();
        assert_eq!(s.password_lengths, [0, 0]);
        assert_eq!(s.mode, Mode::Browse);
        d.users_len = 0;
        assert_eq!(s.selected_index(&d), None);
    }
    // ------------------------=
    // FUNC: bounded_editing_and_focus_scrolling
    // DESC: Exercises insertion, deletion, clipped hit regions and keyboard reachability in short narrow windows.
    // ------------------=
    #[test]
    fn bounded_editing_and_focus_scrolling() {
        let mut t = Text::from_bytes(b"ab");
        t.edit(Edit::Left);
        t.edit(Edit::Insert(b'c'));
        assert_eq!(t.as_bytes(), b"acb");
        t.edit(Edit::Backspace);
        assert_eq!(t.as_bytes(), b"ab");
        let mut d = Directory::empty();
        d.available = true;
        d.administrator = true;
        d.users_len = 8;
        d.groups_len = 16;
        for i in 0..8 {
            d.users[i].id = [i as u8 + 1; 16];
        }
        let mut s = State::new();
        s.select(&d, 0);
        for width in [280, 420, 620, 900] {
            let rect = Rect {
                x: 10,
                y: 20,
                width,
                height: 260,
            };
            for _ in 0..60 {
                let g = layout(rect, &s, &d, 1);
                g.tab(&mut s, false);
                let g = layout(rect, &s, &d, 1);
                let c = g.controls[..g.count]
                    .iter()
                    .find(|c| c.target == s.focus)
                    .unwrap();
                assert!(c.bounds.intersects(c.clip));
                let visible = c.bounds.intersection(c.clip);
                assert_eq!(
                    g.hit(Point {
                        x: visible.x,
                        y: visible.y
                    }),
                    Some(c.target)
                );
            }
            let g = layout(rect, &s, &d, 1);
            for c in &g.controls[..g.count] {
                assert!(c.bounds.width <= rect.width);
            }
        }
    }
}
