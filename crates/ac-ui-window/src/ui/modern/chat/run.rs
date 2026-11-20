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
use super::model::ShownLine;
use super::names::{NameAction, NameFonts, NameMenu};
use super::panel::{ChatFonts, ChatPanel, ChatRequest};
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
        });
        modern.replace(crate::ui::window::CHAT_WINDOW);
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
                self.start_modern_chat_line(&format!("@tell {who}, "));
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
        let selected_player = self
            .context
            .selected_target
            .clone()
            .filter(|(_, guid)| ac_core::chat::is_player_object_id(*guid))
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
            selected_player,
        }
    }
}
