//! The inventory window: a header with search and close; the equipment
//! (slots round the figure, the jewelry row, the held and Aetheria row);
//! the pack rail; the grid's title, Group and Sort, the compact and list
//! buttons and the filter chips; the item details on the right; burden and
//! pyreals in the foot. The frame draws the chrome and answers its own
//! buttons; the cells, the grid, the pack buttons, the search field and the
//! details are elements of their own inside it.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{DragPayload, ItemDragPayload, ItemDragSource, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::{fill_glass, hex};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::cell::CellLook;
use super::fit;
use super::model::{Burden, GroupBy, Kind, SortBy};
use crate::ui::modern::hud::lines::stroke;

/// The window's size with the details docked, and without them.
pub const WIDE: f32 = 1080.0;
pub const NARROW: f32 = 792.0;
pub const HEIGHT: f32 = 640.0;
/// The header's and the foot's heights.
const HEADER: f32 = 52.0;
const FOOT: f32 = 58.0;
/// The equipment column.
pub const EQUIP_X: f32 = 24.0;
pub const EQUIP_Y: f32 = 68.0;
pub const SLOT: f32 = 40.0;
pub const SLOT_GAP: f32 = 6.0;
/// The figure between the two columns of slots.
pub const FIGURE_X: f32 = EQUIP_X + SLOT + 8.0;
pub const FIGURE_W: f32 = 204.0;
pub const FIGURE_Y: f32 = EQUIP_Y + 34.0;
pub const FIGURE_H: f32 = 270.0;
/// The rows under the figure: their titles' and slots' tops.
pub const JEWELRY_TITLE: f32 = FIGURE_Y + FIGURE_H + 10.0;
pub const JEWELRY_Y: f32 = JEWELRY_TITLE + 16.0;
pub const HELD_TITLE: f32 = JEWELRY_Y + SLOT + 10.0;
pub const HELD_Y: f32 = HELD_TITLE + 16.0;
/// The pack rail.
pub const RAIL_X: f32 = 348.0;
pub const RAIL_W: f32 = 56.0;
/// The grid's column.
pub const GRID_X: f32 = RAIL_X + RAIL_W + 12.0;
pub const GRID_W: f32 = 352.0;
const TOOLBAR_Y: f32 = EQUIP_Y + 26.0;
const CHIPS_Y: f32 = TOOLBAR_Y + 34.0;
pub const GRID_Y: f32 = CHIPS_Y + 30.0;
pub const GRID_H: f32 = HEIGHT - FOOT - 8.0 - GRID_Y;
/// The details.
pub const DETAILS_X: f32 = 792.0;
pub const DETAILS_W: f32 = 264.0;
pub const DETAILS_H: f32 = HEIGHT - FOOT - 8.0 - EQUIP_Y;
/// The search box.
pub const SEARCH_W: f32 = 240.0;
/// The details as a card over the equipment column, when the wide window
/// does not fit the screen.
pub const CARD_X: f32 = 16.0;
pub const CARD_Y: f32 = EQUIP_Y - 4.0;
pub const CARD_W: f32 = RAIL_X - 8.0 - CARD_X;
pub const CARD_H: f32 = HEIGHT - FOOT - 8.0 - CARD_Y;
/// The card's title strip, over the sheet.
const CARD_TITLE: f32 = 32.0;

/// Where the sheet sits in the card.
#[must_use]
pub fn card_sheet_rect() -> (f32, f32, f32, f32) {
    (
        CARD_X + 6.0,
        CARD_Y + CARD_TITLE,
        CARD_W - 12.0,
        CARD_H - CARD_TITLE - 6.0,
    )
}

/// The search box's left edge in a window `w` wide.
#[must_use]
pub fn search_x(w: f32) -> f32 {
    w - 16.0 - 30.0 - 12.0 - SEARCH_W
}

/// The fonts the frame draws with.
#[derive(Debug, Clone, Default)]
pub struct FrameFonts {
    /// The window's title.
    pub title: Option<Arc<UiFont>>,
    /// The grid's title, buttons' chosen words.
    pub strong: Option<Arc<UiFont>>,
    /// Buttons' words, the character's name, counts.
    pub body: Option<Arc<UiFont>>,
    /// Small capitals.
    pub label: Option<Arc<UiFont>>,
    /// The burden's small words.
    pub small: Option<Arc<UiFont>>,
    /// The filter chips.
    pub chip: Option<Arc<UiFont>>,
}

/// What the frame shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameView {
    /// The character.
    pub name: String,
    /// The details are docked (the window is wide).
    pub details: bool,
    /// The details are a card over the equipment column (a small
    /// screen); the equipment is hidden under it.
    pub card: bool,
    /// The grid's title and count.
    pub title: String,
    pub count: String,
    /// Group applies (all packs, or a search).
    pub grouped: bool,
    pub group: GroupBy,
    pub sort: SortBy,
    pub look: CellLook,
    pub kind: Option<Kind>,
    pub burden: Burden,
    pub pyreals: i64,
    /// The foot's hint.
    pub hint: String,
    /// A drag is under way: the hint names it.
    pub dragging: bool,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameRequest {
    /// Close it.
    Close,
    /// The Group menu, at this screen point.
    GroupMenu((f32, f32)),
    /// The Sort and Tidy menu, at this screen point.
    SortMenu((f32, f32)),
    /// Cells drawn so.
    Look(CellLook),
    /// One kind shown (none: all).
    Kind(Option<Kind>),
    /// Put the details card away.
    CloseCard,
}

/// The inventory's frame.
#[derive(Debug)]
pub struct InventoryFrame {
    theme: Horizon,
    fonts: FrameFonts,
    /// What it shows.
    pub view: FrameView,
    /// What it asked for since the window last looked.
    pub requests: Vec<FrameRequest>,
    hover: Option<(f32, f32)>,
}

/// A button's rectangle.
type Rect = (f32, f32, f32, f32);

fn inside((x, y, w, h): Rect, px: f32, py: f32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

impl InventoryFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: FrameFonts) -> Self {
        Self {
            theme,
            fonts,
            view: FrameView::default(),
            requests: Vec::new(),
            hover: None,
        }
    }

    fn width(&self) -> f32 {
        if self.view.details { WIDE } else { NARROW }
    }

    fn close_rect(&self) -> Rect {
        (self.width() - 16.0 - 30.0, 11.0, 30.0, 30.0)
    }

    fn card_rect() -> Rect {
        (CARD_X, CARD_Y, CARD_W, CARD_H)
    }

    fn card_close_rect() -> Rect {
        (CARD_X + CARD_W - 6.0 - 26.0, CARD_Y + 3.0, 26.0, 26.0)
    }

    fn button_width(&self, label: &str, value: &str) -> f32 {
        let m = |f: &Option<Arc<UiFont>>, s: &str| {
            f.as_ref().map_or(40.0, |f| text::measure(f, s, 0.0))
        };
        8.0 + m(&self.fonts.body, label) + 6.0 + m(&self.fonts.strong, value) + 6.0 + 8.0 + 8.0
    }

    fn group_rect(&self) -> Option<Rect> {
        self.view.grouped.then(|| {
            (
                GRID_X,
                TOOLBAR_Y,
                self.button_width("Group", self.view.group.words()),
                26.0,
            )
        })
    }

    fn sort_rect(&self) -> Rect {
        let x = self.group_rect().map_or(GRID_X, |r| r.0 + r.2 + 6.0);
        (
            x,
            TOOLBAR_Y,
            self.button_width("Sort", self.view.sort.words()),
            26.0,
        )
    }

    fn look_rects(&self) -> [Rect; 2] {
        let right = GRID_X + GRID_W - 12.0;
        [
            (right - 26.0 - 6.0 - 26.0, TOOLBAR_Y, 26.0, 26.0),
            (right - 26.0, TOOLBAR_Y, 26.0, 26.0),
        ]
    }

    fn chip_rects(&self) -> Vec<(Option<Kind>, Rect)> {
        let mut x = GRID_X;
        let mut out = Vec::new();
        let kinds = std::iter::once(None).chain(Kind::ALL.into_iter().map(Some));
        for k in kinds {
            let words = k.map_or("All", Kind::words);
            let w = self
                .fonts
                .chip
                .as_ref()
                .map_or(40.0, |f| text::measure(f, words, 0.0))
                + 16.0;
            out.push((k, (x, CHIPS_Y, w, 22.0)));
            x += w + 4.0;
        }
        out
    }

    fn hot(&self, r: Rect) -> bool {
        self.hover.is_some_and(|(x, y)| inside(r, x, y))
    }
}

/// A down-pointing caret centred on (`cx`, `cy`).
fn caret(rc: &mut RenderContext<'_>, cx: f32, cy: f32, color: Color) {
    let v = |x: f32, y: f32| ac_ui::draw::ColorVertex { x, y, color };
    let _ = rc.fill_convex_polygon(&[
        v(cx - 3.5, cy - 2.0),
        v(cx + 3.5, cy - 2.0),
        v(cx, cy + 2.5),
    ]);
}

/// The bag drawn in the header and on the main pack's button.
pub const BAG: &[&[(f32, f32)]] = &[
    &[
        (5.0, 8.0),
        (19.0, 8.0),
        (18.0, 20.0),
        (6.0, 20.0),
        (5.0, 8.0),
    ],
    &[
        (9.0, 8.0),
        (9.0, 6.0),
        (10.0, 4.0),
        (12.0, 3.0),
        (14.0, 4.0),
        (15.0, 6.0),
        (15.0, 8.0),
    ],
];

/// The stack of layers on the All packs button.
pub const STACK: &[&[(f32, f32)]] = &[
    &[
        (12.0, 3.0),
        (2.0, 8.0),
        (12.0, 13.0),
        (22.0, 8.0),
        (12.0, 3.0),
    ],
    &[(2.0, 13.0), (12.0, 18.0), (22.0, 13.0)],
    &[(2.0, 17.5), (12.0, 22.5), (22.0, 17.5)],
];

impl Widget for InventoryFrame {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
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
                let (sx, sy) = ui.screen_position(me);
                if inside(self.close_rect(), x, y) {
                    self.requests.push(FrameRequest::Close);
                } else if self.view.card && inside(Self::card_close_rect(), x, y) {
                    self.requests.push(FrameRequest::CloseCard);
                } else if self.view.card && inside(Self::card_rect(), x, y) {
                    // The card's own title strip: nothing.
                } else if let Some(r) = self.group_rect().filter(|r| inside(*r, x, y)) {
                    self.requests
                        .push(FrameRequest::GroupMenu((sx + r.0, sy + r.1 + r.3 + 4.0)));
                } else if inside(self.sort_rect(), x, y) {
                    let r = self.sort_rect();
                    self.requests
                        .push(FrameRequest::SortMenu((sx + r.0, sy + r.1 + r.3 + 4.0)));
                } else if let Some(i) = self.look_rects().iter().position(|r| inside(*r, x, y)) {
                    let look = match (i, self.view.look) {
                        (0, CellLook::Compact) => CellLook::Grid,
                        (0, _) => CellLook::Compact,
                        (_, CellLook::List) => CellLook::Grid,
                        _ => CellLook::List,
                    };
                    self.requests.push(FrameRequest::Look(look));
                } else if let Some((k, _)) = self
                    .chip_rects()
                    .into_iter()
                    .find(|(_, r)| inside(*r, x, y))
                {
                    self.requests.push(FrameRequest::Kind(k));
                } else if self.view.card {
                    // A click on empty space puts the card away.
                    self.requests.push(FrameRequest::CloseCard);
                }
                true
            }
            _ => false,
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        let (x, y) = self.hover?;
        let looks = self.look_rects();
        if inside(looks[0], x, y) {
            return Some("Smaller icons".to_owned());
        }
        if inside(looks[1], x, y) {
            return Some("List with names".to_owned());
        }
        if inside(self.close_rect(), x, y) {
            return Some("Close".to_owned());
        }
        if self.view.card && inside(Self::card_close_rect(), x, y) {
            return Some("Close the details".to_owned());
        }
        None
    }

    #[allow(clippy::too_many_lines)]
    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let w = self.width();
        let h = HEIGHT;
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 18.0, 50.0, hex(0, 0.55));
        fill_glass(rc, (0.0, 0.0, w, h), round, tone::ground(0.97));
        // The header.
        rc.fill_vertical_gradient(
            1.0,
            1.0,
            w - 2.0,
            HEADER - 1.0,
            CornerRadii::new(5.0, 5.0, 0.0, 0.0),
            tone::bronze(0.10),
            tone::bronze(0.0),
        );
        rc.fill(0.0, HEADER - 1.0, w, 1.0, tone::bronze(0.35));
        stroke(rc, 20.0, 17.0, 18.0, BAG, p.accent, 1.8);
        let mut x = 48.0;
        if let Some(t) = &f.title {
            text::draw(
                rc,
                t,
                "Inventory",
                x,
                (HEADER - t.line_height) * 0.5,
                TextStyle::plain(p.text),
            );
            x += text::measure(t, "Inventory", 0.0) + 12.0;
        }
        if let Some(b) = &f.body {
            text::draw(
                rc,
                b,
                &fit(b, &self.view.name, search_x(w) - x - 12.0),
                x,
                ((HEADER - b.line_height) * 0.5).round() + 1.0,
                TextStyle::plain(p.text_soft),
            );
        }
        // The search box (its field is a child).
        let sx = search_x(w);
        let r = CornerRadii::all(4.0);
        rc.fill_rounded_rect(sx, 11.0, SEARCH_W, 30.0, r, hex(0, 0.35));
        rc.stroke_rounded_rect(sx, 11.0, SEARCH_W, 30.0, r, tone::bronze(0.45), 1.0);
        stroke(
            rc,
            sx + 9.0,
            19.0,
            14.0,
            &[
                &[
                    (11.0, 4.0),
                    (15.9, 6.1),
                    (18.0, 11.0),
                    (15.9, 15.9),
                    (11.0, 18.0),
                    (6.1, 15.9),
                    (4.0, 11.0),
                    (6.1, 6.1),
                    (11.0, 4.0),
                ],
                &[(16.0, 16.0), (21.0, 21.0)],
            ],
            p.text_soft,
            1.6,
        );
        let close = self.close_rect();
        stroke(
            rc,
            close.0 + 7.0,
            close.1 + 7.0,
            16.0,
            &[&[(6.0, 6.0), (18.0, 18.0)], &[(18.0, 6.0), (6.0, 18.0)]],
            if self.hot(close) { p.accent } else { p.icon },
            1.8,
        );
        if self.view.card {
            // The details' card, over the equipment column.
            let (cx, cy, cw, ch) = Self::card_rect();
            let round = CornerRadii::all(self.theme.metrics.window_radius);
            rc.soft_shadow(cx, cy, cw, ch, 6.0, 8.0, 24.0, hex(0, 0.5));
            rc.fill_rounded_rect(cx, cy, cw, ch, round, tone::ground(1.0));
            rc.stroke_rounded_rect(cx, cy, cw, ch, round, tone::bronze(0.45), 1.0);
            if let Some(l) = &f.label {
                let tracked = TextStyle::plain(p.text_soft).tracked(1.2);
                text::draw(rc, l, "ITEM DETAILS", cx + 12.0, cy + 12.0, tracked);
            }
            let close = Self::card_close_rect();
            stroke(
                rc,
                close.0 + 5.0,
                close.1 + 5.0,
                16.0,
                &[&[(6.0, 6.0), (18.0, 18.0)], &[(18.0, 6.0), (6.0, 18.0)]],
                if self.hot(close) { p.accent } else { p.icon },
                1.6,
            );
        } else {
            // The equipment's labels and the figure's place.
            if let Some(l) = &f.label {
                let tracked = TextStyle::plain(p.text_soft).tracked(1.2);
                text::draw(rc, l, "EQUIPPED", EQUIP_X, EQUIP_Y + 6.0, tracked);
                text::draw(rc, l, "JEWELRY", EQUIP_X, JEWELRY_TITLE, tracked);
                text::draw(rc, l, "HELD \u{b7} AETHERIA", EQUIP_X, HELD_TITLE, tracked);
            }
            rc.fill_rounded_rect(FIGURE_X, FIGURE_Y, FIGURE_W, FIGURE_H, r, tone::gold(0.05));
        }
        // The rail's and the grid's dividing room.
        if let (Some(s), Some(b)) = (&f.strong, &f.body) {
            text::draw(
                rc,
                s,
                &fit(s, &self.view.title, 200.0),
                GRID_X,
                EQUIP_Y,
                TextStyle::plain(p.text),
            );
            let cw = text::measure(b, &self.view.count, 0.0);
            text::draw(
                rc,
                b,
                &self.view.count,
                GRID_X + GRID_W - 12.0 - cw,
                EQUIP_Y + 1.0,
                TextStyle::plain(p.text_soft),
            );
        }
        // Group and Sort.
        let mut buttons = Vec::new();
        if let Some(g) = self.group_rect() {
            buttons.push((g, "Group", self.view.group.words(), false));
        }
        buttons.push((
            self.sort_rect(),
            "Sort",
            self.view.sort.words(),
            self.view.sort != SortBy::Mine,
        ));
        for ((bx, by, bw, bh), label, value, on) in buttons {
            let edge = if on || self.hot((bx, by, bw, bh)) {
                p.accent
            } else {
                tone::bronze(0.45)
            };
            let fill = if on { tone::gold(0.18) } else { hex(0, 0.35) };
            rc.fill_rounded_rect(bx, by, bw, bh, r, fill);
            rc.stroke_rounded_rect(bx, by, bw, bh, r, edge, 1.0);
            if let (Some(b), Some(s)) = (&f.body, &f.strong) {
                let ty = by + (bh - b.line_height) * 0.5;
                text::draw(rc, b, label, bx + 8.0, ty, TextStyle::plain(p.text_soft));
                let vx = bx + 8.0 + text::measure(b, label, 0.0) + 6.0;
                text::draw(rc, s, value, vx, ty, TextStyle::plain(p.text));
                caret(rc, bx + bw - 12.0, by + bh * 0.5, p.text_soft);
            }
        }
        // Compact and list.
        let looks = self.look_rects();
        for (i, rect) in looks.iter().enumerate() {
            let on = matches!(
                (i, self.view.look),
                (0, CellLook::Compact) | (1, CellLook::List)
            );
            let (bx, by, bw, bh) = *rect;
            if on {
                rc.fill_rounded_rect(bx, by, bw, bh, r, tone::gold(0.18));
            }
            let edge = if on || self.hot(*rect) {
                p.accent
            } else {
                tone::bronze(0.3)
            };
            rc.stroke_rounded_rect(bx, by, bw, bh, r, edge, 1.0);
            if i == 0 {
                for row in 0..3 {
                    for col in 0..3 {
                        rc.fill(
                            bx + 7.0 + col as f32 * 4.5,
                            by + 7.0 + row as f32 * 4.5,
                            3.0,
                            3.0,
                            p.icon,
                        );
                    }
                }
            } else {
                for row in 0..3 {
                    let ly = by + 8.0 + row as f32 * 5.0;
                    rc.fill(bx + 7.0, ly, 2.0, 2.0, p.icon);
                    rc.fill(bx + 11.0, ly, 9.0, 1.5, p.icon);
                }
            }
        }
        // The chips.
        if let Some(s) = &f.chip {
            for (k, (cx, cy, cw, ch)) in self.chip_rects() {
                let on = k == self.view.kind;
                let pill = CornerRadii::all(ch * 0.5);
                if on {
                    rc.fill_rounded_rect(cx, cy, cw, ch, pill, tone::gold(0.18));
                }
                let edge = if on || self.hot((cx, cy, cw, ch)) {
                    p.accent
                } else {
                    tone::bronze(0.3)
                };
                rc.stroke_rounded_rect(cx, cy, cw, ch, pill, edge, 1.0);
                text::draw_centred(
                    rc,
                    s,
                    k.map_or("All", Kind::words),
                    cx + cw * 0.5,
                    cy + (ch - s.line_height) * 0.5,
                    TextStyle::plain(if on { p.text } else { p.text_soft }),
                );
            }
        }
        // The details' well when docked.
        if self.view.details {
            rc.fill_rounded_rect(DETAILS_X, EQUIP_Y, DETAILS_W, DETAILS_H, r, hex(0, 0.28));
            rc.stroke_rounded_rect(
                DETAILS_X,
                EQUIP_Y,
                DETAILS_W,
                DETAILS_H,
                r,
                tone::bronze(0.35),
                1.0,
            );
        }
        // The foot: burden and pyreals.
        let fy = h - FOOT;
        rc.fill(0.0, fy, w, 1.0, tone::bronze(0.35));
        let b = self.view.burden;
        let slowed = b.slowed();
        let color = if slowed {
            hex(0xFF_8A5B, 1.0)
        } else {
            p.accent
        };
        if let (Some(l), Some(s), Some(body)) = (&f.label, &f.small, &f.body) {
            let ty = fy + 10.0;
            text::draw(
                rc,
                l,
                "BURDEN",
                24.0,
                ty + 1.0,
                TextStyle::plain(p.text_soft).tracked(1.2),
            );
            let pct = format!("{}%", b.percent);
            let lw = text::measure(l, "BURDEN", 1.2);
            text::draw(
                rc,
                body,
                &pct,
                24.0 + lw + 8.0,
                ty - 1.0,
                TextStyle::plain(color),
            );
            let units = format!(
                "{} of {}",
                ac_ui::panels::appraisal::group_thousands(i64::from(b.carried)),
                ac_ui::panels::appraisal::group_thousands(i64::from(b.capacity))
            );
            let uw = text::measure(s, &units, 0.0);
            text::draw(
                rc,
                s,
                &units,
                24.0 + 420.0 - uw,
                ty,
                TextStyle::plain(p.text_soft),
            );
            let by = ty + 16.0;
            let pill = CornerRadii::all(3.0);
            rc.fill_rounded_rect(24.0, by, 420.0, 6.0, pill, hex(0xFF_FFFF, 0.1));
            let share = (b.percent as f32 / 300.0).clamp(0.0, 1.0);
            rc.fill_rounded_rect(24.0, by, 420.0 * share, 6.0, pill, color);
            rc.fill(24.0 + 140.0 - 1.0, by - 3.0, 2.0, 12.0, tone::words(0.6));
            let marks = by + 9.0;
            text::draw(rc, s, "0%", 24.0, marks, TextStyle::plain(p.text_soft));
            text::draw(
                rc,
                s,
                "100% \u{b7} slowed",
                24.0 + 140.0 - 4.0,
                marks,
                TextStyle::plain(p.text_soft),
            );
            let tw = text::measure(s, "300%", 0.0);
            text::draw(
                rc,
                s,
                "300%",
                24.0 + 420.0 - tw,
                marks,
                TextStyle::plain(p.text_soft),
            );
            // Pyreals.
            let px = 24.0 + 420.0 + 24.0;
            let cy = fy + FOOT * 0.5;
            rc.fill_ellipse(px, cy - 5.0, 10.0, 10.0, p.accent);
            let coins = ac_ui::panels::appraisal::group_thousands(self.view.pyreals);
            let strong = f.strong.as_ref().unwrap_or(body);
            text::draw(
                rc,
                strong,
                &coins,
                px + 16.0,
                cy - strong.line_height * 0.5,
                TextStyle::plain(p.text),
            );
            let cw = text::measure(strong, &coins, 0.0);
            text::draw(
                rc,
                body,
                "pyreals",
                px + 16.0 + cw + 6.0,
                cy - body.line_height * 0.5,
                TextStyle::plain(p.text_soft),
            );
            // The hint.
            let hint = if slowed && !self.view.dragging {
                "You are slowed. Drop, sell or salvage something."
            } else {
                self.view.hint.as_str()
            };
            // Right of the pyreals, on two lines where one does not fit
            // (the narrow window).
            let room =
                w - 24.0 - (px + 16.0 + cw + 6.0 + text::measure(body, "pyreals", 0.0) + 16.0);
            let lines = ac_ui::modern::kit::wrap(s, hint, room.max(40.0));
            let top = cy - s.line_height * lines.len() as f32 * 0.5;
            for (i, line) in lines.iter().take(2).enumerate() {
                let lw = text::measure(s, line, 0.0);
                text::draw(
                    rc,
                    s,
                    line,
                    w - 24.0 - lw,
                    top + s.line_height * i as f32,
                    TextStyle::plain(p.text_soft),
                );
            }
        }
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, tone::bronze(0.8), 1.0);
    }
}

/// What a pack button stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackFace {
    /// Every pack.
    All,
    /// The main pack.
    Main,
    /// A side pack.
    Side,
}

/// What a pack button shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PackShown {
    /// The pack (the player for the main pack; 0 for All).
    pub id: u32,
    /// Its icon (side packs).
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// "22/24" ("All" for All packs).
    pub fill_words: String,
    /// How full, 0..1.
    pub fill: f32,
    /// Chosen.
    pub chosen: bool,
}

/// What a pack button asks of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackRequest {
    /// Show it.
    Choose,
    /// An item was let go on it.
    Dropped(ItemDragPayload),
}

/// A button on the pack rail; it takes item drops.
#[derive(Debug)]
pub struct PackButton {
    font: Option<Arc<UiFont>>,
    /// What it stands for.
    pub face: PackFace,
    /// What it shows.
    pub shown: PackShown,
    /// Whether a drop would be taken (the window answers while it hovers).
    pub takes: Option<bool>,
    /// What it asked for since the window last looked.
    pub requests: Vec<PackRequest>,
    /// A drag hovers it.
    pub over: bool,
    hovered: bool,
}

impl PackButton {
    /// A button for `face`.
    #[must_use]
    pub fn new(font: Option<Arc<UiFont>>, face: PackFace) -> Self {
        Self {
            font,
            face,
            shown: PackShown::default(),
            takes: None,
            requests: Vec::new(),
            over: false,
            hovered: false,
        }
    }

    /// Its height on the rail.
    #[must_use]
    pub fn height(face: PackFace) -> f32 {
        match face {
            PackFace::Main => 50.0,
            _ => 40.0,
        }
    }
}

fn item_payload(p: Option<&DragPayload>) -> Option<&ItemDragPayload> {
    p?.get::<ItemDragPayload>()
        .filter(|p| p.obj_id != 0 && !p.source_kind.is_alias())
}

impl Widget for PackButton {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        match e.kind {
            kind::HOVER_ENTER | kind::MOUSE_MOVE => {
                self.hovered = true;
                false
            }
            kind::HOVER_LEAVE => {
                self.hovered = false;
                false
            }
            kind::CLICK => {
                self.requests.push(PackRequest::Choose);
                true
            }
            kind::DRAG_ENTER => {
                self.over =
                    item_payload(e.payload.as_ref()).is_some() && self.face != PackFace::All;
                true
            }
            kind::DRAG_OVER => {
                self.over = false;
                self.takes = None;
                true
            }
            kind::DROP_RELEASED => {
                self.over = false;
                self.takes = None;
                if self.face != PackFace::All
                    && let Some(p) = item_payload(e.payload.as_ref())
                {
                    self.requests.push(PackRequest::Dropped(p.clone()));
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, _dt: f64) {
        if self.over && ui.drag_payload().is_none() {
            self.over = false;
            self.takes = None;
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        Some(match self.face {
            PackFace::All => "All packs".to_owned(),
            _ => format!("{} \u{b7} {}", self.shown.name, self.shown.fill_words),
        })
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        let p = Horizon::current().palette;
        let s = &self.shown;
        let r = CornerRadii::all(4.0);
        let lit = self.over && self.takes != Some(false);
        let fill = if s.chosen || lit {
            tone::gold(0.16)
        } else {
            hex(0, 0.3)
        };
        rc.fill_rounded_rect(0.0, 0.0, w, h, r, fill);
        let edge = if s.chosen || lit || self.hovered {
            p.accent
        } else {
            tone::bronze(0.3)
        };
        rc.stroke_rounded_rect(0.0, 0.0, w, h, r, edge, if lit { 2.0 } else { 1.0 });
        if self.over && self.takes == Some(false) {
            rc.fill_rounded_rect(0.0, 0.0, w, h, r, hex(0, 0.35));
        }
        let ink = if s.chosen { p.accent } else { p.text_soft };
        let has_bar = self.face != PackFace::All;
        let icon_size = match self.face {
            PackFace::Main => 18.0,
            PackFace::All => 16.0,
            PackFace::Side => 14.0,
        };
        let block = icon_size + if has_bar { 3.0 + 11.0 + 3.0 + 2.0 } else { 0.0 };
        let top = ((h - block) * 0.5).round();
        match (self.face, s.icon) {
            (PackFace::Side, Some(icon)) => {
                let side = 20.0;
                rc.sprite(
                    icon,
                    (w - side) * 0.5,
                    top - 3.0,
                    side,
                    side,
                    (0.0, 0.0, 1.0, 1.0),
                    Color::WHITE,
                );
            }
            (PackFace::All, _) => {
                stroke(rc, (w - icon_size) * 0.5, top, icon_size, STACK, ink, 1.6)
            }
            _ => stroke(rc, (w - icon_size) * 0.5, top, icon_size, BAG, ink, 1.6),
        }
        if !has_bar {
            return;
        }
        let mut y = top + icon_size + 3.0;
        if let Some(f) = &self.font {
            text::draw_centred(
                rc,
                f,
                &s.fill_words,
                w * 0.5,
                y,
                TextStyle::plain(tone::warm(1.0)),
            );
            y += f.line_height + 2.0;
        }
        let bw = 36.0;
        let bx = (w - bw) * 0.5;
        rc.fill(bx, y, bw, 2.0, hex(0xFF_FFFF, 0.12));
        let bar = if s.fill >= 1.0 { p.bad } else { p.accent };
        rc.fill(bx, y, bw * s.fill.clamp(0.0, 1.0), 2.0, bar);
    }
}

/// Where a drag from the equipment says it came from.
pub const EQUIPMENT: ItemDragSource = ItemDragSource::Equipment;

/// Over the figure: a drag turns it, an item dropped on it is worn, and a
/// click (a press let go without turning it) is a click on the player.
#[derive(Debug, Default)]
pub struct FigureTurn {
    /// How far it is turned, radians.
    pub turn: f32,
    /// Items dropped on it since the window last looked.
    pub dropped: Vec<u32>,
    /// Clicked since the window last looked.
    pub clicked: bool,
    held: Option<i32>,
    /// How far the held press has moved across, pixels.
    travelled: i32,
    over: bool,
}

/// How far a press may move and still be a click rather than a turn.
const CLICK_SLOP: i32 = 4;

impl Widget for FigureTurn {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        match e.kind {
            kind::MOUSE_DOWN => {
                self.held = Some(e.data1);
                self.travelled = 0;
                ui.set_capture(me);
                true
            }
            kind::MOUSE_MOVE => {
                if let Some(x0) = self.held {
                    self.turn += (e.data1 - x0) as f32 * 0.012;
                    self.travelled += (e.data1 - x0).abs();
                    self.held = Some(e.data1);
                }
                true
            }
            kind::MOUSE_UP => {
                if self.held.take().is_some() && self.travelled <= CLICK_SLOP {
                    self.clicked = true;
                }
                true
            }
            kind::CAPTURE_CHANGED => {
                self.held = None;
                true
            }
            kind::DOUBLE_CLICK => {
                self.turn = 0.0;
                true
            }
            kind::DRAG_ENTER => {
                self.over = item_payload(e.payload.as_ref()).is_some();
                true
            }
            kind::DRAG_OVER => {
                self.over = false;
                true
            }
            kind::DROP_RELEASED => {
                self.over = false;
                if let Some(p) = item_payload(e.payload.as_ref()) {
                    self.dropped.push(p.obj_id);
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
        Some(
            "Drag to turn \u{b7} double-click to face you \u{b7} drop gear here to wear it"
                .to_owned(),
        )
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        if !self.over {
            return;
        }
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        let r = CornerRadii::all(4.0);
        rc.fill_rounded_rect(0.0, 0.0, w, h, r, tone::gold(0.08));
        rc.stroke_rounded_rect(0.0, 0.0, w, h, r, tone::gold(0.8), 1.5);
    }
}

#[cfg(test)]
mod figure_tests {
    use super::FigureTurn;
    use ac_ui::UiRoot;
    use ac_ui::event::{UiEvent, kind};
    use ac_ui::tree::Element;

    fn press(moves: &[i32]) -> (bool, f32) {
        let mut ui = UiRoot::new();
        let root = ui.root();
        let me = ui.add(
            root,
            Element::at(0.0, 0.0, 200.0, 250.0),
            FigureTurn::default(),
        );
        ui.send_event(
            me,
            &UiEvent::new(0, me, kind::MOUSE_DOWN).with_data(0, 100, 100),
        );
        for &x in moves {
            ui.send_event(
                me,
                &UiEvent::new(0, me, kind::MOUSE_MOVE).with_data(0, x, 100),
            );
        }
        let last = moves.last().copied().unwrap_or(100);
        ui.send_event(
            me,
            &UiEvent::new(0, me, kind::MOUSE_UP).with_data(0, last, 100),
        );
        let t = ui.widget::<FigureTurn>(me).expect("it");
        (t.clicked, t.turn)
    }

    #[test]
    fn a_press_let_go_in_place_is_a_click_and_a_drag_turns() {
        assert_eq!(press(&[]), (true, 0.0));
        assert!(press(&[102]).0, "a twitch is still a click");
        let (clicked, turn) = press(&[130, 160]);
        assert!(!clicked);
        assert!(turn > 0.5, "{turn}");
    }
}
