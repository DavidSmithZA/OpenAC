//! The Options window in the game window, in place of retail's options
//! window: F11, the panel strip and the Effects window's settings button
//! open it. It keeps a copy of the values as retail's window does; each
//! change is made live through the same path, Apply saves the character's
//! options, and closing it with changes not applied reverts them.

use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::input::KeyChord;
use ac_ui::input::action_map::RetailActionClass;
use ac_ui::modern::Horizon;
use ac_ui::panels::items::emit;
use ac_ui::panels::keyboard_config::{
    CaptureVerdict, KeyboardConfigController, KeyboardConfigOutput,
};
use ac_ui::panels::options::OptionValue;
use ac_ui::panels::options::keys::{LiveValues, OptionKey};
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::strings::compute_hash;
use ac_ui::tree::ElementId;
use ac_ui::widgets::Field;

use super::map_row::MapVerb;
use super::model::{OptionsModel, Sources, rows};
use super::spec::Action;
use super::window::{
    Clash, GridTab, HEIGHT, KeyLine, OptionsFonts, OptionsFrame, OptionsRequest, WIDTH,
};
use crate::ui::keyboard_window::KEYBOARD_WINDOW;
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
    /// The key dispatcher was capturing, last frame.
    dispatcher_capturing: bool,
}

/// A key as the modern windows name it ("Alt+D", "Shift"), the game's
/// name for one they do not (a mouse button).
fn key_words(c: &KeyboardConfigController, chord: KeyChord) -> String {
    let words = crate::ui::modern::bars::keys::words(chord);
    if words.is_empty() {
        c.describe(chord)
    } else {
        words
    }
}

/// The heading of a keyboard class.
fn class_words(class: RetailActionClass) -> &'static str {
    match class {
        RetailActionClass::Movement => "MOVEMENT KEYS",
        RetailActionClass::Camera => "CAMERA KEYS",
        RetailActionClass::Combat => "COMBAT KEYS",
        RetailActionClass::Ui => "WINDOW KEYS",
        RetailActionClass::CharacterSettings => "CHARACTER KEYS",
        RetailActionClass::Emote => "EMOTE KEYS",
        RetailActionClass::None | RetailActionClass::Other(_) => "OTHER KEYS",
    }
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
            OptionsFrame::new(fonts, model),
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
            &Horizon::current(),
        );
        ui.hide_window(OPTIONS);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.options = Some(OptionsUi {
            frame,
            search,
            was_open: false,
            dispatcher_capturing: false,
        });
        modern.replace(crate::ui::options::OPTIONS_WINDOW);
        modern.replace(KEYBOARD_WINDOW);
        self.panels.stand_in(panel_id::OPTIONS, OPTIONS);
    }

    /// The section the Options window shows, while it shows.
    pub(in crate::ui) fn modern_options_showing(&self) -> Option<super::spec::Section> {
        let frame = self.modern.as_ref()?.options.as_ref()?.frame;
        if !self.host.root.is_window_visible(OPTIONS) {
            return None;
        }
        let f = self.host.root.widget::<OptionsFrame>(frame)?;
        f.search.trim().is_empty().then_some(f.section)
    }

    /// The Options window opens on `section` (a search typed is cleared,
    /// so the section shows). Without it, retail's options window opens
    /// instead.
    pub(in crate::ui) fn modern_options_on(&mut self, section: super::spec::Section) {
        if let Some((frame, search)) = self
            .modern
            .as_ref()
            .and_then(|m| m.options.as_ref())
            .map(|o| (o.frame, o.search))
        {
            self.host
                .root
                .with::<Field, _>(search, |f, _| f.set_text(""));
            if let Some(f) = self.host.root.widget_mut::<OptionsFrame>(frame) {
                f.search.clear();
                f.show(section);
            }
        }
        self.panels
            .set_visibility(&mut self.host.root, panel_id::OPTIONS, true);
    }

    /// The values changed outside the window (a console command, the
    /// player's options words arriving): the window's copy of those
    /// follows (`settings`: the client settings; else the character's
    /// options words), the rest of it is left as the window has it. While
    /// it is closed the saved copy follows too.
    pub(in crate::ui) fn follow_modern_option_values(&mut self, settings: bool) {
        let Some(frame) = self
            .modern
            .as_ref()
            .and_then(|m| m.options.as_ref())
            .map(|o| o.frame)
        else {
            return;
        };
        let now = self.live_option_values();
        let open = self.host.root.is_window_visible(OPTIONS);
        if let Some(f) = self.host.root.widget_mut::<OptionsFrame>(frame) {
            let mut live = f.model.live.clone();
            if settings {
                live.settings = now.settings;
            } else {
                live.options1 = now.options1;
                live.options2 = now.options2;
            }
            if f.model.live == live {
                return;
            }
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
                self.world_maps.ask();
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
        let (keys, keys_changed) = self.modern_key_lines();
        let map_row = super::map_row::map_row(&self.world_maps.status(), self.chosen_map_tier());
        let asks = {
            let ui = &mut self.host.root;
            let typed = kit::field_text(ui, search);
            let focused = ui.keyboard_focus() == Some(search);
            ui.widget_mut::<OptionsFrame>(frame)
                .map(|f| {
                    f.search = typed;
                    f.searching = focused;
                    f.tabs = tabs;
                    f.keys = keys;
                    f.keys_changed = keys_changed;
                    f.set_map(map_row);
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
                self.with_key_config(|c, ui| {
                    c.keep(ui);
                });
            }
            OptionsRequest::CharacterSelect => self.ask_end_character_session(),
            OptionsRequest::ExitGame => self.send_session(vec![SessionCommand::Shutdown]),
            OptionsRequest::Act(Action::EditBars) => {
                // Editing the bars closes the window: what was changed is
                // applied first rather than taken back by the close.
                let changed = self
                    .host
                    .root
                    .widget_mut::<OptionsFrame>(frame)
                    .is_some_and(|f| {
                        let changed = !f.model.changed().is_empty();
                        f.model.applied();
                        changed
                    });
                if changed {
                    self.send_session(vec![SessionCommand::SaveCharacterOptions]);
                }
                self.with_key_config(|c, ui| {
                    c.keep(ui);
                });
                self.panels
                    .set_visibility(&mut self.host.root, panel_id::OPTIONS, false);
                self.modern_open_bar_edit();
            }
            OptionsRequest::Act(Action::MouseTurning) => {
                // The button changes and saves the camera settings and the
                // character's mouse turning at once: those are saved, not
                // changes for Apply (or for closing to take back).
                let before = self.live_option_values();
                let mut commands = Vec::new();
                self.use_mouse_turning_settings(&mut commands);
                let mut after = self.live_option_values();
                for c in &commands {
                    if let SessionCommand::SetCharacterOption { option, value } = c {
                        after.write(OptionKey::Character(*option), &OptionValue::Bool(*value));
                    }
                }
                self.send_session(commands);
                if let Some(f) = self.host.root.widget_mut::<OptionsFrame>(frame) {
                    let keys: Vec<OptionKey> = f.model.rows.iter().flat_map(|r| r.keys()).collect();
                    for key in keys {
                        if let Some(v) = after.read(key)
                            && before.read(key).as_ref() != Some(&v)
                        {
                            f.model.set_saved(key, &v);
                        }
                    }
                }
            }
            OptionsRequest::Act(Action::LoadKeys) => {
                self.keyboard_output(KeyboardConfigOutput::OpenLoad);
            }
            OptionsRequest::Act(Action::SaveKeysAs) => {
                self.keyboard_output(KeyboardConfigOutput::OpenSaveAs);
            }
            OptionsRequest::Map(verb) => match verb {
                MapVerb::Download(tier) => self.world_maps.download(tier),
                MapVerb::Cancel => self.world_maps.cancel(),
                MapVerb::Remove(tier) => self.world_maps.remove(tier),
            },
            OptionsRequest::CaptureKey { .. } => self.request_key_capture(),
            OptionsRequest::EraseKey { row, slot } => {
                self.with_key_config(|c, ui| {
                    c.erase(ui, row, slot);
                });
            }
            OptionsRequest::TakeKey => {
                self.with_key_config(|c, ui| c.overwrite_answered(ui, true));
            }
            OptionsRequest::KeysRevert => {
                self.with_key_config(|c, ui| {
                    c.revert(ui);
                });
            }
            OptionsRequest::KeysDefaults => {
                self.with_key_config(|c, ui| {
                    c.defaults(ui);
                });
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
        let capturing = self
            .host
            .root
            .widget_mut::<OptionsFrame>(frame)
            .is_some_and(|f| {
                f.clash = None;
                f.capture.take().is_some()
            });
        if capturing {
            self.cancel_key_capture();
        }
        self.with_key_config(|c, ui| {
            c.overwrite_answered(ui, false);
            if c.changed() {
                c.revert(ui);
            }
        });
        let (keys, keys_changed) = self.modern_key_lines();
        if let Some(f) = self.host.root.widget_mut::<OptionsFrame>(frame) {
            f.keys = keys;
            f.keys_changed = keys_changed;
        }
    }

    /// Every key binding as the Controls section lists it, and how many
    /// differ from the saved keys.
    fn modern_key_lines(&mut self) -> (Vec<KeyLine>, usize) {
        self.with_key_config(|c, _| {
            let lines: Vec<KeyLine> = c
                .rows()
                .iter()
                .enumerate()
                .filter_map(|(row, r)| {
                    Some(KeyLine {
                        row,
                        label: r.label.clone()?,
                        group: class_words(r.class),
                        keys: r.current.iter().map(|k| key_words(c, *k)).collect(),
                        changed: r.changed(),
                        mapped: r.mapped.is_some(),
                    })
                })
                .collect();
            let changed = c.rows().iter().filter(|r| r.changed()).count();
            (lines, changed)
        })
        .unwrap_or_default()
    }

    /// The key caught for the Options window, when it asked for one:
    /// whether to keep catching (`None`: it did not ask). Escape gives
    /// up; a key something else holds waits on the row for the player's
    /// word.
    pub(in crate::ui) fn modern_key_captured(&mut self, chord: Option<KeyChord>) -> Option<bool> {
        let frame = self.modern.as_ref()?.options.as_ref()?.frame;
        let (row, slot) = self.host.root.widget::<OptionsFrame>(frame)?.capture?;
        let found = self.with_key_config(|c, ui| {
            let verdict = c.judge_capture(row, chord);
            let labels = |rows: &[usize]| {
                rows.iter()
                    .filter_map(|r| c.rows().get(*r).and_then(|r| r.label.clone()))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let clash = match &verdict {
                CaptureVerdict::Reserved(chord) => Some(Clash {
                    row,
                    slot,
                    words: format!(
                        "{} is kept by the game for its own use.",
                        key_words(c, *chord)
                    ),
                    can_take: false,
                }),
                CaptureVerdict::Taken(chord, rows) => Some(Clash {
                    row,
                    slot,
                    words: format!("{} is used by {}.", key_words(c, *chord), labels(rows)),
                    can_take: true,
                }),
                _ => None,
            };
            match &verdict {
                CaptureVerdict::Free(chord) => c.assign(ui, row, slot, *chord, &[]),
                CaptureVerdict::Taken(chord, rows) => {
                    c.hold_overwrite(row, slot, *chord, rows.clone())
                }
                CaptureVerdict::Reserved(_) => c.overwrite_answered(ui, false),
                _ => {}
            }
            (verdict == CaptureVerdict::KeepWaiting, clash)
        });
        let (waiting, clash) = found.unwrap_or((false, None));
        if waiting {
            return Some(true);
        }
        if let Some(f) = self.host.root.widget_mut::<OptionsFrame>(frame) {
            f.capture = None;
            if clash.is_some() {
                f.clash = clash;
            }
        }
        Some(false)
    }

    /// The key dispatcher's capture as it stands: one that ended without a
    /// key reaching the window (the game lost the focus) leaves no slot
    /// waiting for a key.
    pub fn follow_key_capture(&mut self, capturing: bool) {
        let Some(o) = self.modern.as_mut().and_then(|m| m.options.as_mut()) else {
            return;
        };
        let dropped = o.dispatcher_capturing && !capturing;
        o.dispatcher_capturing = capturing;
        let frame = o.frame;
        if dropped && let Some(f) = self.host.root.widget_mut::<OptionsFrame>(frame) {
            f.capture = None;
        }
    }

    fn send_session(&mut self, commands: Vec<SessionCommand>) {
        for c in commands {
            emit(&mut self.host.root, PanelOutput::Session(c));
        }
    }
}
