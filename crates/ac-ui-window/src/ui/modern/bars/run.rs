//! The bars in the game window: which layout the character uses (loaded,
//! made from its retail bars the first time, saved after each change),
//! the elements built for it and kept in place, what each slot shows, and
//! what a click, a key, a drop or an edit does.

use std::collections::HashSet;

use ac_core::combat::CombatMode;
use ac_host::settings::SavedBars;
use ac_runtime::gameplay::GameplayCommand;
use ac_runtime::inventory::{InteractionMode, ItemCommand, ObjectRef};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::geometry::AnchorEdges;
use ac_ui::input::{ActivationType, InputAction, KeyBindings, KeyChord};
use ac_ui::modern::Horizon;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::{emit, offer_target};
use ac_ui::tree::{Element, ElementId};
use ac_ui::{Key, Modifiers, MouseButton};

use super::edit::{
    Dim, DragHint, EditToolbar, HINT_HEIGHT, KeyPopover, Menu, MenuAction, MenuFor, MenuRow,
    POPOVER_HEIGHT, POPOVER_WIDTH, PopoverRequest, RenameBox, TOOLBAR_HEIGHT, ToolbarPart,
};
use super::keys;
use super::model::{Bar, BarLayout, BarPlace, KeyMode, RetailBars, SLOT, SlotContent, SlotRef};
use super::widgets::{
    BarFonts, BarFrame, BarTab, Dropped, SlotCell, SlotRequest, SlotView, TAB_GAP, TAB_HEIGHT,
    TabPart, TabRequest,
};
use crate::ui::modern::hud;
use crate::ui::modern::places::{self, Anchor};
use crate::ui::window::WindowUi;

/// How long to wait for the character's retail lists before making its
/// first bars without them, seconds.
const WAIT_FOR_LISTS: f64 = 4.0;
/// How long after a change it is saved, seconds (while no button is held).
const SAVE_AFTER: f64 = 0.5;
/// How far down the edit toolbar sits.
const TOOLBAR_TOP: f32 = 92.0;
/// How long Reset waits for its second press, seconds.
const RESET_ARMED: f64 = 4.0;

/// The sizes of the bars' words, pixels to the em.
const KEY_SIZE: f32 = 9.5;
const NAME_SIZE: f32 = 11.0;
const NOTE_SIZE: f32 = 10.5;
const TITLE_SIZE: f32 = 13.0;
const BODY_SIZE: f32 = 12.0;
const SMALL_SIZE: f32 = 11.0;

/// One bar's elements.
#[derive(Debug)]
struct Built {
    frame: ElementId,
    tab: ElementId,
    slots: Vec<ElementId>,
    /// Where the bar was last put (a move from there is the player's).
    placed: (f32, f32),
}

/// The bars of the game window.
#[derive(Debug)]
pub struct BarsUi {
    fonts: BarFonts,
    /// Every character's saved bars, once read.
    saved: Option<SavedBars>,
    /// The character playing, as saved.
    character: Option<String>,
    /// Its name alone.
    character_name: String,
    /// When it came into the world without bars of its own.
    seed_since: Option<f64>,
    /// The layout in use (none while it is being waited for).
    layout: Option<BarLayout>,
    /// The layout is the one the characters share.
    shared: bool,
    built: Vec<Built>,
    /// The shape the elements were built for: each bar's slots and
    /// direction.
    built_shape: Vec<(usize, bool)>,
    /// Edit layout is on.
    edit: bool,
    dim: ElementId,
    toolbar: ElementId,
    popover: ElementId,
    menu: ElementId,
    rename: ElementId,
    /// The slot whose keys the pop-up sets.
    popover_for: Option<SlotRef>,
    /// When Reset was pressed once.
    reset_armed_at: Option<f64>,
    /// When the layout changed and is not yet saved.
    dirty_since: Option<f64>,
    /// The key map, to say when a slot's key also does something there.
    bindings: Option<KeyBindings>,
    /// The bars were shown last frame.
    shown: bool,
    /// The line shown while a spell is carried out of the spellbook.
    hint: ElementId,
    /// A spell is being carried out of the spellbook: the book's opacity
    /// before it faded.
    spell_drag: Option<f32>,
}

impl BarsUi {
    /// How a character is saved: its world and its name.
    fn key(world: Option<&str>, name: &str) -> String {
        format!("{}/{name}", world.unwrap_or(""))
    }

    fn mark_dirty(&mut self, now: f64) {
        self.dirty_since.get_or_insert(now);
    }

    /// The layout in use, into the saved form.
    fn store_layout(&mut self) {
        let (Some(saved), Some(character), Some(layout)) =
            (self.saved.as_mut(), self.character.as_deref(), &self.layout)
        else {
            return;
        };
        let data = layout.to_data();
        let entry = saved.character_mut(character);
        entry.use_shared = self.shared;
        if self.shared {
            saved.shared = Some(data);
        } else {
            entry.own = Some(data);
        }
    }

    /// The layout a character uses, and whether it is the shared one;
    /// none when it has none yet.
    fn layout_of(&self, character: &str) -> Option<(BarLayout, bool)> {
        let saved = self.saved.as_ref()?;
        let entry = saved.character(character)?;
        if entry.use_shared
            && let Some(shared) = &saved.shared
        {
            return Some((BarLayout::from_data(shared), true));
        }
        entry
            .own
            .as_ref()
            .map(|own| (BarLayout::from_data(own), false))
    }
}

/// The place a bar of the first layout named `name` has, if any.
fn default_place_of(name: &str) -> BarPlace {
    let seeded = BarLayout::seeded(&RetailBars::default(), &|_| String::new(), &|_| {
        String::new()
    });
    seeded.bars.iter().find(|b| b.name == name).map_or(
        BarPlace {
            anchor: Anchor::Center,
            dx: 0.0,
            dy: 0.0,
        },
        |b| b.place,
    )
}

/// The keys of `bar` in `mode` in a few words ("1 to =").
fn key_run(bar: &Bar, mode: KeyMode) -> Option<String> {
    let keys: Vec<KeyChord> = bar.slots.iter().filter_map(|s| s.key(mode)).collect();
    let first = keys.first()?;
    let last = keys.last()?;
    Some(if keys.len() == 1 {
        keys::words(*first)
    } else {
        format!("{} to {}", keys::words(*first), keys::words(*last))
    })
}

/// What a bar's tab says of its keys.
fn key_note(bar: &Bar) -> String {
    let normal = key_run(bar, KeyMode::Normal);
    let magic = key_run(bar, KeyMode::Magic);
    match (normal, magic) {
        (Some(n), Some(m)) if n == m => format!("keys {n}"),
        (Some(n), Some(m)) => format!("keys {n} \u{b7} Magic {m}"),
        (Some(n), None) => format!("keys {n}"),
        (None, Some(m)) => format!("Magic: {m}"),
        (None, None) => String::new(),
    }
}

/// An action of the key map in words ("MovementForward": "Movement
/// Forward").
fn action_words(action: InputAction) -> String {
    let name = action.name();
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c == '_' {
            out.push(' ');
            continue;
        }
        if i > 0 && c.is_ascii_uppercase() && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// A key map action a slot's key is expected to stand in for.
fn is_slot_action(action: InputAction) -> bool {
    let name = action.name();
    name.starts_with("UseQuickSlot")
        || name.starts_with("SelectQuickSlot")
        || name.starts_with("UseSpellSlot")
}

impl WindowUi {
    /// Runs `f` with the bars taken out of the layer (so both can be
    /// borrowed), then puts them back.
    fn with_bars<R>(&mut self, f: impl FnOnce(&mut Self, &mut BarsUi) -> R) -> Option<R> {
        let mut bars = self.modern.as_mut()?.bars.take()?;
        let out = f(self, &mut bars);
        if let Some(modern) = self.modern.as_mut() {
            modern.bars = Some(bars);
        }
        Some(out)
    }

    /// The bars: the edit layer's pieces, hidden; the bars themselves are
    /// built once the character's layout is known. They stand in for the
    /// retail toolbar and the combat window's spell bars.
    pub(in crate::ui) fn mount_modern_bars(&mut self) {
        let fonts = BarFonts {
            key: self.modern_font(Weight::SemiBold, KEY_SIZE),
            name: self.modern_font(Weight::SemiBold, NAME_SIZE),
            note: self.modern_font(Weight::Regular, NOTE_SIZE),
            title: self.modern_font(Weight::SemiBold, TITLE_SIZE),
            body: self.modern_font(Weight::Medium, BODY_SIZE),
            small: self.modern_font(Weight::Regular, SMALL_SIZE),
        };
        let theme = Horizon::BRONZE;
        let ui = &mut self.host.root;
        let root = ui.root();
        let (sw, sh) = ui.size();
        let hidden = |x: f32, y: f32, w: f32, h: f32| {
            Element::at(x, y, w, h)
                .with_visible(false)
                .with_anchors(AnchorEdges::NONE)
        };
        let dim = ui.add(root, hidden(0.0, 0.0, sw, sh), Dim);
        let toolbar = ui.add(
            root,
            hidden(0.0, TOOLBAR_TOP, 600.0, TOOLBAR_HEIGHT),
            EditToolbar::new(theme, fonts.clone()),
        );
        let popover = ui.add(
            root,
            hidden(0.0, 0.0, POPOVER_WIDTH, POPOVER_HEIGHT),
            KeyPopover::new(theme, fonts.clone()),
        );
        let menu = ui.add(
            root,
            hidden(0.0, 0.0, super::edit::MENU_WIDTH, 100.0),
            Menu::new(theme, fonts.clone()),
        );
        let rename = RenameBox::mount(ui, theme, fonts.clone());
        let hint = ui.add(
            root,
            hidden(0.0, TOOLBAR_TOP, 400.0, HINT_HEIGHT),
            DragHint::new(theme, fonts.body.clone()),
        );
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.bars = Some(BarsUi {
            fonts,
            saved: None,
            character: None,
            character_name: String::new(),
            seed_since: None,
            layout: None,
            shared: false,
            built: Vec::new(),
            built_shape: Vec::new(),
            edit: false,
            dim,
            toolbar,
            popover,
            menu,
            rename,
            popover_for: None,
            reset_armed_at: None,
            dirty_since: None,
            bindings: None,
            shown: false,
            hint,
            spell_drag: None,
        });
        modern.replace(crate::ui::window::TOOLBAR_WINDOW);
        modern.replace(crate::ui::window::COMBAT_WINDOW);
    }

    /// The key map, for the key pop-up's notes.
    pub(in crate::ui) fn feed_modern_bar_bindings(&mut self, bindings: &KeyBindings) {
        if let Some(b) = self.modern.as_mut().and_then(|m| m.bars.as_mut()) {
            b.bindings = Some(bindings.clone());
        }
    }

    /// One frame of the bars.
    pub(in crate::ui) fn tick_modern_bars(&mut self) {
        self.with_bars(|w, b| w.run_bars(b));
    }

    fn run_bars(&mut self, b: &mut BarsUi) {
        let now = self.status.clock.elapsed().as_secs_f64();
        let in_world = self.player_view.is_some() && !self.pregame_active();
        let name = self.status.playing.clone().filter(|_| in_world);
        if b.saved.is_none() && name.is_some() {
            // Without a settings file the bars last the session.
            b.saved = Some(
                self.status
                    .settings_store
                    .as_ref()
                    .map(ac_host::settings::SettingsStore::load_modern_bars)
                    .unwrap_or_default(),
            );
        }
        let who = name
            .as_deref()
            .map(|n| BarsUi::key(self.status.server_world.as_deref(), n));
        if who != b.character {
            self.save_bars(b, true);
            if b.edit {
                self.leave_bar_edit(b);
            }
            b.character.clone_from(&who);
            b.character_name = name.unwrap_or_default();
            b.layout = None;
            b.shared = false;
            b.seed_since = None;
            if let Some(character) = &who {
                match b.layout_of(character) {
                    Some((layout, shared)) => {
                        b.layout = Some(layout);
                        b.shared = shared;
                    }
                    None => b.seed_since = Some(now),
                }
            }
        }
        if b.layout.is_none()
            && let Some(since) = b.seed_since
        {
            let lists = self.facts.state.shortcuts.revision() > 0
                && self.facts.numbers().and_then(|n| n.spellbook()).is_some();
            if lists || now - since >= WAIT_FOR_LISTS {
                b.layout = Some(self.seeded_layout());
                b.seed_since = None;
                b.mark_dirty(now);
            }
        }
        let show = in_world && b.layout.is_some();
        if !show {
            if b.shown {
                for built in &b.built {
                    self.host.root.set_visible(built.frame, false);
                    self.host.root.set_visible(built.tab, false);
                }
                b.shown = false;
            }
            return;
        }
        self.follow_spell_drag(b);
        self.bar_requests(b, now);
        // A slot taken off (fewer slots, a bar deleted) takes its key
        // pop-up with it.
        if let Some(at) = b.popover_for
            && b.layout.as_ref().and_then(|l| l.slot(at)).is_none()
        {
            self.close_key_popover(b);
        }
        self.build_bars(b);
        self.place_bars(b, now);
        self.feed_bar_slots(b);
        self.place_edit_layer(b, now);
        b.shown = true;
        if b.dirty_since.is_some_and(|t| now - t >= SAVE_AFTER)
            && !self.host.root.button_down(MouseButton::Left)
        {
            self.save_bars(b, false);
        }
    }

    /// A spell carried out of the spellbook: the book fades to 12% and the
    /// bars come in front of it (so a slot behind it takes the drop), with
    /// a line saying where to let go; when the drag ends the book and the
    /// bars go back.
    fn follow_spell_drag(&mut self, b: &mut BarsUi) {
        use ac_ui::panels::magic::{SpellFavoritePayload, SpellbookShortcutPayload};
        const FADED: f32 = 0.12;
        let book = crate::ui::window::SPELLBOOK_WINDOW;
        let ui = &mut self.host.root;
        let carrying = ui.drag_payload().is_some_and(|p| {
            p.get::<SpellbookShortcutPayload>().is_some()
                || p.get::<SpellFavoritePayload>().is_some()
        });
        match (carrying, b.spell_drag) {
            (true, None) => {
                let before = ui
                    .window(book)
                    .and_then(|w| ui.element(w.frame))
                    .map_or(1.0, |e| e.opacity);
                ui.set_window_opacity(book, FADED);
                b.spell_drag = Some(before);
                for built in &b.built {
                    ui.bring_to_front(built.frame);
                }
                let screen = ui.size();
                let width = ui.widget::<DragHint>(b.hint).map_or(400.0, DragHint::width);
                if let Some(e) = ui.element_mut(b.hint) {
                    e.width = width;
                    e.left = ((screen.0 - width) * 0.5).round().max(0.0);
                    e.top = TOOLBAR_TOP;
                }
                ui.set_visible(b.hint, true);
                ui.bring_to_front(b.hint);
            }
            (false, Some(before)) => {
                ui.set_window_opacity(book, before);
                b.spell_drag = None;
                ui.set_visible(b.hint, false);
                // Back under the windows, unless Edit layout holds them up.
                if !b.edit {
                    for built in &b.built {
                        ui.set_z_order(built.frame, 0);
                    }
                }
            }
            _ => {}
        }
    }

    /// The first layout of the character playing, from its retail lists.
    fn seeded_layout(&self) -> BarLayout {
        let book = self.facts.numbers().and_then(|n| n.spellbook());
        let retail = RetailBars {
            shortcuts: self.facts.state.shortcuts.snapshot().to_vec(),
            favorites: (0..8)
                .map(|t| {
                    book.map(|b| b.get_favorites(t).to_vec())
                        .unwrap_or_default()
                })
                .collect(),
        };
        let objects = &self.facts.objects;
        BarLayout::seeded(
            &retail,
            &|id| objects.get(id).map(|o| o.name.clone()).unwrap_or_default(),
            &|id| {
                book.and_then(|b| b.try_get_metadata(id))
                    .map(|m| m.name.clone())
                    .unwrap_or_default()
            },
        )
    }

    /// Writes the bars out: when they changed (or `now_or_never` and they
    /// changed at all).
    fn save_bars(&mut self, b: &mut BarsUi, _now_or_never: bool) {
        if b.dirty_since.take().is_none() {
            return;
        }
        b.store_layout();
        if let (Some(store), Some(saved)) = (&self.status.settings_store, &b.saved)
            && let Err(e) = store.save_modern_bars(saved)
        {
            self.notices.push(format!("ui: saving your bars: {e}"));
        }
    }

    /// The elements follow the layout's shape: built again when a bar
    /// comes or goes, or gains, loses or turns its slots.
    fn build_bars(&mut self, b: &mut BarsUi) {
        let Some(layout) = &b.layout else {
            return;
        };
        let shape: Vec<(usize, bool)> = layout
            .bars
            .iter()
            .map(|bar| (bar.slots.len(), bar.vertical))
            .collect();
        if shape == b.built_shape && b.built.len() == layout.bars.len() {
            return;
        }
        let theme = Horizon::BRONZE;
        let ui = &mut self.host.root;
        for built in b.built.drain(..) {
            ui.destroy(built.frame);
            ui.destroy(built.tab);
        }
        let root = ui.root();
        let screen = ui.size();
        for (i, bar) in layout.bars.iter().enumerate() {
            let size = bar.size();
            let (x, y) = bar.place.top_left(size, screen);
            let mut element = Element::at(x, y, size.0, size.1).with_anchors(AnchorEdges::NONE);
            element.name = Some(format!("{} bar", bar.name));
            element.quick_move = !bar.locked;
            let frame = ui.add(root, element, BarFrame::new(theme, bar.name.clone()));
            let slots = (0..bar.slots.len())
                .map(|j| {
                    let (sx, sy) = bar.slot_origin(j);
                    ui.add(
                        frame,
                        Element::at(sx, sy, SLOT, SLOT).with_anchors(AnchorEdges::NONE),
                        SlotCell::new(theme, b.fonts.key.clone(), SlotRef::new(i, j)),
                    )
                })
                .collect();
            let tab = ui.add(
                root,
                Element::at(0.0, 0.0, 100.0, TAB_HEIGHT)
                    .with_visible(false)
                    .with_anchors(AnchorEdges::NONE),
                BarTab::new(theme, b.fonts.clone(), i),
            );
            b.built.push(Built {
                frame,
                tab,
                slots,
                placed: (x, y),
            });
        }
        // While editing (or carrying a spell) the bars stay over the dim and
        // the windows; the edit layer stays over the bars.
        if b.edit || b.spell_drag.is_some() {
            for built in &b.built {
                ui.bring_to_front(built.frame);
                ui.bring_to_front(built.tab);
            }
        }
        for id in [b.toolbar, b.popover, b.menu, b.rename, b.hint] {
            ui.bring_to_front(id);
        }
        b.built_shape = shape;
    }

    /// Each bar where its place puts it; a bar the player moved (quick
    /// move, its tab) takes a new place from where it was left.
    fn place_bars(&mut self, b: &mut BarsUi, now: f64) {
        let Some(layout) = b.layout.as_mut() else {
            return;
        };
        let ui = &mut self.host.root;
        let screen = ui.size();
        let mut moved = false;
        for (bar, built) in layout.bars.iter_mut().zip(b.built.iter_mut()) {
            let size = bar.size();
            let Some((ex, ey)) = ui.element(built.frame).map(|e| (e.left, e.top)) else {
                continue;
            };
            if (ex, ey) != built.placed {
                bar.place = BarPlace::at(ex, ey, size, screen);
                moved = true;
            }
            let (x, y) = bar.place.top_left(size, screen);
            built.placed = (x, y);
            let visible = !bar.hidden || b.edit;
            if let Some(e) = ui.element_mut(built.frame) {
                e.left = x;
                e.top = y;
                e.quick_move = !bar.locked;
            }
            if ui.element(built.frame).is_some_and(|e| e.visible()) != visible {
                ui.set_visible(built.frame, visible);
            }
            if let Some(f) = ui.widget_mut::<BarFrame>(built.frame) {
                f.edit = b.edit;
                f.hidden = bar.hidden;
                if f.name != bar.name {
                    f.name.clone_from(&bar.name);
                }
            }
            if let Some(e) = ui.element_mut(built.frame) {
                let label = format!("{} bar", bar.name);
                if e.name.as_deref() != Some(label.as_str()) {
                    e.name = Some(label);
                }
            }
        }
        if moved {
            b.mark_dirty(now);
        }
    }

    /// What each slot shows: its icon, its key in the mode in use, greyed
    /// spells outside Magic, a cooldown's sweep.
    fn feed_bar_slots(&mut self, b: &mut BarsUi) {
        let Some(layout) = &b.layout else {
            return;
        };
        let magic = self.facts.state.combat_mode == CombatMode::MAGIC;
        let mode = if magic {
            KeyMode::Magic
        } else {
            KeyMode::Normal
        };
        let player = self.facts.state.player;
        let now = self.facts.session_now();
        let Self {
            host, facts, icons, ..
        } = self;
        let objects = &facts.objects;
        let mut book = facts
            .player
            .as_mut()
            .and_then(|p| p.numbers.spellbook_mut());
        for (i, (bar, built)) in layout.bars.iter().zip(&b.built).enumerate() {
            for (j, (slot, &cell)) in bar.slots.iter().zip(&built.slots).enumerate() {
                let at = SlotRef::new(i, j);
                let key = slot.key(mode);
                let mut view = SlotView {
                    key,
                    ..SlotView::default()
                };
                match &slot.content {
                    Some(SlotContent::Item { id, name }) => {
                        view.filled = true;
                        let item = objects
                            .get(*id)
                            .filter(|_| *id == player || objects.is_owned_by_object(*id, player));
                        let shown_name = item.map_or(name.as_str(), |o| o.name.as_str());
                        match item {
                            Some(o) => {
                                view.icon = ac_ui::icons::IconSource::item_icon(
                                    icons,
                                    o.item_type,
                                    o.icon_id,
                                    o.icon_underlay_id,
                                    o.icon_overlay_id,
                                    o.effects,
                                );
                                view.count = (o.stack_size > 1).then_some(o.stack_size);
                                if let (Some(id), Some(duration), Some(now), Some(book)) =
                                    (o.cooldown_id, o.cooldown_duration, now, book.as_deref_mut())
                                {
                                    let check = book.on_cooldown(id, now);
                                    if check.active && duration > 0.0 {
                                        view.cooldown = (check.remaining / duration) as f32;
                                    }
                                }
                            }
                            None => view.missing = true,
                        }
                        let mut tip = shown_name.to_owned();
                        if let Some(k) = key {
                            tip.push_str(&format!(" \u{b7} {}", keys::words(k)));
                        }
                        if view.missing {
                            tip.push_str(" \u{b7} not with you");
                        }
                        view.tip = Some(tip);
                    }
                    Some(SlotContent::Spell { id, name }) => {
                        view.filled = true;
                        view.spell = true;
                        view.off = !magic;
                        view.icon = ac_ui::icons::IconSource::spell_icon(icons, *id);
                        let known = book.as_deref().is_none_or(|bk| bk.knows(*id));
                        view.missing = !known;
                        let shown = book
                            .as_deref()
                            .and_then(|bk| bk.try_get_metadata(*id))
                            .map_or(name.clone(), |m| m.name.clone());
                        let mut tip = shown;
                        if let Some(k) = key {
                            tip.push_str(&format!(" \u{b7} {}", keys::words(k)));
                        }
                        if !known {
                            tip.push_str(" \u{b7} not in your spellbook");
                        } else if !magic {
                            tip.push_str(" \u{b7} Magic mode only");
                        }
                        view.tip = Some(tip);
                    }
                    None => {
                        view.tip = key.map(|k| format!("Empty \u{b7} {}", keys::words(k)));
                    }
                }
                if let Some(c) = host.root.widget_mut::<SlotCell>(cell) {
                    if c.view != view {
                        c.view = view;
                    }
                    if c.content != slot.content {
                        c.content.clone_from(&slot.content);
                    }
                    c.at = at;
                    c.edit = b.edit;
                    c.chosen = b.popover_for == Some(at);
                    c.locked = bar.locked;
                }
            }
        }
    }

    /// Edit layout's pieces where they belong: the dim over the screen,
    /// the toolbar top centre, each bar's tab above it (or under it when
    /// another bar or the screen's top is in the way).
    fn place_edit_layer(&mut self, b: &mut BarsUi, now: f64) {
        if b.reset_armed_at.is_some_and(|t| now - t >= RESET_ARMED) {
            b.reset_armed_at = None;
        }
        let Some(layout) = &b.layout else {
            return;
        };
        let ui = &mut self.host.root;
        let screen = ui.size();
        if b.edit {
            if let Some(e) = ui.element_mut(b.dim) {
                e.width = screen.0;
                e.height = screen.1;
            }
            let layout_words = if b.shared {
                "Shared".to_owned()
            } else {
                format!("{} only", b.character_name)
            };
            let width = ui.widget_mut::<EditToolbar>(b.toolbar).map(|t| {
                t.layout_words = layout_words;
                t.reset_armed = b.reset_armed_at.is_some();
                t.width()
            });
            if let (Some(w), Some(e)) = (width, ui.element_mut(b.toolbar)) {
                e.width = w;
                e.left = ((screen.0 - w) * 0.5).round().max(0.0);
                e.top = TOOLBAR_TOP;
            }
        }
        let rects: Vec<(f32, f32, f32, f32)> = layout
            .bars
            .iter()
            .zip(&b.built)
            .map(|(bar, built)| {
                let (w, h) = bar.size();
                (built.placed.0, built.placed.1, w, h)
            })
            .collect();
        for (i, (bar, built)) in layout.bars.iter().zip(&b.built).enumerate() {
            let visible = b.edit;
            if ui.element(built.tab).is_some_and(|e| e.visible()) != visible {
                ui.set_visible(built.tab, visible);
            }
            if !visible {
                continue;
            }
            let width = ui.widget_mut::<BarTab>(built.tab).map(|t| {
                t.bar = i;
                if t.name != bar.name {
                    t.name.clone_from(&bar.name);
                }
                let note = key_note(bar);
                if t.note != note {
                    t.note = note;
                }
                t.vertical = bar.vertical;
                t.locked = bar.locked;
                t.width()
            });
            let Some(width) = width else {
                continue;
            };
            let (x, y, _, h) = rects[i];
            let above = y - TAB_GAP - TAB_HEIGHT;
            let clashes = |top: f32| {
                rects.iter().enumerate().any(|(j, r)| {
                    j != i
                        && x < r.0 + r.2
                        && x + width > r.0
                        && top < r.1 + r.3
                        && top + TAB_HEIGHT > r.1
                })
            };
            let below = y + h + TAB_GAP;
            let top = if above >= 0.0 && !clashes(above) {
                above
            } else if below + TAB_HEIGHT <= screen.1 && !clashes(below) {
                below
            } else if above >= 0.0 {
                above
            } else {
                below
            };
            let left = x.min(screen.0 - width).max(0.0);
            if let Some(e) = ui.element_mut(built.tab) {
                e.left = left;
                e.top = top;
                e.width = width;
            }
            if let Some(t) = ui.widget_mut::<BarTab>(built.tab) {
                t.bar_offset = (x - left, y - top);
            }
        }
    }

    /// What the bars' pieces asked for, carried out.
    #[allow(clippy::too_many_lines)]
    fn bar_requests(&mut self, b: &mut BarsUi, now: f64) {
        let ui = &mut self.host.root;
        let mut slot_requests = Vec::new();
        let mut caught = false;
        for built in &b.built {
            if let Some(f) = ui.widget_mut::<BarFrame>(built.frame) {
                caught |= std::mem::take(&mut f.caught);
            }
            for &cell in &built.slots {
                if let Some(c) = ui.widget_mut::<SlotCell>(cell)
                    && !c.requests.is_empty()
                {
                    let at = c.at;
                    for r in std::mem::take(&mut c.requests) {
                        slot_requests.push((at, r));
                    }
                }
            }
        }
        let mut tab_requests = Vec::new();
        for built in &b.built {
            if let Some(t) = ui.widget_mut::<BarTab>(built.tab)
                && !t.requests.is_empty()
            {
                let bar = t.bar;
                for r in std::mem::take(&mut t.requests) {
                    tab_requests.push((bar, built.frame, built.tab, r));
                }
            }
        }
        let toolbar = ui
            .widget_mut::<EditToolbar>(b.toolbar)
            .map(|t| std::mem::take(&mut t.pressed))
            .unwrap_or_default();
        let popover = ui
            .widget_mut::<KeyPopover>(b.popover)
            .map(|p| std::mem::take(&mut p.requests))
            .unwrap_or_default();
        let chosen = ui.widget_mut::<Menu>(b.menu).and_then(|m| m.chosen.take());
        let renamed = ui
            .widget_mut::<RenameBox>(b.rename)
            .and_then(|r| r.done.take());
        let nothing = slot_requests.is_empty()
            && tab_requests.is_empty()
            && toolbar.is_empty()
            && popover.is_empty()
            && chosen.is_none()
            && renamed.is_none();
        if nothing {
            return;
        }
        let mut changed = false;
        // Drops first, so a drag that ended on a slot is known to have
        // landed.
        let mut landed: HashSet<SlotRef> = HashSet::new();
        let mut ended = Vec::new();
        for (at, request) in slot_requests {
            match request {
                SlotRequest::Use => self.use_bar_slot(b, at),
                SlotRequest::Inspect => self.inspect_bar_slot(b, at),
                SlotRequest::EditKeys => self.open_key_popover(b, at),
                SlotRequest::Clear => {
                    if let Some(l) = b.layout.as_mut() {
                        l.put(at, None);
                        changed = true;
                    }
                }
                SlotRequest::Dropped(what) => {
                    changed |= self.drop_on_bar_slot(b, at, what, &mut landed);
                }
                SlotRequest::DragEnded => ended.push(at),
            }
        }
        for at in ended {
            // Let go off the bars: it comes off.
            if !landed.contains(&at)
                && !caught
                && let Some(l) = b.layout.as_mut()
                && l.bars.get(at.bar).is_some_and(|bar| !bar.locked)
            {
                l.put(at, None);
                changed = true;
            }
        }
        for (bar, frame, tab, request) in tab_requests {
            match request {
                TabRequest::MoveTo(x, y) => {
                    let ui = &mut self.host.root;
                    let screen = ui.size();
                    if let Some(e) = ui.element_mut(frame) {
                        e.left = x.min(screen.0 - e.width).max(0.0).round();
                        e.top = y.min(screen.1 - e.height).max(0.0).round();
                    }
                }
                TabRequest::Dropped => {}
                TabRequest::Pressed(part) => match part {
                    TabPart::Turn => changed |= self.turn_bar(b, bar),
                    TabPart::Fewer => {
                        if let Some(l) = b.layout.as_mut() {
                            l.remove_last_slot(bar);
                            changed = true;
                        }
                    }
                    TabPart::More => {
                        if let Some(l) = b.layout.as_mut() {
                            l.add_slot(bar);
                            changed = true;
                        }
                    }
                    TabPart::Menu => {
                        let at = self
                            .host
                            .root
                            .element(tab)
                            .map(|e| (e.left + e.width - 30.0, e.top + e.height + 4.0))
                            .unwrap_or_default();
                        self.open_bar_menu(b, bar, at, false);
                    }
                    TabPart::Grip => {}
                },
            }
        }
        for part in toolbar {
            if part != ToolbarPart::Reset {
                b.reset_armed_at = None;
            }
            match part {
                ToolbarPart::Layout => {
                    let at = self
                        .host
                        .root
                        .element(b.toolbar)
                        .map(|e| (e.left + 260.0, e.top + e.height + 4.0))
                        .unwrap_or_default();
                    self.open_layout_menu(b, at);
                }
                ToolbarPart::Pieces => {
                    let at = self
                        .host
                        .root
                        .element(b.toolbar)
                        .map(|e| (e.left + e.width * 0.5, e.top + e.height + 4.0))
                        .unwrap_or_default();
                    self.open_pieces_menu(b, at);
                }
                ToolbarPart::NewBar => {
                    if let Some(l) = b.layout.as_mut() {
                        l.new_bar(BarPlace {
                            anchor: Anchor::Center,
                            dx: 0.0,
                            dy: 0.0,
                        });
                        changed = true;
                    }
                }
                ToolbarPart::Reset => {
                    if b.reset_armed_at.take().is_some() {
                        self.close_key_popover(b);
                        b.layout = Some(self.seeded_layout());
                        changed = true;
                    } else {
                        b.reset_armed_at = Some(now);
                    }
                }
                ToolbarPart::Done => self.leave_bar_edit(b),
            }
        }
        for request in popover {
            match request {
                PopoverRequest::Listen(mode) => self.listen_for_key(b, mode),
                PopoverRequest::ClearKeys => {
                    if let (Some(at), Some(l)) = (b.popover_for, b.layout.as_mut()) {
                        for mode in KeyMode::ALL {
                            l.set_key(at, mode, None);
                        }
                        changed = true;
                        self.refresh_key_popover(b, "Both keys cleared.".to_owned());
                    }
                }
                PopoverRequest::Empty => {
                    if let (Some(at), Some(l)) = (b.popover_for, b.layout.as_mut()) {
                        l.put(at, None);
                        changed = true;
                        self.refresh_key_popover(b, "Emptied; its keys stay.".to_owned());
                    }
                }
                PopoverRequest::Done => self.close_key_popover(b),
            }
        }
        if let Some((about, action)) = chosen {
            changed |= self.bar_menu_chosen(b, about, action);
        }
        if let Some((bar, name)) = renamed
            && let Some(l) = b.layout.as_mut()
            && let Some(bar) = l.bars.get_mut(bar)
        {
            bar.name = name;
            changed = true;
        }
        if changed {
            b.mark_dirty(now);
        }
    }

    /// A slot clicked (or its key pressed): an item is used (or, while a
    /// use or examine waits for a target, offered as it); a spell is cast
    /// in Magic, and does nothing otherwise.
    fn use_bar_slot(&mut self, b: &BarsUi, at: SlotRef) {
        let Some(content) = b
            .layout
            .as_ref()
            .and_then(|l| l.slot(at))
            .and_then(|s| s.content.clone())
        else {
            return;
        };
        let target_mode = self.facts.state.mode != InteractionMode::None;
        let ui = &mut self.host.root;
        match content {
            SlotContent::Item { id, .. } => {
                let player = self.facts.state.player;
                let with_me = self.facts.objects.get(id).is_some()
                    && (id == player || self.facts.objects.is_owned_by_object(id, player));
                if !with_me {
                    return;
                }
                if target_mode {
                    offer_target(ui, ObjectRef::Guid(id));
                } else {
                    emit(
                        ui,
                        PanelOutput::Session(SessionCommand::Item(ItemCommand::Use(
                            ObjectRef::Guid(id),
                        ))),
                    );
                }
            }
            SlotContent::Spell { id, .. } => {
                if self.facts.state.combat_mode == CombatMode::MAGIC && !target_mode {
                    emit(
                        ui,
                        PanelOutput::Session(SessionCommand::Gameplay(GameplayCommand::Cast {
                            spell: id.to_string(),
                            target: None,
                        })),
                    );
                }
            }
        }
    }

    /// A slot right-clicked: what it holds is inspected; an empty one
    /// opens its bar's menu.
    fn inspect_bar_slot(&mut self, b: &mut BarsUi, at: SlotRef) {
        let content = b
            .layout
            .as_ref()
            .and_then(|l| l.slot(at))
            .and_then(|s| s.content.clone());
        let ui = &mut self.host.root;
        match content {
            Some(SlotContent::Item { id, .. }) => emit(
                ui,
                PanelOutput::Session(SessionCommand::Item(ItemCommand::Appraise(
                    ObjectRef::Guid(id),
                ))),
            ),
            Some(SlotContent::Spell { id, .. }) => emit(ui, PanelOutput::ExamineSpell(id)),
            None => {
                let (x, y) = ui.mouse();
                self.open_bar_menu(b, at.bar, (x as f32, y as f32), true);
            }
        }
    }

    /// Something let go on slot `at`; true when the layout changed.
    fn drop_on_bar_slot(
        &mut self,
        b: &mut BarsUi,
        at: SlotRef,
        what: Dropped,
        landed: &mut HashSet<SlotRef>,
    ) -> bool {
        let player = self.facts.state.player;
        let content = match what {
            Dropped::Slot(from) => {
                landed.insert(from);
                if let Some(l) = b.layout.as_mut() {
                    l.swap(from, at);
                    return from != at;
                }
                return false;
            }
            Dropped::Item(id) => {
                let owned = id == player || self.facts.objects.is_owned_by_object(id, player);
                let Some(o) = self.facts.objects.get(id).filter(|_| owned) else {
                    self.notices
                        .push("ui: only what you carry goes on a bar".to_owned());
                    return false;
                };
                SlotContent::Item {
                    id,
                    name: o.name.clone(),
                }
            }
            Dropped::Spell(id) => SlotContent::Spell {
                id,
                name: self
                    .facts
                    .numbers()
                    .and_then(|n| n.spellbook())
                    .and_then(|bk| bk.try_get_metadata(id))
                    .map(|m| m.name.clone())
                    .unwrap_or_default(),
            },
        };
        let Some(l) = b.layout.as_mut() else {
            return false;
        };
        l.put(at, Some(content));
        true
    }

    /// Turns bar `bar`, keeping its top left where it is.
    fn turn_bar(&mut self, b: &mut BarsUi, bar: usize) -> bool {
        let screen = self.host.root.size();
        let Some(l) = b.layout.as_mut() else {
            return false;
        };
        let Some(bar) = l.bars.get_mut(bar) else {
            return false;
        };
        let (x, y) = bar.place.top_left(bar.size(), screen);
        bar.vertical = !bar.vertical;
        bar.place = BarPlace::at(x, y, bar.size(), screen);
        true
    }

    /// Opens the key pop-up for slot `at`, over it (or under it when there
    /// is no room above).
    fn open_key_popover(&mut self, b: &mut BarsUi, at: SlotRef) {
        b.popover_for = Some(at);
        self.refresh_key_popover(
            b,
            "Click a key box, then press the key. Esc stops; Backspace clears it.".to_owned(),
        );
        let Some(cell) = b
            .built
            .get(at.bar)
            .and_then(|built| built.slots.get(at.slot))
        else {
            return;
        };
        let ui = &mut self.host.root;
        let (cx, cy) = ui.screen_position(*cell);
        let screen = ui.size();
        let above = cy - 10.0 - POPOVER_HEIGHT;
        let (top, on_top) = if above >= 0.0 {
            (above, false)
        } else {
            (cy + SLOT + 10.0, true)
        };
        let left = (cx + SLOT * 0.5 - POPOVER_WIDTH * 0.5)
            .min(screen.0 - POPOVER_WIDTH)
            .max(0.0)
            .round();
        if let Some(e) = ui.element_mut(b.popover) {
            e.left = left;
            e.top = top.round();
        }
        if let Some(p) = ui.widget_mut::<KeyPopover>(b.popover) {
            p.arrow = (cx + SLOT * 0.5 - left, on_top);
            p.listening = None;
        }
        ui.set_visible(b.popover, true);
        ui.bring_to_front(b.popover);
    }

    /// The pop-up shows its slot as it is now, with `note`.
    fn refresh_key_popover(&mut self, b: &BarsUi, note: String) {
        let (Some(at), Some(l)) = (b.popover_for, b.layout.as_ref()) else {
            return;
        };
        let Some(slot) = l.slot(at) else {
            return;
        };
        let bar = &l.bars[at.bar];
        let title = slot
            .content
            .as_ref()
            .map_or("Empty slot".to_owned(), |c| c.name().to_owned());
        let title = if title.is_empty() {
            "Something no longer known".to_owned()
        } else {
            title
        };
        if let Some(p) = self.host.root.widget_mut::<KeyPopover>(b.popover) {
            p.title = title;
            p.where_words = format!("{} bar, slot {}", bar.name, at.slot + 1);
            p.keys = slot.keys;
            p.filled = slot.content.is_some();
            p.note = note;
        }
    }

    fn close_key_popover(&mut self, b: &mut BarsUi) {
        b.popover_for = None;
        let ui = &mut self.host.root;
        if let Some(p) = ui.widget_mut::<KeyPopover>(b.popover) {
            p.listening = None;
        }
        ui.set_visible(b.popover, false);
    }

    fn listen_for_key(&mut self, b: &mut BarsUi, mode: Option<KeyMode>) {
        if let Some(p) = self.host.root.widget_mut::<KeyPopover>(b.popover) {
            p.listening = mode;
        }
        let note = match mode {
            Some(KeyMode::Normal) => "Press the key for Peace, melee and missile.",
            Some(KeyMode::Magic) => "Press the key for Magic.",
            None => "Click a key box, then press the key.",
        };
        self.refresh_key_popover(b, format!("{note} Esc stops; Backspace clears it."));
    }

    /// The key pop-up is waiting for a key.
    fn bars_listening(&self) -> Option<KeyMode> {
        let b = self.modern.as_ref()?.bars.as_ref()?;
        b.popover_for?;
        self.host
            .root
            .widget::<KeyPopover>(b.popover)
            .and_then(|p| p.listening)
    }

    /// A key, before the interface has it: the key pop-up takes it while it
    /// listens, and Escape leaves the pop-up or Edit layout. True when
    /// taken.
    pub(in crate::ui) fn modern_bars_key_first(
        &mut self,
        key: Key,
        modifiers: Modifiers,
        pressed: bool,
    ) -> bool {
        let listening = self.bars_listening();
        let edit = self
            .modern
            .as_ref()
            .and_then(|m| m.bars.as_ref())
            .is_some_and(|b| b.edit);
        if listening.is_none() && !(edit && key == Key::Escape) {
            return false;
        }
        if !pressed {
            return listening.is_some();
        }
        self.with_bars(|w, b| {
            let now = w.status.clock.elapsed().as_secs_f64();
            match listening {
                Some(mode) => {
                    if key == Key::Escape {
                        w.listen_for_key(b, None);
                        return true;
                    }
                    let Some(at) = b.popover_for else {
                        return true;
                    };
                    if matches!(key, Key::Backspace | Key::Delete) && modifiers == Modifiers::NONE {
                        if let Some(l) = b.layout.as_mut() {
                            l.set_key(at, mode, None);
                        }
                        b.mark_dirty(now);
                        if let Some(p) = w.host.root.widget_mut::<KeyPopover>(b.popover) {
                            p.listening = None;
                        }
                        w.refresh_key_popover(b, "Key cleared.".to_owned());
                        return true;
                    }
                    let Some(chord) = keys::slot_chord(key, modifiers) else {
                        // A modifier on its own: wait for the key with it.
                        return true;
                    };
                    let note = w.assign_bar_key(b, at, mode, chord);
                    b.mark_dirty(now);
                    if let Some(p) = w.host.root.widget_mut::<KeyPopover>(b.popover) {
                        p.listening = None;
                    }
                    w.refresh_key_popover(b, note);
                    true
                }
                None => {
                    if b.popover_for.is_some() {
                        w.close_key_popover(b);
                    } else {
                        w.leave_bar_edit(b);
                    }
                    true
                }
            }
        })
        .unwrap_or(false)
    }

    /// Gives slot `at` the key `chord` in `mode`; what that did, in words.
    fn assign_bar_key(
        &mut self,
        b: &mut BarsUi,
        at: SlotRef,
        mode: KeyMode,
        chord: KeyChord,
    ) -> String {
        let Some(l) = b.layout.as_mut() else {
            return String::new();
        };
        let old = l.slot(at).and_then(|s| s.key(mode));
        let clash = l.clash(chord, mode, at);
        l.set_key(at, mode, Some(chord));
        let words = keys::words(chord);
        let mut note = match clash {
            Some(other) => {
                let bar = &l.bars[other.bar].name;
                let given = old.map_or("no key".to_owned(), keys::words);
                format!(
                    "{words} was {bar} bar slot {}'s; that slot has {given} now.",
                    other.slot + 1
                )
            }
            None => format!("{words} set."),
        };
        if let Some(binding) = b
            .bindings
            .as_ref()
            .and_then(|k| k.find(chord, ActivationType::Press))
            && !is_slot_action(binding.action)
        {
            note.push_str(&format!(
                " It is also {} in the key map; the bar takes it first.",
                action_words(binding.action)
            ));
        }
        note
    }

    /// A key the interface left for the world: a slot with that key in the
    /// mode in use is used. True when one was.
    pub(in crate::ui) fn modern_bars_key(&mut self, key: Key, modifiers: Modifiers) -> bool {
        let magic = self.facts.state.combat_mode == CombatMode::MAGIC;
        let mode = if magic {
            KeyMode::Magic
        } else {
            KeyMode::Normal
        };
        let Some(chord) = keys::slot_chord(key, modifiers) else {
            return false;
        };
        self.with_bars(|w, b| {
            if b.edit || !b.shown {
                return false;
            }
            let Some(at) = b.layout.as_ref().and_then(|l| l.slot_for_key(chord, mode)) else {
                return false;
            };
            w.use_bar_slot(b, at);
            true
        })
        .unwrap_or(false)
    }

    /// The retail toolbar's and spell bars' keys with the bars up: New
    /// shortcut puts the selection on the first empty slot; the slot keys
    /// of the retail bars (which are hidden) do nothing. None without the
    /// bars.
    pub(in crate::ui) fn modern_bars_hotkey(
        &mut self,
        hotkey: crate::ui::window::Hotkey,
    ) -> Option<bool> {
        use crate::ui::window::Hotkey;
        use ac_ui::panels::toolbar::ToolbarKey;
        self.modern.as_ref()?.bars.as_ref()?;
        match hotkey {
            Hotkey::Toolbar(ToolbarKey::CreateShortcut) => {
                let selected = self.player_view.and_then(|v| v.selected);
                Some(selected.is_some_and(|item| self.put_on_first_empty_item(item)))
            }
            Hotkey::Toolbar(_) | Hotkey::Spell(_) => Some(true),
            _ => None,
        }
    }

    fn put_on_first_empty_item(&mut self, item: u32) -> bool {
        let player = self.facts.state.player;
        if item == 0 || !self.facts.objects.is_owned_by_object(item, player) {
            return false;
        }
        let name = self
            .facts
            .objects
            .get(item)
            .map(|o| o.name.clone())
            .unwrap_or_default();
        self.with_bars(|w, b| {
            let now = w.status.clock.elapsed().as_secs_f64();
            let content = SlotContent::Item { id: item, name };
            let Some(l) = b.layout.as_mut() else {
                return false;
            };
            if l.holds(&content) {
                return true;
            }
            let Some(at) = l.first_empty(false) else {
                w.notices.push("ui: your bars are full".to_owned());
                return false;
            };
            l.put(at, Some(content));
            b.mark_dirty(now);
            true
        })
        .unwrap_or(false)
    }

    /// The spellbook asked for `spell` on a bar (a double click): it goes
    /// on the first empty slot of a spell bar, unless a bar has it. False
    /// without the bars.
    pub(in crate::ui) fn modern_bars_favorite(&mut self, spell: u32) -> bool {
        let name = self
            .facts
            .numbers()
            .and_then(|n| n.spellbook())
            .and_then(|bk| bk.try_get_metadata(spell))
            .map(|m| m.name.clone())
            .unwrap_or_default();
        self.with_bars(|w, b| {
            let now = w.status.clock.elapsed().as_secs_f64();
            let content = SlotContent::Spell { id: spell, name };
            if let Some(l) = b.layout.as_mut()
                && !l.holds(&content)
            {
                match l.first_empty(true) {
                    Some(at) => {
                        l.put(at, Some(content));
                        b.mark_dirty(now);
                    }
                    None => w.notices.push("ui: your bars are full".to_owned()),
                }
            }
            true
        })
        .unwrap_or(false)
    }

    /// Quick move's menu for `piece` at (`x`, `y`): a bar's quick edits,
    /// or for another piece its default place; and Edit layout.
    pub(in crate::ui) fn modern_quick_move_menu(&mut self, piece: ElementId, x: i32, y: i32) {
        self.with_bars(|w, b| {
            let at = (x as f32, y as f32);
            if let Some(bar) = b.built.iter().position(|built| built.frame == piece) {
                w.open_bar_menu(b, bar, at, true);
                return;
            }
            let name = w
                .host
                .root
                .element(piece)
                .and_then(|e| e.name.clone())
                .unwrap_or_default();
            let mut rows = vec![MenuRow::Header(hud::piece_words(&name).to_owned())];
            if hud::PIECES.contains(&name.as_str()) {
                rows.push(MenuRow::entry(
                    MenuAction::Lock(true),
                    "Lock in place",
                    Some("Ctrl will not move it; Edit layout unlocks it"),
                ));
                rows.push(MenuRow::entry(
                    MenuAction::Hide(true),
                    "Hide",
                    Some("Edit layout brings it back"),
                ));
            }
            if places::default_place(&name).is_some() {
                rows.push(MenuRow::entry(
                    MenuAction::DefaultPlace,
                    "Back to its default place",
                    None,
                ));
            }
            rows.push(MenuRow::Line);
            rows.push(MenuRow::entry(
                MenuAction::OpenEdit,
                "Open Edit layout\u{2026}",
                Some("keys, new bars"),
            ));
            Menu::open(&mut w.host.root, b.menu, MenuFor::Piece(piece), rows, at);
        });
    }

    /// Bar `bar`'s menu at `at`: quick move's (turn, slots, lock, hide,
    /// place, Edit layout) or, in Edit layout, the tab's (rename, lock,
    /// hide or show, delete).
    fn open_bar_menu(&mut self, b: &mut BarsUi, bar: usize, at: (f32, f32), quick: bool) {
        let Some(l) = &b.layout else {
            return;
        };
        let Some(this) = l.bars.get(bar) else {
            return;
        };
        let mut rows = vec![MenuRow::Header(format!("{} bar", this.name))];
        if quick && !b.edit {
            rows.push(MenuRow::entry(
                MenuAction::Turn,
                if this.vertical {
                    "Turn it across"
                } else {
                    "Turn upright"
                },
                None,
            ));
            rows.push(MenuRow::entry(MenuAction::AddSlot, "Add a slot", None));
            if this.slots.len() > 1 {
                rows.push(MenuRow::entry(
                    MenuAction::RemoveSlot,
                    "Remove the last slot",
                    None,
                ));
            }
        } else {
            rows.push(MenuRow::entry(MenuAction::Rename, "Rename\u{2026}", None));
        }
        rows.push(if this.locked {
            MenuRow::entry(MenuAction::Lock(false), "Unlock", None)
        } else {
            MenuRow::entry(
                MenuAction::Lock(true),
                "Lock in place",
                Some("Ctrl will not move it; its slots stay"),
            )
        });
        rows.push(if this.hidden {
            MenuRow::entry(MenuAction::Hide(false), "Show", None)
        } else {
            MenuRow::entry(
                MenuAction::Hide(true),
                "Hide",
                Some("Edit layout brings it back"),
            )
        });
        rows.push(MenuRow::entry(
            MenuAction::DefaultPlace,
            "Back to its default place",
            None,
        ));
        rows.push(MenuRow::Line);
        if b.edit {
            rows.push(MenuRow::entry(MenuAction::Delete, "Delete bar", None));
        } else {
            rows.push(MenuRow::entry(
                MenuAction::OpenEdit,
                "Open Edit layout\u{2026}",
                Some("keys, new bars"),
            ));
        }
        Menu::open(&mut self.host.root, b.menu, MenuFor::Bar(bar), rows, at);
    }

    /// The hidden and locked HUD pieces, each to show or unlock.
    fn open_pieces_menu(&mut self, b: &mut BarsUi, at: (f32, f32)) {
        let pieces = self.modern_pieces().cloned().unwrap_or_default();
        let mut rows = Vec::new();
        let hidden: Vec<&'static str> = hud::PIECES
            .into_iter()
            .filter(|n| pieces.is_hidden(n))
            .collect();
        let locked: Vec<&'static str> = hud::PIECES
            .into_iter()
            .filter(|n| pieces.is_locked(n))
            .collect();
        if !hidden.is_empty() {
            rows.push(MenuRow::Header("Hidden".to_owned()));
            for name in hidden {
                rows.push(MenuRow::entry(
                    MenuAction::ShowPiece(name),
                    &format!("Show {}", hud::piece_words(name)),
                    None,
                ));
            }
        }
        if !locked.is_empty() {
            if !rows.is_empty() {
                rows.push(MenuRow::Line);
            }
            rows.push(MenuRow::Header("Locked in place".to_owned()));
            for name in locked {
                rows.push(MenuRow::entry(
                    MenuAction::UnlockPiece(name),
                    &format!("Unlock {}", hud::piece_words(name)),
                    None,
                ));
            }
        }
        if rows.is_empty() {
            rows.push(MenuRow::Header("Hidden and locked".to_owned()));
            rows.push(
                MenuRow::entry(
                    MenuAction::OpenEdit,
                    "Nothing is hidden or locked",
                    Some("Ctrl + right-click a piece to hide or lock it"),
                )
                .disabled(),
            );
        }
        Menu::open(&mut self.host.root, b.menu, MenuFor::Pieces, rows, at);
    }

    /// The layout menu: this character's own or the shared one; copy
    /// another character's.
    fn open_layout_menu(&mut self, b: &mut BarsUi, at: (f32, f32)) {
        let mut rows = vec![
            MenuRow::Header("This character".to_owned()),
            MenuRow::entry(
                MenuAction::UseOwn,
                &format!("{} only", b.character_name),
                None,
            )
            .ticked(!b.shared),
            MenuRow::entry(
                MenuAction::UseShared,
                "Shared by your characters",
                Some("Every character set to share uses it"),
            )
            .ticked(b.shared),
        ];
        let others: Vec<String> = b
            .saved
            .as_ref()
            .map(|s| {
                s.characters
                    .iter()
                    .filter(|c| Some(&c.character) != b.character.as_ref() && c.own.is_some())
                    .map(|c| c.character.clone())
                    .collect()
            })
            .unwrap_or_default();
        if !others.is_empty() {
            rows.push(MenuRow::Line);
            rows.push(MenuRow::Header("Copy from".to_owned()));
            for other in others {
                let (world, name) = other.split_once('/').unwrap_or(("", other.as_str()));
                let hint = (!world.is_empty()).then_some(world);
                rows.push(MenuRow::entry(
                    MenuAction::CopyFrom(other.clone()),
                    name,
                    hint,
                ));
            }
        }
        Menu::open(&mut self.host.root, b.menu, MenuFor::Layout, rows, at);
    }

    /// A menu entry chosen; true when the layout changed.
    fn bar_menu_chosen(&mut self, b: &mut BarsUi, about: MenuFor, action: MenuAction) -> bool {
        let screen = self.host.root.size();
        match (about, action) {
            (_, MenuAction::OpenEdit) => {
                self.enter_bar_edit(b);
                false
            }
            (MenuFor::Bar(bar), MenuAction::Turn) => self.turn_bar(b, bar),
            (MenuFor::Bar(bar), MenuAction::AddSlot) => b.layout.as_mut().is_some_and(|l| {
                l.add_slot(bar);
                true
            }),
            (MenuFor::Bar(bar), MenuAction::RemoveSlot) => b.layout.as_mut().is_some_and(|l| {
                l.remove_last_slot(bar);
                true
            }),
            (MenuFor::Bar(bar), MenuAction::Lock(on)) => b
                .layout
                .as_mut()
                .and_then(|l| l.bars.get_mut(bar))
                .is_some_and(|this| {
                    this.locked = on;
                    true
                }),
            (MenuFor::Bar(bar), MenuAction::Hide(on)) => {
                if on && b.popover_for.is_some_and(|at| at.bar == bar) {
                    self.close_key_popover(b);
                }
                b.layout
                    .as_mut()
                    .and_then(|l| l.bars.get_mut(bar))
                    .is_some_and(|this| {
                        this.hidden = on;
                        true
                    })
            }
            (MenuFor::Bar(bar), MenuAction::DefaultPlace) => b
                .layout
                .as_mut()
                .and_then(|l| l.bars.get_mut(bar))
                .is_some_and(|this| {
                    this.place = default_place_of(&this.name);
                    true
                }),
            (MenuFor::Bar(bar), MenuAction::Rename) => {
                let name = b
                    .layout
                    .as_ref()
                    .and_then(|l| l.bars.get(bar))
                    .map(|this| this.name.clone())
                    .unwrap_or_default();
                let at = b
                    .built
                    .get(bar)
                    .and_then(|built| self.host.root.element(built.tab))
                    .map(|e| (e.left, e.top + e.height + 4.0))
                    .unwrap_or_default();
                RenameBox::show(&mut self.host.root, b.rename, bar, &name, at);
                false
            }
            (MenuFor::Bar(bar), MenuAction::Delete) => {
                let name = b
                    .layout
                    .as_ref()
                    .and_then(|l| l.bars.get(bar))
                    .map(|this| this.name.clone())
                    .unwrap_or_default();
                let (x, y) = self.host.root.mouse();
                let rows = vec![
                    MenuRow::Header(format!("Delete {name} bar?")),
                    MenuRow::entry(
                        MenuAction::DeleteNow,
                        "Delete it",
                        Some("Its slots and keys go with it"),
                    ),
                ];
                Menu::open(
                    &mut self.host.root,
                    b.menu,
                    MenuFor::Bar(bar),
                    rows,
                    (x as f32, y as f32),
                );
                false
            }
            (MenuFor::Bar(bar), MenuAction::DeleteNow) => {
                self.close_key_popover(b);
                b.layout.as_mut().is_some_and(|l| {
                    if bar < l.bars.len() {
                        l.bars.remove(bar);
                        true
                    } else {
                        false
                    }
                })
            }
            (MenuFor::Piece(piece), MenuAction::Lock(on)) => {
                if let Some(name) = piece_name(&self.host.root, piece) {
                    self.set_modern_piece_locked(name, on);
                }
                false
            }
            (MenuFor::Piece(piece), MenuAction::Hide(on)) => {
                if let Some(name) = piece_name(&self.host.root, piece) {
                    self.set_modern_piece_hidden(name, on);
                }
                false
            }
            (_, MenuAction::ShowPiece(name)) => {
                self.set_modern_piece_hidden(name, false);
                false
            }
            (_, MenuAction::UnlockPiece(name)) => {
                self.set_modern_piece_locked(name, false);
                false
            }
            (MenuFor::Piece(piece), MenuAction::DefaultPlace) => {
                let ui = &mut self.host.root;
                let name = ui.element(piece).and_then(|e| e.name.clone());
                if let Some(place) = name.as_deref().and_then(places::default_place) {
                    let rect = place.rect(screen);
                    if let Some(e) = ui.element_mut(piece) {
                        e.left = rect.x;
                        e.top = rect.y;
                    }
                }
                false
            }
            (MenuFor::Layout, MenuAction::UseOwn) if b.shared => {
                b.store_layout();
                b.shared = false;
                let own = b
                    .saved
                    .as_ref()
                    .zip(b.character.as_deref())
                    .and_then(|(s, c)| s.character(c))
                    .and_then(|c| c.own.as_ref())
                    .map(BarLayout::from_data);
                if let Some(own) = own {
                    b.layout = Some(own);
                }
                self.close_key_popover(b);
                true
            }
            (MenuFor::Layout, MenuAction::UseShared) if !b.shared => {
                b.store_layout();
                b.shared = true;
                let shared = b
                    .saved
                    .as_ref()
                    .and_then(|s| s.shared.as_ref())
                    .map(BarLayout::from_data);
                if let Some(shared) = shared {
                    b.layout = Some(shared);
                }
                self.close_key_popover(b);
                true
            }
            (MenuFor::Layout, MenuAction::CopyFrom(other)) => {
                let copy = b
                    .saved
                    .as_ref()
                    .and_then(|s| s.character(&other))
                    .and_then(|c| c.own.as_ref())
                    .map(BarLayout::from_data);
                match copy {
                    Some(copy) => {
                        self.close_key_popover(b);
                        b.layout = Some(copy);
                        true
                    }
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// Edit layout begins: the world dims, the toolbar and the tabs show,
    /// hidden bars show faded.
    fn enter_bar_edit(&mut self, b: &mut BarsUi) {
        if b.edit {
            return;
        }
        b.edit = true;
        let ui = &mut self.host.root;
        let screen = ui.size();
        if let Some(e) = ui.element_mut(b.dim) {
            e.width = screen.0;
            e.height = screen.1;
        }
        ui.set_visible(b.dim, true);
        ui.set_visible(b.toolbar, true);
        // The bars over the dim, the edit layer over the bars.
        for built in &b.built {
            ui.bring_to_front(built.frame);
            ui.bring_to_front(built.tab);
        }
        for id in [b.toolbar, b.popover, b.menu, b.rename] {
            ui.bring_to_front(id);
        }
    }

    /// Edit layout ends; the layout is saved.
    fn leave_bar_edit(&mut self, b: &mut BarsUi) {
        b.edit = false;
        b.reset_armed_at = None;
        self.close_key_popover(b);
        let ui = &mut self.host.root;
        for id in [b.dim, b.toolbar] {
            ui.set_visible(id, false);
        }
        Menu::close(ui, b.menu);
        RenameBox::hide(ui, b.rename);
        for built in &b.built {
            ui.set_visible(built.tab, false);
        }
        self.save_bars(b, true);
    }
}

/// The HUD piece window name of `piece`, if it is one.
fn piece_name(ui: &ac_ui::UiRoot, piece: ElementId) -> Option<&'static str> {
    let name = ui.element(piece)?.name.as_deref()?;
    hud::PIECES.into_iter().find(|n| *n == name)
}

#[cfg(test)]
mod tests {
    use super::super::model::bar_size;
    use super::*;

    #[test]
    fn a_tab_says_its_keys_in_a_few_words() {
        let layout = BarLayout::seeded(&RetailBars::default(), &|_| String::new(), &|_| {
            String::new()
        });
        assert_eq!(key_note(&layout.bars[0]), "Magic: 1 to =");
        assert_eq!(
            key_note(&layout.bars[1]),
            "keys 1 to = \u{b7} Magic Shift+1 to Shift+="
        );
        let empty = Bar::new("x", 2, default_place_of("x"));
        assert_eq!(key_note(&empty), "");
    }

    #[test]
    fn key_map_actions_are_put_in_words() {
        assert_eq!(
            action_words(InputAction::MovementForward),
            "Movement Forward"
        );
        assert!(is_slot_action(InputAction::UseQuickSlot_1));
        assert!(!is_slot_action(InputAction::MovementForward));
    }

    #[test]
    fn the_main_bar_goes_back_to_its_first_place() {
        let main = default_place_of("Main");
        assert_eq!(main.anchor, Anchor::Bottom);
        assert_eq!(default_place_of("Mine").anchor, Anchor::Center);
        assert_eq!(BarsUi::key(Some("Leafcull"), "Aerin"), "Leafcull/Aerin");
        assert_eq!(bar_size(1, false).0, 48.0);
    }
}
