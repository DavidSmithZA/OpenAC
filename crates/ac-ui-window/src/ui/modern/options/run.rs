//! The Options window in the game window, in place of retail's options
//! window: F11, the panel strip and the Effects window's settings button
//! open it. It keeps a copy of the values as retail's window does; each
//! change is made live through the same path, Apply saves the character's
//! options, and closing it with changes not applied reverts them.

use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::modern::Horizon;
use ac_ui::panels::items::emit;
use ac_ui::panels::options::keys::LiveValues;
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::strings::compute_hash;
use ac_ui::tree::ElementId;
use ac_ui::widgets::Field;

use super::model::{OptionsModel, Sources, rows};
use super::spec::Action;
use super::window::{GridTab, HEIGHT, OptionsFonts, OptionsFrame, OptionsRequest, WIDTH};
use crate::ui::modern::{hud, kit};
use crate::ui::window::WindowUi;

/// The window's name.
pub const OPTIONS: &str = "modern-options";

/// The Options window's elements and what it remembers.
#[derive(Debug)]
pub struct OptionsUi {
    /// The frame (the window).
    pub frame: ElementId,
    /// The search field.
    pub search: ElementId,
    /// It was showing last frame.
    was_open: bool,
}

impl WindowUi {
    /// The values an options window shows: the settings in force and the
    /// character's two option words.
    pub(in crate::ui) fn live_option_values(&self) -> LiveValues {
        LiveValues {
            settings: self.settings().clone(),
            options1: self
                .player_view
                .map_or(ac_runtime::character::DEFAULT_OPTIONS1, |v| v.options1),
            options2: self
                .player_view
                .map_or(ac_runtime::character::DEFAULT_OPTIONS2, |v| v.options2),
        }
    }

    /// The Options window, hidden, in place of retail's.
    pub(in crate::ui) fn mount_modern_options(&mut self) {
        let fonts = OptionsFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            nav: self.modern_font(Weight::SemiBold, 13.0),
            heading: self.modern_font(Weight::SemiBold, 18.0),
            intro: self.modern_font(Weight::Regular, 12.0),
            label: self.modern_font(Weight::SemiBold, 10.5),
            name: self.modern_font(Weight::SemiBold, 13.0),
            desc: self.modern_font(Weight::Regular, 11.5),
            control: self.modern_font(Weight::SemiBold, 12.0),
            small: self.modern_font(Weight::SemiBold, 9.5),
        };
        let field_font = self.modern_font(Weight::Regular, 12.0);
        let model = {
            let strings = self.content.strings();
            let say = |key: &str| {
                strings.resolve(ac_ui::panels::options::STRING_TABLE, compute_hash(key), 0)
            };
            let src = Sources {
                strings: &say,
                resolutions: &self.resolutions,
            };
            OptionsModel::new(rows(&src), self.live_option_values())
        };
        let frame = match hud::mount_piece(
            &mut self.host.root,
            OPTIONS,
            (WIDTH, HEIGHT),
            OptionsFrame::new(Horizon::BRONZE, fonts, model),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern options: {e}"));
                return;
            }
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        let search = kit::mount_search(
            ui,
            frame,
            OptionsFrame::search_rect(),
            field_font,
            &Horizon::BRONZE,
        );
        ui.hide_window(OPTIONS);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.options = Some(OptionsUi {
            frame,
            search,
            was_open: false,
        });
        modern.replace(crate::ui::options::OPTIONS_WINDOW);
        self.panels.stand_in(panel_id::OPTIONS, OPTIONS);
    }

    /// The Options window opens on `section`. Without it, retail's options
    /// window opens instead.
    pub(in crate::ui) fn modern_options_on(&mut self, section: super::spec::Section) {
        if let Some(frame) = self
            .modern
            .as_ref()
            .and_then(|m| m.options.as_ref())
            .map(|o| o.frame)
            && let Some(f) = self.host.root.widget_mut::<OptionsFrame>(frame)
        {
            f.show(section);
        }
        self.panels
            .set_visibility(&mut self.host.root, panel_id::OPTIONS, true);
    }

    /// The values changed outside the window (a console command, the
    /// player's options words arriving): the window's copy follows. While
    /// it is closed the saved copy follows too.
    pub(in crate::ui) fn follow_modern_option_values(&mut self) {
        let Some(frame) = self
            .modern
            .as_ref()
            .and_then(|m| m.options.as_ref())
            .map(|o| o.frame)
        else {
            return;
        };
        let live = self.live_option_values();
        let open = self.host.root.is_window_visible(OPTIONS);
        if let Some(f) = self.host.root.widget_mut::<OptionsFrame>(frame)
            && f.model.live != live
        {
            f.model.live = live;
            if !open {
                f.model.applied();
            }
        }
    }

    /// Each frame: the search, the chat tabs for the grid, what the frame
    /// asked for; on opening the values are taken afresh, on closing what
    /// was not applied is reverted.
    pub(in crate::ui) fn tick_modern_options(&mut self) {
        let Some((frame, search, was_open)) = self
            .modern
            .as_ref()
            .and_then(|m| m.options.as_ref())
            .map(|o| (o.frame, o.search, o.was_open))
        else {
            return;
        };
        let open = self.host.root.is_window_visible(OPTIONS);
        if open != was_open {
            if let Some(o) = self.modern.as_mut().and_then(|m| m.options.as_mut()) {
                o.was_open = open;
            }
            if open {
                let live = self.live_option_values();
                if let Some(f) = self.host.root.widget_mut::<OptionsFrame>(frame) {
                    f.model.live = live;
                    f.model.applied();
                }
            } else {
                self.revert_modern_options(frame);
                if self.host.root.keyboard_focus() == Some(search) {
                    self.host.root.set_keyboard_focus(None);
                }
            }
        }
        if !open {
            return;
        }
        let tabs: Vec<GridTab> = self
            .modern_chat_tab_filters()
            .into_iter()
            .map(|(name, filter)| GridTab { name, filter })
            .collect();
        let asks = {
            let ui = &mut self.host.root;
            let typed = kit::field_text(ui, search);
            let focused = ui.keyboard_focus() == Some(search);
            ui.widget_mut::<OptionsFrame>(frame)
                .map(|f| {
                    f.search = typed;
                    f.searching = focused;
                    f.tabs = tabs;
                    std::mem::take(&mut f.requests)
                })
                .unwrap_or_default()
        };
        for ask in asks {
            self.modern_options_request(frame, search, ask);
        }
    }

    fn modern_options_request(&mut self, frame: ElementId, search: ElementId, ask: OptionsRequest) {
        match ask {
            OptionsRequest::Close => {
                self.panels
                    .set_visibility(&mut self.host.root, panel_id::OPTIONS, false);
            }
            OptionsRequest::Changed(section, persist) => {
                let Some(live) = self
                    .host
                    .root
                    .widget::<OptionsFrame>(frame)
                    .map(|f| f.model.live.clone())
                else {
                    return;
                };
                let mut commands = Vec::new();
                self.option_changed(&live, section, persist, &mut commands);
                self.send_session(commands);
            }
            OptionsRequest::Applied => {
                self.send_session(vec![SessionCommand::SaveCharacterOptions]);
            }
            OptionsRequest::CharacterSelect => self.ask_end_character_session(),
            OptionsRequest::ExitGame => self.send_session(vec![SessionCommand::Shutdown]),
            OptionsRequest::Act(Action::EditBars) => {
                self.panels
                    .set_visibility(&mut self.host.root, panel_id::OPTIONS, false);
                self.modern_open_bar_edit();
            }
            OptionsRequest::Act(Action::MouseTurning) => {
                let mut commands = Vec::new();
                self.use_mouse_turning_settings(&mut commands);
                self.send_session(commands);
                self.follow_modern_option_values();
            }
            OptionsRequest::Act(Action::Keyboard) => {
                self.toggle_keyboard_config();
            }
            OptionsRequest::TabShows { tab, group, on } => {
                let mask = crate::ui::modern::chat::model::group_mask(group);
                if let Some((_, filter)) = self.modern_chat_tab_filters().get(tab) {
                    let filter = if on { filter | mask } else { filter & !mask };
                    self.set_modern_chat_tab_filter(tab, filter);
                }
            }
            OptionsRequest::ClearSearch => {
                self.host
                    .root
                    .with::<Field, _>(search, |f, _| f.set_text(""));
                if self.host.root.keyboard_focus() == Some(search) {
                    self.host.root.set_keyboard_focus(None);
                }
            }
        }
    }

    /// What was not applied goes back to what it was.
    fn revert_modern_options(&mut self, frame: ElementId) {
        let Some((touched, live)) = self
            .host
            .root
            .widget_mut::<OptionsFrame>(frame)
            .map(|f| (f.model.revert(), f.model.live.clone()))
        else {
            return;
        };
        let mut commands = Vec::new();
        for section in touched {
            self.option_changed(&live, section, true, &mut commands);
        }
        self.send_session(commands);
    }

    fn send_session(&mut self, commands: Vec<SessionCommand>) {
        for c in commands {
            emit(&mut self.host.root, PanelOutput::Session(c));
        }
    }
}
