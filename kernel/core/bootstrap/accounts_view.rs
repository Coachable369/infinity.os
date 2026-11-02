//! Production account renderer; accepts only explicit non-secret presentation data.
use super::DisplayDevice;
use crate::ui::{
    accounts::{self, Control, Feedback, Mode, Tab, Target, View},
    geometry::Rect,
    panel_recipe::{Material, Surface},
    system_layout::SettingsWindowState,
};

// ------------------------=
// FUNC: feedback
// DESC: Resolves actual validation or durable service outcomes into bounded feedback copy.
// ------------------=
fn feedback(value: Feedback) -> (&'static [u8], &'static [u8]) {
    match value {
        Feedback::None => (b"", b""),
        Feedback::Saved => (b"Changes saved.", b""),
        Feedback::Deleted => (b"Deleted.", b"Personal files were retained."),
        Feedback::Invalid => (
            b"Check the names and password.",
            b"All fields must contain a valid value.",
        ),
        Feedback::PasswordMismatch => (
            b"Passwords do not match.",
            b"Enter the same password twice.",
        ),
        Feedback::Denied => (
            b"This change is not permitted.",
            b"An administrator or a new sign-in may be required.",
        ),
        Feedback::Conflict => (
            b"The name is already in use.",
            b"Choose another name and try again.",
        ),
        Feedback::Full => (
            b"The account directory is full.",
            b"Remove an unused entry before creating another.",
        ),
        Feedback::Unavailable => (
            b"Accounts are unavailable.",
            b"Sign in again to refresh access.",
        ),
        Feedback::Credential => (
            b"The password was not accepted.",
            b"Use a valid password and try again.",
        ),
        Feedback::RateLimited => (b"Please wait before trying again.", b""),
        Feedback::Failed => (
            b"The change could not be saved.",
            b"No success has been confirmed. Try again.",
        ),
        Feedback::TooLong => (
            b"The field is full; input was not added.",
            b"Names: 48 characters. Passwords: 72 characters.",
        ),
        Feedback::PasswordLength => (b"Use a password of 8 to 72 characters.", b""),
        Feedback::RestartRequired => (
            b"Storage could not confirm this change.",
            b"Restart required.",
        ),
    }
}
impl DisplayDevice {
    // ------------------------=
    // FUNC: render_accounts_view
    // DESC: Renders the complete production section from a fixture or authenticated UI snapshot.
    // ------------------=
    pub(super) fn render_accounts_view(
        &mut self,
        state: SettingsWindowState,
        scale: usize,
        view: &View,
    ) {
        let old = self.render_clip;
        let chrome = self.paint_settings_chrome(2, state, scale);
        self.paint_accounts_body(chrome.geometry.viewport, scale, view, chrome);
        self.render_clip = old;
    }
    // ------------------------=
    // FUNC: paint_accounts_body
    // DESC: Composes unframed responsive lists and forms with the approved shared control surfaces.
    // ------------------=
    pub(super) fn paint_accounts_body(
        &mut self,
        viewport: Rect,
        scale: usize,
        view: &View,
        chrome: super::settings_view::SettingsChrome,
    ) {
        let old = self.render_clip;
        self.intersect_render_clip(
            viewport.x.max(0) as usize,
            viewport.y.max(0) as usize,
            viewport.width as usize,
            viewport.height as usize,
        );
        let s = scale.max(1);
        let layout = accounts::layout(viewport, &view.state, &view.data, s);
        let entry = view.state.selected_entry(&view.data);
        if layout.details.width > 0 {
            let title: &[u8] = match view.state.mode {
                Mode::Create => {
                    if view.state.tab == Tab::Users {
                        b"New user"
                    } else {
                        b"New group"
                    }
                }
                Mode::Rename => b"Edit names",
                Mode::Password => b"Set password",
                Mode::Delete => b"Confirm deletion",
                Mode::Browse => entry.map(|e| e.name.as_bytes()).unwrap_or(b""),
            };
            self.app_label(layout.title, title, (255, 255, 255), true, s);
        }
        if view.data.entries(view.state.tab).is_empty() && layout.list.width > 0 {
            self.app_label(
                Rect {
                    height: 48 * s as u32,
                    ..layout.list
                },
                if view.data.available {
                    b"No entries"
                } else {
                    b"Accounts unavailable"
                },
                (177, 195, 217),
                false,
                s,
            );
        }
        for c in &layout.controls[..layout.count] {
            self.paint_account_control(*c, view, s, chrome);
        }
        let (mut first, mut second) = feedback(view.state.feedback);
        if !view.data.available && view.state.feedback == Feedback::None {
            (first, second) = feedback(Feedback::Unavailable);
        }
        if view.state.mode == Mode::Delete && view.state.feedback == Feedback::None {
            first = entry.map(|e| e.name.as_bytes()).unwrap_or(b"");
            second = if view.state.tab == Tab::Users {
                b"Delete this user? Personal files will be retained."
            } else {
                b"Delete this group and its memberships?"
            };
        }
        if view.state.mode == Mode::Browse
            && view.state.feedback == Feedback::None
            && view.data.available
        {
            if let Some(e) = entry {
                first = e.handle.as_bytes();
                second = if view.state.tab == Tab::Groups {
                    if e.builtin {
                        b"System group"
                    } else {
                        b"Custom group"
                    }
                } else if !e.enabled {
                    b"Disabled account"
                } else if e.admin {
                    b"Administrator"
                } else {
                    b"Standard user"
                };
            }
        }
        let color = if matches!(
            view.state.feedback,
            Feedback::None | Feedback::Saved | Feedback::Deleted
        ) {
            (174, 205, 226)
        } else {
            (255, 153, 162)
        };
        let line = layout.footer.height / 2;
        self.app_label(
            Rect {
                height: line,
                ..layout.footer
            },
            first,
            color,
            false,
            s,
        );
        self.app_label(
            Rect {
                y: layout.footer.y + line as i32,
                height: line,
                ..layout.footer
            },
            second,
            color,
            false,
            s,
        );
        self.render_clip = old;
    }
    // ------------------------=
    // FUNC: paint_account_control
    // DESC: Paints one shared hit target, keeping password bytes absent and clipping all text to its control.
    // ------------------=
    fn paint_account_control(
        &mut self,
        c: Control,
        view: &View,
        s: usize,
        chrome: super::settings_view::SettingsChrome,
    ) {
        let clip = c.bounds.intersection(c.clip);
        if clip.width == 0 || clip.height == 0 {
            return;
        }
        let old = self.render_clip;
        self.intersect_render_clip(
            clip.x.max(0) as usize,
            clip.y.max(0) as usize,
            clip.width as usize,
            clip.height as usize,
        );
        let pointer = crate::ui::app_launcher::window_controls::state(c.bounds);
        let selected = match c.target {
            Target::Users => view.state.tab == Tab::Users,
            Target::Groups => view.state.tab == Tab::Groups,
            Target::Row(i) => view.state.selected_index(&view.data) == Some(i),
            _ => false,
        };
        let focused = c.enabled && (view.state.focus == c.target || pointer != 0);
        let color = if c.enabled {
            (241, 247, 255)
        } else {
            (122, 141, 164)
        };
        let material = Material {
            fill: chrome.selection,
            edge: chrome.outline,
            opacity: if c.enabled { 230 } else { 112 },
            edge_opacity: if c.enabled { 255 } else { 128 },
            focus: if focused { 255 } else { 0 },
            pressed: pointer == 2 && c.enabled,
            scale: s,
        };
        let mut label = c.bounds;
        label.x += (10 * s) as i32;
        label.width = label.width.saturating_sub((20 * s) as u32);
        let entry = view.state.selected_entry(&view.data);
        if let Target::Field(index) = c.target {
            let title: &[u8] = match index {
                0 => b"Display name",
                1 => {
                    if view.state.tab == Tab::Users {
                        b"User name"
                    } else {
                        b"Group name"
                    }
                }
                2 => b"New password",
                _ => b"Confirm password",
            };
            self.app_label(
                Rect {
                    height: (24 * s) as u32,
                    ..c.bounds
                },
                title,
                (176, 197, 223),
                false,
                s,
            );
            let field = Rect {
                y: c.bounds.y + (26 * s) as i32,
                height: c.bounds.height.saturating_sub((26 * s) as u32),
                ..c.bounds
            };
            self.precision_surface(field, Surface::Field, material);
            let mask = [b'*'; 72];
            let text = match index {
                0 => view.state.name.as_bytes(),
                1 => view.state.handle.as_bytes(),
                _ => &mask[..view.state.password_lengths[index - 2].min(72)],
            };
            let field_label = Rect {
                x: field.x + (10 * s) as i32,
                width: field.width.saturating_sub((20 * s) as u32),
                ..field
            };
            let cursor = match index {
                0 => view.state.name.cursor,
                1 => view.state.handle.cursor,
                _ => view.state.password_cursors[index - 2],
            }
            .min(text.len());
            let mut start = 0;
            while start < cursor
                && self.app_text_width(&text[start..cursor], false, s) + s
                    > field_label.width as usize
            {
                start += 1;
            }
            if (index == 0 && view.state.name.selected)
                || (index == 1 && view.state.handle.selected)
            {
                self.fill_rect_alpha(
                    field_label.x.max(0) as usize,
                    field_label.y.max(0) as usize + 5 * s,
                    self.app_text_width(&text[start..], false, s)
                        .min(field_label.width as usize),
                    24 * s,
                    62,
                    111,
                    255,
                    150,
                );
            }
            self.app_label(field_label, &text[start..], color, false, s);
            if view.state.focus == c.target {
                let x = field_label.x.max(0) as usize
                    + self.app_text_width(&text[start..cursor], false, s);
                self.fill_rect(
                    x,
                    field_label.y.max(0) as usize + 8 * s,
                    s,
                    18 * s,
                    134,
                    199,
                    255,
                );
            }
        } else if matches!(c.target, Target::Enabled | Target::Member(_)) {
            let (checked, text) = match c.target {
                Target::Enabled => (
                    entry.is_some_and(|e| e.enabled),
                    b"Account enabled".as_slice(),
                ),
                Target::Member(i) if view.state.tab == Tab::Users => {
                    let group = &view.data.groups[i];
                    (
                        view.state
                            .selected_index(&view.data)
                            .is_some_and(|u| group.members & (1 << u) != 0),
                        group.name.as_bytes(),
                    )
                }
                Target::Member(i) => (
                    entry.is_some_and(|g| g.members & (1 << i) != 0),
                    view.data.users[i].name.as_bytes(),
                ),
                _ => unreachable!(),
            };
            if focused {
                self.precision_surface(c.bounds, Surface::Secondary, material);
            }
            self.precision_checkbox(
                Rect {
                    x: c.bounds.x + (8 * s) as i32,
                    y: c.bounds.y + (10 * s) as i32,
                    width: (20 * s) as u32,
                    height: (20 * s) as u32,
                },
                checked,
                c.enabled,
                material,
            );
            label.x += (28 * s) as i32;
            label.width = label.width.saturating_sub((28 * s) as u32);
            self.app_label(label, text, color, false, s);
        } else {
            if selected || focused || !matches!(c.target, Target::Row(_)) {
                self.precision_surface(
                    c.bounds,
                    if selected || c.target == Target::Save {
                        Surface::Primary
                    } else {
                        Surface::Secondary
                    },
                    material,
                );
            }
            let text: &[u8] = match c.target {
                Target::Users => b"Users",
                Target::Groups => b"Groups",
                Target::New => b"New",
                Target::Back => b"Back",
                Target::Row(i) => view.data.entries(view.state.tab)[i].name.as_bytes(),
                Target::Rename => b"Edit names",
                Target::Password => b"Set / reset password",
                Target::Delete => b"Delete",
                Target::Save => b"Save",
                Target::Cancel => b"Cancel",
                Target::Confirm => b"Confirm delete",
                Target::ListUp | Target::DetailUp => b"",
                Target::ListDown | Target::DetailDown => b"",
                _ => b"",
            };
            if matches!(
                c.target,
                Target::ListUp | Target::ListDown | Target::DetailUp | Target::DetailDown
            ) {
                let down = matches!(c.target, Target::ListDown | Target::DetailDown);
                self.app_symbol(c.bounds, if down { b'v' } else { b'^' }, color, s);
            } else {
                self.app_label(
                    label,
                    text,
                    if matches!(c.target, Target::Delete | Target::Confirm) && c.enabled {
                        (255, 164, 174)
                    } else {
                        color
                    },
                    selected,
                    s,
                );
            }
        }
        self.render_clip = old;
    }
}
