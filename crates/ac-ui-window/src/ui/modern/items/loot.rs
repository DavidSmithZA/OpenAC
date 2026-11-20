//! The Loot window, in place of retail's outside-container window: what a
//! corpse, a chest or a pack on the ground holds, as rows. A selected row
//! has Inspect and Loot; a double click loots; a right click offers Loot
//! into each of the player's packs. A chest shows the packs inside it as
//! tabs and takes things dropped on it. The foot gives the player's burden
//! now and with everything looted, and Loot all (main pack first). The
//! window closes the container itself when the player walks away (retail's
//! window does that only while it shows).

use std::sync::Arc;

use ac_core::items::placement::{is_pack, loose_contents};
use ac_core::items::{ClientObject, EquipMask, ItemType};
use ac_runtime::inventory::{ItemCommand, ObjectRef};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{ItemDragPayload, ItemDragSource, UiEvent, kind};
use ac_ui::geometry::AnchorEdges;
use ac_ui::icons::IconSource;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::panels::pack_drops::{self, DropSpot};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::cell::{CellLook, CellRequest, CellView};
use super::fit;
use super::grid::{GridFonts, GridGroup, ItemGrid};
use super::menu::{MenuFonts, PopMenu, Row};
use super::model::{self, Burden};
use crate::ui::modern::hud;
use crate::ui::modern::hud::lines::stroke;
use crate::ui::window::WindowUi;

/// The window's name, width and heights (a corpse; a chest, with tabs and
/// its drop zone).
pub const LOOT: &str = "modern-loot";
pub const WIDTH: f32 = 400.0;
pub const HEIGHT: f32 = 470.0;
pub const CHEST_HEIGHT: f32 = 520.0;
const HEADER: f32 = 56.0;
const TABS: f32 = 36.0;
const DROP: f32 = 56.0;
const FOOT: f32 = 56.0;

/// What the menu's entries mean.
const MENU_LOOT: u32 = 11;
const DO_INSPECT: u32 = 1;
const DO_LOOT: u32 = 2;
/// "Loot into" a pack: the pack's index in the player's packs, from here.
const DO_INTO: u32 = 100;

/// The fonts the frame draws with.
#[derive(Debug, Clone, Default)]
pub struct LootFonts {
    /// The container's name.
    pub title: Option<Arc<UiFont>>,
    /// Its line, the burden words.
    pub small: Option<Arc<UiFont>>,
    /// Tabs and buttons.
    pub strong: Option<Arc<UiFont>>,
    /// The drop zone's words.
    pub body: Option<Arc<UiFont>>,
}

/// A tab: a pack inside the chest (the chest itself first).
#[derive(Debug, Clone, PartialEq)]
pub struct LootTab {
    /// The pack.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Items in it.
    pub count: usize,
}

/// What the frame shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LootView {
    /// The container's icon.
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// Its line ("7 items").
    pub sub: String,
    /// The tabs (none for a corpse).
    pub tabs: Vec<LootTab>,
    /// The tab shown.
    pub open: u32,
    /// It takes things put in (a chest).
    pub takes: bool,
    /// The player's burden now and with everything here.
    pub burden: Burden,
    pub with_all: Burden,
    /// Something is being dragged over the window from elsewhere.
    pub drop_lit: bool,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum LootRequest {
    /// Close it.
    Close,
    /// Show tab `id`.
    Tab(u32),
    /// Loot everything in the tab shown.
    LootAll,
    /// An item was let go on the drop zone.
    Dropped(ItemDragPayload),
}

/// The Loot window's frame.
#[derive(Debug)]
pub struct LootFrame {
    theme: Horizon,
    fonts: LootFonts,
    /// What it shows.
    pub view: LootView,
    /// What it asked for since the window last looked.
    pub requests: Vec<LootRequest>,
    hover: Option<(f32, f32)>,
    over: bool,
}

type Rect = (f32, f32, f32, f32);

fn inside((x, y, w, h): Rect, px: f32, py: f32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

impl LootFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: LootFonts) -> Self {
        Self {
            theme,
            fonts,
            view: LootView::default(),
            requests: Vec::new(),
            hover: None,
            over: false,
        }
    }

    fn height(&self) -> f32 {
        if self.view.takes {
            CHEST_HEIGHT
        } else {
            HEIGHT
        }
    }

    fn close_rect() -> Rect {
        (WIDTH - 10.0 - 30.0, (HEADER - 30.0) * 0.5, 30.0, 30.0)
    }

    fn loot_all_rect(&self) -> Rect {
        let h = self.height();
        (WIDTH - 12.0 - 132.0, h - FOOT + 12.0, 132.0, 32.0)
    }

    fn drop_rect(&self) -> Option<Rect> {
        self.view.takes.then(|| {
            (
                12.0,
                self.height() - FOOT - DROP + 2.0,
                WIDTH - 24.0,
                DROP - 10.0,
            )
        })
    }

    fn tab_rects(&self) -> Vec<(u32, Rect)> {
        let mut x = 12.0;
        let mut out = Vec::new();
        for t in &self.view.tabs {
            let w = self.fonts.strong.as_ref().map_or(60.0, |f| {
                text::measure(f, &t.name, 0.0) + 8.0 + text::measure(f, &t.count.to_string(), 0.0)
            }) + 20.0
                + 18.0;
            out.push((t.id, (x, HEADER + 8.0, w, 28.0)));
            x += w + 4.0;
        }
        out
    }

    /// The list's place in the window.
    #[must_use]
    pub fn list_rect(&self) -> Rect {
        let top = HEADER + if self.view.tabs.is_empty() { 0.0 } else { TABS } + 8.0;
        let bottom = self.height() - FOOT - if self.view.takes { DROP } else { 0.0 } - 4.0;
        (8.0, top, WIDTH - 16.0, (bottom - top).max(40.0))
    }
}

impl Widget for LootFrame {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hover = Some((x, y));
                false
            }
            kind::HOVER_LEAVE => {
                self.hover = None;
                false
            }
            kind::CLICK => {
                if inside(Self::close_rect(), x, y) {
                    self.requests.push(LootRequest::Close);
                } else if inside(self.loot_all_rect(), x, y) {
                    self.requests.push(LootRequest::LootAll);
                } else if let Some((id, _)) =
                    self.tab_rects().into_iter().find(|(_, r)| inside(*r, x, y))
                {
                    self.requests.push(LootRequest::Tab(id));
                }
                true
            }
            kind::DRAG_ENTER => {
                self.over = self.view.takes
                    && e.payload
                        .as_ref()
                        .and_then(|p| p.get::<ItemDragPayload>())
                        .is_some_and(|p| p.source_kind != ItemDragSource::Ground);
                true
            }
            kind::DRAG_OVER => {
                self.over = false;
                true
            }
            kind::DROP_RELEASED => {
                self.over = false;
                if let Some(p) = e
                    .payload
                    .as_ref()
                    .and_then(|p| p.get::<ItemDragPayload>())
                    .filter(|p| {
                        p.source_kind != ItemDragSource::Ground && !p.source_kind.is_alias()
                    })
                    && self.view.takes
                {
                    self.requests.push(LootRequest::Dropped(p.clone()));
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, _dt: f64) {
        if self.over && ui.drag_payload().is_none() {
            self.over = false;
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        let (x, y) = self.hover?;
        if inside(Self::close_rect(), x, y) {
            return Some("Close (walking away closes it too)".to_owned());
        }
        if inside(self.loot_all_rect(), x, y) {
            return Some("Loots everything here into your packs, main pack first".to_owned());
        }
        None
    }

    #[allow(clippy::too_many_lines)]
    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let (w, h) = (WIDTH, self.height());
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        let r = CornerRadii::all(4.0);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 18.0, 50.0, hex(0, 0.55));
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, hex(0x14_100C, 0.97));
        rc.fill_vertical_gradient(
            1.0,
            1.0,
            w - 2.0,
            HEADER - 1.0,
            CornerRadii::new(5.0, 5.0, 0.0, 0.0),
            hex(0xC9_A062, 0.10),
            hex(0xC9_A062, 0.0),
        );
        rc.fill(0.0, HEADER - 1.0, w, 1.0, hex(0xC9_A062, 0.35));
        // The container.
        let tile = (14.0, (HEADER - 34.0) * 0.5);
        rc.fill_rounded_rect(tile.0, tile.1, 34.0, 34.0, r, hex(0, 0.45));
        rc.stroke_rounded_rect(tile.0, tile.1, 34.0, 34.0, r, hex(0xC9_A062, 0.6), 1.0);
        if let Some(icon) = self.view.icon {
            rc.sprite(
                icon,
                tile.0 + 1.0,
                tile.1 + 1.0,
                32.0,
                32.0,
                (0.0, 0.0, 1.0, 1.0),
                Color::WHITE,
            );
        } else {
            stroke(
                rc,
                tile.0 + 8.0,
                tile.1 + 8.0,
                18.0,
                super::inventory::BAG,
                p.accent,
                1.8,
            );
        }
        let x = tile.0 + 34.0 + 10.0;
        let room = WIDTH - x - 50.0;
        if let (Some(t), Some(s)) = (&f.title, &f.small) {
            let top = ((HEADER - t.line_height - 2.0 - s.line_height) * 0.5).round();
            text::draw(
                rc,
                t,
                &fit(t, &self.view.name, room),
                x,
                top,
                TextStyle::plain(p.text),
            );
            text::draw(
                rc,
                s,
                &self.view.sub,
                x,
                top + t.line_height + 2.0,
                TextStyle::plain(p.text_soft),
            );
        }
        let c = Self::close_rect();
        let hot = self.hover.is_some_and(|(hx, hy)| inside(c, hx, hy));
        stroke(
            rc,
            c.0 + 7.0,
            c.1 + 7.0,
            16.0,
            &[&[(6.0, 6.0), (18.0, 18.0)], &[(18.0, 6.0), (6.0, 18.0)]],
            if hot { p.accent } else { p.icon },
            1.8,
        );
        // The tabs.
        if let Some(s) = &f.strong {
            for (id, (tx, ty, tw, th)) in self.tab_rects() {
                let on = id == self.view.open;
                let fill = if on {
                    hex(0xE0_B068, 0.14)
                } else {
                    hex(0, 0.25)
                };
                rc.fill_rounded_rect(tx, ty, tw, th, r, fill);
                let edge = if on { p.accent } else { hex(0xC9_A062, 0.3) };
                rc.stroke_rounded_rect(tx, ty, tw, th, r, edge, 1.0);
                stroke(
                    rc,
                    tx + 9.0,
                    ty + 8.0,
                    12.0,
                    super::inventory::BAG,
                    p.icon,
                    1.6,
                );
                let Some(t) = self.view.tabs.iter().find(|t| t.id == id) else {
                    continue;
                };
                let ny = ty + (th - s.line_height) * 0.5;
                text::draw(rc, s, &t.name, tx + 27.0, ny, TextStyle::plain(p.text));
                let nx = tx + 27.0 + text::measure(s, &t.name, 0.0) + 6.0;
                text::draw(
                    rc,
                    s,
                    &t.count.to_string(),
                    nx,
                    ny,
                    TextStyle::plain(p.muted),
                );
            }
        }
        // The drop zone.
        if let (Some((dx, dy, dw, dh)), Some(b)) = (self.drop_rect(), &f.body) {
            let lit = self.over || self.view.drop_lit;
            if lit {
                rc.fill_rounded_rect(dx, dy, dw, dh, r, hex(0xE0_B068, 0.10));
            }
            let edge = if lit { p.accent } else { hex(0xC9_A062, 0.45) };
            // A dashed edge: short runs round the box.
            let dash: f32 = 6.0;
            let mut t = dx + 4.0;
            while t < dx + dw - 4.0 {
                let run = dash.min(dx + dw - 4.0 - t);
                rc.fill(t, dy, run, 1.5, edge);
                rc.fill(t, dy + dh - 1.5, run, 1.5, edge);
                t += dash * 2.0;
            }
            let mut t = dy + 4.0;
            while t < dy + dh - 4.0 {
                let run = dash.min(dy + dh - 4.0 - t);
                rc.fill(dx, t, 1.5, run, edge);
                rc.fill(dx + dw - 1.5, t, 1.5, run, edge);
                t += dash * 2.0;
            }
            let words = if lit {
                "Let go to put it in"
            } else {
                "Drop items here to put them in"
            };
            text::draw_centred(
                rc,
                b,
                words,
                dx + dw * 0.5,
                dy + (dh - b.line_height) * 0.5,
                TextStyle::plain(if lit { hex(0xF2_D29A, 1.0) } else { p.muted }),
            );
        }
        // The foot.
        let fy = h - FOOT;
        rc.fill(0.0, fy, w, 1.0, hex(0xC9_A062, 0.3));
        if let (Some(s), Some(strong)) = (&f.small, &f.strong) {
            let words = format!(
                "Burden {}% \u{b7} {}% with everything",
                self.view.burden.percent, self.view.with_all.percent
            );
            let (lx, lw) = (12.0, WIDTH - 12.0 - 132.0 - 12.0 - 12.0);
            text::draw(
                rc,
                s,
                &fit(s, &words, lw),
                lx,
                fy + 14.0,
                TextStyle::plain(p.text_soft),
            );
            let by = fy + 14.0 + s.line_height + 4.0;
            let pill = CornerRadii::all(2.0);
            rc.fill_rounded_rect(lx, by, lw, 4.0, pill, hex(0, 0.5));
            let now = (self.view.burden.percent as f32 / 300.0).clamp(0.0, 1.0);
            let all = (self.view.with_all.percent as f32 / 300.0).clamp(0.0, 1.0);
            rc.fill(
                lx + lw * now,
                by,
                (lw * (all - now)).max(0.0),
                4.0,
                p.accent,
            );
            rc.fill_rounded_rect(lx, by, lw * now, 4.0, pill, p.text_soft);
            let (ax, ay, aw, ah) = self.loot_all_rect();
            let hot = self
                .hover
                .is_some_and(|(hx, hy)| inside((ax, ay, aw, ah), hx, hy));
            rc.fill_rounded_rect(ax, ay, aw, ah, r, hex(0, 0.35));
            rc.stroke_rounded_rect(
                ax,
                ay,
                aw,
                ah,
                r,
                if hot { p.accent } else { hex(0xC9_A062, 0.6) },
                1.0,
            );
            let ty = ay + (ah - strong.line_height) * 0.5;
            text::draw(
                rc,
                strong,
                "Loot all",
                ax + 12.0,
                ty,
                TextStyle::plain(p.text),
            );
            // The OPENAC tag: not in the original client.
            let tag = "OPENAC";
            let tw = text::measure(s, tag, 0.6) + 12.0;
            let tx = ax + aw - 10.0 - tw;
            rc.stroke_rounded_rect(
                tx,
                ay + 8.0,
                tw,
                16.0,
                CornerRadii::all(8.0),
                hex(0xC9_A062, 0.5),
                1.0,
            );
            text::draw(
                rc,
                s,
                tag,
                tx + 6.0,
                ay + 8.0 + (16.0 - s.line_height) * 0.5,
                TextStyle::plain(p.accent).tracked(0.6),
            );
        }
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, hex(0xC9_A062, 0.8), 1.0);
    }
}

/// The Loot window's elements and what it remembers.
#[derive(Debug)]
pub struct LootUi {
    /// The window.
    pub frame: ElementId,
    /// The rows.
    pub grid: ElementId,
    /// The right-click menu.
    pub menu: ElementId,
    /// The container shown (0: none).
    pub root: u32,
    /// The tab shown.
    pub open: u32,
    /// It asked to close this container already.
    closing: bool,
    /// Items still to loot (Loot all), one request at a time.
    queue: Vec<u32>,
    /// The packs the menu's "Loot into" entries named.
    into: Vec<u32>,
}

impl WindowUi {
    /// The Loot window, hidden, in place of retail's outside-container
    /// window.
    pub(in crate::ui) fn mount_modern_loot(&mut self) {
        let theme = Horizon::BRONZE;
        let fonts = LootFonts {
            title: self.modern_font(Weight::SemiBold, 14.0),
            small: self.modern_font(Weight::Regular, 10.5),
            strong: self.modern_font(Weight::SemiBold, 12.0),
            body: self.modern_font(Weight::Regular, 12.0),
        };
        let grid_fonts = GridFonts {
            heading: self.modern_font(Weight::SemiBold, 11.0),
            small: self.modern_font(Weight::Regular, 10.5),
            name: self.modern_font(Weight::SemiBold, 12.5),
        };
        let menu_fonts = MenuFonts {
            body: self.modern_font(Weight::Regular, 12.0),
            small: self.modern_font(Weight::Regular, 10.5),
        };
        let frame = match hud::mount_piece(
            &mut self.host.root,
            LOOT,
            (WIDTH, HEIGHT),
            LootFrame::new(theme, fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern loot: {e}"));
                return;
            }
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        let list = ui.widget::<LootFrame>(frame).map_or(
            (8.0, HEADER + 8.0, WIDTH - 16.0, 300.0),
            LootFrame::list_rect,
        );
        let grid = ui.add(
            frame,
            Element::at(list.0, list.1, list.2, list.3).with_anchors(AnchorEdges::NONE),
            ItemGrid::new(theme, grid_fonts),
        );
        ItemGrid::set_look(ui, grid, CellLook::List);
        ItemGrid::set_source(ui, grid, ItemDragSource::Ground, true);
        let menu = PopMenu::mount(ui, theme, menu_fonts);
        ui.hide_window(LOOT);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.loot = Some(LootUi {
            frame,
            grid,
            menu,
            root: 0,
            open: 0,
            closing: false,
            queue: Vec::new(),
            into: Vec::new(),
        });
        modern.replace(crate::ui::container_window::EXTERNAL_CONTAINER_WINDOW);
    }

    fn loot_mut(&mut self) -> Option<&mut LootUi> {
        self.modern.as_mut().and_then(|m| m.loot.as_mut())
    }

    fn send_loot(&mut self, command: ItemCommand) {
        emit(
            &mut self.host.root,
            PanelOutput::Session(SessionCommand::Item(command)),
        );
    }

    /// Item `id` from here into pack `pack` (the main pack: the player),
    /// by retail's pack drop rules (a stack joins one already there).
    fn loot_into(&mut self, id: u32, pack: u32) {
        let player = self.facts.state.player;
        let spot = if pack == player {
            DropSpot::MainPack
        } else {
            DropSpot::SidePack
        };
        let p = ItemDragPayload::new(id, ItemDragSource::Ground, -1);
        let answer = self.with_loot_items_view(|view, selected| {
            pack_drops::answer(view, selected, spot, pack, -1, &p)
        });
        match answer {
            pack_drops::DropAnswer::Nothing => {}
            pack_drops::DropAnswer::Notice(text) => {
                emit(&mut self.host.root, PanelOutput::Notice(text));
            }
            pack_drops::DropAnswer::Send { command, .. } => self.send_loot(command),
        }
    }

    fn with_loot_items_view<R>(
        &mut self,
        f: impl FnOnce(&ac_ui::panels::items::ItemsView<'_>, Option<u32>) -> R,
    ) -> R {
        let selected = self.player_view.and_then(|v| v.selected);
        let pending: Vec<u32> = self.facts.state.pending_item.into_iter().collect();
        let resolve = crate::ui::facts::composed_names(&self.status.names);
        let view = self.facts.items_view(selected, &pending, None, &resolve);
        // A loot moves whole stacks: no split from the toolbar.
        f(&view, None)
    }

    /// Each frame: the container the session says is open, shown; what
    /// was asked carried out; closed when the player walked away.
    #[allow(clippy::too_many_lines)]
    pub(in crate::ui) fn tick_modern_loot(&mut self) {
        let Some(l) = self.modern.as_ref().and_then(|m| m.loot.as_ref()) else {
            return;
        };
        let (frame, grid, menu) = (l.frame, l.grid, l.menu);
        let root = self.facts.state.external_container;
        let in_range = self.facts.state.external_container_in_range;
        // A container opened or closed.
        if root != l.root {
            let ui = &mut self.host.root;
            if root == 0 {
                ui.hide_window(LOOT);
            } else {
                ItemGrid::scroll_home(ui, grid);
                ui.show_window(LOOT);
                ui.bring_to_front(frame);
            }
            if let Some(l) = self.loot_mut() {
                l.root = root;
                l.open = root;
                l.closing = false;
                l.queue.clear();
            }
        }
        if root == 0 {
            return;
        }
        // What the parts asked for.
        let ui = &mut self.host.root;
        let frame_asks = ui
            .widget_mut::<LootFrame>(frame)
            .map(|f| std::mem::take(&mut f.requests))
            .unwrap_or_default();
        let cell_asks = ItemGrid::take_requests(ui, grid);
        let chosen = PopMenu::take(ui, menu);
        let mut close = false;
        for ask in frame_asks {
            match ask {
                LootRequest::Close => close = true,
                LootRequest::Tab(id) => {
                    if let Some(l) = self.loot_mut() {
                        l.open = id;
                    }
                    ItemGrid::scroll_home(&mut self.host.root, grid);
                }
                LootRequest::LootAll => {
                    let open = self.loot_mut().map_or(root, |l| l.open);
                    let mut items = loose_contents(&self.facts.objects, open);
                    items.reverse();
                    if let Some(l) = self.loot_mut() {
                        l.queue = items;
                    }
                }
                LootRequest::Dropped(p) => {
                    let open = self.loot_mut().map_or(root, |l| l.open);
                    let answer = self.with_loot_items_view(|view, sel| {
                        pack_drops::answer(view, sel, DropSpot::Grid { open }, 0, -1, &p)
                    });
                    match answer {
                        pack_drops::DropAnswer::Send { command, .. } => self.send_loot(command),
                        pack_drops::DropAnswer::Notice(t) => {
                            emit(&mut self.host.root, PanelOutput::Notice(t));
                        }
                        pack_drops::DropAnswer::Nothing => {}
                    }
                }
            }
        }
        let player = self.facts.state.player;
        for ask in cell_asks {
            match ask {
                CellRequest::Select(id) => {
                    emit(&mut self.host.root, PanelOutput::SelectObject(id));
                }
                CellRequest::Use(id) | CellRequest::Loot(id) => self.loot_into(id, player),
                CellRequest::Inspect(id) => {
                    self.send_loot(ItemCommand::Appraise(ObjectRef::Guid(id)));
                }
                CellRequest::Menu(id, at) => {
                    let packs = model::packs(&self.facts.objects, player);
                    let mut rows = vec![
                        Row::entry(DO_INSPECT, "Inspect"),
                        Row::entry(DO_LOOT, "Loot").note("double-click"),
                        Row::Line,
                        Row::Header("LOOT INTO".to_owned()),
                    ];
                    for (i, pack) in packs.iter().enumerate() {
                        let free = pack.free();
                        rows.push(
                            Row::entry(DO_INTO + i as u32, pack.name.clone())
                                .note(if free == 0 {
                                    "full".to_owned()
                                } else {
                                    format!("{free} free")
                                })
                                .on(free > 0),
                        );
                    }
                    if let Some(l) = self.loot_mut() {
                        l.into = packs.iter().map(|p| p.id).collect();
                    }
                    emit(&mut self.host.root, PanelOutput::SelectObject(id));
                    PopMenu::open(&mut self.host.root, menu, MENU_LOOT, id, rows, 230.0, at);
                }
                CellRequest::Dropped { item, from, .. } => {
                    if from != ItemDragSource::Ground {
                        let open = self.loot_mut().map_or(root, |l| l.open);
                        let p = ItemDragPayload::new(item, from, -1);
                        let answer = self.with_loot_items_view(|view, sel| {
                            pack_drops::answer(view, sel, DropSpot::Grid { open }, 0, -1, &p)
                        });
                        if let pack_drops::DropAnswer::Send { command, .. } = answer {
                            self.send_loot(command);
                        }
                    }
                }
                CellRequest::Lifted(_) | CellRequest::DragEnded(_) => {}
            }
        }
        if let Some((MENU_LOOT, id, tag)) = chosen {
            match tag {
                DO_INSPECT => self.send_loot(ItemCommand::Appraise(ObjectRef::Guid(id))),
                DO_LOOT => self.loot_into(id, player),
                t if t >= DO_INTO => {
                    let pack = self
                        .loot_mut()
                        .and_then(|l| l.into.get((t - DO_INTO) as usize).copied());
                    if let Some(pack) = pack {
                        self.loot_into(id, pack);
                    }
                }
                _ => {}
            }
        }
        // Out of reach, or closed: the container is closed once.
        if (close || !in_range) && !self.loot_mut().is_some_and(|l| l.closing) {
            if let Some(l) = self.loot_mut() {
                l.closing = true;
                l.queue.clear();
            }
            self.host.root.hide_window(LOOT);
            self.send_loot(ItemCommand::CloseContainer(ObjectRef::Guid(root)));
            return;
        }
        // Loot all: the next item once nothing is in flight.
        if self.facts.state.can_make_request
            && self.facts.state.pending_item.is_none()
            && let Some(next) = self.loot_mut().and_then(|l| l.queue.pop())
            && self.facts.objects.get(next).is_some()
        {
            self.loot_into(next, player);
        }
        self.feed_modern_loot(frame, grid, root);
    }

    fn feed_modern_loot(&mut self, frame: ElementId, grid: ElementId, root: u32) {
        let selected = self.player_view.and_then(|v| v.selected);
        let player = self.facts.state.player;
        let pending = self.facts.state.pending_item;
        let strength = self.facts.strength();
        let open = self
            .modern
            .as_ref()
            .and_then(|m| m.loot.as_ref())
            .map_or(root, |l| l.open);
        let Self {
            host,
            facts,
            icons,
            status,
            ..
        } = self;
        let objects = &facts.objects;
        let ui = &mut host.root;
        let names = crate::ui::facts::composed_names(&status.names);
        let Some(container) = objects.get(root) else {
            return;
        };
        let chest = !container.item_type.intersects(ItemType::CREATURE)
            && container
                .flags()
                .contains(ac_core::items::PublicWeenieFlags::OPENABLE)
            && !container.name.starts_with("Corpse");
        // The tabs: the chest, then the packs inside it.
        let inner: Vec<&ClientObject> = objects
            .get_contents(root)
            .iter()
            .filter_map(|&g| objects.get(g))
            .filter(|o| is_pack(o) && o.currently_equipped_location == EquipMask::NONE)
            .collect();
        let open = if open == root || inner.iter().any(|o| o.object_id() == open) {
            open
        } else {
            root
        };
        let mut tabs = Vec::new();
        if !inner.is_empty() {
            tabs.push(LootTab {
                id: root,
                name: if chest {
                    "Chest".to_owned()
                } else {
                    "Inside".to_owned()
                },
                count: loose_contents(objects, root).len(),
            });
            for o in &inner {
                tabs.push(LootTab {
                    id: o.object_id(),
                    name: o.appropriate_name().into_owned(),
                    count: loose_contents(objects, o.object_id()).len(),
                });
            }
        }
        let items = loose_contents(objects, open);
        let all: Vec<u32> = std::iter::once(root)
            .chain(inner.iter().map(|o| o.object_id()))
            .flat_map(|c| loose_contents(objects, c))
            .collect();
        let total_burden: i32 = all
            .iter()
            .filter_map(|&g| objects.get(g))
            .map(|o| o.burden)
            .sum();
        let count = all.len();
        let sub = if chest && container.items_capacity > 0 {
            format!("{count} of {} items", container.items_capacity)
        } else if count == 1 {
            "1 item".to_owned()
        } else {
            format!("{count} items")
        };
        let cells: Vec<CellView> = items
            .iter()
            .filter_map(|&g| objects.get(g))
            .map(|o| {
                let id = o.object_id();
                let moving = pending == Some(id);
                let sub = if o.item_type.intersects(ItemType::MONEY) {
                    format!("{} \u{b7} burden {}", o.value, o.burden)
                } else if o.value > 0 {
                    format!(
                        "value {} \u{b7} burden {}",
                        ac_ui::panels::appraisal::group_thousands(i64::from(o.value)),
                        o.burden
                    )
                } else {
                    format!("burden {}", o.burden)
                };
                CellView {
                    id,
                    icon: icons.item_icon(
                        o.item_type,
                        o.icon_id,
                        o.icon_underlay_id,
                        o.icon_overlay_id,
                        o.effects,
                    ),
                    count: (o.stack_size > 1).then_some(o.stack_size),
                    stamp: None,
                    name: names(o),
                    sub,
                    name_color: o.attuned.then_some(hex(0xF2_D29A, 1.0)),
                    selected: selected == Some(id),
                    waiting: moving,
                    cooldown: 0.0,
                    faint: false,
                    lit: false,
                    actions: true,
                    note: moving.then(|| "Looting\u{2026}".to_owned()),
                }
            })
            .collect();
        ItemGrid::set_groups(
            ui,
            grid,
            vec![GridGroup {
                key: u64::from(open),
                title: None,
                meta: String::new(),
                cells,
                empty: 0,
                container: Some(open),
            }],
        );
        let burden = Burden::of(objects, player, strength);
        let icon = icons.item_icon(
            container.item_type,
            container.icon_id,
            container.icon_underlay_id,
            container.icon_overlay_id,
            container.effects,
        );
        let view = LootView {
            icon,
            name: names(container),
            sub,
            tabs,
            open,
            takes: chest,
            burden,
            with_all: burden.with(total_burden),
            drop_lit: false,
        };
        let changed = ui
            .widget::<LootFrame>(frame)
            .is_some_and(|f| f.view.takes != view.takes || f.view.tabs.len() != view.tabs.len());
        let list = {
            let Some(f) = ui.widget_mut::<LootFrame>(frame) else {
                return;
            };
            f.view = view;
            f.list_rect()
        };
        if changed
            || ui
                .element(grid)
                .is_some_and(|e| e.top != list.1 || e.height != list.3)
        {
            if let Some(e) = ui.element_mut(grid) {
                e.top = list.1;
                e.height = list.3;
            }
            let h = ui
                .widget::<LootFrame>(frame)
                .map_or(HEIGHT, LootFrame::height);
            if let Some(e) = ui.element_mut(frame) {
                e.height = h;
            }
        }
        if let Some(l) = self.modern.as_mut().and_then(|m| m.loot.as_mut()) {
            l.open = open;
        }
    }
}
