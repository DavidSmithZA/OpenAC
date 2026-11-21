//! The Trade window's frame: the header (who the trade is with, its
//! status), the banner when something changed or was refused, the two
//! sides (each with its acceptance, items and total; the player's own takes
//! items dropped from the packs and has Add from packs), the foot (Clear
//! both sides, and Accept trade or Withdraw acceptance), the Add from packs
//! list, and a card for the item clicked.

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{ItemDragPayload, ItemDragSource, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::trade_model::{PickRow, TradeRow, TradeScreen, TradeSide};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, ButtonLook, Rect, Scroll, bronze, gold, hot, inside};

/// The window's size.
pub const WIDTH: f32 = 900.0;
pub const HEIGHT: f32 = 560.0;
const HEADER: f32 = 52.0;
const FOOT: f32 = 72.0;
const PAD: f32 = 20.0;
const GAP: f32 = 24.0;
const COL_W: f32 = (WIDTH - 2.0 * PAD - GAP) / 2.0;
const HEAD: f32 = 48.0;
const SIDE_FOOT: f32 = 36.0;
const ROW: f32 = 46.0;
const DROP: f32 = 64.0;
const PICK_W: f32 = 380.0;
const PICK_ROW: f32 = 42.0;
const PICK_ROWS: usize = 6;
const CARD_W: f32 = 340.0;
const CARD_H: f32 = 196.0;

/// A tick, for an acceptance.
const CHECK: &[&[(f32, f32)]] = &[&[(5.0, 12.0), (10.0, 17.0), (20.0, 7.0)]];
/// A plus, for Add from packs.
const PLUS: &[&[(f32, f32)]] = &[&[(12.0, 5.0), (12.0, 19.0)], &[(5.0, 12.0), (19.0, 12.0)]];
/// Two arrows passing, the window's glyph.
const SWAP: &[&[(f32, f32)]] = &[
    &[(4.0, 8.0), (20.0, 8.0)],
    &[(16.0, 4.0), (20.0, 8.0), (16.0, 12.0)],
    &[(20.0, 16.0), (4.0, 16.0)],
    &[(8.0, 12.0), (4.0, 16.0), (8.0, 20.0)],
];

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct TradeFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// Names, buttons.
    pub strong: Option<Arc<UiFont>>,
    /// Quiet lines.
    pub small: Option<Arc<UiFont>>,
    /// The pills, NEW.
    pub tiny: Option<Arc<UiFont>>,
    /// Words.
    pub body: Option<Arc<UiFont>>,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum TradeAsk {
    /// End the trade.
    Close,
    /// Accept, or take the acceptance back.
    Accept,
    Withdraw,
    /// Empty both sides.
    Clear,
    /// Show or hide Add from packs.
    Pick(bool),
    /// Offer an item from the packs (the window asks how many of a
    /// stack).
    Offer(u32, (f32, f32)),
    /// Show the card for an item, or close it.
    Card(Option<u32>),
    /// The item's full appraisal.
    Details(u32),
    /// An item from the packs let go on the player's side.
    Dropped(ItemDragPayload),
}

/// The card for the item clicked.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeCard {
    /// The item.
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name and line.
    pub name: String,
    pub sub: String,
    /// Its facts, label and value.
    pub facts: Vec<(String, String)>,
    /// It is on the other player's side (the card sits over the player's).
    pub theirs: bool,
}

/// The Trade window's frame.
#[derive(Debug)]
pub struct TradeFrame {
    theme: Horizon,
    fonts: TradeFonts,
    /// What it shows.
    pub screen: TradeScreen,
    /// Add from packs shows, with these rows.
    pub picking: bool,
    pub pick: Vec<PickRow>,
    /// The search is empty.
    pub pick_empty: bool,
    /// The card shows.
    pub card: Option<TradeCard>,
    /// The height the plugins' band takes over the foot (0: none).
    pub band: f32,
    /// What it asked for since the window last looked.
    pub asks: Vec<TradeAsk>,
    hover: Option<(f32, f32)>,
    over: bool,
    scroll: [Scroll; 2],
    pick_scroll: Scroll,
}

impl TradeFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: TradeFonts) -> Self {
        Self {
            theme,
            fonts,
            screen: TradeScreen::default(),
            picking: false,
            pick: Vec::new(),
            pick_empty: true,
            card: None,
            band: 0.0,
            asks: Vec::new(),
            hover: None,
            over: false,
            scroll: [Scroll::default(); 2],
            pick_scroll: Scroll::default(),
        }
    }

    fn close_rect() -> Rect {
        kit::close_rect(WIDTH, HEADER)
    }

    fn columns(&self) -> [Rect; 2] {
        let top = if self.screen.banner.is_some() {
            120.0
        } else {
            68.0
        };
        let bottom = HEIGHT
            - FOOT
            - 12.0
            - if self.band > 0.0 {
                self.band + 4.0
            } else {
                0.0
            };
        [
            (PAD, top, COL_W, bottom - top),
            (PAD + COL_W + GAP, top, COL_W, bottom - top),
        ]
    }

    /// Where the plugins' band goes (over the foot).
    #[must_use]
    pub fn band_top(&self) -> f32 {
        HEIGHT - FOOT - 4.0 - self.band
    }

    fn rows_room(col: Rect) -> Rect {
        (
            col.0 + 8.0,
            col.1 + HEAD + 8.0,
            col.2 - 16.0,
            col.3 - HEAD - 8.0 - SIDE_FOOT - 8.0,
        )
    }

    /// The player's side's rows and the drop zone under them.
    fn row_rects(&self, side: usize) -> Vec<(u32, Rect)> {
        let col = self.columns()[side];
        let room = Self::rows_room(col);
        let rows = if side == 0 {
            &self.screen.mine.rows
        } else {
            &self.screen.theirs.rows
        };
        rows.iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    r.id,
                    (
                        room.0,
                        room.1 + i as f32 * (ROW + 2.0) - self.scroll[side].at,
                        room.2,
                        ROW,
                    ),
                )
            })
            .collect()
    }

    fn drop_rect(&self) -> Rect {
        let col = self.columns()[0];
        let room = Self::rows_room(col);
        let n = self.screen.mine.rows.len() as f32;
        let y = room.1 + n * (ROW + 2.0) + 4.0 - self.scroll[0].at;
        (room.0, y, room.2, DROP)
    }

    fn add_button(&self) -> Rect {
        let d = self.drop_rect();
        let w = 130.0;
        (d.0 + (d.2 - w) * 0.5, d.1 + d.3 - 8.0 - 28.0, w, 28.0)
    }

    fn content_height(&self, side: usize) -> f32 {
        let n = if side == 0 {
            self.screen.mine.rows.len()
        } else {
            self.screen.theirs.rows.len()
        };
        n as f32 * (ROW + 2.0) + if side == 0 { 4.0 + DROP } else { 0.0 }
    }

    fn clear_rect() -> Rect {
        (PAD, HEIGHT - FOOT + (FOOT - 34.0) * 0.5, 140.0, 34.0)
    }

    fn accept_rect(&self) -> Rect {
        let w = if self.screen.mine.accepted {
            180.0
        } else {
            150.0
        };
        (
            WIDTH - PAD - w,
            HEIGHT - FOOT + (FOOT - 40.0) * 0.5,
            w,
            40.0,
        )
    }

    fn pick_box(&self) -> Rect {
        let rows = self.pick.len().clamp(1, PICK_ROWS) as f32;
        (
            60.0,
            140.0,
            PICK_W,
            12.0 + 24.0 + 8.0 + 30.0 + 8.0 + rows * (PICK_ROW + 2.0) + 10.0,
        )
    }

    /// The search field's place in Add from packs.
    #[must_use]
    pub fn pick_search_rect() -> Rect {
        (60.0 + 12.0, 140.0 + 12.0 + 24.0 + 8.0, PICK_W - 24.0, 30.0)
    }

    fn pick_rows_room(&self) -> Rect {
        let b = self.pick_box();
        let top = b.1 + 12.0 + 24.0 + 8.0 + 30.0 + 8.0;
        (b.0 + 12.0, top, b.2 - 24.0, b.1 + b.3 - 10.0 - top)
    }

    fn pick_rects(&self) -> Vec<(usize, Rect, Rect)> {
        let room = self.pick_rows_room();
        self.pick
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let y = room.1 + i as f32 * (PICK_ROW + 2.0) - self.pick_scroll.at;
                let row = (room.0, y, room.2, PICK_ROW);
                let add = (
                    row.0 + row.2 - 6.0 - 62.0,
                    y + (PICK_ROW - 26.0) * 0.5,
                    62.0,
                    26.0,
                );
                (i, row, add)
            })
            .collect()
    }

    fn pick_close(&self) -> Rect {
        let b = self.pick_box();
        (b.0 + b.2 - 12.0 - 24.0, b.1 + 10.0, 24.0, 24.0)
    }

    fn card_box(&self) -> Option<Rect> {
        let c = self.card.as_ref()?;
        let cols = self.columns();
        let col = if c.theirs { cols[0] } else { cols[1] };
        Some((col.0 + (col.2 - CARD_W) * 0.5, col.1 + 40.0, CARD_W, CARD_H))
    }

    fn card_buttons(&self) -> Option<(Rect, Rect)> {
        let b = self.card_box()?;
        let y = b.1 + b.3 - 14.0 - 30.0;
        let details = (b.0 + 14.0, y, 120.0, 30.0);
        let close = (b.0 + b.2 - 14.0 - 80.0, y, 80.0, 30.0);
        Some((details, close))
    }

    #[allow(clippy::too_many_lines)]
    fn click(&mut self, x: f32, y: f32, right: bool) {
        if inside(Self::close_rect(), x, y) {
            self.asks.push(TradeAsk::Close);
            return;
        }
        if self.picking {
            let b = self.pick_box();
            if inside(self.pick_close(), x, y) || !inside(b, x, y) {
                self.asks.push(TradeAsk::Pick(false));
                return;
            }
            let room = self.pick_rows_room();
            if inside(room, x, y)
                && let Some((i, row, _)) = self
                    .pick_rects()
                    .into_iter()
                    .find(|(_, r, _)| inside(*r, x, y))
                && let Some(p) = self.pick.get(i)
                && !p.refused
            {
                let at = (row.0 + row.2 - 70.0, row.1 + row.3);
                self.asks.push(TradeAsk::Offer(p.id, at));
            }
            return;
        }
        if let Some(b) = self.card_box() {
            if let Some((details, close)) = self.card_buttons() {
                if inside(details, x, y) {
                    if let Some(c) = &self.card {
                        self.asks.push(TradeAsk::Details(c.id));
                    }
                    return;
                }
                if inside(close, x, y) {
                    self.asks.push(TradeAsk::Card(None));
                    return;
                }
            }
            if inside(b, x, y) {
                return;
            }
        }
        for side in 0..2 {
            let room = Self::rows_room(self.columns()[side]);
            if !inside(room, x, y) {
                continue;
            }
            if let Some((id, _)) = self
                .row_rects(side)
                .into_iter()
                .find(|(_, r)| inside(*r, x, y))
            {
                self.asks.push(TradeAsk::Card(Some(id)));
                return;
            }
            if side == 0 && !right && inside(self.add_button(), x, y) {
                self.asks.push(TradeAsk::Pick(true));
                return;
            }
        }
        if self.card.is_some() {
            self.asks.push(TradeAsk::Card(None));
        }
        if inside(Self::clear_rect(), x, y) {
            self.asks.push(TradeAsk::Clear);
            return;
        }
        if inside(self.accept_rect(), x, y) {
            self.asks.push(if self.screen.mine.accepted {
                TradeAsk::Withdraw
            } else {
                TradeAsk::Accept
            });
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_side(&self, rc: &mut RenderContext<'_>, side: usize, icons: &[Option<TextureRef>]) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let col = self.columns()[side];
        let s: &TradeSide = if side == 0 {
            &self.screen.mine
        } else {
            &self.screen.theirs
        };
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(col.0, col.1, col.2, col.3, round, hex(0, 0.28));
        let edge = if s.accepted {
            hex(0x7F_D08A, 0.6)
        } else {
            bronze(0.35)
        };
        rc.stroke_rounded_rect(
            col.0,
            col.1,
            col.2,
            col.3,
            round,
            edge,
            if s.accepted { 1.5 } else { 1.0 },
        );
        // The head.
        rc.fill(
            col.0 + 1.0,
            col.1 + HEAD - 1.0,
            col.2 - 2.0,
            1.0,
            bronze(0.2),
        );
        let cy = col.1 + (HEAD - 28.0) * 0.5;
        rc.fill_ellipse(col.0 + 12.0, cy, 28.0, 28.0, gold(0.18));
        rc.stroke_ellipse(col.0 + 12.0, cy, 28.0, 28.0, bronze(0.6), 1.0);
        if let Some(st) = &f.strong {
            text::draw_centred(
                rc,
                st,
                &s.initial,
                col.0 + 26.0,
                cy + ((28.0 - st.line_height) * 0.5).round(),
                TextStyle::plain(gold(1.0)),
            );
        }
        let (words, ink, fill) = if s.accepted {
            ("Accepted", kit::GOOD, hex(0x7F_D08A, 0.14))
        } else {
            ("Not yet", kit::SOFT, hex(0xFF_FFFF, 0.06))
        };
        let mut right = col.0 + col.2 - 12.0;
        if let Some(t) = &f.tiny {
            let w = text::measure(t, words, 0.0) + 16.0 + if s.accepted { 14.0 } else { 0.0 };
            let chip = (right - w, col.1 + (HEAD - 22.0) * 0.5, w, 22.0);
            rc.fill_rounded_rect(chip.0, chip.1, chip.2, chip.3, CornerRadii::all(11.0), fill);
            let mut tx = chip.0 + 8.0;
            if s.accepted {
                stroke(rc, tx, chip.1 + 5.0, 12.0, CHECK, ink, 1.6);
                tx += 14.0;
            }
            text::draw(
                rc,
                t,
                words,
                tx,
                chip.1 + ((22.0 - t.line_height) * 0.5).round(),
                TextStyle::plain(ink),
            );
            right = chip.0 - 8.0;
        }
        if let Some(st) = &f.title {
            let title = kit::fit(st, &s.title, right - (col.0 + 50.0));
            text::draw(
                rc,
                st,
                &title,
                col.0 + 50.0,
                col.1 + ((HEAD - st.line_height) * 0.5).round(),
                TextStyle::plain(p.text),
            );
        }
        // The rows.
        let room = Self::rows_room(col);
        rc.push_clip(room.0, room.1, room.2, room.3);
        for ((_, r), (row, icon)) in self
            .row_rects(side)
            .into_iter()
            .zip(s.rows.iter().zip(icons))
        {
            self.draw_row(rc, row, r, *icon);
        }
        if side == 0 {
            let d = self.drop_rect();
            let lit = self.over;
            let edge = if lit { gold(1.0) } else { bronze(0.45) };
            dashed(rc, d, edge);
            if lit {
                rc.fill_rounded_rect(d.0, d.1, d.2, d.3, round, gold(0.08));
            }
            if let Some(sm) = &f.body {
                text::draw_centred(
                    rc,
                    sm,
                    "Drag items here from your packs",
                    d.0 + d.2 * 0.5,
                    d.1 + 8.0,
                    TextStyle::plain(kit::SOFT),
                );
            }
            let a = self.add_button();
            let lit = hot(self.hover, a);
            rc.fill_rounded_rect(
                a.0,
                a.1,
                a.2,
                a.3,
                round,
                hex(0, if lit { 0.5 } else { 0.35 }),
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
        } else if s.rows.is_empty()
            && let Some(sm) = &f.body
        {
            let words = format!("{} has not offered anything yet.", self.screen.partner);
            let w = kit::fit(sm, &words, room.2 - 20.0);
            text::draw_centred(
                rc,
                sm,
                &w,
                room.0 + room.2 * 0.5,
                room.1 + room.3 * 0.4,
                TextStyle::plain(hex(0xA8_9A82, 1.0)),
            );
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            col.0 + col.2 - 6.0,
            room.1,
            room.3,
            self.content_height(side),
            self.scroll[side].at,
        );
        // The foot: count and total.
        let fy = col.1 + col.3 - SIDE_FOOT;
        rc.fill(col.0 + 1.0, fy, col.2 - 2.0, 1.0, bronze(0.2));
        if let (Some(sm), Some(st)) = (&f.small, &f.strong) {
            let ty = fy + ((SIDE_FOOT - sm.line_height) * 0.5).round();
            text::draw(
                rc,
                sm,
                &s.count,
                col.0 + 12.0,
                ty,
                TextStyle::plain(kit::SOFT),
            );
            let w = text::measure(st, &s.total, 0.0);
            let ty = fy + ((SIDE_FOOT - st.line_height) * 0.5).round();
            text::draw(
                rc,
                st,
                &s.total,
                col.0 + col.2 - 12.0 - w,
                ty,
                TextStyle::plain(p.text),
            );
            // Totals are OpenAC's own: retail shows none.
            if let Some(t) = &f.tiny {
                let tag = "OPENAC";
                let tw = text::measure(t, tag, 0.0) + 10.0;
                let tx = col.0 + col.2 - 12.0 - w - 8.0 - tw;
                let tr = (tx, fy + (SIDE_FOOT - 16.0) * 0.5, tw, 16.0);
                rc.stroke_rounded_rect(
                    tr.0,
                    tr.1,
                    tr.2,
                    tr.3,
                    CornerRadii::all(3.0),
                    bronze(0.5),
                    1.0,
                );
                text::draw(
                    rc,
                    t,
                    tag,
                    tr.0 + 5.0,
                    tr.1 + ((16.0 - t.line_height) * 0.5).round(),
                    TextStyle::plain(bronze(0.9)),
                );
            }
        }
    }

    fn draw_row(
        &self,
        rc: &mut RenderContext<'_>,
        row: &TradeRow,
        r: Rect,
        icon: Option<TextureRef>,
    ) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let on = self.card.as_ref().is_some_and(|c| c.id == row.id);
        let round = CornerRadii::all(4.0);
        if on {
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, gold(0.14));
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, gold(1.0), 1.0);
        } else if row.new {
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, gold(0.10));
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, gold(0.7), 1.0);
        } else if hot(self.hover, r) {
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0xFF_FFFF, 0.04));
        }
        let tile = (r.0 + 6.0, r.1 + 6.0, 34.0, 34.0);
        rc.fill_rounded_rect(tile.0, tile.1, tile.2, tile.3, round, hex(0, 0.45));
        if let Some(i) = icon {
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
        let value_w = 80.0;
        let new_w = if row.new { 44.0 } else { 0.0 };
        let x = tile.0 + 44.0;
        let right = r.0 + r.2 - 10.0 - value_w - new_w - 8.0;
        if let Some(st) = &f.strong {
            let name = kit::fit(st, &row.name, right - x);
            text::draw(rc, st, &name, x, r.1 + 7.0, TextStyle::plain(p.text));
            if let (Some((m, color)), Some(t)) = (&row.mark, &f.tiny) {
                let mx = x + text::measure(st, &name, 0.0) + 8.0;
                if mx + text::measure(t, m, 0.0) < right {
                    text::draw(rc, t, m, mx, r.1 + 9.0, TextStyle::plain(*color));
                }
            }
        }
        if let Some(sm) = &f.small {
            let sub = kit::fit(sm, &row.sub, right - x);
            text::draw(rc, sm, &sub, x, r.1 + 26.0, TextStyle::plain(kit::SOFT));
        }
        if row.new
            && let Some(t) = &f.tiny
        {
            let nx = r.0 + r.2 - 10.0 - value_w - 40.0;
            let ny = r.1 + (ROW - 18.0) * 0.5;
            rc.fill_rounded_rect(nx, ny, 36.0, 18.0, CornerRadii::all(3.0), gold(1.0));
            text::draw_centred(
                rc,
                t,
                "NEW",
                nx + 18.0,
                ny + ((18.0 - t.line_height) * 0.5).round(),
                TextStyle::plain(hex(0x1A_140D, 1.0)),
            );
        }
        if let Some(st) = &f.strong {
            let w = text::measure(st, &row.value, 0.0);
            text::draw(
                rc,
                st,
                &row.value,
                r.0 + r.2 - 10.0 - w,
                r.1 + 8.0,
                TextStyle::plain(p.text),
            );
        }
        if let Some(t) = &f.tiny {
            let w = text::measure(t, row.value_sub, 0.0);
            text::draw(
                rc,
                t,
                row.value_sub,
                r.0 + r.2 - 10.0 - w,
                r.1 + 27.0,
                TextStyle::plain(hex(0xA8_9A82, 1.0)),
            );
        }
    }

    fn draw_pick(&self, rc: &mut RenderContext<'_>, icons: &[Option<TextureRef>]) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let b = self.pick_box();
        rc.soft_shadow(b.0, b.1, b.2, b.3, 6.0, 16.0, 40.0, hex(0, 0.6));
        rc.fill_rounded_rect(
            b.0,
            b.1,
            b.2,
            b.3,
            CornerRadii::all(6.0),
            hex(0x18_130E, 0.98),
        );
        rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, CornerRadii::all(6.0), bronze(0.8), 1.0);
        if let Some(st) = &f.title {
            text::draw(
                rc,
                st,
                "Add from your packs",
                b.0 + 12.0,
                b.1 + 12.0,
                TextStyle::plain(p.text),
            );
        }
        let c = self.pick_close();
        kit::close(
            rc,
            &self.theme,
            (c.0 - 3.0, c.1 - 3.0, 30.0, 30.0),
            hot(self.hover, c),
        );
        kit::search_box(
            rc,
            &self.theme,
            Self::pick_search_rect(),
            "Search your packs",
            self.pick_empty,
            false,
            f.body.as_ref(),
        );
        let room = self.pick_rows_room();
        rc.push_clip(room.0, room.1, room.2, room.3);
        if self.pick.is_empty()
            && let Some(sm) = &f.small
        {
            text::draw(
                rc,
                sm,
                "Nothing to add.",
                room.0 + 4.0,
                room.1 + 10.0,
                TextStyle::plain(kit::SOFT),
            );
        }
        for ((_, row, add), (r, icon)) in self
            .pick_rects()
            .into_iter()
            .zip(self.pick.iter().zip(icons))
        {
            let fade = if r.refused { 0.5 } else { 1.0 };
            if !r.refused && hot(self.hover, row) {
                rc.fill_rounded_rect(
                    row.0,
                    row.1,
                    row.2,
                    row.3,
                    CornerRadii::all(4.0),
                    hex(0xFF_FFFF, 0.04),
                );
            }
            let tile = (row.0 + 4.0, row.1 + 5.0, 32.0, 32.0);
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
                    tile.0,
                    tile.1,
                    32.0,
                    32.0,
                    (0.0, 0.0, 1.0, 1.0),
                    Color::rgba(1.0, 1.0, 1.0, fade),
                );
            }
            let x = tile.0 + 40.0;
            let room_w = add.0 - 8.0 - x;
            if let Some(st) = &f.strong {
                let name = kit::fit(st, &r.name, room_w);
                text::draw(
                    rc,
                    st,
                    &name,
                    x,
                    row.1 + 5.0,
                    TextStyle::plain(Color { a: fade, ..p.text }),
                );
            }
            if let Some(sm) = &f.small {
                let sub = kit::fit(sm, &r.sub, room_w);
                let ink = if r.refused {
                    hex(0xFF_B3B6, 1.0)
                } else {
                    kit::SOFT
                };
                text::draw(rc, sm, &sub, x, row.1 + 23.0, TextStyle::plain(ink));
            }
            if !r.refused {
                let words = if r.stack > 1 { "Add\u{2026}" } else { "Add" };
                kit::button(
                    rc,
                    &self.theme,
                    add,
                    words,
                    f.strong.as_ref(),
                    ButtonLook::Plain,
                    hot(self.hover, add),
                    true,
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

    fn draw_card(&self, rc: &mut RenderContext<'_>) {
        let (Some(c), Some(b)) = (&self.card, self.card_box()) else {
            return;
        };
        let p = self.theme.palette;
        let f = &self.fonts;
        rc.soft_shadow(b.0, b.1, b.2, b.3, 6.0, 16.0, 40.0, hex(0, 0.6));
        rc.fill_rounded_rect(
            b.0,
            b.1,
            b.2,
            b.3,
            CornerRadii::all(6.0),
            hex(0x18_130E, 0.98),
        );
        rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, CornerRadii::all(6.0), bronze(0.8), 1.0);
        let tile = (b.0 + 14.0, b.1 + 14.0, 40.0, 40.0);
        rc.fill_rounded_rect(
            tile.0,
            tile.1,
            tile.2,
            tile.3,
            CornerRadii::all(4.0),
            hex(0, 0.45),
        );
        if let Some(i) = c.icon {
            rc.sprite(
                i,
                tile.0 + 4.0,
                tile.1 + 4.0,
                32.0,
                32.0,
                (0.0, 0.0, 1.0, 1.0),
                Color::WHITE,
            );
        }
        let x = tile.0 + 50.0;
        if let Some(st) = &f.strong {
            let n = kit::fit(st, &c.name, b.0 + b.2 - 14.0 - x);
            text::draw(rc, st, &n, x, tile.1 + 3.0, TextStyle::plain(p.text));
        }
        if let Some(sm) = &f.small {
            let s = kit::fit(sm, &c.sub, b.0 + b.2 - 14.0 - x);
            text::draw(rc, sm, &s, x, tile.1 + 22.0, TextStyle::plain(kit::SOFT));
            let mut y = tile.1 + tile.3 + 12.0;
            for (k, v) in &c.facts {
                text::draw(rc, sm, k, b.0 + 14.0, y, TextStyle::plain(kit::SOFT));
                let w = text::measure(sm, v, 0.0);
                text::draw(rc, sm, v, b.0 + b.2 - 14.0 - w, y, TextStyle::plain(p.text));
                y += sm.line_height + 6.0;
            }
        }
        if let Some((details, close)) = self.card_buttons() {
            kit::button(
                rc,
                &self.theme,
                details,
                "Full details",
                f.strong.as_ref(),
                ButtonLook::Plain,
                hot(self.hover, details),
                true,
            );
            kit::button(
                rc,
                &self.theme,
                close,
                "Close",
                f.strong.as_ref(),
                ButtonLook::Plain,
                hot(self.hover, close),
                true,
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_frame(&self, rc: &mut RenderContext<'_>) {
        let f = &self.fonts;
        kit::window_back(rc, &self.theme, WIDTH, HEIGHT, HEADER, true);
        let mut x = kit::title(
            rc,
            &self.theme,
            f.title.as_ref(),
            SWAP,
            &format!("Trade with {}", self.screen.partner),
            HEADER,
        );
        // The status pill.
        if let Some(t) = &f.tiny {
            let (ink, fill, edge) = match self.screen.status_tone {
                1 => (kit::GOOD, hex(0x7F_D08A, 0.12), hex(0x7F_D08A, 0.5)),
                -1 => (kit::HARM, hex(0xFF_6B70, 0.12), hex(0xFF_8A8E, 0.5)),
                _ => (gold(1.0), gold(0.12), gold(0.5)),
            };
            x += 12.0;
            let w = text::measure(t, &self.screen.status, 0.0) + 30.0;
            let r = (x, (HEADER - 22.0) * 0.5, w, 22.0);
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(11.0), fill);
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(11.0), edge, 1.0);
            rc.fill_ellipse(r.0 + 9.0, r.1 + 8.0, 6.0, 6.0, ink);
            text::draw(
                rc,
                t,
                &self.screen.status,
                r.0 + 21.0,
                r.1 + ((22.0 - t.line_height) * 0.5).round(),
                TextStyle::plain(ink),
            );
        }
        if let Some(sm) = &f.small {
            let hint = "Click any item to look at it";
            let w = text::measure(sm, hint, 0.0);
            let c = Self::close_rect();
            text::draw(
                rc,
                sm,
                hint,
                c.0 - 12.0 - w,
                ((HEADER - sm.line_height) * 0.5).round(),
                TextStyle::plain(hex(0xA8_9A82, 1.0)),
            );
        }
        kit::close(
            rc,
            &self.theme,
            Self::close_rect(),
            hot(self.hover, Self::close_rect()),
        );
        // The banner.
        if let Some(b) = &self.screen.banner {
            let r = (PAD, 64.0, WIDTH - 2.0 * PAD, 44.0);
            let (fill, edge) = if b.bad {
                (hex(0xFF_6B70, 0.10), hex(0xFF_8A8E, 0.6))
            } else {
                (gold(0.12), gold(0.7))
            };
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), fill);
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), edge, 1.0);
            if let Some(st) = &f.strong {
                let t = kit::fit(st, &b.title, r.2 - 28.0);
                text::draw(
                    rc,
                    st,
                    &t,
                    r.0 + 14.0,
                    r.1 + 6.0,
                    TextStyle::plain(if b.bad { kit::HARM } else { gold(1.0) }),
                );
            }
            if let Some(sm) = &f.small {
                let t = kit::fit(sm, &b.text, r.2 - 28.0);
                text::draw(
                    rc,
                    sm,
                    &t,
                    r.0 + 14.0,
                    r.1 + 24.0,
                    TextStyle::plain(kit::WARM),
                );
            }
        }
        // The foot.
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
        let clear = Self::clear_rect();
        kit::button(
            rc,
            &self.theme,
            clear,
            "Clear both sides",
            f.strong.as_ref(),
            ButtonLook::Plain,
            hot(self.hover, clear),
            true,
        );
        if let Some(sm) = &f.small {
            let lines = kit::wrap(
                sm,
                "Anything that changes takes back both acceptances.",
                250.0,
            );
            let top = fy + (FOOT - lines.len() as f32 * (sm.line_height + 3.0)) * 0.5;
            kit::lines(
                rc,
                sm,
                &lines,
                clear.0 + clear.2 + 12.0,
                top,
                sm.line_height + 3.0,
                hex(0xA8_9A82, 1.0),
            );
        }
        let accept = self.accept_rect();
        let mine = self.screen.mine.accepted;
        let (words, look) = if mine {
            ("Withdraw acceptance", ButtonLook::Plain)
        } else {
            ("Accept trade", ButtonLook::Primary)
        };
        kit::button(
            rc,
            &self.theme,
            accept,
            words,
            f.strong.as_ref(),
            look,
            hot(self.hover, accept),
            true,
        );
        if let Some(sm) = &f.body {
            let (hint, ink) = if mine && !self.screen.theirs.accepted {
                (
                    format!("Waiting for {} to accept\u{2026}", self.screen.partner),
                    kit::SOFT,
                )
            } else if mine {
                ("Both accepted.".to_owned(), kit::GOOD)
            } else {
                (
                    self.screen.hint.clone(),
                    if self.screen.hint_good {
                        kit::GOOD
                    } else {
                        kit::SOFT
                    },
                )
            };
            let w = text::measure(sm, &hint, 0.0);
            text::draw(
                rc,
                sm,
                &hint,
                accept.0 - 14.0 - w,
                fy + ((FOOT - sm.line_height) * 0.5).round(),
                TextStyle::plain(ink),
            );
        }
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

/// Whether `p` comes from the player's packs.
/// Only what is carried in the packs comes in (as retail: worn gear and
/// things on the ground do not).
fn from_packs(p: &ItemDragPayload) -> bool {
    p.source_kind == ItemDragSource::Inventory && p.obj_id != 0
}

impl Widget for TradeFrame {
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
                self.click(x, y, false);
                true
            }
            kind::RIGHT_CLICK => {
                self.click(x, y, true);
                true
            }
            kind::SCROLL => {
                let Some((hx, hy)) = self.hover else {
                    return false;
                };
                if self.picking {
                    let room = self.pick_rows_room();
                    let total = self.pick.len() as f32 * (PICK_ROW + 2.0);
                    self.pick_scroll
                        .wheel(e.data0, PICK_ROW * 2.0, total, room.3);
                    return true;
                }
                for side in 0..2 {
                    let room = Self::rows_room(self.columns()[side]);
                    if inside(room, hx, hy) {
                        let total = self.content_height(side);
                        self.scroll[side].wheel(e.data0, ROW * 2.0, total, room.3);
                        return true;
                    }
                }
                false
            }
            kind::DRAG_ENTER => {
                self.over = e
                    .payload
                    .as_ref()
                    .and_then(|p| p.get::<ItemDragPayload>())
                    .is_some_and(from_packs);
                true
            }
            kind::DRAG_OVER => true,
            kind::DROP_RELEASED => {
                self.over = false;
                if let Some(p) = e
                    .payload
                    .as_ref()
                    .and_then(|p| p.get::<ItemDragPayload>())
                    .filter(|p| from_packs(p))
                {
                    self.asks.push(TradeAsk::Dropped(p.clone()));
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
        for side in 0..2 {
            let room = Self::rows_room(self.columns()[side]);
            let total = self.content_height(side);
            self.scroll[side].hold(total, room.3);
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        self.draw_frame(rc);
        let mine: Vec<Option<TextureRef>> = self.screen.mine.rows.iter().map(|r| r.icon).collect();
        let theirs: Vec<Option<TextureRef>> =
            self.screen.theirs.rows.iter().map(|r| r.icon).collect();
        self.draw_side(rc, 0, &mine);
        self.draw_side(rc, 1, &theirs);
        // Over the sides, under the search field (a child).
        self.draw_card(rc);
        if self.picking {
            let icons: Vec<Option<TextureRef>> = self.pick.iter().map(|r| r.icon).collect();
            self.draw_pick(rc, &icons);
        }
    }
}
