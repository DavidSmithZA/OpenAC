//! The Inspect sheet in the game window, in place of retail's examination
//! window: one sheet for everything the player assesses (items, creatures,
//! characters) and for spells examined from a bar or the spellbook. It
//! opens on the answer to the player's own question, follows the selection
//! while it shows, and asks again about a creature every 0.75 seconds in
//! combat, as retail's window does.

use ac_core::combat::CombatMode;
use ac_core::items::{EquipMask, ItemType};
use ac_runtime::inventory::{ItemCommand, ObjectRef};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::appraisal::{AppraisalPage, page_for};
use ac_ui::panels::items::emit;
use ac_ui::tree::ElementId;

use super::sheet_view::{SheetFonts, SheetRequest, SheetView};
use super::sheets;
use crate::ui::modern::hud;
use crate::ui::window::WindowUi;

/// The sheet's window name and size.
pub const INSPECT: &str = "modern-inspect";
pub const WIDTH: f32 = 360.0;
pub const HEIGHT: f32 = 620.0;

/// How often a creature is asked about again in combat, seconds.
const REFRESH: f64 = 0.75;

/// What the sheet shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// An object, by id, and the page its answer opened.
    Object(u32, AppraisalPage),
    /// A spell, by id.
    Spell(u32),
}

/// The Inspect sheet's element and what it remembers.
#[derive(Debug)]
pub struct InspectUi {
    /// The sheet (the window).
    pub sheet: ElementId,
    /// What it shows.
    pub subject: Option<Subject>,
    /// The selection it last followed.
    followed: Option<u32>,
    /// The field writing an inscription on the sheet.
    scribe: super::scribe::Scribe,
    /// An item's sheet as its answer made it, before the plugins' cards;
    /// and the notes those cards were made from.
    base: Option<super::sheet::Sheet>,
    notes_seen:
        Option<std::sync::Arc<std::collections::BTreeMap<u32, Vec<ac_plugin_ui::ItemNoteEntry>>>>,
    /// Time since the last combat refresh.
    since: f64,
    /// The tab to open when the answer about this item comes (the
    /// inventory's Inscribe opens Details).
    want_tab: Option<(u32, &'static str)>,
}

impl WindowUi {
    /// The Inspect sheet, hidden, in place of retail's examination window.
    pub(in crate::ui) fn mount_modern_inspect(&mut self) {
        let writing = self.modern_font(Weight::Regular, 12.5);
        let fonts = SheetFonts {
            name: self.modern_font(Weight::SemiBold, 16.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
            body: self.modern_font(Weight::Regular, 12.5),
            strong: self.modern_font(Weight::SemiBold, 12.5),
            big: self.modern_font(Weight::SemiBold, 18.0),
        };
        let mut view = SheetView::new(Horizon::current(), fonts);
        view.closable = true;
        view.framed = true;
        let sheet = match hud::mount_piece(&mut self.host.root, INSPECT, (WIDTH, HEIGHT), view) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern inspect: {e}"));
                return;
            }
        };
        if let Some(e) = self.host.root.element_mut(sheet) {
            e.draggable = true;
        }
        let scribe =
            super::scribe::Scribe::mount(&mut self.host.root, sheet, Horizon::current(), writing);
        self.host.root.hide_window(INSPECT);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.inspect = Some(InspectUi {
            sheet,
            subject: None,
            followed: None,
            scribe,
            base: None,
            notes_seen: None,
            since: 0.0,
            want_tab: None,
        });
        modern.replace(crate::ui::window::EXAMINATION_WINDOW);
    }

    /// The answers that came: the sheet opens on the player's own question
    /// and refreshes on any about what it shows. Called before retail's
    /// examination code takes them.
    pub(in crate::ui) fn feed_modern_inspect(&mut self) {
        let Some(inspect) = self.modern.as_ref().and_then(|m| m.inspect.as_ref()) else {
            return;
        };
        let sheet = inspect.sheet;
        let shown = inspect.subject;
        let mut chosen = None;
        for a in self.facts.appraisals.iter().filter(|a| a.present_in_ui) {
            let guid = a.response.guid;
            let about_shown = matches!(shown, Some(Subject::Object(id, _)) if id == guid);
            if a.opens_window || about_shown {
                chosen = Some(a.clone());
            }
        }
        let Some(a) = chosen else {
            return;
        };
        let opens = a.opens_window;
        let guid = a.response.guid;
        let page = page_for(&a.response);
        let player = self.facts.state.player;
        let book = self.facts.numbers().and_then(|n| n.spellbook());
        let spell = |id| book.and_then(|b| b.try_get_metadata(id)).cloned();
        let objects = &self.facts.objects;
        let o = objects.get(guid);
        let names = crate::ui::window::ViewerNames {
            tables: &self.status.names,
            numbers: self.facts.numbers(),
            character_title: &|_| None,
        };
        let me = self.status.playing.clone().unwrap_or_default();
        let made = match page {
            AppraisalPage::Character => Some(sheets::creature_sheet(
                o,
                &a.response,
                true,
                &names,
                Horizon::current(),
            )),
            AppraisalPage::Creature => Some(sheets::creature_sheet(
                o,
                &a.response,
                false,
                &names,
                Horizon::current(),
            )),
            AppraisalPage::Item | AppraisalPage::Spell => o.map(|o| {
                let composed = crate::ui::facts::composed_names(&self.status.names);
                let owned = guid != player && objects.is_owned_by_object(guid, player);
                let sheet = sheets::item_sheet(
                    o,
                    Some(&a.response),
                    &composed,
                    &spell,
                    &self.status.names,
                    self.facts.numbers(),
                    &me,
                    owned,
                    objects,
                    &mut self.icons,
                );
                let place = o
                    .item_type
                    .intersects(ItemType::PORTAL | ItemType::LOCKABLE | ItemType::LIFE_STONE)
                    || (o.item_type.intersects(ItemType::CONTAINER) && !owned);
                if place && o.valid_locations == EquipMask::NONE {
                    sheets::one_column(sheet)
                } else {
                    sheet
                }
            }),
        };
        let Some(made) = made else {
            return;
        };
        let item = matches!(page, AppraisalPage::Item | AppraisalPage::Spell);
        if let Some(i) = self.modern.as_mut().and_then(|m| m.inspect.as_mut()) {
            i.subject = Some(Subject::Object(guid, page));
            i.followed = Some(guid);
            i.base = item.then(|| made.clone());
            i.notes_seen = None;
        }
        let tab = self
            .modern
            .as_mut()
            .and_then(|m| m.inspect.as_mut())
            .and_then(|i| i.want_tab.take_if(|(id, _)| *id == guid))
            .map(|(_, tab)| tab);
        let ui = &mut self.host.root;
        if let Some(v) = ui.widget_mut::<SheetView>(sheet) {
            v.set(made);
            if let Some(tab) = tab {
                v.choose_tab(tab);
            }
        }
        self.add_inspect_cards();
        if opens {
            self.show_modern_inspect();
        }
    }

    /// Asks about the player's item `id` for the sheet, which opens on
    /// `tab` when the answer comes.
    pub(in crate::ui) fn inspect_item_on_tab(&mut self, id: u32, tab: &'static str) {
        if let Some(i) = self.modern.as_mut().and_then(|m| m.inspect.as_mut()) {
            i.want_tab = Some((id, tab));
        }
        ac_ui::panels::items::emit(
            &mut self.host.root,
            ac_ui::panels::PanelOutput::Session(ac_runtime::session::SessionCommand::Item(
                ac_runtime::inventory::ItemCommand::Appraise(
                    ac_runtime::inventory::ObjectRef::Guid(id),
                ),
            )),
        );
    }

    fn show_modern_inspect(&mut self) {
        let Some(sheet) = self
            .modern
            .as_ref()
            .and_then(|m| m.inspect.as_ref())
            .map(|i| i.sheet)
        else {
            return;
        };
        if !self.host.root.is_window_visible(INSPECT) {
            self.host.root.show_window(INSPECT);
        }
        self.host.root.bring_to_front(sheet);
    }

    /// A spell examined (from a bar or the spellbook): the sheet shows it.
    /// False without the modern sheet.
    pub(in crate::ui) fn modern_examine_spell(
        &mut self,
        spell: &ac_core::spells::SpellMetadata,
        magic_skill: u32,
        components: &[ac_ui::panels::appraisal::SpellExamineComponent],
    ) -> bool {
        let Some(sheet) = self
            .modern
            .as_ref()
            .and_then(|m| m.inspect.as_ref())
            .map(|i| i.sheet)
        else {
            return false;
        };
        let icon = ac_ui::icons::IconSource::spell_icon(&mut self.icons, spell.spell_id);
        let made = sheets::spell_sheet(spell, icon, magic_skill, components);
        if let Some(v) = self.host.root.widget_mut::<SheetView>(sheet) {
            v.set(made);
        }
        if let Some(i) = self.modern.as_mut().and_then(|m| m.inspect.as_mut()) {
            i.subject = Some(Subject::Spell(spell.spell_id));
            i.base = None;
        }
        self.show_modern_inspect();
        true
    }

    /// An item's sheet takes the plugins' cards, again whenever their
    /// notes change.
    fn add_inspect_cards(&mut self) {
        let notes = std::sync::Arc::clone(&self.facts.plugin_ui.item_notes);
        let Some(i) = self.modern.as_mut().and_then(|m| m.inspect.as_mut()) else {
            return;
        };
        if i.notes_seen
            .as_ref()
            .is_some_and(|seen| std::sync::Arc::ptr_eq(seen, &notes))
        {
            return;
        }
        i.notes_seen = Some(notes);
        let (Some(base), Some(Subject::Object(id, _))) = (i.base.clone(), i.subject) else {
            return;
        };
        let sheet = i.sheet;
        let cards = super::plugin_notes::cards(&self.facts.plugin_ui, &Horizon::current(), id);
        let made = super::plugin_notes::with_cards(base, cards);
        if let Some(v) = self.host.root.widget_mut::<SheetView>(sheet) {
            v.set(made);
        }
    }

    /// The examine key closes the sheet while it shows. False when it is
    /// not the sheet's.
    pub(in crate::ui) fn modern_inspect_key(&mut self) -> bool {
        if self
            .modern
            .as_ref()
            .and_then(|m| m.inspect.as_ref())
            .is_none()
            || !self.host.root.is_window_visible(INSPECT)
        {
            return false;
        }
        self.host.root.hide_window(INSPECT);
        true
    }

    /// Each frame: the close button; the selection followed while it
    /// shows; a creature asked about again in combat.
    pub(in crate::ui) fn tick_modern_inspect(&mut self, dt: f64) {
        let Some(sheet) = self
            .modern
            .as_ref()
            .and_then(|m| m.inspect.as_ref())
            .map(|i| i.sheet)
        else {
            return;
        };
        let asks = self
            .host
            .root
            .widget_mut::<SheetView>(sheet)
            .map(|v| std::mem::take(&mut v.requests))
            .unwrap_or_default();
        if asks.contains(&SheetRequest::Close) {
            self.host.root.hide_window(INSPECT);
        }
        let open = self.host.root.is_window_visible(INSPECT);
        self.add_inspect_cards();
        // The inscription field: what was written goes out.
        if let Some(i) = self.modern.as_mut().and_then(|m| m.inspect.as_mut()) {
            let ui = &mut self.host.root;
            if !open && ui.keyboard_focus() == Some(i.scribe.field) {
                ui.set_keyboard_focus(None);
            }
            if let Some(c) = i.scribe.tick(ui, sheet) {
                emit(ui, PanelOutput::Session(SessionCommand::Item(c)));
            }
        }
        let selected = self.player_view.and_then(|v| v.selected);
        let in_combat = self.facts.state.combat_mode != CombatMode::NON_COMBAT;
        let Some(i) = self.modern.as_mut().and_then(|m| m.inspect.as_mut()) else {
            return;
        };
        if !open {
            i.followed = selected;
            return;
        }
        let mut ask = None;
        if let Some(sel) = selected
            && i.followed != Some(sel)
        {
            i.followed = Some(sel);
            ask = Some(ItemCommand::Appraise(ObjectRef::Guid(sel)));
        }
        let creature = matches!(
            i.subject,
            Some(Subject::Object(
                _,
                AppraisalPage::Creature | AppraisalPage::Character
            ))
        );
        if creature && in_combat {
            i.since += dt;
            if i.since >= REFRESH && ask.is_none() {
                i.since = 0.0;
                ask = Some(ItemCommand::RefreshCurrentAppraisal);
            }
        } else {
            i.since = 0.0;
        }
        if let Some(c) = ask {
            emit(
                &mut self.host.root,
                PanelOutput::Session(SessionCommand::Item(c)),
            );
        }
    }
}
