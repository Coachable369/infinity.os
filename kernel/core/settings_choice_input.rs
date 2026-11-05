//! Choice lists use explicit service setters. Opening or moving focus never writes a preference.
use super::*;
use crate::ui::controls::choices;

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: show_settings_choices
    // DESC: Builds an anchored option list from detected hardware, installed models, or the setting's bounded domain.
    // ------------------=
    pub(super) fn show_settings_choices(&mut self, row: usize) {
        let section = self.system_focus;
        if self.settings_window.expanded_row == Some(row) && choices::count(section, row) > 0 {
            self.dismiss_settings_choices();
            return;
        }
        let user = self.current_user;
        let prefs = crate::ui::input_preferences::current();
        let (count, selected) = match (section, row) {
            (5, 0) => {
                let modes = self.display_modes.detected();
                (
                    modes.len().max(1),
                    modes
                        .iter()
                        .position(|m| *m == self.display_modes.selected())
                        .unwrap_or(0),
                )
            }
            (3, 0) => (
                2,
                crate::runtime::with_runtime(|r| {
                    r.identity.ai_profile(user).is_some_and(|p| {
                        p.provider_policy == crate::runtime::identity::AiProviderPolicy::PreferLocal
                    })
                })
                .unwrap_or(false) as usize,
            ),
            (3, 2) => (
                crate::runtime::ai::chat::CHAT_MODELS.len(),
                crate::runtime::ai::with_ai_runtime(|r| r.chat.selected_model_index()),
            ),
            (3, 5) => (
                2,
                crate::runtime::with_runtime(|r| {
                    r.identity.voice_profile(user).is_some_and(|v| {
                        v.wake_word == crate::runtime::identity::WakeWord::Computer
                    })
                })
                .unwrap_or(false) as usize,
            ),
            (3, 7) => (
                5,
                crate::runtime::with_runtime(|r| {
                    r.identity
                        .voice_profile(user)
                        .map_or(0, |p| p.output_voice as usize)
                })
                .unwrap_or(0),
            ),
            (1, 0) => (
                2,
                crate::runtime::with_runtime(|r| {
                    r.ui.skins.active().id
                        == crate::ui::skin::SkinId::from_bytes(b"infinity.default.light")
                })
                .unwrap_or(false) as usize,
            ),
            (1, 1) => (
                crate::ui::icon_theme::ICON_THEME_COUNT as usize,
                crate::runtime::with_runtime(|r| r.ui.icons.active() as usize).unwrap_or(0),
            ),
            (10, 0..=6) => (
                match row {
                    0 | 1 => 10,
                    5 | 6 => 4,
                    _ => 2,
                },
                prefs.choice_index(row),
            ),
            _ => return,
        };
        choices::open(section, row, count, selected, |index, out| {
            let label: &[u8] = match (section, row) {
                (5, 0) => {
                    let mode = self
                        .display_modes
                        .detected()
                        .get(index)
                        .copied()
                        .unwrap_or(self.display_modes.selected());
                    let len = write_decimal(out, mode[0] as usize);
                    out[len..len + 3].copy_from_slice(b" x ");
                    write_decimal(&mut out[len + 3..], mode[1] as usize);
                    return;
                }
                (3, 0) => [b"Local only".as_slice(), b"Prefer local"][index],
                (3, 2) => crate::runtime::ai::chat::CHAT_MODELS[index].name,
                (3, 5) => [b"Infinity".as_slice(), b"Computer"][index],
                (3, 7) => crate::runtime::identity::OUTPUT_VOICE_NAMES[index],
                (1, 0) => [b"Dark".as_slice(), b"Light"][index],
                (1, 1) => crate::ui::icon_theme::IconThemeId::from_u8(index as u8)
                    .unwrap()
                    .name(),
                (10, _) => {
                    let mut candidate = prefs;
                    candidate.select_choice(row, index);
                    candidate.value(row)
                }
                _ => return,
            };
            let n = label.len().min(out.len());
            out[..n].copy_from_slice(&label[..n]);
        });
        self.settings_window.expanded_row = Some(row);
        self.reveal_settings_choice(row);
    }

    // ------------------------=
    // FUNC: reveal_settings_choice
    // DESC: Scrolls only far enough to keep the highlighted item inside the content viewport.
    // ------------------=
    pub(super) fn reveal_settings_choice(&mut self, row: usize) {
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let window =
            layout.settings_window_geometry_for_section(self.settings_window, self.system_focus);
        let item = layout.settings_choice_item(
            self.settings_window,
            self.system_focus,
            row,
            choices::focus(),
        );
        let delta = if item.y < window.viewport.y {
            item.y - window.viewport.y
        } else {
            (item.bottom() - window.viewport.bottom()).max(0)
        };
        self.settings_window.scroll_offset = self
            .settings_window
            .scroll_offset
            .saturating_add_signed(delta as isize / layout.scale() as isize)
            .min(window.maximum_scroll);
        self.settings_scroll_target = self.settings_window.scroll_offset;
    }

    // ------------------------=
    // FUNC: dismiss_settings_choices
    // DESC: Closes the active selection list and clamps scrolling to the collapsed form.
    // ------------------=
    pub(super) fn dismiss_settings_choices(&mut self) {
        choices::close();
        self.settings_window.expanded_row = None;
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        self.settings_window.scroll_offset = self.settings_window.scroll_offset.min(
            layout
                .settings_window_geometry_for_section(self.settings_window, self.system_focus)
                .maximum_scroll,
        );
        self.settings_scroll_target = self.settings_window.scroll_offset;
    }

    // ------------------------=
    // FUNC: apply_display_choice
    // DESC: Applies the selected hardware mode before changing layout or storing a boot preference.
    // ------------------=
    pub(super) fn apply_display_choice(&mut self,index:usize) {
        let Some([w,h])=self.display_modes.detected().get(index).copied() else{return;};
        if crate::bootstrap::change_display_mode(w,h) {
            self.system.framebuffer_width=w as usize;self.system.framebuffer_height=h as usize;
            self.display_modes.applied(w,h);
            self.display_modes.select(self.system.firmware_runtime_services,index);
        }else{self.display_modes.rejected();}
    }
    // ------------------------=
    // FUNC: commit_settings_choice
    // DESC: Applies the clicked or keyboard-selected option exactly once through its owning service.
    // ------------------=
    pub(super) fn commit_settings_choice(&mut self, row: usize, index: usize) {
        if index >= choices::count(self.system_focus, row) {
            return;
        }
        let user = self.current_user;
        match (self.system_focus, row) {
            (5, 0) => self.apply_display_choice(index),
            (3, 2) => {
                crate::runtime::ai::with_ai_runtime(|r| r.chat.select_model_index(index));
                self.persist_ai_chat_preferences();
            }
            (3, 7) => {
                if crate::runtime::with_runtime(|r| {
                    r.identity.update_output_voice(user, user, index as u8)
                })
                .is_some_and(|r| r.is_ok())
                {
                    let _ = crate::runtime::persist_identity_state();
                }
            }
            (3, 0) => {
                let policy = if index == 0 {
                    crate::runtime::identity::AiProviderPolicy::LocalOnly
                } else {
                    crate::runtime::identity::AiProviderPolicy::PreferLocal
                };
                if crate::runtime::with_runtime(|r| {
                    r.identity.update_ai_profile(user, user, policy)
                })
                .is_some_and(|r| r.is_ok())
                {
                    let _ = crate::runtime::persist_identity_state();
                }
            }
            (3, 5) => {
                let wake = if index == 0 {
                    b"Infinity".as_slice()
                } else {
                    b"Computer"
                };
                if crate::runtime::ai::with_ai_runtime(|r| r.save_hermes_field(b"wake_word", wake))
                {
                    let _ = crate::runtime::persist_identity_state();
                }
            }
            (1, 0) => {
                self.dispatch_setting(
                    crate::runtime::ai::control::setting_command::Request::Theme(index == 1),
                );
            }
            (1, 1) => self.select_icon_theme(index as u8),
            (10, _) => {
                let mut prefs = crate::ui::input_preferences::current();
                prefs.select_choice(row, index);
                crate::ui::input_preferences::apply(prefs);
                let _ = self.persist_desktop_layout();
            }
            _ => {}
        }
        self.dismiss_settings_choices();
        crate::bootstrap::system_ui_settings_changed();
    }
}
