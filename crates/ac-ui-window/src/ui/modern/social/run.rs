//! The Social window in the game window, in place of retail's social
//! panel: O and the panel strip open it, the allegiance and fellowship keys
//! open it on their page. It reads the session's social facts, keeps the
//! fellows' vitals coming while the fellowship page or the fellows on
//! screen want them, sends what retail's pages send, asks before anything
//! that parts people, and sends Manage allegiance's and the ignored list's
//! finer choices as the chat box's commands.

use ac_runtime::character::option_id;
use ac_runtime::gameplay::{AllegianceCommand, FellowshipCommand, GameplayCommand};
use ac_runtime::inventory::ObjectRef;
use ac_runtime::session::{Recall, SessionCommand};
use ac_ui::bundled_font::Weight;
use ac_ui::modern::Horizon;
use ac_ui::panels::items::emit;
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::tree::ElementId;

use super::model::{self, HIDEABLE, SocialInput, Tab};
use super::window::{
    FellowSwitch, HEIGHT, ManageAsk, Slot, SocialAsk, SocialFonts, SocialFrame, WIDTH,
};
use crate::ui::modern::dialogs::look::{GOLD, Glyph, Look, RED};
use crate::ui::modern::hud::fellows::{FellowsFonts, FellowsHud};
use crate::ui::modern::{hud, kit};
use crate::ui::social_panels::{SOCIAL_WINDOW, SocialPage};
use crate::ui::window::{DialogAction, WindowUi};

/// The window's name.
pub const SOCIAL: &str = "modern-social";

/// The Social window's elements and what it remembers.
#[derive(Debug)]
pub struct SocialUi {
    /// The window.
    pub frame: ElementId,
    /// The fields over it.
    pub fields: Vec<(Slot, ElementId)>,
    /// The fellows on screen.
    pub fellows: Option<ElementId>,
    /// The fellows' vitals were last asked for (or stopped) this way, in
    /// the world (none: not since entering it).
    declared: Option<bool>,
}

fn session(command: SessionCommand) -> PanelOutput {
    PanelOutput::Session(command)
}

fn gameplay(command: GameplayCommand) -> PanelOutput {
    session(SessionCommand::Gameplay(command))
}

fn fellowship(command: FellowshipCommand) -> PanelOutput {
    gameplay(GameplayCommand::Fellowship(command))
}

fn option(option: u32, value: bool) -> PanelOutput {
    session(SessionCommand::SetCharacterOption { option, value })
}

fn chat(line: String) -> PanelOutput {
    session(SessionCommand::Chat(line))
}

impl WindowUi {
    /// The Social window, hidden, in place of retail's social panel; and
    /// the fellows on screen.
    pub(in crate::ui) fn mount_modern_social(&mut self) {
        let theme = Horizon::BRONZE;
        let fonts = SocialFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            heading: self.modern_font(Weight::SemiBold, 16.0),
            name: self.modern_font(Weight::SemiBold, 14.0),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            body: self.modern_font(Weight::Regular, 12.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
        };
        let field_font = self.modern_font(Weight::Regular, 12.5);
        let fellows_fonts = FellowsFonts {
            name: self.modern_font(Weight::SemiBold, 12.0),
            small: self.modern_font(Weight::Regular, 10.5),
            label: self.modern_font(Weight::SemiBold, 9.5),
        };
        let frame = match hud::mount_piece(
            &mut self.host.root,
            SOCIAL,
            (WIDTH, HEIGHT),
            SocialFrame::new(theme, fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern social: {e}"));
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
                    (0.0, 0.0, 200.0, if slot.lines() { 54.0 } else { 30.0 }),
                    field_font.clone(),
                    &theme,
                    slot.lines(),
                );
                ui.set_visible(id, false);
                (slot, id)
            })
            .collect();
        ui.hide_window(SOCIAL);
        let fellows = match hud::mount_piece(
            ui,
            hud::FELLOWS,
            (hud::fellows::WIDTH, 60.0),
            FellowsHud::new(theme, fellows_fonts),
        ) {
            Ok(id) => {
                ui.hide_window(hud::FELLOWS);
                Some(id)
            }
            Err(e) => {
                self.notices.push(format!("ui: modern fellows: {e}"));
                None
            }
        };
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.social = Some(SocialUi {
            frame,
            fields,
            fellows,
            declared: None,
        });
        modern.replace(SOCIAL_WINDOW);
        self.panels.stand_in(panel_id::SOCIAL, SOCIAL);
    }

    fn social_ui(&self) -> Option<&SocialUi> {
        self.modern.as_ref()?.social.as_ref()
    }

    /// The allegiance, fellowship or friends key: the window opens on that
    /// page, or closes when it shows it. None without the modern window.
    pub(in crate::ui) fn modern_open_social(&mut self, page: SocialPage) -> Option<bool> {
        let frame = self.social_ui()?.frame;
        let tab = match page {
            SocialPage::Allegiance => Tab::Allegiance,
            SocialPage::Fellowship => Tab::Fellowship,
            SocialPage::Friends => Tab::Friends,
        };
        let ui = &mut self.host.root;
        let on_page = ui.is_window_visible(SOCIAL)
            && ui
                .widget::<SocialFrame>(frame)
                .is_some_and(|f| f.tab == tab);
        if on_page {
            self.panels
                .set_visibility(&mut self.host.root, panel_id::SOCIAL, false);
            return Some(true);
        }
        if let Some(f) = ui.widget_mut::<SocialFrame>(frame) {
            f.show(tab);
        }
        self.panels
            .set_visibility(&mut self.host.root, panel_id::SOCIAL, true);
        Some(true)
    }

    /// Each frame: the fellows' vitals asked for while wanted, the fellows
    /// on screen fed, and while the window shows, its fields placed, the
    /// screen fed and what it asked for done.
    pub(in crate::ui) fn tick_modern_social(&mut self) {
        let Some(s) = self.social_ui() else {
            return;
        };
        let (frame, fields, fellows) = (s.frame, s.fields.clone(), s.fellows);
        let open = self.host.root.is_window_visible(SOCIAL);
        self.declare_fellow_vitals(frame, open);
        if let Some(id) = fellows {
            self.feed_modern_fellows(id);
        }
        if !open {
            let ui = &mut self.host.root;
            if let Some(focus) = ui.keyboard_focus()
                && fields.iter().any(|(_, id)| *id == focus)
            {
                ui.set_keyboard_focus(None);
            }
            return;
        }
        self.place_social_fields(frame, &fields);
        self.feed_social_screen(frame);
        let asks = self
            .host
            .root
            .widget_mut::<SocialFrame>(frame)
            .map(|f| std::mem::take(&mut f.asks))
            .unwrap_or_default();
        for ask in asks {
            self.social_ask(ask, frame, &fields);
        }
    }

    /// The fellows' vitals stream while the fellowship page shows or the
    /// fellows are on screen; asked for again after each entry into the
    /// world.
    fn declare_fellow_vitals(&mut self, frame: ElementId, open: bool) {
        let Some(social) = &self.facts.social else {
            return;
        };
        let in_world = social.in_world;
        let on_page = open
            && self
                .host
                .root
                .widget::<SocialFrame>(frame)
                .is_some_and(|f| f.tab == Tab::Fellowship);
        let on_screen = !self
            .modern_pieces()
            .is_some_and(|p| p.is_hidden(hud::FELLOWS));
        let wanted = on_page || (on_screen && social.fellowship.in_fellowship);
        let Some(s) = self.modern.as_mut().and_then(|m| m.social.as_mut()) else {
            return;
        };
        if !in_world {
            s.declared = None;
            return;
        }
        if s.declared == Some(wanted) {
            return;
        }
        s.declared = Some(wanted);
        emit(
            &mut self.host.root,
            fellowship(FellowshipCommand::SetPanelOpen(wanted)),
        );
    }

    fn place_social_fields(&mut self, frame: ElementId, fields: &[(Slot, ElementId)]) {
        let ui = &mut self.host.root;
        let rects: Vec<(Slot, Option<kit::Rect>)> = ui
            .widget::<SocialFrame>(frame)
            .map(|f| fields.iter().map(|(s, _)| (*s, f.field_rect(*s))).collect())
            .unwrap_or_default();
        let focus = ui.keyboard_focus();
        let mut empty = Vec::new();
        let mut focused = None;
        for ((slot, id), (_, rect)) in fields.iter().zip(rects) {
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
            if kit::field_text(ui, *id).trim().is_empty() {
                empty.push(*slot);
            }
            if focus == Some(*id) {
                focused = Some(*slot);
            }
        }
        let on_screen = !self
            .modern_pieces()
            .is_some_and(|p| p.is_hidden(hud::FELLOWS));
        if let Some(f) = self.host.root.widget_mut::<SocialFrame>(frame) {
            f.empty = empty;
            f.focused = focused;
            f.on_screen = on_screen;
        }
    }

    fn feed_social_screen(&mut self, frame: ElementId) {
        let Some(social) = &self.facts.social else {
            return;
        };
        let player = self.facts.state.player;
        let selected = self
            .player_view
            .and_then(|v| v.selected)
            .filter(|id| *id != player && ac_core::chat::is_player_object_id(*id))
            .and_then(|id| {
                let names = crate::ui::facts::composed_names(&self.status.names);
                self.facts.objects.get(id).map(|o| (id, names(o)))
            });
        let levels = self.experience.as_ref().map(|e| e.levels.clone());
        let to_raise = |level: u32| {
            levels.as_deref().map_or(0, |l| {
                crate::ui::social_panels::experience_to_raise(l, level)
            })
        };
        let screen = model::screen(&SocialInput {
            player,
            facts: social,
            selected,
            to_raise: Some(&to_raise),
            now_unix_s: ac_clock::unix_millis() / 1000,
        });
        if let Some(f) = self.host.root.widget_mut::<SocialFrame>(frame)
            && f.screen != screen
        {
            f.screen = screen;
        }
    }

    fn social_field(
        &self,
        fields: &[(Slot, ElementId)],
        slot: Slot,
    ) -> Option<(ElementId, String)> {
        let id = fields.iter().find(|(s, _)| *s == slot).map(|(_, id)| *id)?;
        Some((id, kit::field_text(&self.host.root, id).trim().to_owned()))
    }

    /// The words typed in `slot`, the field emptied.
    fn take_social_field(&mut self, fields: &[(Slot, ElementId)], slot: Slot) -> Option<String> {
        let (id, words) = self.social_field(fields, slot)?;
        if words.is_empty() {
            return None;
        }
        kit::set_field_text(&mut self.host.root, id, "");
        Some(words)
    }

    #[allow(clippy::too_many_lines)]
    fn social_ask(&mut self, ask: SocialAsk, frame: ElementId, fields: &[(Slot, ElementId)]) {
        let out = match ask {
            SocialAsk::Close => {
                self.panels
                    .set_visibility(&mut self.host.root, panel_id::SOCIAL, false);
                return;
            }
            SocialAsk::Tab(_) => {
                let ui = &mut self.host.root;
                if let Some(focus) = ui.keyboard_focus()
                    && fields.iter().any(|(_, id)| *id == focus)
                {
                    ui.set_keyboard_focus(None);
                }
                return;
            }
            SocialAsk::AllegianceChat => {
                self.start_modern_chat_line("@a ");
                return;
            }
            SocialAsk::Hometown => session(SessionCommand::Recall(Recall::AllegianceHometown)),
            SocialAsk::Swear(id, name) => {
                let look = Look::new(
                    &format!("Swear allegiance to {name}?"),
                    Glyph::Question,
                    GOLD,
                    "Swear",
                    "Cancel",
                );
                let text = format!(
                    "Part of the experience you earn passes up to {name}. {name} is asked to accept."
                );
                self.modern_ask_as(
                    look,
                    text,
                    DialogAction::Panel(gameplay(GameplayCommand::Allegiance(
                        AllegianceCommand::Swear(ObjectRef::Guid(id)),
                    ))),
                );
                return;
            }
            SocialAsk::Part(id) => {
                let parting = self
                    .host
                    .root
                    .widget::<SocialFrame>(frame)
                    .and_then(|f| f.screen.allegiance.as_ref())
                    .and_then(|a| a.card(id))
                    .and_then(|c| c.parting.clone());
                if let Some(p) = parting {
                    let look = Look::new(&p.title, Glyph::Warn, RED, &p.yes, "Cancel").dangerous();
                    self.modern_ask_as(
                        look,
                        p.text,
                        DialogAction::Panel(gameplay(GameplayCommand::Allegiance(p.command))),
                    );
                }
                return;
            }
            SocialAsk::IgnoreAllegiance(on) => option(option_id::IGNORE_ALLEGIANCE_REQUESTS, on),
            SocialAsk::Tell(name) => {
                self.start_modern_chat_line(&format!("@tell {name}, "));
                return;
            }
            SocialAsk::Invite(id) => fellowship(FellowshipCommand::Recruit(ObjectRef::Guid(id))),
            SocialAsk::Befriend(name) => session(SessionCommand::AddFriend(name)),
            SocialAsk::Unfriend(name) => session(SessionCommand::RemoveFriend(name)),
            SocialAsk::Manage(m) => match self.manage_line(m, fields) {
                Some(line) => chat(line),
                None => return,
            },
            SocialAsk::SelectFellow(id) => PanelOutput::SelectObject(id),
            SocialAsk::Recruit(id) => fellowship(FellowshipCommand::Recruit(ObjectRef::Guid(id))),
            SocialAsk::MakeLeader(id) => {
                fellowship(FellowshipCommand::AssignLeader(ObjectRef::Guid(id)))
            }
            SocialAsk::Dismiss(id) => fellowship(FellowshipCommand::Dismiss(ObjectRef::Guid(id))),
            SocialAsk::SetOpen(on) => fellowship(FellowshipCommand::SetOpen(on)),
            SocialAsk::Leave => fellowship(FellowshipCommand::Quit { disband: false }),
            SocialAsk::Disband => {
                let name = self
                    .facts
                    .social
                    .as_ref()
                    .map(|s| s.fellowship.name.clone())
                    .unwrap_or_default();
                let look = Look::new(
                    &format!("Disband {name}?"),
                    Glyph::Warn,
                    RED,
                    "Disband",
                    "Cancel",
                )
                .dangerous();
                self.modern_ask_as(
                    look,
                    "Everyone leaves the fellowship, and it ends.".to_owned(),
                    DialogAction::Panel(fellowship(FellowshipCommand::Quit { disband: true })),
                );
                return;
            }
            SocialAsk::Switch(which, on) => match which {
                FellowSwitch::ShareLoot => option(option_id::FELLOWSHIP_SHARE_LOOT, on),
                FellowSwitch::IgnoreRequests => option(option_id::IGNORE_FELLOWSHIP_REQUESTS, on),
                FellowSwitch::AutoAccept => option(option_id::FELLOWSHIP_AUTO_ACCEPT_REQUESTS, on),
                FellowSwitch::ShareXp => option(option_id::FELLOWSHIP_SHARE_XP, on),
                FellowSwitch::OnScreen => {
                    self.set_modern_piece_hidden(hud::FELLOWS, !on);
                    return;
                }
            },
            SocialAsk::Create => {
                let share_xp = self
                    .facts
                    .social
                    .as_ref()
                    .is_some_and(|s| s.options.get(option_id::FELLOWSHIP_SHARE_XP));
                match self.take_social_field(fields, Slot::FellowshipName) {
                    Some(name) => fellowship(FellowshipCommand::Create { name, share_xp }),
                    None => return,
                }
            }
            SocialAsk::AddFriend => match self.take_social_field(fields, Slot::FriendName) {
                Some(name) => session(SessionCommand::AddFriend(name)),
                None => return,
            },
            SocialAsk::AppearOffline(on) => option(option_id::APPEAR_OFFLINE, on),
            SocialAsk::Ignore(account) => match self.take_social_field(fields, Slot::IgnoreName) {
                Some(name) => session(SessionCommand::Squelch {
                    add: true,
                    name,
                    account,
                }),
                None => return,
            },
            SocialAsk::Unignore(name, account) => session(SessionCommand::Squelch {
                add: false,
                name,
                account,
            }),
            SocialAsk::Hide(who, kind, on) => {
                let Some(k) = HIDEABLE.iter().find(|h| h.kind == kind) else {
                    return;
                };
                chat(match who {
                    Some(name) => model::squelch_line(&name, k, on),
                    None => model::filter_line(k, on),
                })
            }
        };
        emit(&mut self.host.root, out);
    }

    /// The chat box's command for Manage allegiance's `ask`, with the words
    /// typed for it (the field emptied where the words are used up).
    fn manage_line(&mut self, ask: ManageAsk, fields: &[(Slot, ElementId)]) -> Option<String> {
        let typed = |me: &Self, slot| {
            me.social_field(fields, slot)
                .map(|(_, w)| w)
                .unwrap_or_default()
        };
        Some(match ask {
            ManageAsk::Rename => {
                format!(
                    "@allegiance name set {}",
                    self.take_social_field(fields, Slot::AllegianceName)?
                )
            }
            ManageAsk::PostMotd => {
                let words = typed(self, Slot::Motd).replace('\n', " ");
                if words.is_empty() {
                    return None;
                }
                format!("@allegiance motd set {words}")
            }
            ManageAsk::ClearMotd => {
                if let Some((id, _)) = self.social_field(fields, Slot::Motd) {
                    kit::set_field_text(&mut self.host.root, id, "");
                }
                "@allegiance motd clear".to_owned()
            }
            ManageAsk::ListOfficers => "@allegiance officer list".to_owned(),
            ManageAsk::Appoint(level) => {
                let name = self.take_social_field(fields, Slot::Officer)?;
                format!("@allegiance officer set {level} {name}")
            }
            ManageAsk::RemoveOffice => {
                let name = self.take_social_field(fields, Slot::Officer)?;
                format!("@allegiance officer remove {name}")
            }
            ManageAsk::TitleOffice(level) => {
                let title = self.take_social_field(fields, Slot::Officer)?;
                format!("@allegiance title set {level} {title}")
            }
            ManageAsk::Lock(on) => format!("@allegiance lock {}", if on { "on" } else { "off" }),
            ManageAsk::Approve => {
                let name = self.take_social_field(fields, Slot::Approved)?;
                format!("@allegiance lock bypass {name}")
            }
            ManageAsk::ListBans => "@allegiance ban list".to_owned(),
            ManageAsk::Ban => format!("@allegiance ban add {}", typed(self, Slot::Banned)),
            ManageAsk::LiftBan => format!("@allegiance ban remove {}", typed(self, Slot::Banned)),
            ManageAsk::Gag => format!("@allegiance chat gag {}", typed(self, Slot::Banned)),
            ManageAsk::PutOutOfChat => {
                format!("@allegiance chat kick {}", typed(self, Slot::Banned))
            }
        })
    }

    /// The fellows on screen follow the fellowship: shown while there are
    /// fellows and the player has not hidden them; a click selects one.
    fn feed_modern_fellows(&mut self, id: ElementId) {
        let hidden = self
            .modern_pieces()
            .is_some_and(|p| p.is_hidden(hud::FELLOWS));
        let player = self.facts.state.player;
        let selected = self.player_view.and_then(|v| v.selected);
        let none = ac_runtime::gameplay::FellowshipView::default();
        let fellowship = self.facts.social.as_ref().map_or(&none, |s| &s.fellowship);
        let others = fellowship
            .members
            .iter()
            .filter(|m| m.guid != player)
            .count();
        let show = !hidden && fellowship.in_fellowship && others > 0;
        let ui = &mut self.host.root;
        let visible = ui.is_window_visible(hud::FELLOWS);
        if show != visible {
            if show {
                ui.show_window(hud::FELLOWS);
            } else {
                ui.hide_window(hud::FELLOWS);
            }
        }
        if !show {
            return;
        }
        // Fed again only when the fellowship or the selection moved.
        let from = (fellowship.revision, player, selected);
        let picked = ui.widget_mut::<FellowsHud>(id).and_then(|f| {
            if f.fed_from != Some(from) {
                f.fed_from = Some(from);
                f.feed(fellowship, player, selected);
            }
            f.picked.take()
        });
        let height = ui.widget::<FellowsHud>(id).map_or(60.0, FellowsHud::height);
        if let Some(e) = ui.element_mut(id)
            && (e.height - height).abs() > 0.5
        {
            e.height = height;
        }
        if let Some(guid) = picked {
            emit(ui, PanelOutput::SelectObject(guid));
        }
    }
}
