//! The Salvage window's frame, a slim play window beside the inventory:
//! the header (the tool and the player's Salvaging skill), the items to
//! salvage (each with its material and workmanship, taken out with its
//! cross or a double click) over the drop zone (which shows why a drop was
//! refused), the bags they make underneath, the foot (Clear, Salvage N
//! items), Add from packs with its material chips, and what a salvage
//! produced, one bag under another.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{ItemDragPayload, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::salvage_list::drag_stages;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::salvage_model::{MaterialChip, SalvagePick, SalvageScreen};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, ButtonLook, Rect, Scroll, bronze, gold, hot, inside};

/// The window's size.
pub const WIDTH: f32 = 420.0;
pub const HEIGHT: f32 = 600.0;
const HEADER: f32 = 52.0;
const FOOT: f32 = 72.0;
const PAD: f32 = 16.0;
const TOP: f32 = 64.0;
const LEFT_W: f32 = WIDTH - 2.0 * PAD;
const BODY_H: f32 = 300.0;
/// The bags, under the items.
const BAGS_Y: f32 = TOP + BODY_H + 8.0;
const BAGS_H: f32 = HEIGHT - FOOT - 8.0 - BAGS_Y;
const BAG_ROW: f32 = 30.0;
const ROW: f32 = 46.0;
const PICK_X: f32 = 10.0;
const PICK_Y: f32 = 100.0;
const PICK_W: f32 = 400.0;
const PICK_ROW: f32 = 40.0;
const PICK_ROWS: usize = 7;

/// A cross.
const CROSS: &[&[(f32, f32)]] = &[&[(7.0, 7.0), (17.0, 17.0)], &[(17.0, 7.0), (7.0, 17.0)]];
/// A plus.
const PLUS: &[&[(f32, f32)]] = &[&[(12.0, 5.0), (12.0, 19.0)], &[(5.0, 12.0), (19.0, 12.0)]];
/// An anvil-ish glyph: the window's.
const ANVIL: &[&[(f32, f32)]] = &[
    &[
        (3.0, 7.0),
        (21.0, 7.0),
        (21.0, 10.0),
        (16.0, 13.0),
        (16.0, 16.0),
        (8.0, 16.0),
        (8.0, 13.0),
        (3.0, 10.0),
        (3.0, 7.0),
    ],
    &[(6.0, 20.0), (18.0, 20.0)],
    &[(10.0, 16.0), (9.0, 20.0)],
    &[(14.0, 16.0), (15.0, 20.0)],
];

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct SalvageFonts {
    /// The title, the result's figures.
    pub title: Option<Arc<UiFont>>,
    /// Names, buttons.
    pub strong: Option<Arc<UiFont>>,
    /// Quiet lines.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals, chips.
    pub tiny: Option<Arc<UiFont>>,
    /// Words.
    pub body: Option<Arc<UiFont>>,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum SalvageAsk {
    /// Close it.
    Close,
    /// Show or hide Add from packs.
    Pick(bool),
    /// Narrow Add from packs to a material (0: all).
    Chip(u32),
    /// Stage an item.
    Add(u32),
    /// Take one out.
    Remove(u32),
    /// Its full appraisal.
    Inspect(u32),
    /// Empty the list.
    Clear,
    /// Salvage the list.
    Salvage,
    /// After a salvage: go again.
    More,
    /// An item from the packs let go on the window.
    Dropped(ItemDragPayload),
}

/// The Salvage window's frame.
#[derive(Debug)]
pub struct SalvageFrame {
    fonts: SalvageFonts,
    /// What it shows.
    pub screen: SalvageScreen,
    /// Add from packs shows, with these chips and rows, of this material.
    pub picking: bool,
    pub chips: Vec<MaterialChip>,
    pub pick: Vec<SalvagePick>,
    pub chip: u32,
    /// The last drop refused: the item and why.
    pub refused: Option<(String, String)>,
    /// What it asked for since the window last looked.
    pub asks: Vec<SalvageAsk>,
    hover: Option<(f32, f32)>,
    over: bool,
    scroll: Scroll,
    pick_scroll: Scroll,
}

impl SalvageFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(fonts: SalvageFonts) -> Self {
        Self {
            fonts,
            screen: SalvageScreen::default(),
            picking: false,
            chips: Vec::new(),
            pick: Vec::new(),
            chip: 0,
            refused: None,
            asks: Vec::new(),
            hover: None,
            over: false,
            scroll: Scroll::default(),
            pick_scroll: Scroll::default(),
        }
    }

    fn close_rect() -> Rect {
        kit::close_rect(WIDTH, HEADER)
    }

    fn add_button() -> Rect {
        (PAD + LEFT_W - 140.0, TOP, 140.0, 28.0)
    }

    fn rows_room(&self) -> Rect {
        let drop = self.drop_height();
        (PAD, TOP + 36.0, LEFT_W, BODY_H - 36.0 - drop - 6.0)
    }

    fn drop_height(&self) -> f32 {
        if self.screen.rows.is_empty() {
            200.0
        } else {
            52.0
        }
    }

    fn drop_rect(&self) -> Rect {
        let h = self.drop_height();
        (PAD, TOP + BODY_H - h, LEFT_W, h)
    }

    fn row_rects(&self) -> Vec<(u32, Rect, Rect)> {
        let room = self.rows_room();
        self.screen
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let y = room.1 + i as f32 * (ROW + 2.0) - self.scroll.at;
                let row = (room.0, y, room.2, ROW);
                let x = (
                    row.0 + row.2 - 6.0 - 24.0,
                    y + (ROW - 24.0) * 0.5,
                    24.0,
                    24.0,
                );
                (r.id, row, x)
            })
            .collect()
    }

    fn clear_rect() -> Rect {
        (PAD, HEIGHT - FOOT + (FOOT - 34.0) * 0.5, 80.0, 34.0)
    }

    fn go_rect() -> Rect {
        (
            WIDTH - PAD - 190.0,
            HEIGHT - FOOT + (FOOT - 40.0) * 0.5,
            190.0,
            40.0,
        )
    }

    fn done_buttons() -> (Rect, Rect) {
        let y = HEIGHT - FOOT + (FOOT - 40.0) * 0.5;
        let more = (WIDTH - PAD - 150.0, y, 150.0, 40.0);
        let close = (more.0 - 8.0 - 90.0, y, 90.0, 40.0);
        (close, more)
    }

    fn pick_box(&self) -> Rect {
        let rows = self.pick.len().clamp(1, PICK_ROWS) as f32;
        let chips = self
            .chip_rects()
            .last()
            .map_or(24.0, |(_, r)| r.1 + r.3 - (PICK_Y + 44.0));
        (
            PICK_X,
            PICK_Y,
            PICK_W,
            44.0 + chips + 10.0 + rows * (PICK_ROW + 2.0) + 12.0,
        )
    }

    fn chip_rects(&self) -> Vec<(u32, Rect)> {
        let mut x = PICK_X + 12.0;
        let mut y = PICK_Y + 44.0;
        let mut out = Vec::new();
        for c in &self.chips {
            let n = c.count.to_string();
            let w = kit::chip_width(
                &c.name,
                Some(&n),
                self.fonts.tiny.as_ref(),
                self.fonts.tiny.as_ref(),
            ) + 10.0;
            if x + w > PICK_X + PICK_W - 12.0 {
                x = PICK_X + 12.0;
                y += 28.0;
            }
            out.push((c.material, (x, y, w, 24.0)));
            x += w + 4.0;
        }
        out
    }

    fn pick_room(&self) -> Rect {
        let b = self.pick_box();
        let top = self
            .chip_rects()
            .last()
            .map_or(PICK_Y + 44.0, |(_, r)| r.1 + r.3)
            + 10.0;
        (b.0 + 12.0, top, b.2 - 24.0, b.1 + b.3 - 12.0 - top)
    }

    fn pick_rects(&self) -> Vec<(usize, Rect, Rect)> {
        let room = self.pick_room();
        (0..self.pick.len())
            .map(|i| {
                let y = room.1 + i as f32 * (PICK_ROW + 2.0) - self.pick_scroll.at;
                let row = (room.0, y, room.2, PICK_ROW);
                let add = (
                    row.0 + row.2 - 6.0 - 64.0,
                    y + (PICK_ROW - 26.0) * 0.5,
                    64.0,
                    26.0,
                );
                (i, row, add)
            })
            .collect()
    }

    fn pick_close() -> Rect {
        (PICK_X + PICK_W - 12.0 - 24.0, PICK_Y + 10.0, 24.0, 24.0)
    }

    fn click(&mut self, x: f32, y: f32, right: bool, double: bool) {
        if inside(Self::close_rect(), x, y) {
            self.asks.push(SalvageAsk::Close);
            return;
        }
        if self.screen.done.is_some() {
            let (close, more) = Self::done_buttons();
            if inside(close, x, y) {
                self.asks.push(SalvageAsk::Close);
            } else if inside(more, x, y) {
                self.asks.push(SalvageAsk::More);
            }
            return;
        }
        if self.picking {
            if inside(Self::pick_close(), x, y) || !inside(self.pick_box(), x, y) {
                self.asks.push(SalvageAsk::Pick(false));
                return;
            }
            if let Some((m, _)) = self
                .chip_rects()
                .into_iter()
                .find(|(_, r)| inside(*r, x, y))
            {
                self.asks.push(SalvageAsk::Chip(m));
                return;
            }
            let room = self.pick_room();
            if inside(room, x, y)
                && let Some((i, _, _)) = self
                    .pick_rects()
                    .into_iter()
                    .find(|(_, r, _)| inside(*r, x, y))
                && let Some(p) = self.pick.get(i)
                && !p.off
            {
                self.asks.push(SalvageAsk::Add(p.id));
            }
            return;
        }
        if inside(Self::add_button(), x, y) {
            self.asks.push(SalvageAsk::Pick(true));
            return;
        }
        let room = self.rows_room();
        if inside(room, x, y)
            && let Some((id, _, cross)) = self
                .row_rects()
                .into_iter()
                .find(|(_, r, _)| inside(*r, x, y))
        {
            if right {
                self.asks.push(SalvageAsk::Inspect(id));
            } else if double || inside(cross, x, y) {
                self.asks.push(SalvageAsk::Remove(id));
            }
            return;
        }
        if inside(Self::clear_rect(), x, y) && !self.screen.rows.is_empty() {
            self.asks.push(SalvageAsk::Clear);
        } else if inside(Self::go_rect(), x, y) && !self.screen.rows.is_empty() {
            self.asks.push(SalvageAsk::Salvage);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_staging(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        // The heading and Add from packs.
        let mut x = PAD;
        if let Some(st) = &f.strong {
            text::draw(rc, st, "To salvage", x, TOP + 6.0, TextStyle::plain(p.text));
            x += text::measure(st, "To salvage", 0.0) + 8.0;
        }
        if let Some(sm) = &f.small {
            text::draw(
                rc,
                sm,
                &self.screen.count,
                x,
                TOP + 7.0,
                TextStyle::plain(tone::soft(1.0)),
            );
        }
        let a = Self::add_button();
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(
            a.0,
            a.1,
            a.2,
            a.3,
            round,
            hex(0, if hot(self.hover, a) { 0.5 } else { 0.35 }),
        );
        rc.stroke_rounded_rect(a.0, a.1, a.2, a.3, round, bronze(0.6), 1.0);
        stroke(rc, a.0 + 10.0, a.1 + 7.0, 14.0, PLUS, p.text, 1.6);
        if let Some(st) = &f.strong {
            text::draw(
                rc,
                st,
                "Add from packs",
                a.0 + 30.0,
                a.1 + ((a.3 - st.line_height) * 0.5).round(),
                TextStyle::plain(p.text),
            );
        }
        // The rows.
        let room = self.rows_room();
        rc.push_clip(room.0, room.1, room.2, room.3);
        for ((_, row, cross), r) in self.row_rects().into_iter().zip(&self.screen.rows) {
            if hot(self.hover, row) {
                rc.fill_rounded_rect(row.0, row.1, row.2, row.3, round, hex(0xFF_FFFF, 0.04));
            }
            let tile = (row.0 + 6.0, row.1 + 6.0, 34.0, 34.0);
            rc.fill_rounded_rect(tile.0, tile.1, tile.2, tile.3, round, hex(0, 0.45));
            if let Some(i) = r.icon {
                rc.sprite(
                    i,
                    tile.0 + 1.0,
                    tile.1 + 1.0,
                    32.0,
                    32.0,
                    (0.0, 0.0, 1.0, 1.0),
                    Color::WHITE,
                );
            }
            // Material chip, workmanship, cross from the right.
            let ws_x = cross.0 - 8.0 - 36.0;
            let chip_w = f
                .tiny
                .as_ref()
                .map_or(60.0, |t| text::measure(t, &r.material, 0.0))
                + 25.0;
            let chip = (
                ws_x - 8.0 - chip_w,
                row.1 + (ROW - 20.0) * 0.5,
                chip_w,
                20.0,
            );
            rc.fill_rounded_rect(
                chip.0,
                chip.1,
                chip.2,
                chip.3,
                CornerRadii::all(3.0),
                hex(0xFF_FFFF, 0.06),
            );
            rc.fill_rounded_rect(
                chip.0 + 7.0,
                chip.1 + 6.0,
                8.0,
                8.0,
                CornerRadii::all(2.0),
                r.color,
            );
            if let Some(t) = &f.tiny {
                text::draw(
                    rc,
                    t,
                    &r.material,
                    chip.0 + 20.0,
                    chip.1 + ((20.0 - t.line_height) * 0.5).round(),
                    TextStyle::plain(p.text),
                );
            }
            if let Some(st) = &f.strong {
                let w = text::measure(st, &r.workmanship, 0.0);
                text::draw(
                    rc,
                    st,
                    &r.workmanship,
                    ws_x + 36.0 - w,
                    row.1 + ((ROW - st.line_height) * 0.5).round(),
                    TextStyle::plain(p.text),
                );
            }
            stroke(
                rc,
                cross.0 + 5.0,
                cross.1 + 5.0,
                14.0,
                CROSS,
                if hot(self.hover, cross) {
                    p.accent
                } else {
                    tone::soft(1.0)
                },
                1.5,
            );
            let tx = tile.0 + 44.0;
            let room_w = chip.0 - 8.0 - tx;
            if let Some(st) = &f.strong {
                let n = kit::fit(st, &r.name, room_w);
                text::draw(rc, st, &n, tx, row.1 + 7.0, TextStyle::plain(p.text));
            }
            if let Some(sm) = &f.small {
                let s = kit::fit(sm, &r.sub, room_w);
                text::draw(
                    rc,
                    sm,
                    &s,
                    tx,
                    row.1 + 26.0,
                    TextStyle::plain(tone::soft(1.0)),
                );
            }
        }
        rc.pop_clip();
        let total = self.screen.rows.len() as f32 * (ROW + 2.0);
        kit::scrollbar(
            rc,
            room.0 + room.2 - 5.0,
            room.1,
            room.3,
            total,
            self.scroll.at,
        );
        // The drop zone.
        let d = self.drop_rect();
        let bad = self.refused.is_some();
        let edge = if bad {
            hex(0xFF_8A8E, 1.0)
        } else if self.over {
            gold(1.0)
        } else {
            bronze(0.45)
        };
        if bad {
            rc.fill_rounded_rect(d.0, d.1, d.2, d.3, round, hex(0xFF_6B70, 0.10));
        } else if self.over {
            rc.fill_rounded_rect(d.0, d.1, d.2, d.3, round, gold(0.08));
        }
        dashed(rc, d, edge);
        let cx = d.0 + d.2 * 0.5;
        if let Some((name, why)) = &self.refused {
            if let (Some(st), Some(sm)) = (&f.strong, &f.small) {
                let top = d.1 + (d.3 - st.line_height - 4.0 - sm.line_height) * 0.5;
                text::draw_centred(rc, st, name, cx, top, TextStyle::plain(p.text));
                text::draw_centred(
                    rc,
                    sm,
                    why,
                    cx,
                    top + st.line_height + 4.0,
                    TextStyle::plain(hex(0xFF_B3B6, 1.0)),
                );
            }
        } else if let (Some(b), Some(sm)) = (&f.body, &f.small) {
            let words = if self.screen.rows.is_empty() {
                "Drag items here from your packs, or use Add from packs"
            } else {
                "Drag more items here"
            };
            let top = d.1 + (d.3 - b.line_height - 4.0 - sm.line_height) * 0.5;
            text::draw_centred(rc, b, words, cx, top, TextStyle::plain(tone::soft(1.0)));
            text::draw_centred(
                rc,
                sm,
                "A whole pack adds everything in it that can be salvaged.",
                cx,
                top + b.line_height + 4.0,
                TextStyle::plain(tone::muted(1.0)),
            );
        }
    }

    fn draw_bags(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let b = (PAD, BAGS_Y, LEFT_W, BAGS_H);
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(b.0, b.1, b.2, b.3, round, hex(0, 0.28));
        rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, round, bronze(0.35), 1.0);
        if let Some(t) = &f.tiny {
            text::draw(
                rc,
                t,
                "YOU GET ONE BAG PER MATERIAL",
                b.0 + 12.0,
                b.1 + 10.0,
                TextStyle::plain(tone::soft(1.0)).tracked(kit::LABEL_TRACKING),
            );
        }
        let mut y = b.1 + 28.0;
        if self.screen.bags.is_empty()
            && let Some(sm) = &f.body
        {
            text::draw_centred(
                rc,
                sm,
                "Add items to see which bags you get.",
                b.0 + b.2 * 0.5,
                y + 24.0,
                TextStyle::plain(tone::muted(1.0)),
            );
        }
        let shown = ((b.1 + b.3 - 6.0 - y) / BAG_ROW).floor().max(0.0) as usize;
        let more = self.screen.bags.len().saturating_sub(shown);
        let fits = if more > 0 {
            shown.saturating_sub(1)
        } else {
            shown
        };
        for bag in self.screen.bags.iter().take(fits) {
            let tile = (b.0 + 12.0, y + 4.0, 22.0, 22.0);
            rc.fill_rounded_rect(
                tile.0,
                tile.1,
                tile.2,
                tile.3,
                round,
                Color {
                    a: 0.25,
                    ..bag.color
                },
            );
            rc.fill_rounded_rect(
                tile.0 + 7.0,
                tile.1 + 7.0,
                8.0,
                8.0,
                CornerRadii::all(2.0),
                bag.color,
            );
            let nx = tile.0 + 32.0;
            let mut sub_x = nx;
            if let Some(st) = &f.strong {
                let ty = y + ((BAG_ROW - st.line_height) * 0.5).round();
                text::draw(rc, st, &bag.name, nx, ty, TextStyle::plain(p.text));
                sub_x += text::measure(st, &bag.name, 0.0) + 10.0;
            }
            if let Some(sm) = &f.small {
                let room = b.0 + b.2 - 12.0 - sub_x;
                let ty = y + ((BAG_ROW - sm.line_height) * 0.5).round() + 1.0;
                text::draw(
                    rc,
                    sm,
                    &kit::fit(sm, &bag.sub, room),
                    sub_x,
                    ty,
                    TextStyle::plain(tone::soft(1.0)),
                );
            }
            rc.fill(b.0 + 10.0, y + BAG_ROW - 1.0, b.2 - 20.0, 1.0, bronze(0.12));
            y += BAG_ROW;
        }
        if more > 0
            && let Some(sm) = &f.small
        {
            let words = format!("and {} more", more + usize::from(shown > 0));
            let ty = y + ((BAG_ROW - sm.line_height) * 0.5).round();
            text::draw(
                rc,
                sm,
                &words,
                b.0 + 44.0,
                ty,
                TextStyle::plain(tone::muted(1.0)),
            );
        }
    }

    fn draw_foot(&self, rc: &mut RenderContext<'_>) {
        let f = &self.fonts;
        let fy = HEIGHT - FOOT;
        rc.fill(0.0, fy, WIDTH, 1.0, bronze(0.35));
        rc.fill_rounded_rect(
            1.0,
            fy + 1.0,
            WIDTH - 2.0,
            FOOT - 2.0,
            CornerRadii::new(0.0, 0.0, 5.0, 5.0),
            hex(0, 0.2),
        );
        if self.screen.done.is_some() {
            let (close, more) = Self::done_buttons();
            kit::button(
                rc,
                &Horizon::current(),
                close,
                "Close",
                f.strong.as_ref(),
                ButtonLook::Plain,
                hot(self.hover, close),
                true,
            );
            kit::button(
                rc,
                &Horizon::current(),
                more,
                "Salvage more",
                f.strong.as_ref(),
                ButtonLook::Primary,
                hot(self.hover, more),
                true,
            );
            return;
        }
        let any = !self.screen.rows.is_empty();
        let clear = Self::clear_rect();
        kit::button(
            rc,
            &Horizon::current(),
            clear,
            "Clear",
            f.strong.as_ref(),
            ButtonLook::Plain,
            hot(self.hover, clear),
            any,
        );
        let go = Self::go_rect();
        kit::button(
            rc,
            &Horizon::current(),
            go,
            &self.screen.go,
            f.strong.as_ref(),
            ButtonLook::Primary,
            hot(self.hover, go),
            any,
        );
    }

    fn draw_done(&self, rc: &mut RenderContext<'_>) {
        let Some(d) = &self.screen.done else {
            return;
        };
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let mut top = TOP;
        if let Some(t) = &f.title {
            text::draw(rc, t, &d.title, PAD, top, TextStyle::plain(p.text));
            top += t.line_height + 4.0;
        }
        if let Some(sm) = &f.small {
            text::draw(
                rc,
                sm,
                "The bags are in your main pack.",
                PAD,
                top,
                TextStyle::plain(tone::soft(1.0)),
            );
            top += sm.line_height;
        }
        let w = LEFT_W;
        let round = CornerRadii::all(4.0);
        let first = top + 12.0;
        let mut bottom = first;
        for (i, c) in d.cards.iter().enumerate() {
            let r = (PAD, first + i as f32 * 72.0, w, 64.0);
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0, 0.28));
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.35), 1.0);
            let tile = (r.0 + 12.0, r.1 + 12.0, 40.0, 40.0);
            rc.fill_rounded_rect(
                tile.0,
                tile.1,
                tile.2,
                tile.3,
                round,
                Color { a: 0.25, ..c.color },
            );
            rc.fill_rounded_rect(
                tile.0 + 13.0,
                tile.1 + 13.0,
                14.0,
                14.0,
                CornerRadii::all(3.0),
                c.color,
            );
            if let Some(st) = &f.strong {
                text::draw(
                    rc,
                    st,
                    &c.name,
                    tile.0 + 52.0,
                    r.1 + 14.0,
                    TextStyle::plain(p.text),
                );
            }
            if let Some(sm) = &f.small {
                text::draw(
                    rc,
                    sm,
                    &c.from,
                    tile.0 + 52.0,
                    r.1 + 34.0,
                    TextStyle::plain(tone::soft(1.0)),
                );
                let ww = text::measure(sm, &c.workmanship, 0.0);
                text::draw(
                    rc,
                    sm,
                    &c.workmanship,
                    r.0 + r.2 - 12.0 - ww,
                    r.1 + 36.0,
                    TextStyle::plain(tone::soft(1.0)),
                );
            }
            if let Some(t) = &f.title {
                let uw = text::measure(t, &c.units, 0.0);
                text::draw(
                    rc,
                    t,
                    &c.units,
                    r.0 + r.2 - 12.0 - uw,
                    r.1 + 12.0,
                    TextStyle::plain(p.text),
                );
            }
            bottom = r.1 + r.3;
        }
        if let Some(sm) = &f.small {
            let mut y = bottom + 12.0;
            if let Some(r) = &d.refused {
                text::draw(rc, sm, r, PAD, y, TextStyle::plain(hex(0xFF_B3B6, 1.0)));
                y += sm.line_height + 6.0;
            }
            text::draw(
                rc,
                sm,
                "The same lines are in chat.",
                PAD,
                y,
                TextStyle::plain(tone::muted(1.0)),
            );
        }
    }

    fn draw_pick(&self, rc: &mut RenderContext<'_>, icons: &[Option<TextureRef>]) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let b = self.pick_box();
        rc.soft_shadow(b.0, b.1, b.2, b.3, 6.0, 16.0, 40.0, hex(0, 0.6));
        rc.fill_rounded_rect(
            b.0,
            b.1,
            b.2,
            b.3,
            CornerRadii::all(6.0),
            tone::raised(0.98),
        );
        rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, CornerRadii::all(6.0), bronze(0.8), 1.0);
        if let Some(st) = &f.strong {
            text::draw(
                rc,
                st,
                "Add from your packs",
                b.0 + 12.0,
                b.1 + 14.0,
                TextStyle::plain(p.text),
            );
        }
        let c = Self::pick_close();
        kit::close(
            rc,
            &Horizon::current(),
            (c.0 - 3.0, c.1 - 3.0, 30.0, 30.0),
            hot(self.hover, c),
        );
        for ((m, r), chip) in self.chip_rects().into_iter().zip(&self.chips) {
            let on = m == self.chip;
            let round = CornerRadii::all(12.0);
            rc.fill_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                round,
                if on { gold(0.16) } else { Color::TRANSPARENT },
            );
            rc.stroke_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                round,
                if on { gold(1.0) } else { bronze(0.35) },
                1.0,
            );
            rc.fill_rounded_rect(
                r.0 + 8.0,
                r.1 + 8.5,
                7.0,
                7.0,
                CornerRadii::all(2.0),
                chip.color,
            );
            if let Some(t) = &f.tiny {
                let ty = r.1 + ((r.3 - t.line_height) * 0.5).round();
                text::draw(rc, t, &chip.name, r.0 + 20.0, ty, TextStyle::plain(p.text));
                let w = text::measure(t, &chip.name, 0.0);
                text::draw(
                    rc,
                    t,
                    &chip.count.to_string(),
                    r.0 + 25.0 + w,
                    ty,
                    TextStyle::plain(tone::soft(1.0)),
                );
            }
        }
        let room = self.pick_room();
        rc.push_clip(room.0, room.1, room.2, room.3);
        if self.pick.is_empty()
            && let Some(sm) = &f.small
        {
            text::draw(
                rc,
                sm,
                "Nothing you carry can be salvaged.",
                room.0 + 4.0,
                room.1 + 10.0,
                TextStyle::plain(tone::soft(1.0)),
            );
        }
        for ((_, row, add), (r, icon)) in self
            .pick_rects()
            .into_iter()
            .zip(self.pick.iter().zip(icons))
        {
            let fade = if r.off { 0.5 } else { 1.0 };
            if !r.off && hot(self.hover, row) {
                rc.fill_rounded_rect(
                    row.0,
                    row.1,
                    row.2,
                    row.3,
                    CornerRadii::all(4.0),
                    hex(0xFF_FFFF, 0.04),
                );
            }
            let tile = (row.0 + 6.0, row.1 + 6.0, 28.0, 28.0);
            rc.fill_rounded_rect(
                tile.0,
                tile.1,
                tile.2,
                tile.3,
                CornerRadii::all(4.0),
                hex(0, 0.45 * fade),
            );
            if let Some(i) = icon {
                rc.sprite(
                    *i,
                    tile.0 - 2.0,
                    tile.1 - 2.0,
                    32.0,
                    32.0,
                    (0.0, 0.0, 1.0, 1.0),
                    Color::rgba(1.0, 1.0, 1.0, fade),
                );
            }
            let x = tile.0 + 38.0;
            let w = add.0 - 8.0 - x;
            if let Some(st) = &f.strong {
                let n = kit::fit(st, &r.name, w);
                text::draw(
                    rc,
                    st,
                    &n,
                    x,
                    row.1 + 4.0,
                    TextStyle::plain(Color { a: fade, ..p.text }),
                );
            }
            if let Some(sm) = &f.small {
                let s = kit::fit(sm, &r.sub, w);
                let ink = if r.refused {
                    hex(0xFF_B3B6, 1.0)
                } else {
                    tone::soft(1.0)
                };
                text::draw(rc, sm, &s, x, row.1 + 21.0, TextStyle::plain(ink));
            }
            if !r.refused {
                let words = if r.added { "Added" } else { "Add" };
                kit::button(
                    rc,
                    &Horizon::current(),
                    add,
                    words,
                    f.strong.as_ref(),
                    ButtonLook::Plain,
                    hot(self.hover, add),
                    !r.off,
                );
            }
        }
        rc.pop_clip();
        let total = self.pick.len() as f32 * (PICK_ROW + 2.0);
        kit::scrollbar(
            rc,
            room.0 + room.2 + 2.0,
            room.1,
            room.3,
            total,
            self.pick_scroll.at,
        );
    }
}

/// A dashed rounded outline (the drop zone's).
fn dashed(rc: &mut RenderContext<'_>, r: Rect, color: Color) {
    let dash: f32 = 6.0;
    let gap = 4.0;
    let mut x = r.0 + 4.0;
    while x < r.0 + r.2 - 4.0 {
        let w = dash.min(r.0 + r.2 - 4.0 - x);
        rc.fill(x, r.1, w, 1.5, color);
        rc.fill(x, r.1 + r.3 - 1.5, w, 1.5, color);
        x += dash + gap;
    }
    let mut y = r.1 + 4.0;
    while y < r.1 + r.3 - 4.0 {
        let h = dash.min(r.1 + r.3 - 4.0 - y);
        rc.fill(r.0, y, 1.5, h, color);
        rc.fill(r.0 + r.2 - 1.5, y, 1.5, h, color);
        y += dash + gap;
    }
}

impl Widget for SalvageFrame {
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
                self.click(x, y, false, false);
                true
            }
            kind::DOUBLE_CLICK => {
                self.click(x, y, false, true);
                true
            }
            kind::RIGHT_CLICK => {
                self.click(x, y, true, false);
                true
            }
            kind::SCROLL => {
                if self.picking {
                    let room = self.pick_room();
                    let total = self.pick.len() as f32 * (PICK_ROW + 2.0);
                    self.pick_scroll
                        .wheel(e.data0, PICK_ROW * 2.0, total, room.3);
                } else {
                    let room = self.rows_room();
                    let total = self.screen.rows.len() as f32 * (ROW + 2.0);
                    self.scroll.wheel(e.data0, ROW * 2.0, total, room.3);
                }
                true
            }
            kind::DRAG_ENTER => {
                self.over = e
                    .payload
                    .as_ref()
                    .and_then(|p| p.get::<ItemDragPayload>())
                    .is_some_and(drag_stages);
                true
            }
            kind::DRAG_OVER => true,
            kind::DROP_RELEASED => {
                self.over = false;
                if let Some(p) = e
                    .payload
                    .as_ref()
                    .and_then(|p| p.get::<ItemDragPayload>())
                    .filter(|p| drag_stages(p))
                {
                    self.asks.push(SalvageAsk::Dropped(p.clone()));
                }
                true
            }
            _ => false,
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        let (x, y) = self.hover?;
        // What the options allow, on the button it bears on.
        (self.screen.done.is_none()
            && !self.picking
            && !self.screen.rule.is_empty()
            && inside(Self::go_rect(), x, y))
        .then(|| self.screen.rule.clone())
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, _dt: f64) {
        if self.over && ui.drag_payload().is_none() {
            self.over = false;
        }
        let room = self.rows_room();
        self.scroll
            .hold(self.screen.rows.len() as f32 * (ROW + 2.0), room.3);
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let f = &self.fonts;
        kit::window_back(rc, &Horizon::current(), WIDTH, HEIGHT, HEADER, true);
        let x = kit::title(
            rc,
            &Horizon::current(),
            f.title.as_ref(),
            ANVIL,
            "Salvage",
            HEADER,
        );
        if let Some(sm) = &f.small {
            let c = Self::close_rect();
            let w = kit::fit(sm, &self.screen.sub, c.0 - 12.0 - (x + 14.0));
            let ty = ((HEADER - sm.line_height) * 0.5).round();
            text::draw(rc, sm, &w, x + 14.0, ty, TextStyle::plain(tone::soft(1.0)));
        }
        kit::close(
            rc,
            &Horizon::current(),
            Self::close_rect(),
            hot(self.hover, Self::close_rect()),
        );
        if self.screen.done.is_some() {
            self.draw_done(rc);
        } else {
            self.draw_staging(rc);
            self.draw_bags(rc);
        }
        self.draw_foot(rc);
        if self.picking && self.screen.done.is_none() {
            let icons: Vec<Option<TextureRef>> = self.pick.iter().map(|r| r.icon).collect();
            self.draw_pick(rc, &icons);
        }
    }
}
