//! The modern chat in the game window: mounted in place of retail's chat
//! window, fed the session's chat lines and its chat entry, and its entry
//! field wired to the session's chat entry exactly as retail's is (lines
//! sent, drafts kept, sent lines walked back with Up and Down).

use std::cell::RefCell;
use std::rc::Rc;

use ac_runtime::communication::expand_reply_abbreviation;
use ac_runtime::communication::{ChatChannelKind, ChatEntryCommand, ChatLine};
use ac_runtime::session::interface::ChatEntryView;
use ac_ui::bundled_font::Weight;
use ac_ui::draw::Color;
use ac_ui::geometry::AnchorEdges;
use ac_ui::host::PanelController;
use ac_ui::modern::Horizon;
use ac_ui::modern::theme::hex;
use ac_ui::panels::chat::entry_command;
use ac_ui::root::{ElementMessage, RecallRequest};
use ac_ui::tree::{Element, ElementId};
use ac_ui::widgets::Field;
use ac_ui::{Key, UiRoot};

use super::channels::{ChannelMenu, MenuFonts, TalkFacts, TalkTo};
use super::model::ChatTab;
use super::model::ShownLine;
use super::names::{NameAction, NameFonts, NameMenu};
use super::panel::{ChatFonts, ChatPanel, ChatRequest};
use super::tab_dialog::{TabDialog, TabDialogDone};
use crate::ui::modern::hud;
use crate::ui::window::WindowUi;

/// The chat's window name.
pub const CHAT: &str = "modern-chat";

/// The sizes of the chat's words, pixels to the em.
const BODY_SIZE: f32 = 12.5;
const TAG_SIZE: f32 = 9.5;
const SMALL_SIZE: f32 = 10.5;
const TAB_SIZE: f32 = 11.0;
const MENU_LABEL_SIZE: f32 = 12.0;

/// The chat's elements.
#[derive(Debug)]
pub struct ChatUi {
    /// The panel (the window).
    pub panel: ElementId,
    /// The entry's field.
    pub field: ElementId,
    /// The channel menu.
    pub menu: ElementId,
    /// The menu a speaker's name opens.
    pub names: ElementId,
    /// The search field.
    pub search: ElementId,
    /// Who last sent the player a tell (for "/r " in the field).
    teller: Rc<RefCell<Option<String>>>,
    /// The entry as the session last told it.
    entry: ChatEntryView,
    /// The shown revision of the draft last put in the field.
    shown: Option<u64>,
    /// The entry had the keyboard last frame.
    had_keys: bool,
    /// The dialog that makes and changes tabs.
    pub dialog: ElementId,
    /// The popped-out tabs' windows made so far (hidden, not destroyed,
    /// when put back or deleted).
    windows: Vec<TabWindow>,
    /// Windows hidden by the floating chat keys (their tabs stay popped
    /// out; the close button puts a tab back instead).
    closed: std::collections::HashSet<u32>,
    /// The saved tabs have been read.
    loaded: bool,
    /// The fonts new windows are made with.
    fonts: ChatFonts,
    /// What a window's entry talks on and to, by tab id.
    window_talk: WindowTalk,
}

/// What each popped-out tab's entry talks on and to, by tab id.
type WindowTalk = Rc<RefCell<std::collections::HashMap<u32, (ChatChannelKind, Option<String>)>>>;

/// A popped-out tab's window.
#[derive(Debug)]
struct TabWindow {
    /// The tab's id.
    id: u32,
    /// The window's name.
    name: String,
    /// Its panel.
    panel: ElementId,
    /// Its entry's field.
    field: ElementId,
}

/// A popped-out tab's entry: it talks on the tab's own channel, straight
/// to the session (the shared entry keeps its own channel and draft).
struct WindowEntryController {
    field: ElementId,
    id: u32,
    talk: WindowTalk,
}

impl PanelController for WindowEntryController {
    fn on_element_message(
        &mut self,
        ui: &mut UiRoot,
        element: ElementId,
        message: &ElementMessage,
    ) -> bool {
        if element != self.field {
            return false;
        }
        if let ElementMessage::Submitted(text) = message {
            let (channel, tell_target) = self
                .talk
                .borrow()
                .get(&self.id)
                .cloned()
                .unwrap_or((ChatChannelKind::Say, None));
            let line = ac_ui::panels::chat::ChatSubmission {
                text: text.clone(),
                channel,
                tell_target,
                tell_target_guid: 0,
            };
            if let Some(command) = line.to_session_command() {
                ac_ui::panels::items::emit(ui, ac_ui::panels::PanelOutput::Session(command));
            }
            return true;
        }
        matches!(
            message,
            ElementMessage::TextChanged(_) | ElementMessage::Recall(_)
        )
    }
}

/// Wires the entry's field to the session's chat entry, as retail's chat
/// window does with its own.
struct EntryController {
    field: ElementId,
}

impl PanelController for EntryController {
    fn on_element_message(
        &mut self,
        ui: &mut UiRoot,
        element: ElementId,
        message: &ElementMessage,
    ) -> bool {
        if element != self.field {
            return false;
        }
        match message {
            ElementMessage::Submitted(text) => {
                entry_command(ui, ChatEntryCommand::Submit(Some(text.clone())));
                true
            }
            ElementMessage::TextChanged(text) => {
                entry_command(ui, ChatEntryCommand::SetDraft(text.clone()));
                true
            }
            ElementMessage::Recall(request) => {
                entry_command(
                    ui,
                    match request {
                        RecallRequest::Previous => ChatEntryCommand::RecallPrevious,
                        RecallRequest::Next => ChatEntryCommand::RecallNext,
                        RecallRequest::Stop => ChatEntryCommand::ResetRecall,
                    },
                );
                true
            }
            _ => false,
        }
    }
}

impl WindowUi {
    /// The chat: its panel, entry and channel menu, in place of retail's
    /// chat window.
    pub(in crate::ui) fn mount_modern_chat(&mut self) {
        let fonts = ChatFonts {
            body: self.modern_font(Weight::Regular, BODY_SIZE),
            name: self.modern_font(Weight::SemiBold, BODY_SIZE),
            tag: self.modern_font(Weight::SemiBold, TAG_SIZE),
            small: self.modern_font(Weight::Regular, SMALL_SIZE),
            tab: self.modern_font(Weight::SemiBold, TAB_SIZE),
        };
        let menu_fonts = MenuFonts {
            heading: fonts.tag.clone(),
            label: self.modern_font(Weight::SemiBold, MENU_LABEL_SIZE),
            note: fonts.small.clone(),
        };
        let body = fonts.body.clone();
        let kept_fonts = fonts.clone();
        let dialog_fonts = super::tab_dialog::TabDialogFonts {
            title: self.modern_font(Weight::SemiBold, 13.5),
            body: self.modern_font(Weight::Medium, 12.0),
            small: fonts.small.clone(),
        };
        let search_font = fonts.small.clone();
        let name_fonts = NameFonts {
            label: menu_fonts.label.clone(),
            note: fonts.small.clone(),
        };
        let panel = match hud::mount_piece(
            &mut self.host.root,
            CHAT,
            (super::panel::WIDTH, super::panel::HEIGHT),
            ChatPanel::new(Horizon::BRONZE, fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern chat: {e}"));
                return;
            }
        };
        let teller = Rc::new(RefCell::new(None::<String>));
        let ui = &mut self.host.root;
        let mut e = Element::at(0.0, 0.0, 200.0, 24.0)
            .with_visible(false)
            .with_anchors(AnchorEdges::NONE);
        Field::configure(&mut e);
        let mut f = Field::default();
        f.font = body;
        f.one_line = true;
        f.selectable = true;
        f.padding = 5.0;
        f.text_color = Horizon::BRONZE.palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = hex(0xE0_B068, 0.35);
        // The session's chat entry sends the line, remembers it and keeps
        // the draft; the field keeps the caret and the drawing.
        f.record_history = false;
        f.shared_history = true;
        let replies = teller.clone();
        f.text_replacer = Some(Box::new(move |text: &str| {
            expand_reply_abbreviation(text, replies.borrow().as_deref())
        }));
        f.set_editable_detached(&mut e, true);
        let field = ui.add(panel, e, f);
        let mut e = Element::at(0.0, 0.0, 200.0, 22.0)
            .with_visible(false)
            .with_anchors(AnchorEdges::NONE);
        Field::configure(&mut e);
        let mut f = Field::default();
        f.font = search_font;
        f.one_line = true;
        f.selectable = true;
        f.padding = 4.0;
        f.record_history = false;
        f.clear_on_submit = false;
        f.stay_focused_after_submit = true;
        f.text_color = Horizon::BRONZE.palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = hex(0xE0_B068, 0.35);
        f.set_editable_detached(&mut e, true);
        let search = ui.add(panel, e, f);
        let root = ui.root();
        let menu = ui.add(
            root,
            Element::at(0.0, 0.0, super::channels::WIDTH, 100.0)
                .with_visible(false)
                .with_anchors(AnchorEdges::NONE),
            ChannelMenu::new(Horizon::BRONZE, menu_fonts),
        );
        let names = ui.add(
            root,
            Element::at(0.0, 0.0, super::names::WIDTH, 100.0)
                .with_visible(false)
                .with_anchors(AnchorEdges::NONE),
            NameMenu::new(Horizon::BRONZE, name_fonts),
        );
        let dialog = TabDialog::mount(ui, Horizon::BRONZE, dialog_fonts);
        if let Err(e) = self
            .host
            .attach_controller(CHAT, Box::new(EntryController { field }))
        {
            self.notices.push(format!("ui: modern chat entry: {e}"));
        }
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.chat = Some(ChatUi {
            panel,
            field,
            menu,
            names,
            search,
            teller,
            entry: ChatEntryView::default(),
            shown: None,
            had_keys: false,
            dialog,
            windows: Vec::new(),
            closed: std::collections::HashSet::new(),
            loaded: false,
            fonts: kept_fonts,
            window_talk: Rc::new(RefCell::new(std::collections::HashMap::new())),
        });
        modern.replace(crate::ui::window::CHAT_WINDOW);
        for name in crate::ui::chat_windows::FLOATING_CHAT_WINDOWS {
            modern.replace(name);
        }
    }

    /// New chat lines for the modern chat.
    pub(in crate::ui) fn feed_modern_chat(&mut self, lines: &[ChatLine]) {
        let Some(panel) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| c.panel)
        else {
            return;
        };
        let me = self.status.playing.clone().unwrap_or_default();
        let windows: Vec<ElementId> = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| c.windows.iter().map(|w| w.panel).collect())
            .unwrap_or_default();
        for w in windows {
            if let Some(p) = self.host.root.widget_mut::<ChatPanel>(w) {
                for line in lines {
                    p.push(ShownLine::of(line, &me));
                }
            }
        }
        if let Some(p) = self.host.root.widget_mut::<ChatPanel>(panel) {
            for line in lines {
                p.push(ShownLine::of(line, &me));
            }
            p.times = lines.last().map_or(p.times, |l| {
                l.spans.as_ref().is_some_and(|s| {
                    s.iter()
                        .any(|s| s.role == ac_core::chat::ChatSpanRole::Timestamp)
                })
            });
        }
    }

    /// The session's chat entry, for the modern chat: where lines go, and a
    /// draft put there other than by typing (a recalled or staged line).
    pub(in crate::ui) fn feed_modern_chat_entry(&mut self, view: &ChatEntryView) {
        let Some(chat) = self.modern.as_mut().and_then(|m| m.chat.as_mut()) else {
            return;
        };
        chat.teller.replace(view.last_incoming_tell_sender.clone());
        chat.entry = view.clone();
        let (panel, field) = (chat.panel, chat.field);
        let fresh = chat.shown != Some(view.shown_revision);
        chat.shown = Some(view.shown_revision);
        let ui = &mut self.host.root;
        if let Some(p) = ui.widget_mut::<ChatPanel>(panel) {
            p.channel = view.channel;
            p.tell_target = view.tell_target.clone();
        }
        if fresh {
            if let Some(f) = ui.widget_mut::<Field>(field)
                && f.text() != view.draft
            {
                f.show_text(&view.draft);
            }
            if view.composed {
                self.focus_modern_chat(None);
            }
        }
    }

    /// The entry takes the keyboard (and the panel comes up); `key`, the
    /// key that did it, types nothing until it is let go. False without
    /// the modern chat.
    pub(in crate::ui) fn focus_modern_chat(&mut self, key: Option<Key>) -> bool {
        let Some((panel, field)) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| (c.panel, c.field))
        else {
            return false;
        };
        let ui = &mut self.host.root;
        if let Some(p) = ui.widget_mut::<ChatPanel>(panel) {
            p.wake();
        }
        ui.set_visible(field, true);
        ui.set_keyboard_focus(Some(field));
        if let Some(k) = key {
            ui.suppress_physical_key_until_release(k);
        }
        true
    }

    /// The chat keys for the modern chat: Enter Chat Mode focuses the
    /// entry and selects its text; Toggle Chat Entry focuses it or gives
    /// the focus up. False without the modern chat.
    pub(in crate::ui) fn modern_chat_entry_key(&mut self, select: bool, key: Option<Key>) -> bool {
        let Some(field) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| c.field)
        else {
            return false;
        };
        if !select && self.host.root.keyboard_focus() == Some(field) {
            self.host.root.set_keyboard_focus(None);
            if let Some(k) = key {
                self.host.root.suppress_physical_key_until_release(k);
            }
            return true;
        }
        self.focus_modern_chat(key);
        if select && let Some(f) = self.host.root.widget_mut::<Field>(field) {
            f.select_all_text();
        }
        true
    }

    /// Puts `text` in the entry as the player's own and focuses it ("@tell
    /// Rook, ", "/"). False without the modern chat.
    pub(in crate::ui) fn start_modern_chat_line(&mut self, text: &str) -> bool {
        let Some(field) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| c.field)
        else {
            return false;
        };
        let requests = self
            .host
            .root
            .widget_mut::<Field>(field)
            .map(|f| {
                f.set_text(text);
                f.take_recall_requests()
            })
            .unwrap_or_default();
        let ui = &mut self.host.root;
        for r in requests {
            if r == RecallRequest::Stop {
                entry_command(ui, ChatEntryCommand::ResetRecall);
            }
        }
        entry_command(ui, ChatEntryCommand::SetDraft(text.to_owned()));
        self.focus_modern_chat(None);
        true
    }

    /// Each frame: the field follows the panel, and what the panel and its
    /// menu asked for is done.
    pub(in crate::ui) fn tick_modern_chat(&mut self) {
        let Some((panel, field, menu, names)) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| (c.panel, c.field, c.menu, c.names))
        else {
            return;
        };
        self.load_modern_chat_tabs();
        self.tick_modern_chat_windows();
        self.follow_modern_chat_dialog();
        self.follow_modern_chat_search();
        // Escape gave the keyboard up: the entry's menus close with it.
        let focus = self.host.root.keyboard_focus();
        if let Some(chat) = self.modern.as_mut().and_then(|m| m.chat.as_mut()) {
            let had = std::mem::replace(&mut chat.had_keys, focus == Some(field));
            if had && focus.is_none() {
                for popup in [menu, names] {
                    let ui = &mut self.host.root;
                    if ui.element(popup).is_some_and(|e| e.visible) {
                        ui.clear_active_popup(popup);
                        ui.set_visible(popup, false);
                    }
                }
            }
        }
        let stay = self.context.stay_in_chat_mode;
        let (channel, teller, target) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| {
                (
                    c.entry.channel,
                    c.entry.last_incoming_tell_sender.clone(),
                    c.entry.tell_target.clone(),
                )
            })
            .unwrap_or_default();
        let search = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| c.search);
        let ui = &mut self.host.root;
        let in_search = search.is_some() && ui.keyboard_focus() == search;
        let typing = ui.keyboard_focus() == Some(field) || in_search;
        let typed = ui
            .widget::<Field>(field)
            .map(Field::text)
            .unwrap_or_default();
        let empty = typed.is_empty();
        let suggestions = super::commands::suggestions(&typed, 5);
        let going =
            super::commands::going_to(&typed, channel, teller.as_deref(), target.as_deref());
        if let Some(f) = ui.widget_mut::<Field>(field) {
            f.stay_focused_after_submit = stay;
        }
        let (active, rect, requests) = match ui.widget_mut::<ChatPanel>(panel) {
            Some(p) => {
                p.typing = typing;
                p.entry_empty = empty;
                p.suggestions = suggestions;
                p.going_to = going;
                p.last_teller = teller;
                (
                    p.active || typing,
                    p.field_rect(),
                    std::mem::take(&mut p.requests),
                )
            }
            None => return,
        };
        if let Some(e) = ui.element_mut(field) {
            e.left = rect.0;
            e.top = rect.1;
            e.width = rect.2;
            e.height = rect.3;
        }
        if ui.element(field).is_some_and(|e| e.visible) != active {
            ui.set_visible(field, active);
        }
        let chosen = ui
            .widget_mut::<ChannelMenu>(menu)
            .and_then(|m| m.chosen.take());
        if let Some(to) = chosen {
            self.aim_modern_chat(to);
            self.focus_modern_chat(None);
        }
        let named = self
            .host
            .root
            .widget_mut::<NameMenu>(names)
            .and_then(|m| m.chosen.take().map(|a| (a, m.who.clone())));
        if let Some((action, (who, guid, _))) = named {
            self.name_chosen(action, &who, guid);
        }
        for request in requests {
            match request {
                ChatRequest::Focus => {
                    self.focus_modern_chat(None);
                }
                ChatRequest::Search(open) => self.modern_chat_search(open),
                ChatRequest::NewTab => {
                    if let Some(d) = self
                        .modern
                        .as_ref()
                        .and_then(|m| m.chat.as_ref())
                        .map(|c| c.dialog)
                    {
                        TabDialog::show(&mut self.host.root, d, None);
                    }
                }
                ChatRequest::EditTab(n) => self.edit_modern_chat_tab(n),
                ChatRequest::PopOut(n) => self.set_modern_chat_window(n, true),
                ChatRequest::PutBack | ChatRequest::Moved => {}
                ChatRequest::Complete(verb) => {
                    self.start_modern_chat_line(&format!("{verb} "));
                }
                ChatRequest::Tab(tab) => self.modern_chat_tab_picked(tab),
                ChatRequest::Channels((x, y)) => {
                    let facts = self.talk_facts();
                    let current = self.modern_chat_talks_to();
                    let ui = &mut self.host.root;
                    let (px, py) = ui.screen_position(panel);
                    if let Some(m) = ui.widget_mut::<ChannelMenu>(menu) {
                        m.fill(&facts, current);
                    }
                    let _ = x;
                    ChannelMenu::open(ui, menu, (px, py + y - 6.0));
                }
                ChatRequest::Name(who, guid, (x, y)) => {
                    let nearby = guid != 0 && self.facts.objects.get(guid).is_some();
                    let ui = &mut self.host.root;
                    let place = ui
                        .widget::<ChatPanel>(panel)
                        .and_then(|p| p.tabs.get(p.current))
                        .map(|t| t.name.clone())
                        .unwrap_or_default();
                    let (px, py) = ui.screen_position(panel);
                    if let Some(m) = ui.widget_mut::<NameMenu>(names) {
                        m.fill(&who, guid, &place, nearby);
                    }
                    NameMenu::open(ui, names, (px + x, py + y - 4.0));
                }
            }
        }
    }

    /// Search opens (its field takes the keyboard) or closes.
    fn modern_chat_search(&mut self, open: bool) {
        let Some((panel, search)) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| (c.panel, c.search))
        else {
            return;
        };
        let ui = &mut self.host.root;
        if let Some(p) = ui.widget_mut::<ChatPanel>(panel) {
            p.searching = open;
            p.query.clear();
            p.match_at = 0;
            if open {
                p.wake();
            } else {
                p.follow();
            }
        }
        if let Some(f) = ui.widget_mut::<Field>(search) {
            f.set_text("");
        }
        ui.set_visible(search, open);
        ui.set_keyboard_focus(open.then_some(search));
    }

    /// The search field follows the panel; what is typed in it is searched
    /// for, the newest match brought into view as it changes.
    fn follow_modern_chat_search(&mut self) {
        let Some((panel, search)) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| (c.panel, c.search))
        else {
            return;
        };
        let ui = &mut self.host.root;
        let typed = ui
            .widget::<Field>(search)
            .map(Field::text)
            .unwrap_or_default();
        let focused = ui.keyboard_focus() == Some(search);
        let Some(p) = ui.widget_mut::<ChatPanel>(panel) else {
            return;
        };
        if p.searching && !p.active {
            p.searching = false;
        }
        let (searching, rect) = (p.searching, p.search_field_rect());
        if searching && p.query != typed {
            p.query = typed;
            p.match_at = 0;
            p.step_match(false);
        }
        if focused {
            p.typing = true;
        }
        if let Some(e) = ui.element_mut(search) {
            e.left = rect.0;
            e.top = rect.1;
            e.width = rect.2;
            e.height = rect.3;
        }
        if ui.element(search).is_some_and(|e| e.visible) != searching {
            ui.set_visible(search, searching);
        }
    }

    /// A name menu row chosen for `who` (object `guid`).
    fn name_chosen(&mut self, action: NameAction, who: &str, guid: u32) {
        use ac_runtime::gameplay::{FellowshipCommand, GameplayCommand};
        use ac_runtime::inventory::ObjectRef;
        use ac_runtime::session::SessionCommand;
        let command = match action {
            NameAction::Tell => {
                if guid != 0 && !ac_core::chat::is_player_object_id(guid) {
                    // A creature or an NPC has no name the server can look
                    // up: the entry is aimed at it by id instead.
                    self.aim_modern_chat(TalkTo::Tell(who.to_owned(), guid));
                    self.focus_modern_chat(None);
                } else {
                    self.start_modern_chat_line(&format!("@tell {who}, "));
                }
                return;
            }
            NameAction::Copy => {
                if let Err(Some(e)) = self.clipboard.set_text(who) {
                    self.notices.push(format!("ui: copying the name: {e}"));
                }
                return;
            }
            NameAction::Select => {
                SessionCommand::Gameplay(GameplayCommand::Select(ObjectRef::Guid(guid)))
            }
            NameAction::Invite => SessionCommand::Gameplay(GameplayCommand::Fellowship(
                FellowshipCommand::Recruit(ObjectRef::Guid(guid)),
            )),
            NameAction::AddFriend => SessionCommand::AddFriend(who.to_owned()),
            NameAction::Squelch => SessionCommand::Squelch {
                add: true,
                name: who.to_owned(),
                account: false,
            },
        };
        ac_ui::panels::items::emit(
            &mut self.host.root,
            ac_ui::panels::PanelOutput::Session(command),
        );
    }

    /// The saved tabs, read once (the starting tabs when none are saved),
    /// and their windows made.
    fn load_modern_chat_tabs(&mut self) {
        let Some(chat) = self.modern.as_mut().and_then(|m| m.chat.as_mut()) else {
            return;
        };
        if chat.loaded {
            return;
        }
        chat.loaded = true;
        let panel = chat.panel;
        let saved = self
            .status
            .settings_store
            .as_ref()
            .map(ac_host::settings::SettingsStore::load_chat_tabs)
            .unwrap_or_default();
        if saved.is_empty() {
            return;
        }
        let mut tabs: Vec<ChatTab> = saved.iter().map(ChatTab::from_data).collect();
        tabs[0].window = false;
        let popped: Vec<usize> = (0..tabs.len()).filter(|&i| tabs[i].window).collect();
        if let Some(p) = self.host.root.widget_mut::<ChatPanel>(panel) {
            p.tabs = tabs;
            p.current = 0;
        }
        for n in popped {
            self.show_modern_chat_window(n);
        }
    }

    /// The tabs as they stand, saved (each window's place with them).
    fn save_modern_chat_tabs(&mut self) {
        let Some(chat) = self.modern.as_ref().and_then(|m| m.chat.as_ref()) else {
            return;
        };
        let places: Vec<(u32, (f32, f32))> = chat
            .windows
            .iter()
            .filter_map(|w| {
                self.host
                    .root
                    .element(w.panel)
                    .map(|e| (w.id, (e.left, e.top)))
            })
            .collect();
        let panel = chat.panel;
        let Some(p) = self.host.root.widget_mut::<ChatPanel>(panel) else {
            return;
        };
        for tab in &mut p.tabs {
            if let Some((_, at)) = places.iter().find(|(id, _)| *id == tab.id) {
                tab.place = Some(*at);
            }
        }
        let data: Vec<_> = p.tabs.iter().map(ChatTab::to_data).collect();
        if let Some(store) = &self.status.settings_store
            && let Err(e) = store.save_chat_tabs(&data)
        {
            self.notices.push(format!("ui: saving the chat tabs: {e}"));
        }
    }

    /// The dialog for tab `n`.
    fn edit_modern_chat_tab(&mut self, n: usize) {
        let Some((panel, dialog)) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| (c.panel, c.dialog))
        else {
            return;
        };
        let tab = self
            .host
            .root
            .widget::<ChatPanel>(panel)
            .and_then(|p| p.tabs.get(n).cloned());
        if let Some(tab) = tab {
            TabDialog::show(&mut self.host.root, dialog, Some((n, &tab)));
        }
    }

    /// What the dialog ended with is done: a tab made, changed or deleted.
    fn follow_modern_chat_dialog(&mut self) {
        let Some((panel, dialog)) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| (c.panel, c.dialog))
        else {
            return;
        };
        let Some(done) = self
            .host
            .root
            .widget_mut::<TabDialog>(dialog)
            .and_then(|d| d.done.take())
        else {
            return;
        };
        let Some(p) = self.host.root.widget_mut::<ChatPanel>(panel) else {
            return;
        };
        let mut window_change: Option<(usize, bool)> = None;
        match done {
            TabDialogDone::Save(None, mut tab) => {
                tab.id = p.tabs.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                let popped = tab.window;
                tab.window = false;
                p.tabs.push(tab);
                let n = p.tabs.len() - 1;
                if popped {
                    window_change = Some((n, true));
                } else {
                    p.pick_tab(n);
                }
            }
            TabDialogDone::Save(Some(n), tab) => {
                if let Some(old) = p.tabs.get_mut(n) {
                    let was = old.window;
                    let popped = tab.window;
                    *old = ChatTab { window: was, ..tab };
                    if was != popped {
                        window_change = Some((n, popped));
                    } else if was {
                        window_change = Some((n, true));
                    }
                }
            }
            TabDialogDone::Delete(n) if n > 0 && n < p.tabs.len() => {
                let id = p.tabs[n].id;
                p.tabs.remove(n);
                if n < p.log.unread.len() {
                    p.log.unread.remove(n);
                }
                if p.current >= n {
                    p.current = p.current.saturating_sub(1);
                }
                self.hide_modern_chat_window(id);
            }
            TabDialogDone::Delete(_) => {}
        }
        if let Some((n, popped)) = window_change {
            self.set_modern_chat_window(n, popped);
        }
        self.save_modern_chat_tabs();
    }

    /// Tab `n` pops out into its window (`popped`) or goes back to being a
    /// tab.
    fn set_modern_chat_window(&mut self, n: usize, popped: bool) {
        let Some(panel) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| c.panel)
        else {
            return;
        };
        let Some(id) = self.host.root.widget_mut::<ChatPanel>(panel).and_then(|p| {
            let tab = p.tabs.get_mut(n).filter(|_| n > 0)?;
            tab.window = popped;
            let id = tab.id;
            if popped && p.current == n {
                p.pick_tab(0);
            }
            Some(id)
        }) else {
            return;
        };
        if popped {
            if let Some(chat) = self.modern.as_mut().and_then(|m| m.chat.as_mut()) {
                chat.closed.remove(&id);
            }
            self.show_modern_chat_window(n);
        } else {
            self.hide_modern_chat_window(id);
        }
        self.save_modern_chat_tabs();
    }

    /// Tab `n`'s window shows (made the first time), showing the tab as it
    /// is now.
    fn show_modern_chat_window(&mut self, n: usize) {
        let Some(chat) = self.modern.as_ref().and_then(|m| m.chat.as_ref()) else {
            return;
        };
        let main = chat.panel;
        let fonts = chat.fonts.clone();
        let talk = chat.window_talk.clone();
        let Some((tab, lines)) = self
            .host
            .root
            .widget::<ChatPanel>(main)
            .and_then(|p| Some((p.tabs.get(n)?.clone(), p.log.lines.clone())))
        else {
            return;
        };
        let known = chat
            .windows
            .iter()
            .find(|w| w.id == tab.id)
            .map(|w| (w.panel, w.name.clone()));
        let screen = self.host.root.size();
        let (panel, name) = match known {
            Some(found) => found,
            None => {
                let name = format!("modern-chat-tab-{}", tab.id);
                let mut window = ChatPanel::for_window(Horizon::BRONZE, fonts.clone(), tab.clone());
                for line in lines {
                    window.push(line);
                }
                let height = window.height();
                let id = match hud::mount_piece(
                    &mut self.host.root,
                    &name,
                    (super::panel::WIDTH, height),
                    window,
                ) {
                    Ok(id) => id,
                    Err(e) => {
                        self.notices.push(format!("ui: chat window: {e}"));
                        return;
                    }
                };
                let field = self.mount_window_field(id, fonts.body.clone());
                if let Err(e) = self.host.attach_controller(
                    &name,
                    Box::new(WindowEntryController {
                        field,
                        id: tab.id,
                        talk,
                    }),
                ) {
                    self.notices.push(format!("ui: chat window entry: {e}"));
                }
                if let Some(chat) = self.modern.as_mut().and_then(|m| m.chat.as_mut()) {
                    let k = chat.windows.len() as f32;
                    chat.windows.push(TabWindow {
                        id: tab.id,
                        name: name.clone(),
                        panel: id,
                        field,
                    });
                    let (x, y) = tab
                        .place
                        .unwrap_or((screen.0 * 0.5 - 220.0 + 24.0 * k, 120.0 + 30.0 * k));
                    if let Some(e) = self.host.root.element_mut(id) {
                        e.left = x.clamp(0.0, (screen.0 - super::panel::WIDTH).max(0.0));
                        e.top = y.clamp(0.0, (screen.1 - height).max(0.0));
                    }
                }
                (id, name)
            }
        };
        let ui = &mut self.host.root;
        if let Some(w) = ui.widget_mut::<ChatPanel>(panel) {
            w.tabs = vec![tab];
            w.current = 0;
            let h = w.height();
            if let Some(e) = ui.element_mut(panel) {
                e.height = h;
            }
        }
        ui.show_window(&name);
    }

    /// A popped-out tab's entry field, under its window `panel`.
    fn mount_window_field(
        &mut self,
        panel: ElementId,
        font: Option<std::sync::Arc<ac_ui::UiFont>>,
    ) -> ElementId {
        let ui = &mut self.host.root;
        let mut e = Element::at(0.0, 0.0, 200.0, 24.0)
            .with_visible(false)
            .with_anchors(AnchorEdges::NONE);
        Field::configure(&mut e);
        let mut f = Field::default();
        f.font = font;
        f.one_line = true;
        f.selectable = true;
        f.padding = 5.0;
        f.record_history = false;
        f.text_color = Horizon::BRONZE.palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = hex(0xE0_B068, 0.35);
        f.set_editable_detached(&mut e, true);
        ui.add(panel, e, f)
    }

    /// Tab `id`'s window hides.
    fn hide_modern_chat_window(&mut self, id: u32) {
        let Some(name) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .and_then(|c| c.windows.iter().find(|w| w.id == id))
            .map(|w| w.name.clone())
        else {
            return;
        };
        let ui = &mut self.host.root;
        if ui.is_window_visible(&name) {
            ui.hide_window(&name);
        }
    }

    /// The floating chat keys: key `n` shows or hides the `n`th popped-out
    /// tab's window. False without the modern chat.
    pub(in crate::ui) fn toggle_modern_chat_window(&mut self, n: u8) -> bool {
        let Some(chat) = self.modern.as_ref().and_then(|m| m.chat.as_ref()) else {
            return false;
        };
        let panel = chat.panel;
        let id = self.host.root.widget::<ChatPanel>(panel).and_then(|p| {
            p.tabs
                .iter()
                .filter(|t| t.window)
                .nth(usize::from(n).wrapping_sub(1))
                .map(|t| t.id)
        });
        let Some(id) = id else {
            return true;
        };
        let shown = chat
            .windows
            .iter()
            .find(|w| w.id == id)
            .is_some_and(|w| self.host.root.is_window_visible(&w.name));
        if let Some(chat) = self.modern.as_mut().and_then(|m| m.chat.as_mut()) {
            if shown {
                chat.closed.insert(id);
            } else {
                chat.closed.remove(&id);
            }
        }
        if shown {
            self.hide_modern_chat_window(id);
        } else if let Some(n) = self
            .host
            .root
            .widget::<ChatPanel>(panel)
            .and_then(|p| p.tabs.iter().position(|t| t.id == id))
        {
            self.show_modern_chat_window(n);
        }
        true
    }

    /// Each window, each frame: its entry follows it, what it talks on is
    /// kept for its entry, and what it asked for is done.
    fn tick_modern_chat_windows(&mut self) {
        let Some(chat) = self.modern.as_ref().and_then(|m| m.chat.as_ref()) else {
            return;
        };
        let windows: Vec<(u32, String, ElementId, ElementId)> = chat
            .windows
            .iter()
            .map(|w| (w.id, w.name.clone(), w.panel, w.field))
            .collect();
        let teller = chat.entry.last_incoming_tell_sender.clone();
        let talk = chat.window_talk.clone();
        let names = chat.names;
        let main = chat.panel;
        for (id, name, panel, field) in windows {
            let ui = &mut self.host.root;
            if !ui.is_window_visible(&name) {
                continue;
            }
            let typing = ui.keyboard_focus() == Some(field);
            let empty = ui
                .widget::<Field>(field)
                .is_none_or(|f| f.text().is_empty());
            let Some((rect, entry, requests, channel)) =
                ui.widget_mut::<ChatPanel>(panel).map(|p| {
                    p.typing = typing;
                    p.entry_empty = empty;
                    let channel = p.tabs.first().and_then(|t| t.talk);
                    p.channel = channel.unwrap_or(ChatChannelKind::Say);
                    p.tell_target = teller
                        .clone()
                        .filter(|_| channel == Some(ChatChannelKind::Tell));
                    (
                        p.field_rect(),
                        p.has_entry(),
                        std::mem::take(&mut p.requests),
                        channel,
                    )
                })
            else {
                continue;
            };
            if let Some(c) = channel {
                talk.borrow_mut().insert(id, (c, teller.clone()));
            }
            if let Some(e) = ui.element_mut(field) {
                e.left = rect.0;
                e.top = rect.1;
                e.width = rect.2;
                e.height = rect.3;
            }
            if ui.element(field).is_some_and(|e| e.visible) != entry {
                ui.set_visible(field, entry);
            }
            let n = ui
                .widget::<ChatPanel>(main)
                .and_then(|p| p.tabs.iter().position(|t| t.id == id));
            for request in requests {
                match request {
                    ChatRequest::Focus if entry => {
                        self.host.root.set_keyboard_focus(Some(field));
                    }
                    ChatRequest::EditTab(_) => {
                        if let Some(n) = n {
                            self.edit_modern_chat_tab(n);
                        }
                    }
                    ChatRequest::PutBack => {
                        if let Some(n) = n {
                            self.set_modern_chat_window(n, false);
                        }
                    }
                    ChatRequest::Moved => self.save_modern_chat_tabs(),
                    ChatRequest::Name(who, guid, (x, y)) => {
                        let nearby = guid != 0 && self.facts.objects.get(guid).is_some();
                        let ui = &mut self.host.root;
                        let (px, py) = ui.screen_position(panel);
                        let place = ui
                            .widget::<ChatPanel>(panel)
                            .and_then(|p| p.tabs.first())
                            .map(|t| t.name.clone())
                            .unwrap_or_default();
                        if let Some(m) = ui.widget_mut::<NameMenu>(names) {
                            m.fill(&who, guid, &place, nearby);
                        }
                        NameMenu::open(ui, names, (px + x, py + y - 4.0));
                    }
                    _ => {}
                }
            }
        }
    }

    /// Where the entry talks now.
    fn modern_chat_talks_to(&self) -> TalkTo {
        let entry = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| c.entry.clone())
            .unwrap_or_default();
        match (entry.channel, entry.tell_target) {
            (ChatChannelKind::Tell, Some(who)) => TalkTo::Tell(who, entry.tell_target_guid),
            (kind, _) => TalkTo::Channel(kind),
        }
    }

    /// Aims the entry at `to`.
    fn aim_modern_chat(&mut self, to: TalkTo) {
        let command = match to {
            TalkTo::Channel(kind) => ChatEntryCommand::SetChannel(kind),
            TalkTo::Tell(name, guid) => ChatEntryCommand::SetTellTarget { name, guid },
        };
        entry_command(&mut self.host.root, command);
    }

    /// A tab picked: the entry talks on the tab's own channel (Tells: to
    /// whoever last sent one); Main and Combat keep the last one used.
    fn modern_chat_tab_picked(&mut self, tab: usize) {
        let Some(chat) = self.modern.as_ref().and_then(|m| m.chat.as_ref()) else {
            return;
        };
        let talk = self
            .host
            .root
            .widget::<ChatPanel>(chat.panel)
            .and_then(|p| p.tabs.get(tab))
            .and_then(|t| t.talk);
        let teller = chat.entry.last_incoming_tell_sender.clone();
        match talk {
            Some(ChatChannelKind::Tell) => {
                if let Some(who) = teller {
                    self.aim_modern_chat(TalkTo::Tell(who, 0));
                }
            }
            Some(kind) => self.aim_modern_chat(TalkTo::Channel(kind)),
            None => {}
        }
    }

    /// What the channel menu greys, from what the client knows.
    fn talk_facts(&self) -> TalkFacts {
        let social = self.facts.social.as_ref().filter(|s| s.in_world);
        let allegiance = social.map(|s| s.allegiance.as_ref());
        // As retail's Tell to Selected: anything selected but the player.
        let selected = self
            .context
            .selected_target
            .clone()
            .filter(|(_, guid)| *guid != self.facts.state.player);
        TalkFacts {
            in_fellowship: social.map(|s| s.fellowship.in_fellowship),
            in_allegiance: allegiance.map(|a| a.is_some_and(|a| a.monarch.is_some())),
            // The client cannot tell patron and vassals apart from the
            // allegiance's records yet: the server answers for those.
            has_patron: None,
            has_vassals: None,
            last_teller: self
                .modern
                .as_ref()
                .and_then(|m| m.chat.as_ref())
                .and_then(|c| c.entry.last_incoming_tell_sender.clone()),
            selected,
        }
    }
}
