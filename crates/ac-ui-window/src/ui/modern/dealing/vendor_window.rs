//! The Vendor window's frame and its list. The frame draws the header (the
//! vendor's name, the Buy and Sell tabs with how many rows wait on each
//! list, the purse, the search), the rail down the left (the categories the
//! shop stocks, or the player's packs and what the vendor buys), the list's
//! heading and the restock banner, the chosen thing with its quantity and
//! price, and the To buy or To sell list with its total. The list of goods
//! or of the player's items is a child of its own, so it scrolls by itself.
//! Anything dropped on the window from the packs goes on the sell list.

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{ItemDragPayload, ItemDragSource, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::vendor_model::{Tone, VendorRow, VendorScreen, VendorTab};
use crate::ui::modern::hud::lines::{circle, stroke};
use crate::ui::modern::kit::{self, ButtonLook, Rect, Scroll, Tab, bronze, gold, hot, inside};

/// The window's size.
pub const WIDTH: f32 = 1040.0;
/// Its width on a small screen: the categories or packs drop down over
/// the list, and what the vendor buys shows with them.
pub const NARROW_WIDTH: f32 = 792.0;
/// The list's width in the narrow window.
const NARROW_MID_W: f32 = NARROW_WIDTH - RAIL_X - 16.0 - SIDE_W - 20.0;
pub const HEIGHT: f32 = 640.0;
const HEADER: f32 = 56.0;
const TOP: f32 = 72.0;
const BOTTOM: f32 = TOP + 552.0;
const RAIL_X: f32 = 20.0;
const RAIL_W: f32 = 180.0;
const MID_X: f32 = 216.0;
const MID_W: f32 = 440.0;
const SIDE_W: f32 = 348.0;
const SEARCH_W: f32 = 200.0;
const RAIL_ROW: f32 = 30.0;
const BANNER: f32 = 56.0;
/// The list's rows.
pub const ROW: f32 = 46.0;
const CARD_H: f32 = 214.0;
const CART_ROW: f32 = 34.0;

/// The coin's glyph.
const COIN: &[&[(f32, f32)]] = &[
    &[
        (15.0, 9.0),
        (10.5, 9.0),
        (9.0, 10.5),
        (10.5, 12.0),
        (13.5, 12.0),
        (15.0, 13.5),
        (13.5, 15.0),
        (9.0, 15.0),
    ],
    &[(12.0, 7.0), (12.0, 9.0)],
    &[(12.0, 15.0), (12.0, 17.0)],
];
/// The shop's glyph (a stall).
const SHOP: &[&[(f32, f32)]] = &[
    &[(3.0, 9.0), (5.0, 4.0), (19.0, 4.0), (21.0, 9.0), (3.0, 9.0)],
    &[(5.0, 9.0), (5.0, 20.0), (19.0, 20.0), (19.0, 9.0)],
    &[(10.0, 20.0), (10.0, 14.0), (14.0, 14.0), (14.0, 20.0)],
];
/// A cross, to take a row off the list.
const CROSS: &[&[(f32, f32)]] = &[&[(7.0, 7.0), (17.0, 17.0)], &[(17.0, 7.0), (7.0, 17.0)]];
/// The details button's glyph.
const EYE: &[&[(f32, f32)]] = &[&[
    (2.0, 12.0),
    (6.0, 7.0),
    (12.0, 5.0),
    (18.0, 7.0),
    (22.0, 12.0),
    (18.0, 17.0),
    (12.0, 19.0),
    (6.0, 17.0),
    (2.0, 12.0),
]];

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct VendorFonts {
    /// The vendor's name, big figures.
    pub title: Option<Arc<UiFont>>,
    /// Tabs, buttons, names.
    pub strong: Option<Arc<UiFont>>,
    /// Quiet lines, counts.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals.
    pub label: Option<Arc<UiFont>>,
    /// Words.
    pub body: Option<Arc<UiFont>>,
    /// The pills' figures, the keys.
    pub tiny: Option<Arc<UiFont>>,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum VendorAsk {
    /// Close it (asks first when lists wait).
    Close,
    /// Show a tab.
    Tab(VendorTab),
    /// Pick a rail row (a category, a pack; 0: all).
    Rail(u32),
    /// Show only what the vendor buys.
    OnlyTaken(bool),
    /// The next order.
    Sort,
    /// One fewer, one more, the most.
    Less,
    More,
    Most,
    /// Buy or sell the chosen now.
    Now,
    /// Put the chosen on the list.
    Add,
    /// The chosen thing's full details.
    Inspect,
    /// Empty the list.
    Clear,
    /// Take a row off the list.
    Remove(u32),
    /// Buy or sell everything on the list.
    All,
    /// Put the components the player is short of on the buy list.
    Restock,
    /// The leave question: stay, or leave.
    Stay,
    Leave,
    /// An item from the packs let go on the window.
    Dropped(ItemDragPayload),
}

/// A refusal shown where an item was dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct Refused {
    /// The item's name.
    pub name: String,
    /// The vendor's reason.
    pub reason: String,
    /// The vendor's limit against the item's worth.
    pub limit: Option<String>,
}

/// The Vendor window's frame.
#[derive(Debug)]
pub struct VendorFrame {
    theme: Horizon,
    fonts: VendorFonts,
    /// What it shows.
    pub screen: VendorScreen,
    /// The search is empty, and has the keyboard.
    pub search_empty: bool,
    pub search_focused: bool,
    /// Only what the vendor buys is listed (Sell).
    pub only_taken: bool,
    /// A drop the vendor refused.
    pub refused: Option<Refused>,
    /// The leave question shows.
    pub leaving: bool,
    /// The height the plugins' band takes under the list (0: none).
    pub band: f32,
    /// What it asked for since the window last looked.
    pub asks: Vec<VendorAsk>,
    hover: Option<(f32, f32)>,
    /// Something from the packs is held over it.
    over: bool,
    cart_scroll: Scroll,
    /// Its width: [`WIDTH`], or [`NARROW_WIDTH`] on a small screen.
    pub width: f32,
    /// The narrow window's drop-down of categories or packs is open.
    pub rail_open: bool,
}

impl VendorFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: VendorFonts) -> Self {
        Self {
            theme,
            fonts,
            screen: VendorScreen::default(),
            search_empty: true,
            search_focused: false,
            only_taken: false,
            refused: None,
            leaving: false,
            band: 0.0,
            asks: Vec::new(),
            hover: None,
            over: false,
            cart_scroll: Scroll::default(),
            width: WIDTH,
            rail_open: false,
        }
    }

    /// The narrow window (a small screen).
    #[must_use]
    pub fn narrow(&self) -> bool {
        self.width < WIDTH
    }

    fn mid_x(&self) -> f32 {
        if self.narrow() { RAIL_X } else { MID_X }
    }

    fn mid_w(&self) -> f32 {
        if self.narrow() { NARROW_MID_W } else { MID_W }
    }

    fn side_x(&self) -> f32 {
        self.mid_x() + self.mid_w() + 16.0
    }

    fn search_w(&self) -> f32 {
        if self.narrow() { 150.0 } else { SEARCH_W }
    }

    /// The categories' or packs' left: their column, or the drop-down's.
    fn rail_x(&self) -> f32 {
        if self.narrow() {
            self.mid_x() + 10.0
        } else {
            RAIL_X
        }
    }

    /// Where the categories or packs start: their column's top, or under
    /// the list's heading in the drop-down.
    fn rail_top(&self) -> f32 {
        if self.narrow() { TOP + 40.0 } else { TOP }
    }

    /// The drop-down's button (the list's title) in the narrow window.
    fn rail_button(&self) -> Rect {
        let words = self
            .fonts
            .strong
            .as_ref()
            .map_or(140.0, |f| text::measure(f, &self.screen.list_title, 0.0));
        (self.mid_x(), TOP, words + 34.0, 28.0)
    }

    /// The drop-down's panel: the categories or packs, and on Sell the
    /// toggle and what the vendor buys.
    fn rail_panel(&self) -> Rect {
        let rows = self.rail_rects();
        let mut bottom = rows
            .last()
            .map_or(self.rail_top() + 20.0, |(_, r)| r.1 + r.3);
        if self.screen.tab == VendorTab::Sell {
            bottom = self.toggle_rect().1 + 20.0 + 14.0 + self.buys_height();
        }
        let top = TOP + 30.0;
        (self.mid_x(), top, self.mid_w(), bottom - top + 12.0)
    }

    /// The height of what the vendor buys, in words.
    fn buys_height(&self) -> f32 {
        let (Some(s), Some(st)) = (&self.fonts.small, &self.fonts.strong) else {
            return 0.0;
        };
        let lines: usize = self
            .screen
            .buys
            .iter()
            .map(|l| kit::wrap(s, l, self.rail_w() - 20.0).len())
            .sum();
        10.0 + st.line_height + 4.0 + lines as f32 * (s.line_height + 4.0) + 10.0
    }

    fn rail_w(&self) -> f32 {
        if self.narrow() {
            self.mid_w() - 20.0
        } else {
            RAIL_W
        }
    }

    /// The search box.
    #[must_use]
    pub fn search_rect(&self) -> Rect {
        (
            self.width - 16.0 - 30.0 - 12.0 - self.search_w(),
            (HEADER - 30.0) * 0.5,
            self.search_w(),
            30.0,
        )
    }

    /// The list's place.
    #[must_use]
    pub fn list_rect(&self) -> Rect {
        let top = TOP
            + 36.0
            + if self.screen.restock.is_some() && self.screen.tab == VendorTab::Buy {
                BANNER + 8.0
            } else {
                0.0
            };
        let gap = if self.band > 0.0 {
            self.band + 6.0
        } else {
            0.0
        };
        (self.mid_x(), top, self.mid_w(), BOTTOM - top - gap)
    }

    /// Where the plugins' band goes (under the list).
    #[must_use]
    pub fn band_top(&self) -> f32 {
        BOTTOM - self.band
    }

    /// The middle column's left and width (the band spans it).
    #[must_use]
    pub fn band_span(&self) -> (f32, f32) {
        (self.mid_x(), self.mid_w())
    }

    /// The quantity field's place in the chosen card.
    #[must_use]
    pub fn quantity_rect(&self) -> Rect {
        let (_, minus, _, _) = self.quantity_parts();
        (minus.0 + 28.0, minus.1, 44.0, 28.0)
    }

    fn quantity_parts(&self) -> (Rect, Rect, Rect, Rect) {
        let y = TOP + 14.0 + 48.0 + 12.0;
        let label = (self.side_x() + 14.0, y, 56.0, 28.0);
        let minus = (self.side_x() + 14.0 + 62.0, y, 28.0, 28.0);
        let plus = (minus.0 + 28.0 + 44.0, y, 28.0, 28.0);
        let most = (plus.0 + 28.0 + 8.0, y, 74.0, 28.0);
        (label, minus, plus, most)
    }

    fn close_rect(&self) -> Rect {
        (self.width - 16.0 - 30.0, (HEADER - 30.0) * 0.5, 30.0, 30.0)
    }

    fn tabs(&self) -> [(VendorTab, Rect); 2] {
        let name_w = self
            .fonts
            .title
            .as_ref()
            .map_or(160.0, |f| text::measure(f, &self.screen.name, 0.0))
            .max(
                self.fonts
                    .small
                    .as_ref()
                    .map_or(100.0, |f| text::measure(f, &self.screen.sub, 0.0)),
            );
        let x0 = 20.0 + 28.0 + 10.0 + name_w.min(260.0) + 22.0;
        let w = |words: &str, n: usize| {
            let t = Tab {
                words,
                count: (n > 0).then(|| n.to_string()),
                on: false,
                red: false,
            };
            kit::tab_width(&t, self.fonts.strong.as_ref(), self.fonts.tiny.as_ref())
        };
        let buy = (
            x0,
            (HEADER - 32.0) * 0.5 + 2.0,
            w("Buy", self.screen.buy_waiting),
            32.0,
        );
        let sell = (
            buy.0 + buy.2 + 2.0,
            buy.1,
            w("Sell", self.screen.sell_waiting),
            32.0,
        );
        [(VendorTab::Buy, buy), (VendorTab::Sell, sell)]
    }

    fn rail_rects(&self) -> Vec<(u32, Rect)> {
        if self.narrow() && !self.rail_open {
            return Vec::new();
        }
        self.screen
            .rail
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    r.key,
                    (
                        self.rail_x(),
                        self.rail_top() + 20.0 + i as f32 * (RAIL_ROW + 2.0),
                        self.rail_w(),
                        RAIL_ROW,
                    ),
                )
            })
            .collect()
    }

    fn toggle_rect(&self) -> Rect {
        let y = self
            .rail_rects()
            .last()
            .map_or(TOP + 20.0, |(_, r)| r.1 + r.3)
            + 16.0;
        (self.rail_x(), y, self.rail_w(), 20.0)
    }

    fn sort_rect(&self) -> Rect {
        let words = format!("Sort {}", self.sort_words());
        let w = self
            .fonts
            .small
            .as_ref()
            .map_or(90.0, |f| text::measure(f, &words, 0.0) + 26.0);
        (self.mid_x() + self.mid_w() - w, TOP + 1.0, w, 26.0)
    }

    /// The sort's words.
    fn sort_words(&self) -> &str {
        self.screen.sort
    }

    fn restock_button(&self) -> Rect {
        (
            self.mid_x() + self.mid_w() - 12.0 - 96.0,
            TOP + 36.0 + (BANNER - 30.0) * 0.5,
            96.0,
            30.0,
        )
    }

    fn detail_button(&self) -> Rect {
        (
            self.side_x() + SIDE_W - 14.0 - 28.0,
            TOP + 14.0 + 10.0,
            28.0,
            28.0,
        )
    }

    fn card_buttons(&self) -> (Rect, Rect) {
        let y = TOP + CARD_H - 14.0 - 32.0;
        let w = (SIDE_W - 28.0 - 6.0) / 2.0;
        (
            (self.side_x() + 14.0, y, w, 32.0),
            (self.side_x() + 14.0 + w + 6.0, y, w, 32.0),
        )
    }

    fn cart_box(&self) -> Rect {
        (
            self.side_x(),
            TOP + CARD_H + 10.0,
            SIDE_W,
            BOTTOM - TOP - CARD_H - 10.0,
        )
    }

    fn clear_rect(&self) -> Rect {
        let b = self.cart_box();
        (b.0 + b.2 - 14.0 - 44.0, b.1 + 8.0, 44.0, 22.0)
    }

    fn cart_rows_room(&self) -> Rect {
        let b = self.cart_box();
        let top = b.1 + 36.0;
        let bottom = b.1 + b.3 - 14.0 - 34.0 - 8.0 - 18.0 - 6.0 - 22.0 - 10.0;
        (b.0 + 10.0, top, b.2 - 18.0, (bottom - top).max(0.0))
    }

    fn cart_button(&self) -> Rect {
        let b = self.cart_box();
        (b.0 + 14.0, b.1 + b.3 - 14.0 - 34.0, b.2 - 28.0, 34.0)
    }

    fn cart_rows(&self) -> Vec<(u32, Rect, Rect)> {
        let room = self.cart_rows_room();
        self.screen
            .cart
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let y = room.1 + i as f32 * (CART_ROW + 2.0) - self.cart_scroll.at;
                let row = (room.0, y, room.2, CART_ROW);
                let x = (
                    row.0 + row.2 - 26.0,
                    y + (CART_ROW - 22.0) * 0.5,
                    22.0,
                    22.0,
                );
                (r.id, row, x)
            })
            .collect()
    }

    fn leave_box(&self) -> Rect {
        let w = 400.0;
        let h = 150.0;
        ((self.width - w) * 0.5, (HEIGHT - h) * 0.5, w, h)
    }

    fn leave_buttons(&self) -> (Rect, Rect) {
        let b = self.leave_box();
        let y = b.1 + b.3 - 16.0 - 32.0;
        let leave = (b.0 + b.2 - 16.0 - 90.0, y, 90.0, 32.0);
        let stay = (leave.0 - 8.0 - 90.0, y, 90.0, 32.0);
        (stay, leave)
    }

    fn click(&mut self, x: f32, y: f32) {
        if self.leaving {
            let (stay, leave) = self.leave_buttons();
            if inside(stay, x, y) {
                self.asks.push(VendorAsk::Stay);
            } else if inside(leave, x, y) {
                self.asks.push(VendorAsk::Leave);
            }
            return;
        }
        if inside(self.close_rect(), x, y) {
            self.asks.push(VendorAsk::Close);
            return;
        }
        if let Some((tab, _)) = self.tabs().into_iter().find(|(_, r)| inside(*r, x, y)) {
            self.rail_open = false;
            self.asks.push(VendorAsk::Tab(tab));
            return;
        }
        // The narrow window's drop-down: its button opens and closes it; a
        // click outside it closes it and does nothing else.
        if self.narrow() {
            if inside(self.rail_button(), x, y) {
                self.rail_open = !self.rail_open;
                return;
            }
            if self.rail_open && !inside(self.rail_panel(), x, y) {
                self.rail_open = false;
                return;
            }
        }
        if let Some((key, _)) = self
            .rail_rects()
            .into_iter()
            .find(|(_, r)| inside(*r, x, y))
        {
            self.rail_open = false;
            self.asks.push(VendorAsk::Rail(key));
            return;
        }
        if self.screen.tab == VendorTab::Sell
            && (!self.narrow() || self.rail_open)
            && inside(self.toggle_rect(), x, y)
        {
            self.asks.push(VendorAsk::OnlyTaken(!self.only_taken));
            return;
        }
        if inside(self.sort_rect(), x, y) {
            self.asks.push(VendorAsk::Sort);
            return;
        }
        if self.screen.restock.is_some()
            && self.screen.tab == VendorTab::Buy
            && inside(self.restock_button(), x, y)
        {
            self.asks.push(VendorAsk::Restock);
            return;
        }
        if let Some(c) = &self.screen.chosen {
            let (_, minus, plus, most) = self.quantity_parts();
            let (now, add) = self.card_buttons();
            if inside(minus, x, y) {
                self.asks.push(VendorAsk::Less);
            } else if inside(plus, x, y) {
                self.asks.push(VendorAsk::More);
            } else if inside(most, x, y) {
                self.asks.push(VendorAsk::Most);
            } else if inside(now, x, y) && c.can_primary {
                self.asks.push(VendorAsk::Now);
            } else if inside(add, x, y) && c.can_add {
                self.asks.push(VendorAsk::Add);
            } else if inside(self.detail_button(), x, y) {
                self.asks.push(VendorAsk::Inspect);
            }
        }
        if !self.screen.cart.is_empty() && inside(self.clear_rect(), x, y) {
            self.asks.push(VendorAsk::Clear);
            return;
        }
        let room = self.cart_rows_room();
        if inside(room, x, y)
            && let Some((id, _, _)) = self
                .cart_rows()
                .into_iter()
                .find(|(_, _, cross)| inside(*cross, x, y))
        {
            self.asks.push(VendorAsk::Remove(id));
            return;
        }
        if self.screen.can_cart && inside(self.cart_button(), x, y) {
            self.asks.push(VendorAsk::All);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_header(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let f = &self.fonts;
        stroke(rc, 20.0, (HEADER - 22.0) * 0.5, 22.0, SHOP, gold(1.0), 1.8);
        if let Some(t) = &f.title {
            let name = kit::fit(t, &self.screen.name, 260.0);
            text::draw(rc, t, &name, 58.0, 10.0, TextStyle::plain(p.text));
        }
        if let Some(s) = &f.small {
            let sub = kit::fit(s, &self.screen.sub, 260.0);
            text::draw(rc, s, &sub, 58.0, 30.0, TextStyle::plain(kit::SOFT));
        }
        for (tab, r) in self.tabs() {
            let (words, n) = match tab {
                VendorTab::Buy => ("Buy", self.screen.buy_waiting),
                VendorTab::Sell => ("Sell", self.screen.sell_waiting),
            };
            let t = Tab {
                words,
                count: (n > 0).then(|| n.to_string()),
                on: self.screen.tab == tab,
                red: false,
            };
            kit::draw_tab(
                rc,
                &self.theme,
                &t,
                r,
                f.strong.as_ref(),
                f.tiny.as_ref(),
                hot(self.hover, r),
            );
        }
        // The purse, then the search, then close.
        let search = self.search_rect();
        if let Some(s) = &f.strong {
            let also = self
                .screen
                .purse_also
                .as_deref()
                .map(|a| format!(" \u{b7} {a}"));
            let w1 = text::measure(s, &self.screen.purse, 0.0);
            let w2 = also
                .as_deref()
                .and_then(|a| f.small.as_ref().map(|sm| text::measure(sm, a, 0.0)))
                .unwrap_or(0.0);
            let w = 10.0 + 16.0 + 7.0 + w1 + w2 + 10.0;
            let r = (search.0 - 12.0 - w, (HEADER - 30.0) * 0.5, w, 30.0);
            let round = CornerRadii::all(4.0);
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0, 0.35));
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.35), 1.0);
            circle(
                rc,
                r.0 + 10.0,
                r.1 + 7.0,
                16.0,
                (12.0, 12.0),
                9.0,
                gold(1.0),
                1.5,
            );
            stroke(rc, r.0 + 10.0, r.1 + 7.0, 16.0, COIN, gold(1.0), 1.5);
            let ty = r.1 + ((30.0 - s.line_height) * 0.5).round();
            text::draw(
                rc,
                s,
                &self.screen.purse,
                r.0 + 33.0,
                ty,
                TextStyle::plain(p.text),
            );
            if let (Some(a), Some(sm)) = (also, &f.small) {
                let ty = r.1 + ((30.0 - sm.line_height) * 0.5).round();
                text::draw(rc, sm, &a, r.0 + 33.0 + w1, ty, TextStyle::plain(kit::SOFT));
            }
        }
        let hint = match self.screen.tab {
            VendorTab::Buy => "Search goods",
            VendorTab::Sell => "Search your items",
        };
        kit::search_box(
            rc,
            &self.theme,
            search,
            hint,
            self.search_empty,
            self.search_focused,
            f.body.as_ref(),
        );
        kit::close(
            rc,
            &self.theme,
            self.close_rect(),
            hot(self.hover, self.close_rect()),
        );
    }

    fn draw_rail(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let f = &self.fonts;
        if let Some(l) = &f.label {
            text::draw(
                rc,
                l,
                self.screen.rail_title,
                self.rail_x(),
                self.rail_top(),
                TextStyle::plain(kit::SOFT).tracked(kit::LABEL_TRACKING),
            );
        }
        for ((key, r), row) in self.rail_rects().into_iter().zip(&self.screen.rail) {
            let on = key == self.screen.rail_on;
            let round = CornerRadii::all(4.0);
            if on {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, gold(0.16));
                rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, gold(1.0), 1.0);
            } else if hot(self.hover, r) {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0xFF_FFFF, 0.04));
            }
            if let Some(s) = &f.strong {
                let ty = r.1 + ((r.3 - s.line_height) * 0.5).round();
                let words = kit::fit(s, &row.words, r.2 - 50.0);
                text::draw(rc, s, &words, r.0 + 8.0, ty, TextStyle::plain(p.text));
            }
            if let Some(s) = &f.small {
                let n = row.count.to_string();
                let w = text::measure(s, &n, 0.0);
                let ty = r.1 + ((r.3 - s.line_height) * 0.5).round();
                text::draw(
                    rc,
                    s,
                    &n,
                    r.0 + r.2 - 8.0 - w,
                    ty,
                    TextStyle::plain(kit::SOFT),
                );
            }
        }
        if self.screen.tab != VendorTab::Sell {
            return;
        }
        // Only what they buy, then what they buy in words.
        let t = self.toggle_rect();
        let on = self.only_taken;
        let round = CornerRadii::all(8.0);
        rc.fill_rounded_rect(
            t.0,
            t.1 + 2.0,
            30.0,
            16.0,
            round,
            if on { gold(0.9) } else { hex(0xFF_FFFF, 0.14) },
        );
        let knob = if on { t.0 + 16.0 } else { t.0 + 2.0 };
        rc.fill_ellipse(
            knob,
            t.1 + 4.0,
            12.0,
            12.0,
            if on { hex(0x1A_140D, 1.0) } else { kit::SOFT },
        );
        if let Some(b) = &f.body {
            let ty = t.1 + ((20.0 - b.line_height) * 0.5).round();
            text::draw(
                rc,
                b,
                "Only what they buy",
                t.0 + 38.0,
                ty,
                TextStyle::plain(p.text),
            );
        }
        let (Some(s), Some(st)) = (&f.small, &f.strong) else {
            return;
        };
        let mut lines: Vec<String> = Vec::new();
        for line in &self.screen.buys {
            lines.extend(kit::wrap(s, line, self.rail_w() - 20.0));
        }
        let step = s.line_height + 4.0;
        let h = 10.0 + st.line_height + 4.0 + lines.len() as f32 * step + 10.0;
        let y = t.1 + 20.0 + 14.0;
        rc.fill_rounded_rect(
            self.rail_x(),
            y,
            self.rail_w(),
            h,
            CornerRadii::all(4.0),
            hex(0xFF_FFFF, 0.04),
        );
        let head = format!("{} buys", self.screen.name);
        let head = kit::fit(st, &head, self.rail_w() - 20.0);
        text::draw(
            rc,
            st,
            &head,
            self.rail_x() + 10.0,
            y + 10.0,
            TextStyle::plain(kit::WARM),
        );
        kit::lines(
            rc,
            s,
            &lines,
            self.rail_x() + 10.0,
            y + 10.0 + st.line_height + 4.0,
            step,
            kit::SOFT,
        );
    }

    fn draw_heading(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let mut x = self.mid_x();
        // The narrow window's title is the drop-down's button.
        if self.narrow() {
            let b = self.rail_button();
            let round = CornerRadii::all(4.0);
            let lit = self.rail_open || hot(self.hover, b);
            rc.fill_rounded_rect(b.0, b.1, b.2, b.3, round, hex(0, 0.35));
            rc.stroke_rounded_rect(
                b.0,
                b.1,
                b.2,
                b.3,
                round,
                bronze(if lit { 0.9 } else { 0.45 }),
                1.0,
            );
            let (cx, cy) = (b.0 + b.2 - 14.0, b.1 + b.3 * 0.5);
            let v = |dx: f32, dy: f32| (cx + dx, cy + dy);
            let down: &[(f32, f32)] = &[v(-4.0, -2.0), v(0.0, 2.0), v(4.0, -2.0)];
            for w in down.windows(2) {
                rc.line(w[0].0, w[0].1, w[1].0, w[1].1, kit::SOFT, 1.6);
            }
            x = b.0 + 10.0;
        }
        if let Some(s) = &f.strong {
            let ty = TOP + ((28.0 - s.line_height) * 0.5).round();
            text::draw(
                rc,
                s,
                &self.screen.list_title,
                x,
                ty,
                TextStyle::plain(p.text),
            );
            x += text::measure(s, &self.screen.list_title, 0.0) + 10.0;
            if self.narrow() {
                x += 24.0;
            }
        }
        let sort = self.sort_rect();
        if let Some(sm) = &f.small {
            let ty = TOP + ((28.0 - sm.line_height) * 0.5).round();
            let count = kit::fit(sm, &self.screen.list_count, sort.0 - 8.0 - x);
            text::draw(rc, sm, &count, x, ty, TextStyle::plain(kit::SOFT));
            let round = CornerRadii::all(4.0);
            rc.fill_rounded_rect(sort.0, sort.1, sort.2, sort.3, round, hex(0, 0.35));
            rc.stroke_rounded_rect(
                sort.0,
                sort.1,
                sort.2,
                sort.3,
                round,
                bronze(if hot(self.hover, sort) { 0.8 } else { 0.45 }),
                1.0,
            );
            let ty = sort.1 + ((sort.3 - sm.line_height) * 0.5).round();
            text::draw(
                rc,
                sm,
                "Sort",
                sort.0 + 8.0,
                ty,
                TextStyle::plain(kit::SOFT),
            );
            let w = text::measure(sm, "Sort ", 0.0);
            text::draw(
                rc,
                sm,
                self.sort_words(),
                sort.0 + 8.0 + w,
                ty,
                TextStyle::plain(p.text),
            );
        }
        // The restock banner.
        if self.screen.tab == VendorTab::Buy
            && let Some(r) = &self.screen.restock
        {
            let b = (self.mid_x(), TOP + 36.0, self.mid_w(), BANNER);
            let round = CornerRadii::all(4.0);
            rc.fill_rounded_rect(b.0, b.1, b.2, b.3, round, gold(0.12));
            rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, round, gold(0.6), 1.0);
            let words = if r.count == 1 {
                "1 component is below your restock amount".to_owned()
            } else {
                format!("{} components are below your restock amounts", r.count)
            };
            let button = self.restock_button();
            if let Some(s) = &f.strong {
                let w = kit::fit(s, &words, button.0 - b.0 - 24.0);
                text::draw(rc, s, &w, b.0 + 12.0, b.1 + 10.0, TextStyle::plain(p.text));
            }
            if let Some(sm) = &f.small {
                let line = format!(
                    "{} for all. Amounts are set in Spellbook, Components.",
                    r.cost
                );
                let line = kit::fit(sm, &line, button.0 - b.0 - 24.0);
                text::draw(
                    rc,
                    sm,
                    &line,
                    b.0 + 12.0,
                    b.1 + 30.0,
                    TextStyle::plain(kit::SOFT),
                );
            }
            kit::button(
                rc,
                &self.theme,
                button,
                "Add to list",
                f.strong.as_ref(),
                ButtonLook::Primary,
                hot(self.hover, button),
                true,
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_card(&self, rc: &mut RenderContext<'_>, icons: &CardIcons) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let b = (self.side_x(), TOP, SIDE_W, CARD_H);
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(b.0, b.1, b.2, b.3, round, hex(0, 0.28));
        rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, round, bronze(0.35), 1.0);
        let Some(c) = &self.screen.chosen else {
            if let Some(s) = &f.body {
                let words = match self.screen.tab {
                    VendorTab::Buy => "Pick something to buy.",
                    VendorTab::Sell => "Pick something to sell, or drop it here from your packs.",
                };
                let lines = kit::wrap(s, words, b.2 - 28.0);
                kit::lines(
                    rc,
                    s,
                    &lines,
                    b.0 + 14.0,
                    b.1 + 20.0,
                    s.line_height + 4.0,
                    kit::SOFT,
                );
            }
            return;
        };
        // The thing.
        let tile = (b.0 + 14.0, b.1 + 14.0, 48.0, 48.0);
        rc.fill_rounded_rect(tile.0, tile.1, tile.2, tile.3, round, hex(0, 0.45));
        rc.stroke_rounded_rect(tile.0, tile.1, tile.2, tile.3, round, bronze(0.6), 1.0);
        if let Some(icon) = icons.chosen {
            rc.sprite(
                icon,
                tile.0 + 8.0,
                tile.1 + 8.0,
                32.0,
                32.0,
                (0.0, 0.0, 1.0, 1.0),
                Color::WHITE,
            );
        }
        let room = self.detail_button().0 - (tile.0 + 60.0) - 8.0;
        if let Some(s) = &f.strong {
            let name = kit::fit(s, &c.name, room);
            text::draw(
                rc,
                s,
                &name,
                tile.0 + 60.0,
                tile.1 + 6.0,
                TextStyle::plain(p.text),
            );
        }
        if let Some(sm) = &f.small {
            let lines = kit::wrap(sm, &c.sub, room);
            kit::lines(
                rc,
                sm,
                &lines.into_iter().take(2).collect::<Vec<_>>(),
                tile.0 + 60.0,
                tile.1 + 24.0,
                sm.line_height + 2.0,
                kit::SOFT,
            );
        }
        let d = self.detail_button();
        rc.fill_rounded_rect(d.0, d.1, d.2, d.3, round, hex(0, 0.35));
        rc.stroke_rounded_rect(
            d.0,
            d.1,
            d.2,
            d.3,
            round,
            bronze(if hot(self.hover, d) { 0.8 } else { 0.45 }),
            1.0,
        );
        stroke(rc, d.0 + 6.0, d.1 + 6.0, 16.0, EYE, kit::SOFT, 1.4);
        circle(
            rc,
            d.0 + 6.0,
            d.1 + 6.0,
            16.0,
            (12.0, 12.0),
            3.0,
            kit::SOFT,
            1.4,
        );
        // The quantity.
        let (label, minus, plus, most) = self.quantity_parts();
        if let Some(sm) = &f.small {
            let ty = label.1 + ((28.0 - sm.line_height) * 0.5).round();
            text::draw(rc, sm, "Quantity", label.0, ty, TextStyle::plain(kit::SOFT));
        }
        let qbox = (minus.0, minus.1, 28.0 + 44.0 + 28.0, 28.0);
        rc.fill_rounded_rect(qbox.0, qbox.1, qbox.2, qbox.3, round, hex(0, 0.35));
        rc.stroke_rounded_rect(qbox.0, qbox.1, qbox.2, qbox.3, round, bronze(0.45), 1.0);
        rc.fill(minus.0 + 28.0, minus.1 + 4.0, 1.0, 20.0, bronze(0.3));
        rc.fill(plus.0, plus.1 + 4.0, 1.0, 20.0, bronze(0.3));
        let lit = |r: Rect| if hot(self.hover, r) { p.accent } else { p.text };
        stroke(
            rc,
            minus.0 + 7.0,
            minus.1 + 7.0,
            14.0,
            &[&[(5.0, 12.0), (19.0, 12.0)]],
            lit(minus),
            1.6,
        );
        stroke(
            rc,
            plus.0 + 7.0,
            plus.1 + 7.0,
            14.0,
            &[&[(5.0, 12.0), (19.0, 12.0)], &[(12.0, 5.0), (12.0, 19.0)]],
            lit(plus),
            1.6,
        );
        rc.stroke_rounded_rect(most.0, most.1, most.2, most.3, round, bronze(0.3), 1.0);
        if let Some(sm) = &f.small {
            let w = text::measure(sm, c.most_words, 0.0);
            let ty = most.1 + ((28.0 - sm.line_height) * 0.5).round();
            text::draw(
                rc,
                sm,
                c.most_words,
                most.0 + (most.2 - w) * 0.5,
                ty,
                TextStyle::plain(lit(most)),
            );
        }
        if let Some(t) = &f.title {
            let w = text::measure(t, &c.total, 0.0);
            let ty = label.1 + ((28.0 - t.line_height) * 0.5).round();
            let color = tone_color(c.total_tone, p.text);
            text::draw(
                rc,
                t,
                &c.total,
                b.0 + b.2 - 14.0 - w,
                ty,
                TextStyle::plain(color),
            );
        }
        if let Some(sm) = &f.small {
            let lines = kit::wrap(sm, &c.note, b.2 - 28.0);
            let color = tone_color(c.note_tone, kit::SOFT);
            kit::lines(
                rc,
                sm,
                &lines.into_iter().take(2).collect::<Vec<_>>(),
                b.0 + 14.0,
                label.1 + 36.0,
                sm.line_height + 2.0,
                color,
            );
        }
        let (now, add) = self.card_buttons();
        kit::button(
            rc,
            &self.theme,
            now,
            c.primary,
            f.strong.as_ref(),
            ButtonLook::Primary,
            hot(self.hover, now),
            c.can_primary,
        );
        kit::button(
            rc,
            &self.theme,
            add,
            "Add to list",
            f.strong.as_ref(),
            ButtonLook::Plain,
            hot(self.hover, add),
            c.can_add,
        );
    }

    #[allow(clippy::too_many_lines)]
    fn draw_cart(&mut self, rc: &mut RenderContext<'_>, icons: &CardIcons) {
        let p = self.theme.palette;
        let b = self.cart_box();
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(b.0, b.1, b.2, b.3, round, hex(0, 0.28));
        rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, round, bronze(0.35), 1.0);
        let f = self.fonts.clone();
        if let Some(l) = &f.label {
            text::draw(
                rc,
                l,
                self.screen.cart_title,
                b.0 + 14.0,
                b.1 + 14.0,
                TextStyle::plain(kit::SOFT).tracked(kit::LABEL_TRACKING),
            );
            let x =
                b.0 + 14.0 + text::measure(l, self.screen.cart_title, kit::LABEL_TRACKING) + 8.0;
            if let Some(sm) = &f.small {
                let n = self.screen.cart.len();
                let words = if n == 1 {
                    "1 item".to_owned()
                } else {
                    format!("{n} items")
                };
                text::draw(
                    rc,
                    sm,
                    &words,
                    x,
                    b.1 + 13.0,
                    TextStyle::plain(hex(0xA8_9A82, 1.0)),
                );
            }
        }
        if !self.screen.cart.is_empty() {
            let c = self.clear_rect();
            if let Some(sm) = &f.small {
                let ty = c.1 + ((c.3 - sm.line_height) * 0.5).round();
                let color = if hot(self.hover, c) {
                    p.text
                } else {
                    kit::SOFT
                };
                text::draw(rc, sm, "Clear", c.0 + 6.0, ty, TextStyle::plain(color));
            }
        }
        // The rows.
        let room = self.cart_rows_room();
        let total_h = self.screen.cart.len() as f32 * (CART_ROW + 2.0);
        self.cart_scroll.hold(total_h, room.3);
        rc.push_clip(room.0, room.1, room.2, room.3);
        if self.screen.cart.is_empty()
            && let Some(sm) = &f.small
        {
            let words = match self.screen.tab {
                VendorTab::Buy => {
                    "Nothing yet. Add to list keeps things here until you buy them all."
                }
                VendorTab::Sell => "Nothing yet. Drop things here from your packs, or Add to list.",
            };
            let lines = kit::wrap(sm, words, room.2 - 12.0);
            kit::lines(
                rc,
                sm,
                &lines,
                room.0 + 4.0,
                room.1 + 6.0,
                sm.line_height + 3.0,
                hex(0xA8_9A82, 1.0),
            );
        }
        for ((_, row, cross), r) in self.cart_rows().into_iter().zip(&self.screen.cart) {
            let i = self
                .screen
                .cart
                .iter()
                .position(|c| c.id == r.id)
                .unwrap_or(0);
            let tile = (row.0 + 4.0, row.1 + 5.0, 24.0, 24.0);
            rc.fill_rounded_rect(
                tile.0,
                tile.1,
                tile.2,
                tile.3,
                CornerRadii::all(4.0),
                hex(0, 0.45),
            );
            if let Some(icon) = icons.cart.get(i).copied().flatten() {
                rc.sprite(
                    icon,
                    tile.0,
                    tile.1,
                    24.0,
                    24.0,
                    (0.0, 0.0, 1.0, 1.0),
                    Color::WHITE,
                );
            }
            let price_w = 70.0;
            let (Some(st), Some(sm)) = (&f.strong, &f.small) else {
                continue;
            };
            let qty = format!("\u{d7}{}", r.quantity);
            let qw = text::measure(sm, &qty, 0.0);
            let name_room = cross.0 - 8.0 - price_w - 8.0 - qw - 8.0 - (tile.0 + 32.0);
            let ty = row.1 + ((CART_ROW - st.line_height) * 0.5).round();
            let name = kit::fit(st, &r.name, name_room);
            text::draw(rc, st, &name, tile.0 + 32.0, ty, TextStyle::plain(p.text));
            let nx = tile.0 + 32.0 + text::measure(st, &name, 0.0) + 8.0;
            let sy = row.1 + ((CART_ROW - sm.line_height) * 0.5).round();
            text::draw(rc, sm, &qty, nx, sy, TextStyle::plain(kit::SOFT));
            let pw = text::measure(st, &r.price, 0.0);
            text::draw(
                rc,
                st,
                &r.price,
                cross.0 - 8.0 - pw,
                ty,
                TextStyle::plain(p.text),
            );
            let lit = hot(self.hover, cross);
            stroke(
                rc,
                cross.0 + 4.0,
                cross.1 + 4.0,
                14.0,
                CROSS,
                if lit { p.accent } else { kit::SOFT },
                1.5,
            );
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            room.0 + room.2 + 2.0,
            room.1,
            room.3,
            total_h,
            self.cart_scroll.at,
        );
        // The total, what the player has now and after, the button.
        let button = self.cart_button();
        let total_y = button.1 - 8.0 - 18.0 - 6.0 - 22.0;
        rc.fill(b.0 + 14.0, total_y - 6.0, b.2 - 28.0, 1.0, bronze(0.2));
        if let (Some(st), Some(t)) = (&f.strong, &f.title) {
            text::draw(
                rc,
                st,
                self.screen.total_words,
                b.0 + 14.0,
                total_y + 2.0,
                TextStyle::plain(p.text),
            );
            let w = text::measure(t, &self.screen.total, 0.0);
            let color = if self.screen.tab == VendorTab::Sell {
                kit::GOOD
            } else {
                p.text
            };
            text::draw(
                rc,
                t,
                &self.screen.total,
                b.0 + b.2 - 14.0 - w,
                total_y,
                TextStyle::plain(color),
            );
        }
        if let Some(sm) = &f.small {
            let y = total_y + 22.0 + 4.0;
            text::draw(
                rc,
                sm,
                &self.screen.now,
                b.0 + 14.0,
                y,
                TextStyle::plain(kit::SOFT),
            );
            let w = text::measure(sm, &self.screen.after, 0.0);
            let color = if self.screen.after_bad {
                kit::HARM
            } else {
                kit::SOFT
            };
            text::draw(
                rc,
                sm,
                &self.screen.after,
                b.0 + b.2 - 14.0 - w,
                y,
                TextStyle::plain(color),
            );
        }
        kit::button(
            rc,
            &self.theme,
            button,
            &self.screen.cart_button,
            f.strong.as_ref(),
            ButtonLook::Primary,
            hot(self.hover, button),
            self.screen.can_cart,
        );
    }

    fn draw_refused(&self, rc: &mut RenderContext<'_>) {
        let Some(r) = &self.refused else {
            return;
        };
        let f = &self.fonts;
        let lines = 2 + usize::from(r.limit.is_some());
        let h = 14.0 + lines as f32 * 18.0 + 10.0;
        let b = (self.mid_x(), BOTTOM - h, self.mid_w(), h);
        let round = CornerRadii::all(4.0);
        rc.soft_shadow(b.0, b.1, b.2, b.3, 4.0, 8.0, 20.0, hex(0, 0.5));
        rc.fill_rounded_rect(b.0, b.1, b.2, b.3, round, hex(0x2A_1414, 0.97));
        rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, round, hex(0xFF_6B70, 0.6), 1.0);
        let mut y = b.1 + 12.0;
        if let Some(s) = &f.strong {
            let name = kit::fit(s, &r.name, b.2 - 24.0);
            text::draw(
                rc,
                s,
                &name,
                b.0 + 12.0,
                y,
                TextStyle::plain(self.theme.palette.text),
            );
            y += 18.0;
        }
        if let Some(sm) = &f.small {
            text::draw(
                rc,
                sm,
                &r.reason,
                b.0 + 12.0,
                y,
                TextStyle::plain(hex(0xFF_B3B6, 1.0)),
            );
            y += 18.0;
            if let Some(l) = &r.limit {
                text::draw(rc, sm, l, b.0 + 12.0, y, TextStyle::plain(kit::SOFT));
            }
        }
    }

    fn draw_leave(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.fill_rounded_rect(0.0, 0.0, self.width, HEIGHT, round, hex(0, 0.45));
        let b = self.leave_box();
        rc.soft_shadow(b.0, b.1, b.2, b.3, 6.0, 18.0, 50.0, hex(0, 0.6));
        rc.fill_rounded_rect(
            b.0,
            b.1,
            b.2,
            b.3,
            CornerRadii::all(6.0),
            hex(0x18_130E, 0.98),
        );
        rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, CornerRadii::all(6.0), bronze(0.8), 1.0);
        if let Some(s) = &f.strong {
            let q = format!("Leave {} with your lists unfinished?", self.screen.name);
            let q = kit::fit(s, &q, b.2 - 32.0);
            text::draw(rc, s, &q, b.0 + 16.0, b.1 + 16.0, TextStyle::plain(p.text));
        }
        if let Some(sm) = &f.small {
            let things = |n: usize| {
                if n == 1 {
                    "1 thing".to_owned()
                } else {
                    format!("{n} things")
                }
            };
            let words = match (self.screen.buy_waiting, self.screen.sell_waiting) {
                (b, 0) => format!(
                    "{} to buy {} not gone through.",
                    things(b),
                    if b == 1 { "has" } else { "have" }
                ),
                (0, s) => format!(
                    "{} to sell {} not gone through.",
                    things(s),
                    if s == 1 { "has" } else { "have" }
                ),
                (b, s) => format!(
                    "{} to buy and {s} to sell have not gone through.",
                    things(b)
                ),
            };
            let words = format!("{words} Nothing is bought or sold if you leave.");
            let lines = kit::wrap(sm, &words, b.2 - 32.0);
            kit::lines(
                rc,
                sm,
                &lines,
                b.0 + 16.0,
                b.1 + 42.0,
                sm.line_height + 4.0,
                kit::WARM,
            );
        }
        let (stay, leave) = self.leave_buttons();
        kit::button(
            rc,
            &self.theme,
            stay,
            "Stay",
            f.strong.as_ref(),
            ButtonLook::Plain,
            hot(self.hover, stay),
            true,
        );
        kit::button(
            rc,
            &self.theme,
            leave,
            "Leave",
            f.strong.as_ref(),
            ButtonLook::Primary,
            hot(self.hover, leave),
            true,
        );
    }
}

/// The icons the frame draws (the window makes them).
#[derive(Debug, Clone, Default)]
struct CardIcons {
    chosen: Option<TextureRef>,
    cart: Vec<Option<TextureRef>>,
}

fn tone_color(tone: Tone, quiet: Color) -> Color {
    match tone {
        Tone::Quiet => quiet,
        Tone::Good => kit::GOOD,
        Tone::Warn => gold(1.0),
        Tone::Bad => hex(0xFF_B3B6, 1.0),
    }
}

impl Widget for VendorFrame {
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
                self.click(x, y);
                true
            }
            kind::SCROLL => {
                if let Some((hx, hy)) = self.hover
                    && inside(self.cart_rows_room(), hx, hy)
                {
                    let total = self.screen.cart.len() as f32 * (CART_ROW + 2.0);
                    self.cart_scroll
                        .wheel(e.data0, CART_ROW * 2.0, total, self.cart_rows_room().3);
                    return true;
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
                    self.asks.push(VendorAsk::Dropped(p.clone()));
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

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        kit::window_back(rc, &self.theme, self.width, HEIGHT, HEADER, true);
        self.draw_header(rc);
        if !self.narrow() {
            self.draw_rail(rc);
        }
        self.draw_heading(rc);
        let icons = CardIcons {
            chosen: self.screen.chosen.as_ref().and_then(|c| c.icon),
            cart: self.screen.cart.iter().map(|c| c.icon).collect(),
        };
        self.draw_card(rc, &icons);
        self.draw_cart(rc, &icons);
        if self.over {
            let b = self.cart_box();
            rc.stroke_rounded_rect(
                b.0 - 2.0,
                b.1 - 2.0,
                b.2 + 4.0,
                b.3 + 4.0,
                CornerRadii::all(6.0),
                gold(1.0),
                2.0,
            );
        }
    }

    fn draw_after_children(
        &mut self,
        _ui: &mut UiRoot,
        _me: ElementId,
        rc: &mut RenderContext<'_>,
    ) {
        if self.narrow() && self.rail_open {
            let r = self.rail_panel();
            let round = CornerRadii::all(6.0);
            rc.soft_shadow(r.0, r.1, r.2, r.3, 6.0, 12.0, 30.0, hex(0, 0.5));
            ac_ui::modern::theme::fill_glass(rc, r, round, hex(0x14_100C, 0.97));
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.6), 1.0);
            self.draw_rail(rc);
        }
        self.draw_refused(rc);
        if self.leaving {
            self.draw_leave(rc);
        }
    }
}

/// Whether `p` comes from the player's packs (not a shortcut or the
/// window's own list).
fn from_packs(p: &ItemDragPayload) -> bool {
    !p.source_kind.is_alias() && p.source_kind != ItemDragSource::Ground && p.obj_id != 0
}

/// What the list asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum ListAsk {
    /// Choose a row.
    Choose(u32),
    /// Put a row on the list (a double click).
    Add(u32),
    /// Its full details (a right click).
    Inspect(u32),
    /// An item from the packs let go over the list (as over the window).
    Dropped(ItemDragPayload),
}

/// The list of goods, or of the player's items.
#[derive(Debug)]
pub struct VendorList {
    theme: Horizon,
    fonts: VendorFonts,
    /// The rows.
    pub rows: Vec<VendorRow>,
    /// The chosen row.
    pub chosen: Option<u32>,
    /// What it asked for since the window last looked.
    pub asks: Vec<ListAsk>,
    scroll: Scroll,
    hover: Option<(f32, f32)>,
}

impl VendorList {
    /// A list in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: VendorFonts) -> Self {
        Self {
            theme,
            fonts,
            rows: Vec::new(),
            chosen: None,
            asks: Vec::new(),
            scroll: Scroll::default(),
            hover: None,
        }
    }

    /// Back to the top.
    pub fn to_top(&mut self) {
        self.scroll = Scroll::default();
    }

    fn row_at(&self, y: f32) -> Option<u32> {
        let i = ((y + self.scroll.at) / (ROW + 2.0)).floor();
        if i < 0.0 {
            return None;
        }
        let i = i as usize;
        let within = (y + self.scroll.at) - i as f32 * (ROW + 2.0) < ROW;
        self.rows.get(i).filter(|_| within).map(|r| r.id)
    }

    fn draw_row(&self, rc: &mut RenderContext<'_>, r: &VendorRow, y: f32, w: f32) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let on = self.chosen == Some(r.id);
        let lit = self.hover.is_some_and(|(_, hy)| hy >= y && hy < y + ROW);
        let round = CornerRadii::all(4.0);
        if on {
            rc.fill_rounded_rect(0.0, y, w, ROW, round, gold(0.14));
            rc.stroke_rounded_rect(0.0, y, w, ROW, round, gold(1.0), 1.0);
        } else if lit {
            rc.fill_rounded_rect(0.0, y, w, ROW, round, hex(0xFF_FFFF, 0.04));
        }
        let fade = if r.refused { 0.45 } else { 1.0 };
        let tile = (6.0, y + 6.0, 34.0, 34.0);
        rc.fill_rounded_rect(tile.0, tile.1, tile.2, tile.3, round, hex(0, 0.45 * fade));
        if let Some(icon) = r.icon {
            rc.sprite(
                icon,
                tile.0 + 1.0,
                tile.1 + 1.0,
                32.0,
                32.0,
                (0.0, 0.0, 1.0, 1.0),
                Color::rgba(1.0, 1.0, 1.0, fade),
            );
        }
        let price_w = 86.0;
        let chip_w = r.on_list.as_ref().map_or(0.0, |c| {
            f.tiny
                .as_ref()
                .map_or(60.0, |t| text::measure(t, c, 0.0) + 12.0)
                + 10.0
        });
        let right = w - 10.0 - price_w - chip_w - 10.0;
        let x = tile.0 + 34.0 + 10.0;
        if let Some(s) = &f.strong {
            let mark_w = match (&r.mark, &f.tiny) {
                (Some((m, _)), Some(t)) => text::measure(t, m, 0.0) + 8.0,
                _ => 0.0,
            };
            let name = kit::fit(s, &r.name, right - x - mark_w);
            text::draw(
                rc,
                s,
                &name,
                x,
                y + 7.0,
                TextStyle::plain(Color { a: fade, ..p.text }),
            );
            if let (Some((m, color)), Some(t)) = (&r.mark, &f.tiny) {
                let mx = x + text::measure(s, &name, 0.0) + 8.0;
                text::draw(rc, t, m, mx, y + 9.0, TextStyle::plain(*color));
            }
        }
        if let Some(sm) = &f.small {
            let sub = kit::fit(sm, &r.sub, right - x);
            text::draw(
                rc,
                sm,
                &sub,
                x,
                y + 26.0,
                TextStyle::plain(tone_color(r.sub_tone, kit::SOFT)),
            );
        }
        if let (Some(chip), Some(t)) = (&r.on_list, &f.tiny) {
            let cw = text::measure(t, chip, 0.0) + 12.0;
            let cx = w - 10.0 - price_w - 10.0 - cw;
            let cy = y + (ROW - 18.0) * 0.5;
            rc.fill_rounded_rect(cx, cy, cw, 18.0, CornerRadii::all(3.0), gold(0.18));
            let ty = cy + ((18.0 - t.line_height) * 0.5).round();
            text::draw(rc, t, chip, cx + 6.0, ty, TextStyle::plain(gold(1.0)));
        }
        if let Some(s) = &f.strong {
            let pw = text::measure(s, &r.price, 0.0);
            text::draw(
                rc,
                s,
                &r.price,
                w - 10.0 - pw,
                y + 8.0,
                TextStyle::plain(Color { a: fade, ..p.text }),
            );
        }
        if let Some(t) = &f.tiny {
            let sw = text::measure(t, &r.price_sub, 0.0);
            text::draw(
                rc,
                t,
                &r.price_sub,
                w - 10.0 - sw,
                y + 27.0,
                TextStyle::plain(hex(0xA8_9A82, 1.0)),
            );
        }
    }
}

impl Widget for VendorList {
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
                true
            }
            kind::HOVER_LEAVE => {
                self.hover = None;
                true
            }
            kind::CLICK => {
                if let Some(id) = self.row_at(y) {
                    self.asks.push(ListAsk::Choose(id));
                }
                true
            }
            kind::DOUBLE_CLICK => {
                if let Some(id) = self.row_at(y) {
                    self.asks.push(ListAsk::Add(id));
                }
                true
            }
            kind::RIGHT_CLICK => {
                if let Some(id) = self.row_at(y) {
                    self.asks.push(ListAsk::Choose(id));
                    self.asks.push(ListAsk::Inspect(id));
                }
                true
            }
            kind::DRAG_ENTER | kind::DRAG_OVER => true,
            kind::DROP_RELEASED => {
                if let Some(p) = e
                    .payload
                    .as_ref()
                    .and_then(|p| p.get::<ItemDragPayload>())
                    .filter(|p| from_packs(p))
                {
                    self.asks.push(ListAsk::Dropped(p.clone()));
                }
                true
            }
            kind::SCROLL => {
                let room = ui.element(me).map_or(400.0, |e| e.height);
                let total = self.rows.len() as f32 * (ROW + 2.0);
                self.scroll.wheel(e.data0, ROW * 2.0, total, room);
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let (w, h) = ui
            .element(me)
            .map_or((MID_W, 400.0), |e| (e.width, e.height));
        let total = self.rows.len() as f32 * (ROW + 2.0);
        self.scroll.hold(total, h);
        let inner = w - 8.0;
        rc.push_clip(0.0, 0.0, w, h);
        if self.rows.is_empty()
            && let Some(sm) = &self.fonts.small
        {
            text::draw(
                rc,
                sm,
                "Nothing here.",
                10.0,
                12.0,
                TextStyle::plain(kit::SOFT),
            );
        }
        for (i, r) in self.rows.iter().enumerate() {
            let y = i as f32 * (ROW + 2.0) - self.scroll.at;
            if y + ROW < 0.0 || y > h {
                continue;
            }
            self.draw_row(rc, r, y, inner);
        }
        rc.pop_clip();
        kit::scrollbar(rc, w - 5.0, 0.0, h, total, self.scroll.at);
    }
}
