//! The Character window in the game window, in place of retail's
//! character window (with its titles page) and the character information
//! panel: the character panel, the attributes and skills keys and the
//! panel strip open it on Attributes and skills; the information panel's
//! key on Information. It reads the sheet retail's window is fed (built
//! from the player's numbers), raises one thing at a time until the sheet
//! answers, and asks before training (the game's question).

use ac_core::spells::{EnchantmentTypeFlags, affects_stat};
use ac_runtime::gameplay::GameplayCommand;
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::panels::character::{ATTRIBUTE_ROWS, CharacterSheet, CharacterTab, VITAL_ROWS};
use ac_ui::panels::character_titles::sorted_titles;
use ac_ui::panels::items::emit;
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::tree::{Element, ElementId};

use super::model::Row;
use super::window::{
    CharacterExtras, CharacterFonts, CharacterFrame, CharacterPage, CharacterRequest, HEIGHT,
    InfoCard, SKILLS_H, SKILLS_TOP, SKILLS_W, SKILLS_X, SkillList, SkillRequest, TITLES_H,
    TITLES_TOP, TITLES_W, TITLES_X, TitleGrid, WIDTH,
};
use crate::ui::modern::{hud, kit};
use crate::ui::window::{CHARACTER_WINDOW, WindowUi};

/// The window's name.
pub const CHARACTER: &str = "modern-character";

/// How often the window reads again while it shows, seconds.
const REREAD: f64 = 0.5;

/// How long a raise holds the buttons when the sheet does not change,
/// seconds.
const RAISE_WAIT: f64 = 3.0;

/// The Character window's elements and what it remembers.
#[derive(Debug)]
pub struct CharacterUi {
    /// The frame (the window).
    pub frame: ElementId,
    /// The skill list.
    pub skills: ElementId,
    /// The title grid.
    pub titles: ElementId,
    /// The skills' search field.
    pub skill_search: ElementId,
    /// The titles' search field.
    pub title_search: ElementId,
    /// The sheet a raise was sent from, and how long ago (a raise the
    /// server never answers stops holding the buttons after a while).
    raised_from: Option<CharacterSheet>,
    raised_for: f64,
    /// Time since the last read.
    since: f64,
    /// It was showing last frame.
    was_open: bool,
}

impl WindowUi {
    /// The Character window, hidden, in place of retail's.
    pub(in crate::ui) fn mount_modern_character(&mut self) {
        let fonts = CharacterFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            strong: self.modern_font(Weight::SemiBold, 12.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
            body: self.modern_font(Weight::Regular, 12.0),
            name: self.modern_font(Weight::SemiBold, 12.5),
            value: self.modern_font(Weight::SemiBold, 15.0),
            level: self.modern_font(Weight::SemiBold, 24.0),
            big: self.modern_font(Weight::SemiBold, 18.0),
            huge: self.modern_font(Weight::SemiBold, 30.0),
            heading: self.modern_font(Weight::SemiBold, 15.0),
        };
        let field_font = self.modern_font(Weight::Regular, 11.5);
        let frame = match hud::mount_piece(
            &mut self.host.root,
            CHARACTER,
            (WIDTH, HEIGHT),
            CharacterFrame::new(Horizon::BRONZE, fonts.clone()),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern character: {e}"));
                return;
            }
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        let skills = ui.add(
            frame,
            Element::at(SKILLS_X, SKILLS_TOP, SKILLS_W, SKILLS_H).with_anchors(AnchorEdges::NONE),
            SkillList::new(Horizon::BRONZE, fonts.clone()),
        );
        let titles = ui.add(
            frame,
            Element::at(TITLES_X, TITLES_TOP, TITLES_W, TITLES_H)
                .with_anchors(AnchorEdges::NONE)
                .with_visible(false),
            TitleGrid::new(Horizon::BRONZE, fonts),
        );
        let skill_search = kit::mount_search(
            ui,
            frame,
            CharacterFrame::skill_search_rect(),
            field_font.clone(),
            &Horizon::BRONZE,
        );
        let title_search = kit::mount_search(
            ui,
            frame,
            CharacterFrame::title_search_rect(),
            field_font,
            &Horizon::BRONZE,
        );
        ui.set_visible(title_search, false);
        ui.hide_window(CHARACTER);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.character = Some(CharacterUi {
            frame,
            skills,
            titles,
            skill_search,
            title_search,
            raised_from: None,
            raised_for: 0.0,
            since: 0.0,
            was_open: false,
        });
        modern.replace(CHARACTER_WINDOW);
        modern.replace(crate::ui::character_pages::CHARACTER_INFO_WINDOW);
        self.panels.stand_in(panel_id::CHARACTER, CHARACTER);
    }

    /// The attributes or skills key: the window opens on Attributes and
    /// skills, or closes when it shows that page. None without the modern
    /// window.
    pub(in crate::ui) fn modern_open_character(&mut self, _tab: CharacterTab) -> Option<bool> {
        self.modern.as_ref()?.character.as_ref()?;
        Some(self.toggle_character_page(CharacterPage::Stats))
    }

    /// The character information panel toggled (or closed): the window on
    /// its Information page. False when it is not that panel or the
    /// window is not mounted.
    pub(in crate::ui) fn modern_character_toggle(&mut self, panel: u32, close_only: bool) -> bool {
        if panel != panel_id::CHARACTER_INFORMATION
            || self
                .modern
                .as_ref()
                .and_then(|m| m.character.as_ref())
                .is_none()
        {
            return false;
        }
        if close_only {
            if self.character_page() == Some(CharacterPage::Info) {
                self.panels
                    .set_visibility(&mut self.host.root, panel_id::CHARACTER, false);
            }
            return true;
        }
        self.toggle_character_page(CharacterPage::Info);
        true
    }

    fn character_page(&self) -> Option<CharacterPage> {
        let frame = self.modern.as_ref()?.character.as_ref()?.frame;
        if !self.host.root.is_window_visible(CHARACTER) {
            return None;
        }
        self.host
            .root
            .widget::<CharacterFrame>(frame)
            .map(|f| f.page)
    }

    fn toggle_character_page(&mut self, page: CharacterPage) -> bool {
        if self.character_page() == Some(page) {
            self.panels
                .set_visibility(&mut self.host.root, panel_id::CHARACTER, false);
            return true;
        }
        self.show_character_page(page);
        self.panels
            .set_visibility(&mut self.host.root, panel_id::CHARACTER, true);
        true
    }

    fn show_character_page(&mut self, page: CharacterPage) {
        let Some(c) = self.modern.as_ref().and_then(|m| m.character.as_ref()) else {
            return;
        };
        let (frame, skills, titles, ss, ts) =
            (c.frame, c.skills, c.titles, c.skill_search, c.title_search);
        let ui = &mut self.host.root;
        if let Some(f) = ui.widget_mut::<CharacterFrame>(frame) {
            f.page = page;
        }
        ui.set_visible(skills, page == CharacterPage::Stats);
        ui.set_visible(ss, page == CharacterPage::Stats);
        ui.set_visible(titles, page == CharacterPage::Titles);
        ui.set_visible(ts, page == CharacterPage::Titles);
        if let Some(focus) = ui.keyboard_focus()
            && (focus == ss || focus == ts)
        {
            ui.set_keyboard_focus(None);
        }
    }

    /// Each frame: what the frame and its lists asked for; the sheet,
    /// titles and information read again while it shows.
    pub(in crate::ui) fn tick_modern_character(&mut self, dt: f64) {
        let Some(c) = self.modern.as_ref().and_then(|m| m.character.as_ref()) else {
            return;
        };
        let (frame, skills, titles, ss, ts) =
            (c.frame, c.skills, c.titles, c.skill_search, c.title_search);
        if !self.host.root.is_window_visible(CHARACTER) {
            let ui = &mut self.host.root;
            if let Some(focus) = ui.keyboard_focus()
                && (focus == ss || focus == ts)
            {
                ui.set_keyboard_focus(None);
            }
            if let Some(c) = self.modern.as_mut().and_then(|m| m.character.as_mut()) {
                c.was_open = false;
            }
            return;
        }
        let ui = &mut self.host.root;
        let page = ui
            .widget::<CharacterFrame>(frame)
            .map_or(CharacterPage::Stats, |f| f.page);
        let field = if page == CharacterPage::Titles {
            ts
        } else {
            ss
        };
        let typed = kit::field_text(ui, field);
        let focused = ui.keyboard_focus() == Some(field);
        let (asks, searched) = ui
            .widget_mut::<CharacterFrame>(frame)
            .map(|f| {
                let changed = f.search != typed;
                f.search = typed;
                f.searching = focused;
                (std::mem::take(&mut f.requests), changed)
            })
            .unwrap_or_default();
        let skill_asks = ui
            .widget_mut::<SkillList>(skills)
            .map(|l| std::mem::take(&mut l.requests))
            .unwrap_or_default();
        let picked = ui
            .widget_mut::<TitleGrid>(titles)
            .and_then(|g| g.picked.take());
        let mut reread = searched;
        for ask in asks {
            match ask {
                CharacterRequest::Close => {
                    self.panels
                        .set_visibility(&mut self.host.root, panel_id::CHARACTER, false);
                }
                CharacterRequest::Page(p) => {
                    self.show_character_page(p);
                    reread = true;
                }
                CharacterRequest::Choose(_) => reread = true,
                CharacterRequest::Send(out) => self.send_character(out),
                CharacterRequest::SetTitle(id) => emit(
                    &mut self.host.root,
                    PanelOutput::Session(SessionCommand::Gameplay(GameplayCommand::SetTitle(id))),
                ),
            }
        }
        for ask in skill_asks {
            let ui = &mut self.host.root;
            match ask {
                SkillRequest::Choose(id) => {
                    if let Some(f) = ui.widget_mut::<CharacterFrame>(frame) {
                        f.choose(Row::Skill(id));
                    }
                    reread = true;
                }
                SkillRequest::Raise(id) => {
                    let out = ui.widget_mut::<CharacterFrame>(frame).and_then(|f| {
                        f.send_raise(Row::Skill(id), super::model::Amount::One);
                        f.requests.pop().and_then(|r| match r {
                            CharacterRequest::Send(o) => Some(o),
                            _ => None,
                        })
                    });
                    if let Some(out) = out {
                        self.send_character(out);
                    }
                }
            }
        }
        if let Some(id) = picked
            && let Some(f) = self.host.root.widget_mut::<CharacterFrame>(frame)
        {
            f.title_pick = Some(id);
        }
        let Some(c) = self.modern.as_mut().and_then(|m| m.character.as_mut()) else {
            return;
        };
        c.since += dt;
        if c.raised_from.is_some() {
            c.raised_for += dt;
            if c.raised_for >= RAISE_WAIT {
                c.raised_from = None;
                reread = true;
            }
        }
        let first = !c.was_open;
        c.was_open = true;
        if first || reread || c.since >= REREAD {
            c.since = 0.0;
            self.read_modern_character();
        }
    }

    /// A raise goes to the session (a training through the game's
    /// question); the sheet it was sent from is noted.
    fn send_character(&mut self, out: PanelOutput) {
        let frame = self
            .modern
            .as_ref()
            .and_then(|m| m.character.as_ref())
            .map(|c| c.frame);
        let sheet = frame
            .and_then(|f| self.host.root.widget::<CharacterFrame>(f))
            .map(|f| f.sheet.clone());
        if !matches!(out, PanelOutput::TrainSkill { .. })
            && let Some(c) = self.modern.as_mut().and_then(|m| m.character.as_mut())
        {
            c.raised_from = sheet;
            c.raised_for = 0.0;
        }
        emit(&mut self.host.root, out);
    }

    /// The sheet, the chosen row's spells, titles and information read
    /// again.
    #[allow(clippy::too_many_lines)]
    fn read_modern_character(&mut self) {
        let Some(c) = self.modern.as_ref().and_then(|m| m.character.as_ref()) else {
            return;
        };
        let (frame, skills, titles) = (c.frame, c.skills, c.titles);
        let raised_from = c.raised_from.clone();
        let sheet = self.sheet.clone();
        let waiting = raised_from.as_ref().is_some_and(|s| *s == sheet);
        if !waiting && let Some(c) = self.modern.as_mut().and_then(|m| m.character.as_mut()) {
            c.raised_from = None;
        }
        let (page, chosen, search) = self
            .host
            .root
            .widget::<CharacterFrame>(frame)
            .map(|f| (f.page, f.chosen, f.search.clone()))
            .unwrap_or_default();
        // The spells on the chosen row, when they add up to what spells
        // give it (else the sum shows as one line).
        let book = self.facts.numbers().and_then(|n| n.spellbook());
        let vitae = book
            .map(|b| ac_ui::panels::indicators::current_vitae(Some(b)))
            .filter(|v| *v < 1.0);
        let mut spell_lines = Vec::new();
        if let (Some(book), Some(row)) = (book, chosen) {
            let (family, key) = match row {
                Row::Attribute(i) => (EnchantmentTypeFlags::ATTRIBUTE, ATTRIBUTE_ROWS[i].2),
                Row::Vital(i) => (EnchantmentTypeFlags::SECOND_ATT, VITAL_ROWS[i].2),
                Row::Skill(id) => (EnchantmentTypeFlags::SKILL, id),
            };
            let additive = EnchantmentTypeFlags::ADDITIVE.bits();
            for r in book.enchantments_in_effect_snapshot() {
                if !affects_stat(&r, family, key) {
                    continue;
                }
                let Some(name) = book.try_get_metadata(r.spell_id).map(|m| m.name.clone()) else {
                    continue;
                };
                if r.stat_mod_type.unwrap_or(0) & additive == 0 {
                    spell_lines.clear();
                    break;
                }
                spell_lines.push((name, r.stat_mod_value.unwrap_or(0.0).round() as i32));
            }
            let sum: i32 = spell_lines.iter().map(|(_, v)| v).sum();
            if sum != super::model::spells(&sheet, row) {
                spell_lines.clear();
            }
        }
        let player = self.facts.state.player;
        let vitae_experience = vitae.map(|v| {
            i64::from(ac_ui::panels::indicators::vitae_experience_to_recover(
                v,
                &self.facts.objects,
                player,
            ))
        });
        // The titles.
        let (all_titles, display_title) = match self.facts.titles.clone() {
            Some(t) => {
                let strings = self.content.strings();
                let names = std::cell::RefCell::new(&mut self.character_pages.titles);
                let resolve = |id| names.borrow_mut().resolve(strings, id);
                (
                    sorted_titles(&t.title_ids, &resolve),
                    (t.display_title_id != 0).then_some(t.display_title_id),
                )
            }
            None => (Vec::new(), None),
        };
        // The information.
        let info = if page == CharacterPage::Info {
            self.character_info_cards()
        } else {
            Vec::new()
        };
        let ui = &mut self.host.root;
        if let Some(f) = ui.widget_mut::<CharacterFrame>(frame) {
            f.sheet = sheet.clone();
            f.waiting = waiting;
            let info = if page == CharacterPage::Info {
                info
            } else {
                std::mem::take(&mut f.extras.info)
            };
            f.extras = CharacterExtras {
                vitae,
                vitae_experience,
                spell_lines,
                titles: all_titles.clone(),
                display_title,
                info,
            };
        }
        if let Some(l) = ui.widget_mut::<SkillList>(skills) {
            l.show(
                &sheet,
                if page == CharacterPage::Stats {
                    &search
                } else {
                    ""
                },
            );
            l.waiting = waiting;
            l.chosen = match chosen {
                Some(Row::Skill(id)) => Some(id),
                _ => None,
            };
        }
        if let Some(g) = ui.widget_mut::<TitleGrid>(titles) {
            let wanted = if page == CharacterPage::Titles {
                search.trim().to_lowercase()
            } else {
                String::new()
            };
            g.titles = all_titles
                .into_iter()
                .filter(|(_, w)| wanted.is_empty() || w.to_lowercase().contains(&wanted))
                .collect();
            g.shown = display_title;
        }
    }

    /// The Information page's cards: the game's report in its parts, and
    /// the allegiance and fellowship.
    fn character_info_cards(&mut self) -> Vec<InfoCard> {
        let mut cards = Vec::new();
        let titles = [
            "LIFE",
            "NATURAL RESISTANCES",
            "STARTING ATTRIBUTES",
            "CHESS AND FISHING",
            "MASTERIES AND AUGMENTATIONS",
            "BURDEN",
        ];
        let parts = self.character_info_parts();
        let prose = |part: &str| -> Vec<String> {
            part.split('\n')
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect()
        };
        if let Some(parts) = &parts {
            for (title, part) in titles.iter().zip(parts.iter()).take(1) {
                cards.push(InfoCard {
                    title: (*title).to_owned(),
                    rows: Vec::new(),
                    prose: prose(part),
                });
            }
        }
        if let Some(social) = &self.facts.social {
            let player = self.facts.state.player;
            let mut rows = Vec::new();
            match &social.allegiance {
                Some(a) => {
                    if !a.allegiance_name.trim().is_empty() {
                        rows.push(("Allegiance".to_owned(), a.allegiance_name.clone()));
                    }
                    if let Some(m) = &a.monarch {
                        rows.push(("Monarch".to_owned(), m.name.clone()));
                    }
                    let me = a.records.iter().find(|r| r.character_id == player);
                    if let Some(patron) = me
                        .and_then(|me| a.records.iter().find(|r| r.character_id == me.parent_guid))
                    {
                        rows.push(("Patron".to_owned(), patron.name.clone()));
                    }
                    rows.push(("Rank".to_owned(), social.allegiance_rank.to_string()));
                    let direct = a.records.iter().filter(|r| r.parent_guid == player).count();
                    rows.push(("Vassals".to_owned(), direct.to_string()));
                    rows.push(("Members".to_owned(), a.total_members.to_string()));
                }
                None => rows.push(("Allegiance".to_owned(), "none".to_owned())),
            }
            cards.push(InfoCard {
                title: "ALLEGIANCE".to_owned(),
                rows,
                prose: Vec::new(),
            });
            let f = &social.fellowship;
            let rows = if f.in_fellowship {
                let leader = f
                    .members
                    .iter()
                    .find(|m| m.guid == f.leader_guid)
                    .map_or_else(String::new, |m| m.name.clone());
                vec![
                    ("Fellowship".to_owned(), f.name.clone()),
                    ("Leader".to_owned(), leader),
                    ("Members".to_owned(), f.members.len().to_string()),
                    (
                        "Sharing".to_owned(),
                        if f.share_xp {
                            "experience".to_owned()
                        } else {
                            "nothing".to_owned()
                        },
                    ),
                ]
            } else {
                vec![("Fellowship".to_owned(), "none".to_owned())]
            };
            cards.push(InfoCard {
                title: "FELLOWSHIP".to_owned(),
                rows,
                prose: Vec::new(),
            });
        }
        if let Some(parts) = &parts {
            for (title, part) in titles.iter().zip(parts.iter()).skip(1) {
                let lines = prose(part);
                if lines.is_empty() {
                    continue;
                }
                cards.push(InfoCard {
                    title: (*title).to_owned(),
                    rows: Vec::new(),
                    prose: lines,
                });
            }
        }
        cards
    }
}
