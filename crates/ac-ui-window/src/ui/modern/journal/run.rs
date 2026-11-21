//! The Journal in the game window, in place of retail's journal (its
//! contracts, notes and page list): J and the panel strip open it; the
//! quest keys open it on Quests, the journal keys on Notes. The notes are
//! kept as retail keeps them (the session's journal on this computer):
//! what is typed is sent when the field is left, a page turned or the
//! window closed. The quests on screen are kept per character.

use ac_runtime::communication::JournalCommand;
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::modern::Horizon;
use ac_ui::panels::items::emit;
use ac_ui::panels::journal_notes::{parse_timer_field, timer_field_text};
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::radar::RadarCoordinates;
use ac_ui::tree::ElementId;

use super::model::{self, JournalTab, QuestsInput};
use super::quests_hud::{QuestsHud, QuestsHudFonts};
use super::window::{HEIGHT, JournalAsk, JournalFonts, JournalFrame, Slot, WIDTH};
use crate::scene::Scene;
use crate::ui::modern::dialogs::look::{Glyph, Look, RED};
use crate::ui::modern::{hud, kit};
use crate::ui::panel_keys::JournalPage;
use crate::ui::social_panels::JOURNAL_WINDOW;
use crate::ui::window::{DialogAction, WindowUi};

/// The window's name.
pub const JOURNAL: &str = "modern-journal";

/// The Journal's elements and what it remembers.
#[derive(Debug)]
pub struct JournalUi {
    /// The window.
    pub frame: ElementId,
    /// The fields over it.
    pub fields: Vec<(Slot, ElementId)>,
    /// The quests on screen.
    pub quests: Option<ElementId>,
    /// The quests each character shows (read once).
    shown: Option<ac_host::settings::QuestsOnScreen>,
    /// The journal's revision and page last put in the fields.
    rendered: Option<(u64, usize)>,
    /// The page field that had the keyboard last frame.
    focused: Option<Slot>,
    /// It showed last frame.
    was_open: bool,
    /// What the lists were last built from (rebuilt only when it changes).
    built: Option<ScreensKey>,
}

/// What the Journal's lists are built from: the contracts' and the
/// journal's revisions, the pick, the searches, the quests shown, and the
/// second (the countdowns).
#[derive(Debug, Clone, PartialEq)]
struct ScreensKey {
    contracts: Option<u64>,
    journal: Option<u64>,
    picked: u32,
    quest_search: String,
    note_search: String,
    shown: Vec<u32>,
    second: i64,
}

fn journal(command: JournalCommand) -> PanelOutput {
    PanelOutput::Session(SessionCommand::Journal(command))
}

impl WindowUi {
    /// The Journal, hidden, in place of retail's journal; and the quests on
    /// screen.
    pub(in crate::ui) fn mount_modern_journal(&mut self) {
        let theme = Horizon::BRONZE;
        let fonts = JournalFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            heading: self.modern_font(Weight::SemiBold, 16.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            body: self.modern_font(Weight::Regular, 12.5),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
            big: self.modern_font(Weight::SemiBold, 18.0),
        };
        let field_font = self.modern_font(Weight::Regular, 12.5);
        let hud_fonts = QuestsHudFonts {
            name: self.modern_font(Weight::SemiBold, 12.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
        };
        let frame = match hud::mount_piece(
            &mut self.host.root,
            JOURNAL,
            (WIDTH, HEIGHT),
            JournalFrame::new(theme, fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern journal: {e}"));
                return;
            }
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        let fields = Slot::ALL
            .into_iter()
            .map(|slot| {
                let id = kit::mount_field(
                    ui,
                    frame,
                    (0.0, 0.0, 200.0, if slot.lines() { 200.0 } else { 30.0 }),
                    field_font.clone(),
                    &theme,
                    slot.lines(),
                );
                ui.set_visible(id, false);
                (slot, id)
            })
            .collect();
        ui.hide_window(JOURNAL);
        let quests = match hud::mount_piece(
            ui,
            hud::QUESTS,
            (super::quests_hud::WIDTH, 60.0),
            QuestsHud::new(theme, hud_fonts),
        ) {
            Ok(id) => {
                ui.hide_window(hud::QUESTS);
                Some(id)
            }
            Err(e) => {
                self.notices.push(format!("ui: modern quests: {e}"));
                None
            }
        };
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.journal = Some(JournalUi {
            frame,
            fields,
            quests,
            shown: None,
            rendered: None,
            focused: None,
            was_open: false,
            built: None,
        });
        modern.replace(JOURNAL_WINDOW);
        self.panels.stand_in(panel_id::JOURNAL, JOURNAL);
    }

    fn journal_ui(&self) -> Option<&JournalUi> {
        self.modern.as_ref()?.journal.as_ref()
    }

    /// A journal key: the Journal on its page (a key never closes it, as in
    /// retail). None without the modern window.
    pub(in crate::ui) fn modern_open_journal(&mut self, page: JournalPage) -> Option<bool> {
        let frame = self.journal_ui()?.frame;
        let tab = match page {
            JournalPage::Contracts => JournalTab::Quests,
            JournalPage::Notes | JournalPage::PageList => JournalTab::Notes,
        };
        if let Some(f) = self.host.root.widget_mut::<JournalFrame>(frame) {
            f.show(tab);
        }
        self.panels
            .set_visibility(&mut self.host.root, panel_id::JOURNAL, true);
        Some(true)
    }

    /// The character's key for what is kept per character ("world/name").
    fn journal_character(&self) -> Option<String> {
        let name = self.status.playing.as_deref()?;
        Some(format!(
            "{}/{name}",
            self.status.server_world.as_deref().unwrap_or("")
        ))
    }

    /// The quests the character shows on screen.
    fn shown_quest_ids(&mut self) -> Vec<u32> {
        let key = self.journal_character();
        let store = self.status.settings_store.clone();
        let Some(j) = self.modern.as_mut().and_then(|m| m.journal.as_mut()) else {
            return Vec::new();
        };
        let shown = j.shown.get_or_insert_with(|| {
            store
                .as_ref()
                .map(ac_host::settings::SettingsStore::load_quests_on_screen)
                .unwrap_or_default()
        });
        key.map(|k| shown.shown(&k).to_vec()).unwrap_or_default()
    }

    fn set_quest_shown(&mut self, id: u32, on: bool) {
        let Some(key) = self.journal_character() else {
            return;
        };
        self.shown_quest_ids();
        let store = self.status.settings_store.clone();
        let Some(shown) = self
            .modern
            .as_mut()
            .and_then(|m| m.journal.as_mut())
            .and_then(|j| j.shown.as_mut())
        else {
            return;
        };
        if shown.set_shown(&key, id, on)
            && let Some(store) = store
            && let Err(e) = store.save_quests_on_screen(shown)
        {
            self.notices
                .push(format!("ui: saving the quests on screen: {e}"));
        }
    }

    /// Each frame: the quests on screen fed; while the Journal shows, its
    /// fields placed and read, its pages fed and what it asked for done;
    /// what was typed sent when the window closes.
    pub(in crate::ui) fn tick_modern_journal(&mut self, scene: &Scene) {
        let Some(j) = self.journal_ui() else {
            return;
        };
        let (frame, fields, quests, was_open) = (j.frame, j.fields.clone(), j.quests, j.was_open);
        let now = ac_clock::unix_millis();
        let shown = self.shown_quest_ids();
        if let Some(id) = quests {
            self.feed_modern_quests_hud(id, &shown, now);
        }
        let open = self.host.root.is_window_visible(JOURNAL);
        if let Some(j) = self.modern.as_mut().and_then(|m| m.journal.as_mut()) {
            j.was_open = open;
        }
        if !open {
            if was_open {
                self.commit_journal_page(&fields);
                if let Some(j) = self.modern.as_mut().and_then(|m| m.journal.as_mut()) {
                    j.focused = None;
                    j.rendered = None;
                }
            }
            let ui = &mut self.host.root;
            if let Some(focus) = ui.keyboard_focus()
                && fields.iter().any(|(_, id)| *id == focus)
            {
                ui.set_keyboard_focus(None);
            }
            return;
        }
        self.feed_journal_screens(frame, &fields, &shown, now);
        self.fill_journal_page(&fields);
        self.place_journal_fields(frame, &fields);
        let asks = self
            .host
            .root
            .widget_mut::<JournalFrame>(frame)
            .map(|f| std::mem::take(&mut f.asks))
            .unwrap_or_default();
        for ask in asks {
            self.journal_ask(ask, frame, &fields, scene);
        }
    }

    fn journal_field(&self, fields: &[(Slot, ElementId)], slot: Slot) -> Option<ElementId> {
        fields.iter().find(|(s, _)| *s == slot).map(|(_, id)| *id)
    }

    fn journal_text(&self, fields: &[(Slot, ElementId)], slot: Slot) -> String {
        self.journal_field(fields, slot)
            .map(|id| kit::field_text(&self.host.root, id))
            .unwrap_or_default()
    }

    fn feed_journal_screens(
        &mut self,
        frame: ElementId,
        fields: &[(Slot, ElementId)],
        shown: &[u32],
        now: i64,
    ) {
        let picked = self
            .host
            .root
            .widget::<JournalFrame>(frame)
            .map_or(0, |f| f.picked);
        let quest_search = self.journal_text(fields, Slot::QuestSearch);
        let note_search = self.journal_text(fields, Slot::NoteSearch);
        let key = ScreensKey {
            contracts: self.facts.contracts.as_ref().map(|c| c.revision),
            journal: self.facts.journal.as_ref().map(|v| v.revision),
            picked,
            quest_search: quest_search.clone(),
            note_search: note_search.clone(),
            shown: shown.to_vec(),
            second: now.div_euclid(1000),
        };
        match self.modern.as_mut().and_then(|m| m.journal.as_mut()) {
            Some(j) if j.built.as_ref() == Some(&key) => return,
            Some(j) => j.built = Some(key),
            None => return,
        }
        let quests = self.facts.contracts.as_ref().map(|c| {
            model::quests(&QuestsInput {
                contracts: &c.contracts,
                catalog: &c.catalog,
                picked,
                search: &quest_search,
                shown,
                now_unix_ms: now,
            })
        });
        let notes = self
            .facts
            .journal
            .as_ref()
            .map(|v| model::notes(v, &note_search, now));
        if let Some(f) = self.host.root.widget_mut::<JournalFrame>(frame) {
            if let Some(q) = quests
                && f.quests != q
            {
                f.quests = q;
            }
            if let Some(n) = notes
                && f.notes != n
            {
                f.notes = n;
            }
        }
    }

    /// The page read goes into the page's fields when the journal or the
    /// page changed (not while the player types).
    fn fill_journal_page(&mut self, fields: &[(Slot, ElementId)]) {
        let Some(now) = self
            .facts
            .journal
            .as_ref()
            .map(|v| (v.revision, v.current_page))
        else {
            return;
        };
        let rendered = self.journal_ui().and_then(|j| j.rendered);
        if rendered == Some(now) {
            return;
        }
        // The field being typed in keeps its words unless the page turned.
        let same_page = rendered.is_some_and(|(_, page)| page == now.1);
        let focus = self.host.root.keyboard_focus();
        let Some(page) = self.facts.journal.as_ref().map(|v| v.current().clone()) else {
            return;
        };
        for (slot, words) in [
            (Slot::Label, page.label.clone()),
            (Slot::Title, page.title.clone()),
            (Slot::Notes, page.notes.clone()),
            (Slot::Days, timer_field_text(page.timer_days)),
            (Slot::Hours, timer_field_text(page.timer_hours)),
            (Slot::Minutes, timer_field_text(page.timer_minutes)),
        ] {
            if let Some(id) = self.journal_field(fields, slot)
                && !(same_page && focus == Some(id))
            {
                kit::set_field_text(&mut self.host.root, id, &words);
            }
        }
        if let Some(j) = self.modern.as_mut().and_then(|m| m.journal.as_mut()) {
            j.rendered = Some(now);
        }
    }

    fn place_journal_fields(&mut self, frame: ElementId, fields: &[(Slot, ElementId)]) {
        let ui = &mut self.host.root;
        let rects: Vec<Option<kit::Rect>> = ui
            .widget::<JournalFrame>(frame)
            .map(|f| fields.iter().map(|(s, _)| f.field_rect(*s)).collect())
            .unwrap_or_default();
        let focus = ui.keyboard_focus();
        let mut empty = Vec::new();
        let mut counts = Vec::new();
        let mut focused = None;
        for ((slot, id), rect) in fields.iter().zip(rects) {
            match rect {
                Some(r) => {
                    let (y, h) = if slot.lines() {
                        (r.1 + 4.0, r.3 - 8.0)
                    } else {
                        (r.1 + (r.3 - 24.0) * 0.5, 24.0)
                    };
                    if let Some(e) = ui.element_mut(*id) {
                        e.left = r.0 + 6.0;
                        e.top = y;
                        e.width = r.2 - 12.0;
                        e.height = h;
                    }
                    if !ui.element(*id).is_some_and(|e| e.visible) {
                        ui.set_visible(*id, true);
                    }
                }
                None => {
                    if ui.element(*id).is_some_and(|e| e.visible) {
                        ui.set_visible(*id, false);
                    }
                    if focus == Some(*id) {
                        ui.set_keyboard_focus(None);
                    }
                }
            }
            let mut words = kit::field_text(ui, *id);
            // Each box takes as much as the journal keeps.
            let most = slot.most();
            if most > 0
                && let Some((cut, _)) = words.char_indices().nth(most)
            {
                words.truncate(cut);
                kit::set_field_text(ui, *id, &words);
            }
            if words.trim().is_empty() {
                empty.push(*slot);
            }
            counts.push((*slot, words.chars().count()));
            if focus == Some(*id) {
                focused = Some(*slot);
            }
        }
        if let Some(f) = ui.widget_mut::<JournalFrame>(frame) {
            f.empty = empty;
            f.counts = counts;
            f.focused = focused;
        }
        // A page field the player left: what they typed is kept.
        let was = self.journal_ui().and_then(|j| j.focused);
        if let Some(was) = was
            && was.on_page()
            && focused != Some(was)
        {
            self.commit_journal_page(fields);
        }
        if let Some(j) = self.modern.as_mut().and_then(|m| m.journal.as_mut()) {
            j.focused = focused;
        }
    }

    /// Sends what is typed on the page read (its texts and the countdown's
    /// length) when it differs from the page.
    fn commit_journal_page(&mut self, fields: &[(Slot, ElementId)]) {
        let Some(view) = self.facts.journal.clone() else {
            return;
        };
        if view.current_page == 0 {
            return;
        }
        let page = view.current();
        let text = |me: &Self, slot| me.journal_text(fields, slot);
        let (label, title, notes) = (
            text(self, Slot::Label),
            text(self, Slot::Title),
            text(self, Slot::Notes),
        );
        let ui = &mut self.host.root;
        if label != page.label || title != page.title || notes != page.notes {
            emit(
                ui,
                journal(JournalCommand::Edit {
                    label,
                    title,
                    notes,
                }),
            );
        }
        let (days, hours, minutes) = (
            parse_timer_field(text(self, Slot::Days).trim()),
            parse_timer_field(text(self, Slot::Hours).trim()),
            parse_timer_field(text(self, Slot::Minutes).trim()),
        );
        if (days, hours, minutes) != (page.timer_days, page.timer_hours, page.timer_minutes)
            && !page.is_timer_running()
        {
            emit(
                &mut self.host.root,
                journal(JournalCommand::SetTimer {
                    days,
                    hours,
                    minutes,
                }),
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn journal_ask(
        &mut self,
        ask: JournalAsk,
        frame: ElementId,
        fields: &[(Slot, ElementId)],
        scene: &Scene,
    ) {
        let out = match ask {
            JournalAsk::Close => {
                self.panels
                    .set_visibility(&mut self.host.root, panel_id::JOURNAL, false);
                return;
            }
            JournalAsk::Tab(_) => {
                self.commit_journal_page(fields);
                let ui = &mut self.host.root;
                if let Some(focus) = ui.keyboard_focus()
                    && fields.iter().any(|(_, id)| *id == focus)
                {
                    ui.set_keyboard_focus(None);
                }
                return;
            }
            JournalAsk::Show(id, on) => {
                self.set_quest_shown(id, on);
                return;
            }
            JournalAsk::Abandon(id) => {
                let detail = self
                    .host
                    .root
                    .widget::<JournalFrame>(frame)
                    .and_then(|f| f.quests.detail.clone())
                    .filter(|d| d.id == id);
                let Some(d) = detail else {
                    return;
                };
                let (look, text) = if d.state == model::QuestState::Done {
                    (
                        Look::new(
                            &format!("Take {} off the list?", d.name),
                            Glyph::Warn,
                            RED,
                            "Remove",
                            "Keep it",
                        ),
                        "It leaves your journal. The game still remembers that you did it."
                            .to_owned(),
                    )
                } else {
                    let from = if d.giver.is_empty() {
                        String::new()
                    } else {
                        format!(" from {}", d.giver)
                    };
                    (
                        Look::new(
                            &format!("Abandon {}?", d.name),
                            Glyph::Warn,
                            RED,
                            "Abandon",
                            "Keep it",
                        )
                        .dangerous(),
                        format!("Your progress on it is lost. You can take it up again{from}."),
                    )
                };
                self.modern_ask_as(
                    look,
                    text,
                    DialogAction::Panel(PanelOutput::Session(SessionCommand::AbandonContract(id))),
                );
                return;
            }
            JournalAsk::Map(cell) => {
                if let Some(c) = RadarCoordinates::from_cell(cell) {
                    self.show_on_modern_map(c.x, c.y, "Quest place");
                }
                return;
            }
            JournalAsk::MapAt(x, y) => {
                self.show_on_modern_map(f64::from(x), f64::from(y), "Recorded place");
                return;
            }
            JournalAsk::NewPage => {
                self.commit_journal_page(fields);
                journal(JournalCommand::NewPage)
            }
            JournalAsk::Goto(n) => {
                self.commit_journal_page(fields);
                journal(JournalCommand::GotoPage(n))
            }
            JournalAsk::Delete(n) => {
                let look = Look::new(
                    &format!("Delete page {n}?"),
                    Glyph::Warn,
                    RED,
                    "Delete",
                    "Keep it",
                )
                .dangerous();
                self.modern_ask_as(
                    look,
                    "Its label, title, notes, place and countdown are lost.".to_owned(),
                    DialogAction::Panel(journal(JournalCommand::DeletePage(n))),
                );
                return;
            }
            JournalAsk::Record => {
                // What was typed goes first, so the page read back keeps it.
                self.commit_journal_page(fields);
                let player = self.facts.state.player;
                let cell = scene
                    .object_by_guid(player)
                    .and_then(|o| o.position.as_ref())
                    .map_or(0, |p| p.cell_id);
                let Some(c) = RadarCoordinates::from_cell(cell) else {
                    emit(
                        &mut self.host.root,
                        PanelOutput::Notice(
                            "Indoors has no place on the map to record.".to_owned(),
                        ),
                    );
                    return;
                };
                #[allow(clippy::cast_possible_truncation)]
                journal(JournalCommand::RecordLocation {
                    x: c.x as f32,
                    y: c.y as f32,
                })
            }
            JournalAsk::Start => {
                self.commit_journal_page(fields);
                journal(JournalCommand::StartTimer)
            }
            JournalAsk::Stop => {
                self.commit_journal_page(fields);
                journal(JournalCommand::ResetTimer)
            }
        };
        emit(&mut self.host.root, out);
    }

    /// The quests on screen follow the shown quests; a click opens one in
    /// the Journal.
    fn feed_modern_quests_hud(&mut self, id: ElementId, shown: &[u32], now: i64) {
        let hidden = self
            .modern_pieces()
            .is_some_and(|p| p.is_hidden(hud::QUESTS));
        let in_world = self.player_view.is_some();
        // Worked out only when it could show.
        let list = self
            .facts
            .contracts
            .as_ref()
            .filter(|_| !hidden && in_world && !shown.is_empty())
            .map(|c| {
                model::shown_quests(&QuestsInput {
                    contracts: &c.contracts,
                    catalog: &c.catalog,
                    picked: 0,
                    search: "",
                    shown,
                    now_unix_ms: now,
                })
            })
            .unwrap_or_default();
        let show = !hidden && in_world && !list.is_empty();
        let ui = &mut self.host.root;
        if show != ui.is_window_visible(hud::QUESTS) {
            if show {
                ui.show_window(hud::QUESTS);
            } else {
                ui.hide_window(hud::QUESTS);
            }
        }
        if !show {
            return;
        }
        let picked = ui.widget_mut::<QuestsHud>(id).and_then(|q| {
            if q.quests != list {
                q.quests = list;
            }
            q.picked.take()
        });
        let height = ui.widget::<QuestsHud>(id).map_or(60.0, QuestsHud::height);
        if let Some(e) = ui.element_mut(id)
            && (e.height - height).abs() > 0.5
        {
            e.height = height;
        }
        if let Some(quest) = picked {
            let frame = self.journal_ui().map(|j| j.frame);
            if let Some(f) = frame.and_then(|f| self.host.root.widget_mut::<JournalFrame>(f)) {
                f.pick(quest);
            }
            self.panels
                .set_visibility(&mut self.host.root, panel_id::JOURNAL, true);
        }
    }
}
