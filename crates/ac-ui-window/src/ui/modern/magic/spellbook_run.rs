//! The Spellbook in the game window, in place of retail's spellbook (and
//! its components page): the magic panel, its keys and the panel strip
//! open it. The school and level filters are the server's (retail sets
//! the same bits); casting goes the bars' way; a double click or Add to
//! bar puts a spell on the first free spell slot; a right click examines
//! it in Inspect; forgetting asks first.

use std::collections::HashMap;

use ac_runtime::gameplay::GameplayCommand;
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::draw::Color;
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::panels::items::emit;
use ac_ui::panels::magic::SpellbookPage;
use ac_ui::panels::spell_components::{ComponentsView, MAX_DESIRED, owned_components};
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::tree::{Element, ElementId};
use ac_ui::widgets::Field;

use super::spellbook::{
    ComponentKind, ComponentRow, SpellRow, component_rows, components_shown, level_bit, with_school,
};
use super::spellbook_window::{
    BookFonts, BookFrame, BookPage, BookRequest, ComponentTable, HEIGHT, LIST_H, LIST_TOP, LIST_W,
    LIST_X, ListRequest, SpellComponent, SpellDetails, SpellList, TABLE_H, TABLE_TOP, TABLE_W,
    TABLE_X, TableRequest, WIDTH, groups_for,
};
use crate::ui::modern::{hud, kit};
use crate::ui::window::WindowUi;

/// The window's name.
pub const SPELLBOOK: &str = "modern-spellbook";

/// How often the book reads the spells and components again while it
/// shows, seconds.
const REREAD: f64 = 0.5;

/// The Spellbook's elements and what it remembers.
#[derive(Debug)]
pub struct BookUi {
    /// The frame (the window).
    pub frame: ElementId,
    /// The search field.
    pub search: ElementId,
    /// The spell list.
    pub list: ElementId,
    /// The components table.
    pub table: ElementId,
    /// The field editing a restock amount.
    pub restock: ElementId,
    /// The component being restocked, and the field had the keyboard.
    editing: Option<(u32, bool)>,
    /// The server's filter bits last seen.
    server_bits: Option<u32>,
    /// Time since the last read.
    since: f64,
    /// The book was showing last frame.
    was_open: bool,
}

impl WindowUi {
    /// The Spellbook, hidden, in place of retail's.
    pub(in crate::ui) fn mount_modern_spellbook(&mut self) {
        let fonts = BookFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            strong: self.modern_font(Weight::SemiBold, 12.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
            body: self.modern_font(Weight::Regular, 12.0),
            name: self.modern_font(Weight::SemiBold, 12.5),
            heading: self.modern_font(Weight::SemiBold, 14.0),
            section: self.modern_font(Weight::SemiBold, 13.0),
            nav: self.modern_font(Weight::Medium, 11.5),
        };
        let field_font = self.modern_font(Weight::Regular, 12.0);
        let frame = match hud::mount_piece(
            &mut self.host.root,
            SPELLBOOK,
            (WIDTH, HEIGHT),
            BookFrame::new(Horizon::BRONZE, fonts.clone()),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern spellbook: {e}"));
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
            BookFrame::search_rect(),
            field_font.clone(),
            &Horizon::BRONZE,
        );
        let list = ui.add(
            frame,
            Element::at(LIST_X, LIST_TOP, LIST_W, LIST_H).with_anchors(AnchorEdges::NONE),
            SpellList::new(Horizon::BRONZE, fonts.clone()),
        );
        let table = ui.add(
            frame,
            Element::at(TABLE_X, TABLE_TOP, TABLE_W, TABLE_H)
                .with_anchors(AnchorEdges::NONE)
                .with_visible(false),
            ComponentTable::new(Horizon::BRONZE, fonts),
        );
        let mut e = Element::at(0.0, 0.0, 60.0, 22.0)
            .with_anchors(AnchorEdges::NONE)
            .with_visible(false);
        Field::configure(&mut e);
        let mut f = Field::default();
        f.font = field_font;
        f.one_line = true;
        f.selectable = true;
        f.padding = 3.0;
        f.right_aligned = true;
        f.record_history = false;
        f.clear_on_submit = false;
        f.select_all_on_focus = true;
        f.max_characters = 4;
        f.character_filter = Some(Box::new(|c: char| c.is_ascii_digit()));
        f.text_color = Horizon::BRONZE.palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = kit::gold(0.35);
        f.set_editable_detached(&mut e, true);
        let restock = ui.add(frame, e, f);
        ui.hide_window(SPELLBOOK);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.spellbook = Some(BookUi {
            frame,
            search,
            list,
            table,
            restock,
            editing: None,
            server_bits: None,
            since: 0.0,
            was_open: false,
        });
        modern.replace(crate::ui::window::SPELLBOOK_WINDOW);
        self.panels.stand_in(panel_id::MAGIC, SPELLBOOK);
    }

    /// A spellbook key: the book opens on `page`, or closes when it shows
    /// it already. None without the modern book.
    pub(in crate::ui) fn modern_open_spellbook(&mut self, page: SpellbookPage) -> Option<bool> {
        let frame = self.modern.as_ref()?.spellbook.as_ref()?.frame;
        let page = match page {
            SpellbookPage::Spells => BookPage::Spells,
            SpellbookPage::Components => BookPage::Components,
        };
        let open = self.host.root.is_window_visible(SPELLBOOK);
        let on_page = self
            .host
            .root
            .widget::<BookFrame>(frame)
            .is_some_and(|f| f.page == page);
        if open && on_page {
            self.panels
                .set_visibility(&mut self.host.root, panel_id::MAGIC, false);
            return Some(true);
        }
        self.show_book_page(page);
        self.panels
            .set_visibility(&mut self.host.root, panel_id::MAGIC, true);
        Some(true)
    }

    fn show_book_page(&mut self, page: BookPage) {
        let Some(b) = self.modern.as_ref().and_then(|m| m.spellbook.as_ref()) else {
            return;
        };
        let (frame, list, table) = (b.frame, b.list, b.table);
        let ui = &mut self.host.root;
        if let Some(f) = ui.widget_mut::<BookFrame>(frame) {
            f.page = page;
        }
        ui.set_visible(list, page == BookPage::Spells);
        ui.set_visible(table, page == BookPage::Components);
    }

    /// Each frame: what the frame and its lists asked for; the spells and
    /// components read again while it shows.
    pub(in crate::ui) fn tick_modern_spellbook(&mut self, dt: f64) {
        let Some(b) = self.modern.as_ref().and_then(|m| m.spellbook.as_ref()) else {
            return;
        };
        let (frame, search, list, table) = (b.frame, b.search, b.list, b.table);
        let open = self.host.root.is_window_visible(SPELLBOOK);
        if !open {
            let ui = &mut self.host.root;
            if ui.keyboard_focus() == Some(search) {
                ui.set_keyboard_focus(None);
            }
            self.finish_restock();
            if let Some(b) = self.modern.as_mut().and_then(|m| m.spellbook.as_mut()) {
                b.was_open = false;
            }
            return;
        }
        let ui = &mut self.host.root;
        let typed = kit::field_text(ui, search);
        let focused = ui.keyboard_focus() == Some(search);
        let (asks, searched) = ui
            .widget_mut::<BookFrame>(frame)
            .map(|f| {
                let changed = f.filter.search != typed;
                f.filter.search = typed;
                f.searching = focused;
                (std::mem::take(&mut f.requests), changed)
            })
            .unwrap_or_default();
        let list_asks = ui
            .widget_mut::<SpellList>(list)
            .map(|l| std::mem::take(&mut l.requests))
            .unwrap_or_default();
        let table_asks = ui
            .widget_mut::<ComponentTable>(table)
            .map(|t| std::mem::take(&mut t.requests))
            .unwrap_or_default();
        let mut regroup = searched;
        let mut reread = false;
        for ask in asks {
            match ask {
                BookRequest::Close => {
                    self.panels
                        .set_visibility(&mut self.host.root, panel_id::MAGIC, false);
                }
                BookRequest::Page(page) => {
                    self.show_book_page(page);
                    reread = true;
                }
                BookRequest::School(school) => {
                    self.set_book_bits(|bits| with_school(bits, school));
                    regroup = true;
                }
                BookRequest::Level(level) => {
                    self.set_book_bits(|bits| bits ^ level_bit(level));
                    regroup = true;
                }
                BookRequest::HighestOnly(_) | BookRequest::Sort | BookRequest::Category(_) => {
                    regroup = true;
                }
                BookRequest::Cast(spell) => {
                    if let Err(words) = self.modern_cast(spell) {
                        self.notices.push(words.to_owned());
                    }
                }
                BookRequest::AddToBar(spell) => {
                    self.modern_bars_favorite(spell);
                    reread = true;
                }
                BookRequest::Forget(spell) => emit(
                    &mut self.host.root,
                    PanelOutput::Session(SessionCommand::Gameplay(GameplayCommand::ForgetSpell(
                        spell.to_string(),
                    ))),
                ),
            }
        }
        for ask in list_asks {
            match ask {
                ListRequest::Choose(_) => reread = true,
                ListRequest::Favourite(spell) => {
                    self.modern_bars_favorite(spell);
                    reread = true;
                }
                ListRequest::Examine(spell) => {
                    emit(&mut self.host.root, PanelOutput::ExamineSpell(spell));
                }
            }
        }
        for ask in table_asks {
            match ask {
                TableRequest::Select(object) => {
                    emit(&mut self.host.root, PanelOutput::SelectObject(object));
                }
                TableRequest::EditRestock(component, top) => self.start_restock(component, top),
            }
        }
        self.follow_restock();
        let Some(b) = self.modern.as_mut().and_then(|m| m.spellbook.as_mut()) else {
            return;
        };
        b.since += dt;
        let first = !b.was_open;
        b.was_open = true;
        if first || reread || b.since >= REREAD {
            b.since = 0.0;
            self.read_modern_spellbook(first);
        } else if regroup {
            self.regroup_modern_spellbook(true);
        }
    }

    /// Changes the filter bits here and asks the server to keep them.
    fn set_book_bits(&mut self, change: impl Fn(u32) -> u32) {
        let Some(frame) = self
            .modern
            .as_ref()
            .and_then(|m| m.spellbook.as_ref())
            .map(|b| b.frame)
        else {
            return;
        };
        let ui = &mut self.host.root;
        let Some(bits) = ui.widget_mut::<BookFrame>(frame).map(|f| {
            f.filter.bits = change(f.filter.bits);
            f.filter.bits
        }) else {
            return;
        };
        emit(
            ui,
            PanelOutput::Session(SessionCommand::Gameplay(
                GameplayCommand::SetSpellbookFilter(bits),
            )),
        );
    }

    /// The spells, components and the chosen spell read again.
    #[allow(clippy::too_many_lines)]
    fn read_modern_spellbook(&mut self, first: bool) {
        let Some(b) = self.modern.as_ref().and_then(|m| m.spellbook.as_ref()) else {
            return;
        };
        let (frame, list) = (b.frame, b.list);
        let seen_bits = b.server_bits;
        let Some(numbers) = self.facts.numbers() else {
            return;
        };
        let Some(book) = numbers.spellbook() else {
            return;
        };
        let bars = self.modern_spell_bars();
        let server_bits = book.spellbook_filters();
        let mut spells = Vec::new();
        for id in book.learned_spells() {
            let Some(m) = book.try_get_metadata(id) else {
                continue;
            };
            spells.push(SpellRow {
                id,
                name: m.name.clone(),
                school: m.school_id,
                level: m.generation,
                mana: m.mana_cost,
                family: m.family,
                sort_key: m.sort_key,
                icon: None,
                bar: bars.get(&id).cloned(),
            });
        }
        for s in &mut spells {
            s.icon = ac_ui::icons::IconSource::spell_icon(&mut self.icons, s.id);
        }
        // The components.
        let descriptors = self
            .components
            .as_ref()
            .map(|t| crate::ui::components_page::component_descriptors(t, &self.component_classes))
            .unwrap_or_default();
        let player = self.facts.state.player;
        let view = ComponentsView {
            objects: &self.facts.objects,
            player,
            desired: book.desired_components(),
            components: &descriptors,
            selected: None,
        };
        let owned = owned_components(&view);
        let carried_by_class: HashMap<u32, i32> =
            owned.iter().map(|c| (c.component, c.quantity)).collect();
        let carried: Vec<ComponentRow> = owned
            .iter()
            .map(|c| ComponentRow {
                component: c.component,
                object: c.object,
                name: c.name.clone(),
                icon: ac_ui::icons::IconSource::component_icon(&mut self.icons, c.icon),
                category: c.category,
                carried: c.quantity,
                restock: 0,
            })
            .collect();
        let kinds: HashMap<u32, ComponentKind> = descriptors
            .iter()
            .map(|(class, d)| {
                (
                    *class,
                    ComponentKind {
                        name: d.name.clone(),
                        category: d.category,
                        icon: ac_ui::icons::IconSource::component_icon(&mut self.icons, d.icon),
                    },
                )
            })
            .collect();
        let restock = book.desired_components().clone();
        let comps = component_rows(carried, &restock, &kinds);
        // The chosen spell.
        let chosen = self
            .host
            .root
            .widget::<SpellList>(list)
            .and_then(|l| l.chosen);
        // A spell no longer known (forgotten) is no longer chosen.
        let known = |id: u32| {
            self.facts
                .numbers()
                .and_then(|n| n.spellbook())
                .is_some_and(|b| b.knows(id))
        };
        let chosen = match chosen {
            Some(id) if !known(id) => {
                if let Some(l) = self.host.root.widget_mut::<SpellList>(list) {
                    l.chosen = None;
                }
                if let Some(f) = self.host.root.widget_mut::<BookFrame>(frame) {
                    f.asking_forget = false;
                }
                None
            }
            other => other,
        };
        let details = chosen.and_then(|id| {
            let numbers = self.facts.numbers()?;
            let spell = numbers.spellbook()?.try_get_metadata(id)?.clone();
            let skill = crate::ui::window::magic_skill(numbers, &spell.school);
            let world = ac_runtime::gameplay::magic::TableComponentWorld {
                objects: &self.facts.objects,
                player,
            };
            let formula = ac_runtime::gameplay::magic::examine_components(
                &self.spell_components,
                &world,
                &self.facts.state.account,
                &spell,
            );
            let mut components: Vec<SpellComponent> = Vec::new();
            for (component, _) in formula {
                let Some((_, base)) = self
                    .components
                    .as_ref()
                    .and_then(|t| t.components.iter().find(|(c, _)| *c == component))
                else {
                    continue;
                };
                let class = self.component_classes.get(&component).copied();
                let carried = class
                    .and_then(|c| carried_by_class.get(&c).copied())
                    .unwrap_or(0);
                match components.iter_mut().find(|c| c.name == base.name) {
                    Some(c) => c.uses += 1,
                    None => components.push(SpellComponent {
                        name: base.name.clone(),
                        uses: 1,
                        carried,
                    }),
                }
            }
            let icon = ac_ui::icons::IconSource::spell_icon(&mut self.icons, id);
            Some(SpellDetails {
                id,
                name: spell.name.clone(),
                icon,
                school: spell.school_id,
                line: super::effects::school_words(&spell),
                mana: spell.mana_cost,
                difficulty: spell.difficulty,
                skill,
                description: spell.description.clone(),
                components,
                spots: self.modern_bar_spots(id),
            })
        });
        let can_cast = self.modern_can_cast();
        let ui = &mut self.host.root;
        if let Some(f) = ui.widget_mut::<BookFrame>(frame) {
            f.spells = spells;
            if seen_bits != Some(server_bits) {
                f.filter.bits = server_bits;
            }
            f.components = comps;
            f.show_details(details);
            f.can_cast = can_cast;
        }
        if let Some(b) = self.modern.as_mut().and_then(|m| m.spellbook.as_mut()) {
            b.server_bits = Some(server_bits);
        }
        self.regroup_modern_spellbook(first);
    }

    /// The list and the table follow the frame's filters.
    fn regroup_modern_spellbook(&mut self, to_top: bool) {
        let Some(b) = self.modern.as_ref().and_then(|m| m.spellbook.as_ref()) else {
            return;
        };
        let (frame, list, table) = (b.frame, b.list, b.table);
        let ui = &mut self.host.root;
        let Some((groups, rows)) = ui.widget::<BookFrame>(frame).map(|f| {
            let rows: Vec<ComponentRow> =
                components_shown(&f.components, f.category, &f.filter.search)
                    .into_iter()
                    .cloned()
                    .collect();
            (groups_for(f), rows)
        }) else {
            return;
        };
        if let Some(l) = ui.widget_mut::<SpellList>(list) {
            let changed = l.groups != groups;
            l.groups = groups;
            if to_top && changed {
                l.to_top();
            }
            if to_top {
                l.reveal_chosen();
            }
        }
        if let Some(t) = ui.widget_mut::<ComponentTable>(table) {
            t.rows = rows;
        }
    }

    /// The restock field over the clicked row's box, holding its amount.
    fn start_restock(&mut self, component: u32, top: f32) {
        self.finish_restock();
        let Some(b) = self.modern.as_mut().and_then(|m| m.spellbook.as_mut()) else {
            return;
        };
        let (frame, field) = (b.frame, b.restock);
        b.editing = Some((component, false));
        let ui = &mut self.host.root;
        let amount = ui
            .widget::<BookFrame>(frame)
            .and_then(|f| f.components.iter().find(|c| c.component == component))
            .map_or(0, |c| c.restock);
        let r = ComponentTable::restock_rect(top);
        if let Some(e) = ui.element_mut(field) {
            e.left = TABLE_X + r.0 + 2.0;
            e.top = TABLE_TOP + r.1 + 1.0;
            e.width = r.2 - 6.0;
            e.height = r.3 - 2.0;
        }
        if let Some(f) = ui.widget_mut::<Field>(field) {
            f.set_text(&if amount == 0 {
                String::new()
            } else {
                amount.to_string()
            });
        }
        ui.set_visible(field, true);
        ui.bring_to_front(field);
        ui.set_keyboard_focus(Some(field));
    }

    /// While the restock field is written in: it is finished once it loses
    /// the keyboard (Enter, a click elsewhere, the table scrolled).
    fn follow_restock(&mut self) {
        let Some(b) = self.modern.as_mut().and_then(|m| m.spellbook.as_mut()) else {
            return;
        };
        let Some((component, had)) = b.editing else {
            return;
        };
        let (field, table) = (b.restock, b.table);
        let ui = &self.host.root;
        let focused = ui.keyboard_focus() == Some(field);
        let scrolled_away = ui
            .widget::<ComponentTable>(table)
            .is_some_and(|t| t.editing != Some(component));
        if (had && !focused) || scrolled_away {
            self.finish_restock();
        } else if let Some(b) = self.modern.as_mut().and_then(|m| m.spellbook.as_mut()) {
            b.editing = Some((component, had || focused));
        }
    }

    /// The amount written goes to the server; the field hides.
    fn finish_restock(&mut self) {
        let Some(b) = self.modern.as_mut().and_then(|m| m.spellbook.as_mut()) else {
            return;
        };
        let Some((component, _)) = b.editing.take() else {
            return;
        };
        let (frame, field, table) = (b.frame, b.restock, b.table);
        let ui = &mut self.host.root;
        let typed = ui
            .widget::<Field>(field)
            .map(Field::text)
            .unwrap_or_default();
        if ui.keyboard_focus() == Some(field) {
            ui.set_keyboard_focus(None);
        }
        ui.set_visible(field, false);
        if let Some(t) = ui.widget_mut::<ComponentTable>(table) {
            t.editing = None;
        }
        let amount = typed.trim().parse::<u32>().unwrap_or(0).min(MAX_DESIRED);
        let before = ui
            .widget::<BookFrame>(frame)
            .and_then(|f| f.components.iter().find(|c| c.component == component))
            .map_or(0, |c| c.restock);
        if amount == before {
            return;
        }
        if let Some(f) = ui.widget_mut::<BookFrame>(frame)
            && let Some(c) = f.components.iter_mut().find(|c| c.component == component)
        {
            c.restock = amount;
        }
        emit(
            ui,
            PanelOutput::Session(SessionCommand::Gameplay(
                GameplayCommand::SetDesiredComponent { component, amount },
            )),
        );
        self.regroup_modern_spellbook(false);
    }
}
