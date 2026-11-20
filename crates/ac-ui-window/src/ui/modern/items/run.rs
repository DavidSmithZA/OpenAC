//! The modern inventory in the game window: mounted in place of retail's
//! inventory (its key, the panel strip, the effects' burden chip and a
//! pack used anywhere open it), fed the player's items each frame it
//! shows, and what is clicked, chosen or dropped in it carried out with
//! retail's item requests and pack drop rules.

use std::collections::{HashMap, HashSet};

use ac_core::items::{ClientObject, EquipMask};
use ac_proto::appraise::IdentifyObjectResponse;
use ac_runtime::inventory::{InteractionMode, ItemCommand, ObjectRef};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::draw::Color;
use ac_ui::event::{ItemDragPayload, ItemDragSource};
use ac_ui::geometry::AnchorEdges;
use ac_ui::icons::IconSource;
use ac_ui::modern::Horizon;
use ac_ui::modern::theme::hex;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::{emit, offer_target};
use ac_ui::panels::pack_drops::{self, DropAnswer, DropSpot};
use ac_ui::panels::paperdoll::{is_off_hand_weapon_drop, paperdoll_drop_wield_mask};
use ac_ui::tree::{Element, ElementId};
use ac_ui::widgets::Field;

use super::cell::{CellLook, CellRequest, CellView, ItemCell};
use super::grid::{GridFonts, GridGroup, ItemGrid};
use super::inventory::{self as inv, FrameFonts, FrameRequest, InventoryFrame, PackButton};
use super::inventory::{PackFace, PackRequest, PackShown};
use super::menu::{MenuFonts, PopMenu, Row};
use super::model::{self, EQUIP_SLOTS, GroupBy, PackChoice, SlotRow, SortBy, ViewOptions};
use super::sheet_view::{Foot, FootIcon, SheetFonts, SheetRequest, SheetView};
use super::sheets::{item_sheet, sub_line};
use crate::ui::modern::hud;
use crate::ui::window::WindowUi;

/// The inventory's window name.
pub const INVENTORY: &str = "modern-inventory";

/// What a menu was opened for.
const MENU_ITEM: u32 = 1;
const MENU_GROUP: u32 = 2;
const MENU_SORT: u32 = 3;
const MENU_MORE: u32 = 4;

/// The item menu's entries.
const DO_INSPECT: u32 = 1;
const DO_USE: u32 = 2;
const DO_EQUIP: u32 = 3;
const DO_TAKE_OFF: u32 = 4;
const DO_GIVE: u32 = 5;
const DO_DROP: u32 = 6;
const DO_SALVAGE: u32 = 7;
const DO_INSCRIBE: u32 = 8;
const DO_BAR: u32 = 9;
const DO_LINK: u32 = 10;
const DO_SPLIT: u32 = 11;
/// The sort menu's own entries (the sorts are their index).
const DO_REVERSE: u32 = 100;
const DO_MERGE: u32 = 200;

/// The inventory's elements and what it remembers.
#[derive(Debug)]
pub struct InventoryUi {
    /// The window.
    pub frame: ElementId,
    /// The search field.
    pub search: ElementId,
    /// The grid.
    pub grid: ElementId,
    /// The item details.
    pub details: ElementId,
    /// The field writing an inscription in the details.
    pub scribe: super::scribe::Scribe,
    /// The equipment slots, in [`EQUIP_SLOTS`]' order.
    pub slots: Vec<ElementId>,
    /// The pack rail's buttons: All, Main, then the side packs.
    pub packs: Vec<ElementId>,
    /// The menus.
    pub menu: ElementId,
    /// The split pop-up.
    pub split: ElementId,
    /// The plugins' band over the grid's foot.
    pub band: ElementId,
    /// What the plugins' menu entries in the menu last opened stand for:
    /// (plugin id, entry id), by tag from the plugins' base tag.
    plugin_menu: Vec<(String, String)>,
    /// The figure's viewport and the overlay that turns it.
    pub figure: ElementId,
    pub turner: ElementId,
    /// The grid's options.
    pub options: ViewOptions,
    /// How cells are drawn.
    pub look: CellLook,
    /// When each item was first seen (Newest first).
    arrivals: HashMap<u32, u64>,
    clock: u64,
    /// The answers about the player's items, by item.
    pub appraisals: HashMap<u32, IdentifyObjectResponse>,
    /// Items asked about for the details.
    pub asked: HashSet<u32>,
    /// Those whose answers have not come yet (they open nothing).
    pub quiet: HashSet<u32>,
    /// The item being dragged from the window.
    lifted: Option<u32>,
    /// Stacks still to merge (Tidy), one request at a time.
    merges: Vec<(u32, u32, u32)>,
    /// The window showed last frame.
    was_open: bool,
    /// The rail's font.
    rail_font: Option<std::sync::Arc<ac_ui::UiFont>>,
}

impl WindowUi {
    /// The inventory, hidden, in place of retail's.
    pub(in crate::ui) fn mount_modern_inventory(&mut self) {
        let theme = Horizon::BRONZE;
        let label = self.modern_font(Weight::SemiBold, 10.5);
        let small = self.modern_font(Weight::Regular, 10.5);
        let body = self.modern_font(Weight::Regular, 12.0);
        let strong = self.modern_font(Weight::SemiBold, 12.0);
        let frame_fonts = FrameFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            strong: strong.clone(),
            body: body.clone(),
            label: label.clone(),
            small: small.clone(),
            chip: self.modern_font(Weight::SemiBold, 11.0),
        };
        let grid_fonts = GridFonts {
            heading: self.modern_font(Weight::SemiBold, 11.0),
            small: self.modern_font(Weight::SemiBold, 9.5),
            name: strong.clone(),
        };
        let sheet_fonts = SheetFonts {
            name: self.modern_font(Weight::SemiBold, 14.0),
            small: self.modern_font(Weight::Regular, 11.0),
            label: self.modern_font(Weight::SemiBold, 9.5),
            body: body.clone(),
            strong: strong.clone(),
            big: self.modern_font(Weight::SemiBold, 18.0),
        };
        let menu_fonts = MenuFonts {
            body: body.clone(),
            small: small.clone(),
        };
        let rail_font = self.modern_font(Weight::SemiBold, 9.5);
        let frame = match hud::mount_piece(
            &mut self.host.root,
            INVENTORY,
            (inv::WIDE, inv::HEIGHT),
            InventoryFrame::new(theme, frame_fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern inventory: {e}"));
                return;
            }
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        // The search field.
        let mut e = Element::at(
            inv::search_x(inv::WIDE) + 30.0,
            14.0,
            inv::SEARCH_W - 36.0,
            24.0,
        )
        .with_anchors(AnchorEdges::NONE);
        Field::configure(&mut e);
        let mut f = Field::default();
        f.font = self.fonts_small_for_search(&body);
        f.one_line = true;
        f.selectable = true;
        f.padding = 3.0;
        f.record_history = false;
        f.clear_on_submit = false;
        f.stay_focused_after_submit = true;
        f.text_color = theme.palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = hex(0xE0_B068, 0.35);
        f.set_editable_detached(&mut e, true);
        let ui = &mut self.host.root;
        let search = ui.add(frame, e, f);
        // The equipment slots.
        let mut slots = Vec::with_capacity(EQUIP_SLOTS.len());
        let (mut left, mut right, mut jewel, mut held) = (0, 0, 0, 0);
        for (i, s) in EQUIP_SLOTS.iter().enumerate() {
            let step = inv::SLOT + inv::SLOT_GAP;
            let (x, y) = match s.row {
                SlotRow::Left => {
                    left += 1;
                    (inv::EQUIP_X, inv::FIGURE_Y + (left - 1) as f32 * step)
                }
                SlotRow::Right => {
                    right += 1;
                    (
                        inv::FIGURE_X + inv::FIGURE_W + 8.0,
                        inv::FIGURE_Y + (right - 1) as f32 * step,
                    )
                }
                SlotRow::Jewelry => {
                    jewel += 1;
                    (inv::EQUIP_X + (jewel - 1) as f32 * step, inv::JEWELRY_Y)
                }
                SlotRow::Held => {
                    held += 1;
                    (inv::EQUIP_X + (held - 1) as f32 * step, inv::HELD_Y)
                }
            };
            let mut cell = ItemCell::new(theme, grid_fonts.small.clone(), strong.clone());
            cell.source = ItemDragSource::Equipment;
            cell.glyph = Some(super::glyphs::SlotGlyph::SLOTS[i]);
            slots.push(ui.add(
                frame,
                Element::at(x, y, inv::SLOT, inv::SLOT).with_anchors(AnchorEdges::NONE),
                cell,
            ));
        }
        let figure_at = || {
            Element::at(
                inv::FIGURE_X,
                inv::FIGURE_Y,
                inv::FIGURE_W,
                inv::FIGURE_H - 20.0,
            )
            .with_anchors(AnchorEdges::NONE)
        };
        let figure = ui.add(frame, figure_at(), ac_ui::widgets::Viewport::default());
        let mut turner_at = figure_at();
        turner_at.captures_pointer_drag = true;
        let turner = ui.add(frame, turner_at, inv::FigureTurn::default());
        let grid = ui.add(
            frame,
            Element::at(inv::GRID_X, inv::GRID_Y, inv::GRID_W, inv::GRID_H)
                .with_anchors(AnchorEdges::NONE),
            ItemGrid::new(theme, grid_fonts),
        );
        let mut sheet = SheetView::new(theme, sheet_fonts);
        sheet.foot = Some(Foot::default());
        let details = ui.add(
            frame,
            Element::at(inv::DETAILS_X, inv::EQUIP_Y, inv::DETAILS_W, inv::DETAILS_H)
                .with_anchors(AnchorEdges::NONE)
                .with_visible(false),
            sheet,
        );
        let scribe = super::scribe::Scribe::mount(ui, details, theme, body.clone());
        let menu = PopMenu::mount(ui, theme, menu_fonts);
        let split_fonts = super::split::SplitFonts {
            title: self.modern_font(Weight::SemiBold, 13.0),
            small: self.modern_font(Weight::Regular, 11.0),
            body: self.modern_font(Weight::SemiBold, 12.0),
        };
        let ui = &mut self.host.root;
        let split = super::split::SplitPopover::mount(ui, theme, split_fonts);
        let band = ui.add(
            frame,
            Element::at(
                inv::GRID_X - 12.0,
                inv::GRID_Y + inv::GRID_H,
                inv::GRID_W + 24.0,
                1.0,
            )
            .with_anchors(AnchorEdges::NONE)
            .with_visible(false),
            super::plugin_notes::FootBand::new(theme, label.clone(), small.clone(), strong.clone()),
        );
        ui.hide_window(INVENTORY);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.inventory = Some(InventoryUi {
            frame,
            search,
            grid,
            details,
            scribe,
            slots,
            packs: Vec::new(),
            menu,
            split,
            band,
            plugin_menu: Vec::new(),
            figure,
            turner,
            options: ViewOptions::default(),
            look: CellLook::Grid,
            arrivals: HashMap::new(),
            clock: 0,
            appraisals: HashMap::new(),
            asked: HashSet::new(),
            quiet: HashSet::new(),
            lifted: None,
            merges: Vec::new(),
            was_open: false,
            rail_font,
        });
        modern.replace(crate::ui::window::INVENTORY_WINDOW);
        self.panels
            .stand_in(ac_ui::panels::panel_id::INVENTORY, INVENTORY);
    }

    fn fonts_small_for_search(
        &self,
        body: &Option<std::sync::Arc<ac_ui::UiFont>>,
    ) -> Option<std::sync::Arc<ac_ui::UiFont>> {
        body.clone()
    }

    /// Packs used anywhere open in the inventory (the window itself shows
    /// through the inventory panel).
    fn follow_modern_opened_packs(&mut self) {
        let player = self.facts.state.player;
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        let opened = std::mem::take(&mut modern.opened_packs);
        let Some(inv) = modern.inventory.as_mut() else {
            return;
        };
        if let Some(&pack) = opened.last() {
            inv.options.pack = if pack == player {
                PackChoice::Main
            } else {
                PackChoice::Side(pack)
            };
        }
    }

    /// The answers about the player's items, kept for the details (taken
    /// before retail's examination code takes them).
    pub(in crate::ui) fn keep_modern_appraisals(&mut self) {
        let Some(inv) = self.modern.as_mut().and_then(|m| m.inventory.as_mut()) else {
            return;
        };
        for a in &self.facts.appraisals {
            let guid = a.response.guid;
            if a.response.success || !inv.appraisals.contains_key(&guid) {
                inv.appraisals.insert(guid, a.response.clone());
            }
        }
    }

    fn send_item(&mut self, command: ItemCommand) {
        emit(
            &mut self.host.root,
            PanelOutput::Session(SessionCommand::Item(command)),
        );
    }

    fn select_item(&mut self, id: u32) {
        if self.facts.state.mode != InteractionMode::None {
            offer_target(&mut self.host.root, ObjectRef::Guid(id));
        } else {
            emit(&mut self.host.root, PanelOutput::SelectObject(id));
        }
    }

    /// The player's own item `id` (carried or worn).
    fn own_item(&self, id: u32) -> Option<&ClientObject> {
        let player = self.facts.state.player;
        self.facts
            .objects
            .get(id)
            .filter(|_| id != player && self.facts.objects.is_owned_by_object(id, player))
    }

    /// The inventory, each frame: what was asked carried out, then what it
    /// shows brought up to date while it shows.
    pub(in crate::ui) fn tick_modern_inventory(&mut self) {
        let Some(frame) = self
            .modern
            .as_ref()
            .and_then(|m| m.inventory.as_ref())
            .map(|i| i.frame)
        else {
            return;
        };
        self.follow_modern_opened_packs();
        let open = self.host.root.is_window_visible(INVENTORY);
        self.run_inventory_requests();
        let Some(inv) = self.modern.as_mut().and_then(|m| m.inventory.as_mut()) else {
            return;
        };
        if open && !inv.was_open {
            ItemGrid::scroll_home(&mut self.host.root, inv.grid);
            self.host.root.bring_to_front(frame);
        }
        inv.was_open = open;
        if !open {
            self.write_inscriptions();
            return;
        }
        let search = self
            .host
            .root
            .widget::<Field>(inv.search)
            .map(Field::text)
            .unwrap_or_default();
        inv.options.search = search;
        self.next_tidy_merge();
        self.feed_modern_inventory(frame);
        self.write_inscriptions();
        self.answer_pack_drags();
    }

    /// The details' inscription field: what was written goes out, and
    /// the kept answer takes it so the details show it from now on.
    fn write_inscriptions(&mut self) {
        let open = self.host.root.is_window_visible(INVENTORY);
        let me = self.status.playing.clone().unwrap_or_default();
        let Some(inv) = self.modern.as_mut().and_then(|m| m.inventory.as_mut()) else {
            return;
        };
        let ui = &mut self.host.root;
        if !open && ui.keyboard_focus() == Some(inv.scribe.field) {
            ui.set_keyboard_focus(None);
        }
        let Some(command) = inv.scribe.tick(ui, inv.details) else {
            return;
        };
        if let ItemCommand::Inscribe {
            item: ObjectRef::Guid(id),
            text,
        } = &command
            && let Some(a) = inv.appraisals.get_mut(id)
        {
            super::sheets::note_inscription(a, text, &me);
        }
        self.send_item(command);
    }

    /// Carries out what the window's parts asked for.
    #[allow(clippy::too_many_lines)]
    fn run_inventory_requests(&mut self) {
        let Some(inv) = self.modern.as_ref().and_then(|m| m.inventory.as_ref()) else {
            return;
        };
        let (frame, grid, details, menu) = (inv.frame, inv.grid, inv.details, inv.menu);
        let (band, split, turner) = (inv.band, inv.split, inv.turner);
        let slots = inv.slots.clone();
        let packs = inv.packs.clone();
        let ui = &mut self.host.root;
        let frame_asks = ui
            .widget_mut::<InventoryFrame>(frame)
            .map(|f| std::mem::take(&mut f.requests))
            .unwrap_or_default();
        let mut cell_asks: Vec<(Option<usize>, CellRequest)> = ItemGrid::take_requests(ui, grid)
            .into_iter()
            .map(|r| (None, r))
            .collect();
        for (i, &s) in slots.iter().enumerate() {
            if let Some(c) = ui.widget_mut::<ItemCell>(s) {
                cell_asks.extend(c.requests.drain(..).map(|r| (Some(i), r)));
            }
        }
        let mut pack_asks = Vec::new();
        for &b in &packs {
            if let Some(button) = ui.widget_mut::<PackButton>(b) {
                let id = button.shown.id;
                let face = button.face;
                pack_asks.extend(button.requests.drain(..).map(|r| (face, id, r)));
            }
        }
        let worn_drops = ui
            .widget_mut::<inv::FigureTurn>(turner)
            .map(|t| std::mem::take(&mut t.dropped))
            .unwrap_or_default();
        let sheet_asks = ui
            .widget_mut::<SheetView>(details)
            .map(|s| std::mem::take(&mut s.requests))
            .unwrap_or_default();
        let chosen = PopMenu::take(ui, menu);
        let pressed = super::plugin_notes::FootBand::take(ui, band);
        for (owner, id) in pressed {
            self.send_plugin_input(ac_plugin_ui::UiInput::FootButton {
                owner,
                window: ac_plugin_ui::ItemWindow::Inventory,
                id,
            });
        }
        let ui = &mut self.host.root;
        let splits = super::split::SplitPopover::take(ui, split);
        for super::split::SplitRequest::Split { item, amount } in splits {
            self.split_stack(item, amount);
        }
        for ask in frame_asks {
            self.run_frame_request(ask);
        }
        for id in worn_drops {
            let wearable = self
                .facts
                .objects
                .get(id)
                .is_some_and(|o| o.valid_locations != EquipMask::NONE);
            if wearable {
                self.send_item(ItemCommand::Wield {
                    item: ObjectRef::Guid(id),
                    location: 0,
                });
            }
        }
        for (face, id, ask) in pack_asks {
            match ask {
                PackRequest::Choose => {
                    let choice = match face {
                        PackFace::All => PackChoice::All,
                        PackFace::Main => PackChoice::Main,
                        PackFace::Side => PackChoice::Side(id),
                    };
                    if let Some(inv) = self.inventory_mut() {
                        inv.options.pack = choice;
                    }
                    ItemGrid::scroll_home(&mut self.host.root, grid);
                    if face != PackFace::All {
                        self.send_item(ItemCommand::SetOpenPack(id));
                    }
                }
                PackRequest::Dropped(p) => {
                    let spot = if face == PackFace::Main {
                        DropSpot::MainPack
                    } else {
                        DropSpot::SidePack
                    };
                    self.drop_on_pack(spot, id, -1, &p);
                }
            }
        }
        for (slot, ask) in cell_asks {
            match ask {
                CellRequest::Select(id) => self.select_item(id),
                CellRequest::Use(id) => self.send_item(ItemCommand::Use(ObjectRef::Guid(id))),
                CellRequest::Menu(id, at) => self.open_item_menu(id, at),
                CellRequest::Inspect(id) => self.item_do(id, DO_INSPECT),
                CellRequest::Loot(_) => {}
                CellRequest::Lifted(id) => {
                    if let Some(inv) = self.inventory_mut() {
                        inv.lifted = Some(id);
                    }
                }
                CellRequest::DragEnded(_) => {
                    if let Some(inv) = self.inventory_mut() {
                        inv.lifted = None;
                    }
                }
                CellRequest::Dropped {
                    item,
                    from,
                    container,
                    onto,
                    ..
                } => {
                    let p = ItemDragPayload::new(item, from, -1);
                    match slot {
                        Some(i) => self.drop_on_slot(i, &p),
                        None => self.drop_on_grid(container, onto, &p),
                    }
                }
            }
        }
        for ask in sheet_asks {
            let Some(id) = self
                .host
                .root
                .widget::<SheetView>(details)
                .map(|s| s.sheet().subject)
                .filter(|&id| id != 0)
            else {
                continue;
            };
            match ask {
                SheetRequest::Close => {}
                SheetRequest::Primary => self.item_primary(id),
                SheetRequest::Small(0, _) => self.item_do(id, DO_GIVE),
                SheetRequest::Small(1, _) => self.item_do(id, DO_DROP),
                SheetRequest::Small(2, _) => self.item_do(id, DO_SALVAGE),
                SheetRequest::Small(_, (x, y)) => {
                    let stack = self.own_item(id).is_some_and(|o| o.stack_size > 1);
                    let mut rows = Vec::new();
                    if stack {
                        rows.push(Row::entry(DO_SPLIT, "Split the stack\u{2026}"));
                    }
                    rows.extend([
                        Row::entry(DO_INSCRIBE, "Inscribe"),
                        Row::entry(DO_BAR, "Add to a shortcut bar"),
                        Row::entry(DO_LINK, "Link in chat"),
                        Row::Line,
                        Row::entry(DO_INSPECT, "Inspect"),
                    ]);
                    let (extra, tags) = super::plugin_notes::menu_rows(&self.facts.plugin_ui, id);
                    rows.extend(extra);
                    if let Some(inv) = self.inventory_mut() {
                        inv.plugin_menu = tags;
                    }
                    PopMenu::open(
                        &mut self.host.root,
                        menu,
                        MENU_MORE,
                        id,
                        rows,
                        220.0,
                        (x as f32 - 188.0, y as f32 + 4.0),
                    );
                }
            }
        }
        if let Some((kind, about, tag)) = chosen {
            self.run_menu_choice(kind, about, tag);
        }
    }

    fn inventory_mut(&mut self) -> Option<&mut InventoryUi> {
        self.modern.as_mut().and_then(|m| m.inventory.as_mut())
    }

    fn run_frame_request(&mut self, ask: FrameRequest) {
        let Some(inv) = self.modern.as_ref().and_then(|m| m.inventory.as_ref()) else {
            return;
        };
        let (menu, grid) = (inv.menu, inv.grid);
        let options = inv.options.clone();
        match ask {
            FrameRequest::Close => {
                self.panels.set_visibility(
                    &mut self.host.root,
                    ac_ui::panels::panel_id::INVENTORY,
                    false,
                );
            }
            FrameRequest::GroupMenu(at) => {
                let mut rows = vec![Row::Header("GROUP BY".to_owned())];
                for (i, g) in GroupBy::ALL.iter().enumerate() {
                    rows.push(Row::entry(i as u32, g.words()).ticked(*g == options.group));
                }
                PopMenu::open(&mut self.host.root, menu, MENU_GROUP, 0, rows, 200.0, at);
            }
            FrameRequest::SortMenu(at) => {
                let mut rows = vec![Row::Header("SORT BY".to_owned())];
                for (i, s) in SortBy::ALL.iter().enumerate() {
                    rows.push(
                        Row::entry(i as u32, s.words())
                            .note(s.note())
                            .ticked(*s == options.sort),
                    );
                }
                rows.push(Row::entry(DO_REVERSE, "Reverse order").ticked(options.reverse));
                rows.push(Row::Line);
                rows.push(Row::Header("TIDY".to_owned()));
                let partial = self.partial_stacks().len();
                let note = match partial {
                    0 => "Nothing to join".to_owned(),
                    1 => "Joins 1 stack".to_owned(),
                    n => format!("Joins {n} stacks"),
                };
                rows.push(
                    Row::entry(DO_MERGE, "Merge partial stacks")
                        .under(note)
                        .on(partial > 0),
                );
                PopMenu::open(&mut self.host.root, menu, MENU_SORT, 0, rows, 240.0, at);
            }
            FrameRequest::Look(look) => {
                if let Some(inv) = self.inventory_mut() {
                    inv.look = look;
                }
                ItemGrid::set_look(&mut self.host.root, grid, look);
            }
            FrameRequest::Kind(kind) => {
                if let Some(inv) = self.inventory_mut() {
                    inv.options.kind = kind;
                }
                ItemGrid::scroll_home(&mut self.host.root, grid);
            }
        }
    }

    fn run_menu_choice(&mut self, kind: u32, about: u32, tag: u32) {
        match kind {
            MENU_GROUP => {
                if let (Some(&g), Some(inv)) =
                    (GroupBy::ALL.get(tag as usize), self.inventory_mut())
                {
                    inv.options.group = g;
                }
            }
            MENU_SORT => {
                if tag == DO_MERGE {
                    let merges = self.partial_stacks();
                    if let Some(inv) = self.inventory_mut() {
                        inv.merges = merges;
                    }
                } else if let Some(inv) = self.inventory_mut() {
                    if tag == DO_REVERSE {
                        inv.options.reverse = !inv.options.reverse;
                    } else if let Some(&s) = SortBy::ALL.get(tag as usize) {
                        inv.options.sort = s;
                    }
                }
            }
            MENU_ITEM | MENU_MORE => self.item_do(about, tag),
            _ => {}
        }
    }

    /// The menu a right click on item `id` opens at `at`.
    fn open_item_menu(&mut self, id: u32, at: (f32, f32)) {
        let Some(item) = self.own_item(id) else {
            return;
        };
        let worn = item.currently_equipped_location != EquipMask::NONE;
        let wearable = item.valid_locations != EquipMask::NONE;
        let stack = item.stack_size > 1;
        let name = item.appropriate_name().into_owned();
        let mut rows = vec![Row::Header(name.to_uppercase())];
        rows.push(Row::entry(DO_INSPECT, "Inspect"));
        if worn {
            rows.push(Row::entry(DO_TAKE_OFF, "Take off"));
        } else if wearable {
            rows.push(Row::entry(DO_EQUIP, "Equip"));
        }
        rows.push(Row::entry(DO_USE, "Use"));
        rows.push(Row::Line);
        rows.push(Row::entry(DO_GIVE, "Give to your target"));
        rows.push(Row::entry(DO_DROP, "Drop on the ground"));
        rows.push(Row::entry(DO_SALVAGE, "Salvage"));
        if stack {
            rows.push(Row::entry(DO_SPLIT, "Split the stack\u{2026}"));
        }
        rows.push(Row::Line);
        rows.push(Row::entry(DO_BAR, "Add to a shortcut bar"));
        rows.push(Row::entry(DO_LINK, "Link in chat"));
        let (extra, tags) = super::plugin_notes::menu_rows(&self.facts.plugin_ui, id);
        rows.extend(extra);
        if let Some(inv) = self.inventory_mut() {
            inv.plugin_menu = tags;
        }
        let Some(menu) = self
            .modern
            .as_ref()
            .and_then(|m| m.inventory.as_ref())
            .map(|i| i.menu)
        else {
            return;
        };
        self.select_item(id);
        PopMenu::open(&mut self.host.root, menu, MENU_ITEM, id, rows, 220.0, at);
    }

    /// The details' main button: Equip, Take off or Use.
    fn item_primary(&mut self, id: u32) {
        let Some(item) = self.own_item(id) else {
            return;
        };
        let tag = if item.currently_equipped_location != EquipMask::NONE {
            DO_TAKE_OFF
        } else if item.valid_locations != EquipMask::NONE {
            DO_EQUIP
        } else {
            DO_USE
        };
        self.item_do(id, tag);
    }

    /// Does `tag` with item `id`.
    fn item_do(&mut self, id: u32, tag: u32) {
        if self.own_item(id).is_none() {
            return;
        }
        if tag >= super::plugin_notes::MENU_TAG_BASE {
            // A plugin's entry: it goes back to the plugin.
            let chosen = self.inventory_mut().and_then(|inv| {
                inv.plugin_menu
                    .get((tag - super::plugin_notes::MENU_TAG_BASE) as usize)
                    .cloned()
            });
            if let Some((owner, entry)) = chosen {
                self.send_plugin_input(ac_plugin_ui::UiInput::ItemMenu {
                    owner,
                    id: entry,
                    object: id,
                });
            }
            return;
        }
        let item = ObjectRef::Guid(id);
        match tag {
            DO_INSPECT => self.send_item(ItemCommand::Appraise(item)),
            DO_USE => self.send_item(ItemCommand::Use(item)),
            DO_EQUIP => self.send_item(ItemCommand::Wield { item, location: 0 }),
            DO_TAKE_OFF => self.send_item(ItemCommand::Unwield(item)),
            DO_GIVE => {
                // To what was selected before the item (retail's give to
                // your previous selection).
                self.select_item(id);
                self.send_item(ItemCommand::GiveSelectedToPrevious { amount: 0 });
            }
            DO_DROP => self.send_item(ItemCommand::Drop { item, amount: 0 }),
            DO_SALVAGE => self.send_item(ItemCommand::MarkSalvage(vec![id])),
            DO_INSCRIBE => {
                let details = self
                    .modern
                    .as_ref()
                    .and_then(|m| m.inventory.as_ref())
                    .map(|i| i.details);
                if let Some(d) = details
                    && let Some(s) = self.host.root.widget_mut::<SheetView>(d)
                {
                    s.choose_tab("Details");
                }
            }
            DO_BAR => self.put_item_on_bars(id),
            DO_LINK => self.link_item_in_chat(id),
            DO_SPLIT => self.open_split(id),
            _ => {}
        }
    }

    /// The split pop-up on stack `id`, where its menu was.
    fn open_split(&mut self, id: u32) {
        let Some((name, stack)) = self.own_item(id).map(|o| {
            (
                o.appropriate_name().into_owned(),
                o.stack_size.max(0).unsigned_abs(),
            )
        }) else {
            return;
        };
        let Some((menu, split)) = self
            .modern
            .as_ref()
            .and_then(|m| m.inventory.as_ref())
            .map(|i| (i.menu, i.split))
        else {
            return;
        };
        let ui = &mut self.host.root;
        let at = ui
            .element(menu)
            .map_or((0.0, 0.0), |e| (e.left, e.top + 8.0));
        super::split::SplitPopover::open(ui, split, id, &name, stack, at);
    }

    /// `amount` of stack `id` taken off into a stack of its own, just
    /// after it in the same pack.
    fn split_stack(&mut self, id: u32, amount: u32) {
        let Some(item) = self.own_item(id) else {
            return;
        };
        let full = item.stack_size.max(0).unsigned_abs();
        if amount == 0 || amount >= full {
            return;
        }
        let player = self.facts.state.player;
        let pack = item.container_id;
        let slot = ac_core::items::placement::loose_contents(&self.facts.objects, pack)
            .iter()
            .position(|&g| g == id)
            .map_or(-1, |i| i as i32 + 1);
        let container = if pack == player {
            ObjectRef::Player
        } else {
            ObjectRef::Guid(pack)
        };
        self.send_item(ItemCommand::MoveToContainer {
            item: ObjectRef::Guid(id),
            container,
            slot,
            amount,
            join_stack: false,
        });
    }

    /// Item `id` on the first free slot of the player's bars.
    fn put_item_on_bars(&mut self, id: u32) {
        let _ = self.put_on_first_empty_item(id);
    }

    /// Item `id`'s name in the chat entry.
    fn link_item_in_chat(&mut self, id: u32) {
        let Some(name) = self.facts.objects.get(id).map(|o| o.name.clone()) else {
            return;
        };
        let Some(field) = self
            .modern
            .as_ref()
            .and_then(|m| m.chat.as_ref())
            .map(|c| c.field)
        else {
            return;
        };
        let ui = &mut self.host.root;
        if let Some(f) = ui.widget_mut::<Field>(field) {
            let mut text = f.text();
            if !text.is_empty() && !text.ends_with(' ') {
                text.push(' ');
            }
            text.push_str(&format!("[{name}] "));
            f.set_text(&text);
        }
        ui.set_keyboard_focus(Some(field));
    }

    /// Stacks in the same pack that could be one: (from, to, amount), the
    /// smaller into the larger while it has room.
    fn partial_stacks(&self) -> Vec<(u32, u32, u32)> {
        let objects = &self.facts.objects;
        let player = self.facts.state.player;
        let mut out = Vec::new();
        for pack in model::packs(objects, player) {
            let mut stacks: Vec<&ClientObject> = objects
                .get_contents(pack.id)
                .iter()
                .filter_map(|&g| objects.get(g))
                .filter(|o| o.stack_size_max > 1 && o.stack_size < o.stack_size_max)
                .collect();
            stacks.sort_by_key(|o| std::cmp::Reverse(o.stack_size));
            let mut used = HashSet::new();
            for (i, to) in stacks.iter().enumerate() {
                if used.contains(&to.object_id()) {
                    continue;
                }
                let mut room = to.stack_size_max - to.stack_size;
                for from in &stacks[i + 1..] {
                    if room <= 0 {
                        break;
                    }
                    if from.weenie_class_id != to.weenie_class_id
                        || used.contains(&from.object_id())
                        || from.stack_size > room
                    {
                        continue;
                    }
                    used.insert(from.object_id());
                    room -= from.stack_size;
                    out.push((
                        from.object_id(),
                        to.object_id(),
                        from.stack_size.unsigned_abs(),
                    ));
                }
            }
        }
        out
    }

    /// The next Tidy merge, when no request is in flight.
    fn next_tidy_merge(&mut self) {
        if !self.facts.state.can_make_request {
            return;
        }
        let Some((from, to, amount)) = self.inventory_mut().and_then(|i| i.merges.pop()) else {
            return;
        };
        if self.facts.objects.get(from).is_some() && self.facts.objects.get(to).is_some() {
            self.send_item(ItemCommand::Merge {
                from: ObjectRef::Guid(from),
                to: ObjectRef::Guid(to),
                amount,
            });
        }
    }

    /// Runs `f` with the item panels' view of the facts and the selection.
    fn with_items_view<R>(
        &mut self,
        f: impl FnOnce(&ac_ui::panels::items::ItemsView<'_>, Option<u32>) -> R,
    ) -> R {
        use ac_ui::panels::toolbar::ToolbarWindowController;
        let selected = self.player_view.and_then(|v| v.selected);
        let split = self.host.with_controller::<ToolbarWindowController, _>(
            crate::ui::window::TOOLBAR_WINDOW,
            |c, _| c.split.quantity().clone(),
        );
        let pending: Vec<u32> = self.facts.state.pending_item.into_iter().collect();
        let resolve = crate::ui::facts::composed_names(&self.status.names);
        let view = self
            .facts
            .items_view(selected, &pending, split.as_ref(), &resolve);
        f(&view, selected)
    }

    /// Answers a drop the drop rules gave.
    fn carry_out(&mut self, answer: DropAnswer) {
        match answer {
            DropAnswer::Nothing => {}
            DropAnswer::Notice(text) => emit(&mut self.host.root, PanelOutput::Notice(text)),
            DropAnswer::Send { command, select } => {
                self.send_item(command);
                if let Some(id) = select {
                    emit(&mut self.host.root, PanelOutput::SelectObject(id));
                }
            }
        }
    }

    fn drop_on_pack(&mut self, spot: DropSpot, pack: u32, index: i32, p: &ItemDragPayload) {
        let answer = self.with_items_view(|view, selected| {
            pack_drops::answer(view, selected, spot, pack, index, p)
        });
        self.carry_out(answer);
    }

    /// A drop on a cell of the grid: left of the item it lands on, in that
    /// item's pack; on an empty cell, at the end of the group's pack (the
    /// main pack for a mixed group).
    fn drop_on_grid(&mut self, container: Option<u32>, onto: u32, p: &ItemDragPayload) {
        let objects = &self.facts.objects;
        let player = self.facts.state.player;
        let open = if onto != 0 {
            objects.get(onto).map_or(player, |o| o.container_id)
        } else {
            container.unwrap_or(player)
        };
        let index = if onto == 0 {
            -1
        } else {
            ac_core::items::placement::loose_contents(objects, open)
                .iter()
                .position(|&g| g == onto)
                .map_or(-1, |i| i as i32)
        };
        let answer = self.with_items_view(|view, selected| {
            pack_drops::answer(view, selected, DropSpot::Grid { open }, onto, index, p)
        });
        self.carry_out(answer);
    }

    /// A drop on equipment slot `slot`: worn there (an off-hand weapon on
    /// the shield slot), as retail's slots do.
    fn drop_on_slot(&mut self, slot: usize, p: &ItemDragPayload) {
        let Some(item) = self.facts.objects.get(p.obj_id) else {
            return;
        };
        let mask = EQUIP_SLOTS[slot].mask;
        let off_hand = is_off_hand_weapon_drop(item, mask);
        let wield = paperdoll_drop_wield_mask(item, mask);
        if !off_hand && wield == EquipMask::NONE {
            return;
        }
        if !self.facts.state.can_make_request {
            emit(
                &mut self.host.root,
                PanelOutput::Notice(ac_runtime::inventory::REQUEST_BUSY.to_owned()),
            );
        } else if off_hand {
            self.send_item(ItemCommand::WieldSecondary(ObjectRef::Guid(p.obj_id)));
        } else {
            self.send_item(ItemCommand::Wield {
                item: ObjectRef::Guid(p.obj_id),
                location: wield.bits(),
            });
        }
    }

    /// What the window shows, from the facts.
    #[allow(clippy::too_many_lines)]
    fn feed_modern_inventory(&mut self, frame: ElementId) {
        let selected = self.player_view.and_then(|v| v.selected);
        let player = self.facts.state.player;
        let pending = self.facts.state.pending_item;
        let strength = self.facts.strength();
        let me = self.status.playing.clone().unwrap_or_default();
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        let Some(inv) = modern.inventory.as_mut() else {
            return;
        };
        let Self {
            host,
            facts,
            icons,
            status,
            ..
        } = self;
        let objects = &facts.objects;
        let ui = &mut host.root;
        // Newest first: when each item was first seen.
        for o in objects.objects() {
            if !inv.arrivals.contains_key(&o.object_id()) {
                inv.clock += 1;
                inv.arrivals.insert(o.object_id(), inv.clock);
            }
        }
        let names = crate::ui::facts::composed_names(&status.names);
        let arrivals = &inv.arrivals;
        let arrived = |g: u32| arrivals.get(&g).copied().unwrap_or(0);
        let grid_view = model::grid(
            objects,
            player,
            &inv.options,
            &model::SortFacts {
                name: &names,
                arrived: &arrived,
            },
        );
        let shortcut_keys = shortcut_keys(&facts.state.shortcuts);
        let lifted = inv.lifted.and_then(|id| objects.get(id));
        let cell = |o: &ClientObject, icons: &mut dyn ac_ui::icons::IconSource| CellView {
            id: o.object_id(),
            icon: icons.item_icon(
                o.item_type,
                o.icon_id,
                o.icon_underlay_id,
                o.icon_overlay_id,
                o.effects,
            ),
            count: (o.stack_size > 1).then_some(o.stack_size),
            stamp: shortcut_keys.get(&o.object_id()).cloned(),
            name: names(o),
            sub: sub_line(o),
            name_color: None,
            selected: selected == Some(o.object_id()),
            waiting: pending == Some(o.object_id()),
            cooldown: 0.0,
            faint: false,
            lit: false,
            actions: false,
            note: None,
            mark: super::plugin_notes::mark_on(&facts.plugin_ui, &Horizon::BRONZE, o.object_id()),
        };
        let groups: Vec<GridGroup> = grid_view
            .groups
            .iter()
            .map(|g| GridGroup {
                key: g.key,
                title: g.title.clone(),
                meta: g.meta.clone(),
                cells: g
                    .items
                    .iter()
                    .filter_map(|&id| objects.get(id))
                    .map(|o| cell(o, icons))
                    .collect(),
                empty: g.empty,
                container: g.container,
            })
            .collect();
        ItemGrid::set_groups(ui, inv.grid, groups);
        // The equipment.
        let worn = model::worn(objects, player);
        let lit_slots: Vec<usize> = lifted
            .filter(|o| o.currently_equipped_location == EquipMask::NONE)
            .map(model::slots_for)
            .unwrap_or_default();
        for (i, (&slot, w)) in inv.slots.iter().zip(worn.iter()).enumerate() {
            let def = EQUIP_SLOTS[i];
            let mut v = match w.item.and_then(|id| objects.get(id)) {
                Some(o) => cell(o, icons),
                None => CellView {
                    name: if w.open {
                        def.label.to_owned()
                    } else {
                        format!("{} (locked)", def.label)
                    },
                    ..CellView::default()
                },
            };
            if lifted.is_some() && !lit_slots.is_empty() {
                v.lit = lit_slots.contains(&i);
                v.faint = !v.lit;
            }
            if let Some(c) = ui.widget_mut::<ItemCell>(slot) {
                c.view = v;
                c.no_drops = !w.open;
            }
        }
        // The pack rail.
        let all_packs = model::packs(objects, player);
        let wanted = all_packs.len() + 1;
        while inv.packs.len() < wanted {
            let face = match inv.packs.len() {
                0 => PackFace::All,
                1 => PackFace::Main,
                _ => PackFace::Side,
            };
            let b = ui.add(
                frame,
                Element::at(inv::RAIL_X, 0.0, inv::RAIL_W, PackButton::height(face))
                    .with_anchors(AnchorEdges::NONE),
                PackButton::new(Horizon::BRONZE, inv.rail_font.clone(), face),
            );
            inv.packs.push(b);
        }
        let mut y = inv::EQUIP_Y;
        let searching = !inv.options.search.trim().is_empty();
        for (i, &b) in inv.packs.iter().enumerate() {
            let face = match i {
                0 => PackFace::All,
                1 => PackFace::Main,
                _ => PackFace::Side,
            };
            let shown = match i {
                0 => Some(PackShown {
                    id: 0,
                    name: "All packs".to_owned(),
                    fill_words: "All".to_owned(),
                    chosen: inv.options.pack == PackChoice::All || searching,
                    ..PackShown::default()
                }),
                _ => all_packs.get(i - 1).map(|p| PackShown {
                    id: p.id,
                    icon: objects.get(p.id).filter(|_| !p.main).and_then(|o| {
                        icons.item_icon(
                            o.item_type,
                            o.icon_id,
                            o.icon_underlay_id,
                            o.icon_overlay_id,
                            o.effects,
                        )
                    }),
                    name: p.name.clone(),
                    fill_words: p.fill_words(),
                    fill: p.fill(),
                    chosen: !searching
                        && match inv.options.pack {
                            PackChoice::Main => p.main,
                            PackChoice::Side(id) => id == p.id,
                            PackChoice::All => false,
                        },
                }),
            };
            let Some(shown) = shown else {
                ui.set_visible(b, false);
                continue;
            };
            let h = PackButton::height(face);
            if let Some(e) = ui.element_mut(b) {
                e.top = y;
                e.visible = true;
            }
            y += h + if i == 0 { 12.0 } else { 6.0 };
            if let Some(button) = ui.widget_mut::<PackButton>(b) {
                button.shown = shown;
            }
        }
        // A side pack given away or emptied out of the rail: back to the
        // main pack.
        if let PackChoice::Side(id) = inv.options.pack
            && !all_packs.iter().any(|p| p.id == id)
        {
            inv.options.pack = PackChoice::Main;
        }
        // The details.
        let item = selected
            .filter(|&id| id != player && objects.is_owned_by_object(id, player))
            .and_then(|id| objects.get(id));
        let docked = item.is_some();
        if let Some(o) = item {
            let id = o.object_id();
            if !inv.appraisals.contains_key(&id) && inv.asked.insert(id) {
                inv.quiet.insert(id);
                emit(
                    ui,
                    PanelOutput::Session(SessionCommand::Item(ItemCommand::Appraise(
                        ObjectRef::Guid(id),
                    ))),
                );
            }
        }
        let width = if docked { inv::WIDE } else { inv::NARROW };
        if let Some(e) = ui.element_mut(frame) {
            e.width = width;
        }
        if let Some(e) = ui.element_mut(inv.search) {
            e.left = inv::search_x(width) + 30.0;
        }
        ui.set_visible(inv.details, docked);
        if let Some(o) = item {
            let numbers = facts.numbers();
            let book = numbers.and_then(|n| n.spellbook());
            let spell = |sid| book.and_then(|b| b.try_get_metadata(sid)).cloned();
            let sheet = item_sheet(
                o,
                inv.appraisals.get(&o.object_id()),
                &names,
                &spell,
                &status.names,
                numbers,
                &me,
                true,
                objects,
                icons,
            );
            let sheet = super::plugin_notes::with_cards(
                sheet,
                super::plugin_notes::cards(&facts.plugin_ui, &Horizon::BRONZE, o.object_id()),
            );
            let worn = o.currently_equipped_location != EquipMask::NONE;
            let primary = if worn {
                "Take off"
            } else if o.valid_locations != EquipMask::NONE {
                "Equip"
            } else {
                "Use"
            };
            if let Some(s) = ui.widget_mut::<SheetView>(inv.details) {
                s.set(sheet);
                s.foot = Some(Foot {
                    primary: primary.to_owned(),
                    primary_on: true,
                    small: vec![
                        (FootIcon::Give, "Give to your target".to_owned(), true),
                        (FootIcon::Drop, "Drop on the ground".to_owned(), true),
                        (FootIcon::Salvage, "Salvage".to_owned(), true),
                        (
                            FootIcon::More,
                            "More: inscribe, add to a shortcut bar, link in chat".to_owned(),
                            true,
                        ),
                    ],
                });
            }
        }
        // The plugins' band, over the grid's foot; the grid gives it room.
        let rows =
            super::plugin_notes::feet_of(&facts.plugin_ui, ac_plugin_ui::ItemWindow::Inventory);
        let band_h = super::plugin_notes::band_height(rows.len());
        super::plugin_notes::FootBand::show(
            ui,
            inv.band,
            rows,
            inv::GRID_Y + inv::GRID_H - band_h,
            inv::GRID_W + 24.0,
        );
        if let Some(e) = ui.element_mut(inv.grid)
            && (e.height - (inv::GRID_H - band_h)).abs() > 0.5
        {
            e.height = inv::GRID_H - band_h;
        }
        // The frame.
        let burden = model::Burden::of(objects, player, strength);
        let hint = if let Some(o) = lifted {
            if lit_slots.is_empty() {
                format!("Moving {}", o.name)
            } else {
                "Drop on a highlighted slot to equip \u{b7} Esc cancels".to_owned()
            }
        } else if docked {
            "Double-click to use \u{b7} right-click for more".to_owned()
        } else {
            "Select an item for details".to_owned()
        };
        let options = inv.options.clone();
        let look = inv.look;
        if let Some(f) = ui.widget_mut::<InventoryFrame>(frame) {
            f.view = inv::FrameView {
                name: me,
                details: docked,
                title: grid_view.title,
                count: grid_view.count,
                grouped: options.pack == PackChoice::All || searching,
                group: options.group,
                sort: options.sort,
                look,
                kind: options.kind,
                burden,
                pyreals: model::pyreals(objects, player),
                hint,
                dragging: lifted.is_some(),
            };
        }
    }

    /// A drag over a pack button: whether it would be taken.
    fn answer_pack_drags(&mut self) {
        let Some(inv) = self.modern.as_ref().and_then(|m| m.inventory.as_ref()) else {
            return;
        };
        let packs = inv.packs.clone();
        let payload = self
            .host
            .root
            .drag_payload()
            .and_then(|p| p.get::<ItemDragPayload>().cloned());
        if let Some(p) = payload {
            for b in packs {
                let over = self
                    .host
                    .root
                    .widget::<PackButton>(b)
                    .filter(|x| x.over)
                    .map(|x| (x.face, x.shown.id));
                if let Some((face, id)) = over {
                    let spot = if face == PackFace::Main {
                        DropSpot::MainPack
                    } else {
                        DropSpot::SidePack
                    };
                    let takes = self.with_items_view(|view, sel| {
                        pack_drops::acceptance(view, sel, spot, id, &p)
                            != ac_ui::event::DragAcceptance::Reject
                    });
                    if let Some(x) = self.host.root.widget_mut::<PackButton>(b) {
                        x.takes = Some(takes);
                    }
                }
            }
        }
    }
}

/// The shortcut keys shown on items on retail's shortcut bar (its first
/// nine slots are 1 to 9).
fn shortcut_keys(store: &ac_core::items::ShortcutStore) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    for slot in 0..9 {
        let id = store.get(slot);
        if id != 0 {
            out.entry(id).or_insert_with(|| (slot + 1).to_string());
        }
    }
    out
}
