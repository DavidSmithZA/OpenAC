//! The House window's frame: the house (its type and place, Show on map,
//! the dates, each maintenance item with a bar, the paid or warning note,
//! the rule for buying another) and who may come in (OPENAC: open house,
//! the allegiance, the guests with their storage, add a guest, send
//! visitors out, abandon); with no house, when another may be bought and
//! buttons that show free houses on the map.

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text;
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::{HouseScreen, OwnedHouse, Standing};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{
    self, ButtonLook, Hits, MUTED, Rect, bronze, gold, put, put_mid, put_right, spaced,
};

/// The window's size.
pub const WIDTH: f32 = 900.0;
pub const HEIGHT: f32 = 600.0;
const HEADER: f32 = 52.0;
const PAD: f32 = 20.0;
const TOP: f32 = 72.0;
const LEFT_W: f32 = 420.0;
const RIGHT_X: f32 = 460.0;
const GUEST_ROW: f32 = 34.0;

/// A house, the window's glyph.
pub const HOUSE: &[&[(f32, f32)]] = &[
    &[(3.0, 11.0), (12.0, 4.0), (21.0, 11.0)],
    &[(5.0, 10.0), (5.0, 20.0), (19.0, 20.0), (19.0, 10.0)],
    &[(10.0, 20.0), (10.0, 15.0), (14.0, 15.0), (14.0, 20.0)],
];
const TICK: &[&[(f32, f32)]] = &[&[(6.0, 12.0), (10.0, 16.0), (18.0, 8.0)]];
const WARN: &[&[(f32, f32)]] = &[
    &[(12.0, 3.0), (2.0, 20.0), (22.0, 20.0), (12.0, 3.0)],
    &[(12.0, 10.0), (12.0, 14.0)],
    &[(12.0, 17.0), (12.0, 17.5)],
];
const CROSS: &[&[(f32, f32)]] = &[&[(7.0, 7.0), (17.0, 17.0)], &[(17.0, 7.0), (7.0, 17.0)]];

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct HouseFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// The house's type.
    pub heading: Option<Arc<UiFont>>,
    /// Names, buttons, values.
    pub strong: Option<Arc<UiFont>>,
    /// Words.
    pub body: Option<Arc<UiFont>>,
    /// Quiet lines.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals, tags.
    pub label: Option<Arc<UiFont>>,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseAsk {
    /// Close the window.
    Close,
    /// Open the map on a cell.
    ShowOnMap(u32),
    /// Show free houses of a type on the map (1 cottage, 2 villa, 3
    /// mansion).
    ForSale(u32),
    /// Anyone may come in, or not.
    Open(bool),
    /// The allegiance may come in, or not; may use the storage, or not.
    AllegianceIn(bool),
    AllegianceStorage(bool),
    /// A guest may use the storage, or not.
    Storage(String, bool),
    /// Take a guest off the list.
    RemoveGuest(String),
    /// Add the guest typed.
    AddGuest,
    /// Ask the server who may come in.
    ListGuests,
    /// Send out everyone not on the guest list.
    SendOut,
    /// Abandon the house (asks first).
    Abandon,
}

/// The House window's frame.
#[derive(Debug)]
pub struct HouseFrame {
    theme: Horizon,
    fonts: HouseFonts,
    /// What it shows.
    pub screen: HouseScreen,
    /// The guest field holds nothing; it has the keyboard.
    pub guest_empty: bool,
    pub guest_focused: bool,
    /// What it asked for since the window last looked.
    pub asks: Vec<HouseAsk>,
    hits: Hits<Option<HouseAsk>>,
}

impl HouseFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: HouseFonts) -> Self {
        Self {
            theme,
            fonts,
            screen: HouseScreen::default(),
            guest_empty: true,
            guest_focused: false,
            asks: Vec::new(),
            hits: Hits::default(),
        }
    }

    fn right_box() -> Rect {
        (RIGHT_X, TOP, WIDTH - PAD - RIGHT_X, HEIGHT - PAD - TOP)
    }

    /// Where the guest field goes (none: it hides).
    #[must_use]
    pub fn guest_field_rect(&self) -> Option<Rect> {
        let access = self.screen.access.as_ref()?;
        self.screen.owned.as_ref()?;
        let b = Self::right_box();
        let y = b.1
            + 14.0
            + 20.0
            + 3.0 * 40.0
            + 30.0
            + access.guests.len().max(1) as f32 * (GUEST_ROW + 2.0)
            + 8.0;
        Some((b.0 + 14.0, y, b.2 - 28.0 - 70.0, 30.0))
    }

    #[allow(clippy::too_many_arguments)]
    fn button(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Option<HouseAsk>>,
        r: Rect,
        words: &str,
        look: ButtonLook,
        on: bool,
        ask: HouseAsk,
    ) {
        let lit = h.add(r, on.then_some(ask)) && on;
        kit::button(
            rc,
            &self.theme,
            r,
            words,
            self.fonts.strong.as_ref(),
            look,
            lit,
            on,
        );
    }

    #[allow(clippy::too_many_lines)]
    fn draw_owned(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Option<HouseAsk>>,
        o: &OwnedHouse,
    ) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let x = PAD;
        let mut y = TOP;
        // The house and where it is.
        let tile = (x, y, 44.0, 44.0);
        rc.fill_vertical_gradient(
            tile.0,
            tile.1,
            tile.2,
            tile.3,
            CornerRadii::all(4.0),
            gold(0.2),
            hex(0, 0.5),
        );
        rc.stroke_rounded_rect(
            tile.0,
            tile.1,
            tile.2,
            tile.3,
            CornerRadii::all(4.0),
            bronze(0.6),
            1.0,
        );
        stroke(rc, x + 10.0, y + 10.0, 24.0, HOUSE, gold(1.0), 1.8);
        let mut right = x + LEFT_W;
        if o.cell != 0 {
            let w = kit::button_width("Show on map", f.strong.as_ref());
            right -= w;
            self.button(
                rc,
                h,
                (right, y + 7.0, w, 30.0),
                "Show on map",
                ButtonLook::Plain,
                true,
                HouseAsk::ShowOnMap(o.cell),
            );
            right -= 10.0;
        }
        put(rc, f.heading.as_ref(), o.kind, x + 56.0, y + 3.0, p.text);
        let place = o.place.clone().unwrap_or_else(|| "An apartment".to_owned());
        if let Some(s) = &f.small {
            put(
                rc,
                Some(s),
                &kit::fit(s, &place, right - x - 56.0),
                x + 56.0,
                y + 26.0,
                kit::SOFT,
            );
        }
        y += 56.0;
        for (k, v, red) in &o.rows {
            put_mid(rc, f.body.as_ref(), k, x, y, 26.0, kit::WARM);
            put_right(
                rc,
                f.strong.as_ref(),
                v,
                x + LEFT_W,
                y,
                26.0,
                if *red { kit::HARM } else { p.text },
            );
            y += 26.0;
        }
        y += 10.0;
        kit::group_heading(
            rc,
            &self.theme,
            f.label.as_ref(),
            x,
            y,
            16.0,
            x + LEFT_W - 70.0,
            "MAINTENANCE",
            None,
            kit::SOFT,
        );
        if let Some(l) = &f.label {
            let ink = match o.period {
                "NOT PAID" => kit::HARM,
                "PAID" => kit::GOOD,
                _ => kit::SOFT,
            };
            let w = text::measure(l, o.period, kit::LABEL_TRACKING);
            text::draw(
                rc,
                l,
                o.period,
                x + LEFT_W - w,
                y + ((16.0 - l.line_height) * 0.5).round(),
                text::TextStyle::plain(ink).tracked(kit::LABEL_TRACKING),
            );
        }
        y += 24.0;
        for r in &o.rent {
            let ink = match r.standing {
                Standing::Paid => kit::GOOD,
                Standing::Owed => gold(1.0),
                Standing::Due => kit::RED,
            };
            put(rc, f.strong.as_ref(), &r.name, x, y, p.text);
            put_right(rc, f.strong.as_ref(), &r.text, x + LEFT_W, y, 16.0, ink);
            kit::meter(rc, (x, y + 20.0, LEFT_W, 4.0), r.share as f32 / 1000.0, ink);
            y += 34.0;
        }
        // The note.
        let note_w = LEFT_W - 46.0;
        let lines = f
            .body
            .as_ref()
            .map(|b| kit::wrap(b, &o.note, note_w))
            .unwrap_or_default();
        let lh = f.body.as_ref().map_or(16.0, |b| b.line_height + 4.0);
        let nh = 20.0 + lines.len() as f32 * lh;
        let r = (x, y + 2.0, LEFT_W, nh);
        let (edge, fill, ink): (Color, Color, Color) = if o.warns {
            (hex(0xE5_484D, 0.6), hex(0xE5_484D, 0.10), p.text)
        } else {
            (hex(0x7F_D08A, 0.45), hex(0x7F_D08A, 0.08), kit::WARM)
        };
        rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), fill);
        rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), edge, 1.0);
        if o.warns {
            stroke(rc, r.0 + 10.0, r.1 + 10.0, 16.0, WARN, kit::HARM, 1.6);
        } else {
            stroke(rc, r.0 + 10.0, r.1 + 10.0, 16.0, TICK, kit::GOOD, 1.8);
        }
        if let Some(b) = &f.body {
            kit::lines(rc, b, &lines, r.0 + 36.0, r.1 + 10.0, lh, ink);
        }
        // The rule for buying another.
        if let Some(s) = &f.small {
            let foot = format!(
                "{} Pay maintenance at the house itself.",
                self.screen.buy_another
            );
            let lines = kit::wrap(s, &foot, LEFT_W);
            let lh = s.line_height + 4.0;
            let top = HEIGHT - PAD - lines.len() as f32 * lh;
            kit::lines(rc, s, &lines, x, top, lh, MUTED);
        }
        self.draw_access(rc, h);
    }

    #[allow(clippy::too_many_lines)]
    fn draw_access(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Option<HouseAsk>>) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let b = Self::right_box();
        kit::panel(rc, b, 0.3);
        let x = b.0 + 14.0;
        let w = b.2 - 28.0;
        let mut y = b.1 + 14.0;
        spaced(rc, f.label.as_ref(), "WHO MAY COME IN", x, y + 2.0);
        kit::openac_tag(rc, f.label.as_ref(), x + w - 58.0, y + 8.0);
        y += 26.0;
        let Some(a) = &self.screen.access else {
            if let Some(bf) = &f.body {
                let lines = kit::wrap(bf, "The server has not said who may come in yet.", w);
                y = kit::lines(rc, bf, &lines, x, y, bf.line_height + 4.0, MUTED) + 8.0;
            }
            self.button(
                rc,
                h,
                (x, y, 150.0, 30.0),
                "Ask the server",
                ButtonLook::Plain,
                true,
                HouseAsk::ListGuests,
            );
            self.draw_house_actions(rc, h, b);
            return;
        };
        for (words, note, on, ask) in [
            (
                "Open house",
                "Anyone may come in",
                a.open,
                HouseAsk::Open(!a.open),
            ),
            (
                "Your allegiance may come in",
                "Everyone sworn to your monarch",
                a.allegiance_guests,
                HouseAsk::AllegianceIn(!a.allegiance_guests),
            ),
            (
                "Your allegiance may use storage",
                "The chests in the house",
                a.allegiance_storage,
                HouseAsk::AllegianceStorage(!a.allegiance_storage),
            ),
        ] {
            let r = (x, y, w, 36.0);
            let lit = h.add(r, Some(ask));
            kit::switch(rc, (x, y + 8.0, 38.0, 20.0), on, lit);
            put(rc, f.strong.as_ref(), words, x + 48.0, y + 1.0, p.text);
            put(rc, f.small.as_ref(), note, x + 48.0, y + 19.0, MUTED);
            y += 40.0;
        }
        y += 4.0;
        let count = format!("· {}", a.guests.len());
        kit::group_heading(
            rc,
            &self.theme,
            f.label.as_ref(),
            x,
            y,
            18.0,
            x + w - 70.0,
            "GUESTS",
            Some(&count),
            kit::SOFT,
        );
        if let Some(l) = &f.label {
            let words = "STORAGE";
            let lw = text::measure(l, words, kit::LABEL_TRACKING);
            text::draw(
                rc,
                l,
                words,
                x + w - lw,
                y + ((18.0 - l.line_height) * 0.5).round(),
                text::TextStyle::plain(kit::SOFT).tracked(kit::LABEL_TRACKING),
            );
        }
        y += 26.0;
        if a.guests.is_empty() {
            put_mid(
                rc,
                f.body.as_ref(),
                "Nobody is on the guest list.",
                x,
                y,
                GUEST_ROW,
                MUTED,
            );
        }
        for (name, store) in &a.guests {
            let r = (x, y, w, GUEST_ROW);
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), hex(0, 0.22));
            put_mid(rc, f.strong.as_ref(), name, x + 10.0, y, GUEST_ROW, p.text);
            let cross = (x + w - 30.0, y + 3.0, 28.0, 28.0);
            let lit = h.add(cross, Some(HouseAsk::RemoveGuest(name.clone())));
            stroke(
                rc,
                cross.0 + 6.0,
                cross.1 + 6.0,
                16.0,
                CROSS,
                if lit { kit::HARM } else { kit::SOFT },
                1.6,
            );
            let bx = (x + w - 30.0 - 30.0, y + 8.0, 18.0, 18.0);
            let lit = h.add(
                (bx.0 - 6.0, y, 30.0, GUEST_ROW),
                Some(HouseAsk::Storage(name.clone(), !store)),
            );
            rc.fill_rounded_rect(
                bx.0,
                bx.1,
                bx.2,
                bx.3,
                CornerRadii::all(3.0),
                if *store { gold(1.0) } else { hex(0, 0.0) },
            );
            rc.stroke_rounded_rect(
                bx.0,
                bx.1,
                bx.2,
                bx.3,
                CornerRadii::all(3.0),
                if *store || lit { gold(1.0) } else { gold(0.6) },
                1.5,
            );
            if *store {
                stroke(rc, bx.0, bx.1, 18.0, TICK, hex(0x1A_140D, 1.0), 2.0);
            }
            y += GUEST_ROW + 2.0;
        }
        if let Some(r) = self.guest_field_rect() {
            kit::field_box(
                rc,
                &self.theme,
                r,
                "Add a guest by name",
                self.guest_empty,
                self.guest_focused,
                f.body.as_ref(),
            );
            self.button(
                rc,
                h,
                (r.0 + r.2 + 6.0, r.1, 64.0, 30.0),
                "Add",
                ButtonLook::Plain,
                !self.guest_empty,
                HouseAsk::AddGuest,
            );
        }
        self.draw_house_actions(rc, h, b);
    }

    fn draw_house_actions(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Option<HouseAsk>>,
        b: Rect,
    ) {
        let x = b.0 + 14.0;
        let w = (b.2 - 28.0 - 6.0) * 0.5;
        let y = b.1 + b.3 - 14.0 - 32.0;
        self.button(
            rc,
            h,
            (x, y, w, 32.0),
            "Send visitors out",
            ButtonLook::Plain,
            true,
            HouseAsk::SendOut,
        );
        self.button(
            rc,
            h,
            (x + w + 6.0, y, w, 32.0),
            "Abandon house…",
            ButtonLook::Danger,
            true,
            HouseAsk::Abandon,
        );
    }

    fn draw_none(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Option<HouseAsk>>) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let cx = WIDTH * 0.5;
        let mut y = HEADER + (HEIGHT - HEADER) * 0.5 - 110.0;
        stroke(rc, cx - 18.0, y, 36.0, HOUSE, gold(0.8), 1.8);
        y += 50.0;
        let title = if self.screen.waiting {
            "Asking the server about your house"
        } else {
            "You don't own a house"
        };
        if let Some(hf) = &f.heading {
            text::draw_centred(rc, hf, title, cx, y, text::TextStyle::plain(p.text));
            y += hf.line_height + 10.0;
        }
        if let Some(b) = &f.body {
            for line in kit::wrap(b, &self.screen.buy_another, 460.0) {
                text::draw_centred(rc, b, &line, cx, y, text::TextStyle::plain(kit::SOFT));
                y += b.line_height + 4.0;
            }
        }
        y += 16.0;
        let words = "Show free houses on the map:";
        let tw = f
            .body
            .as_ref()
            .map_or(180.0, |b| text::measure(b, words, 0.0));
        let tx = cx - (tw + 10.0 + 58.0) * 0.5;
        put_mid(rc, f.body.as_ref(), words, tx, y, 20.0, kit::WARM);
        kit::openac_tag(rc, f.label.as_ref(), tx + tw + 10.0, y + 10.0);
        y += 30.0;
        let kinds = [("Cottages", 1), ("Villas", 2), ("Mansions", 3)];
        let widths: Vec<f32> = kinds
            .iter()
            .map(|(w, _)| kit::button_width(w, f.strong.as_ref()))
            .collect();
        let total: f32 = widths.iter().sum::<f32>() + 12.0;
        let mut x = cx - total * 0.5;
        for ((words, kind), w) in kinds.into_iter().zip(widths) {
            self.button(
                rc,
                h,
                (x, y, w, 32.0),
                words,
                ButtonLook::Plain,
                true,
                HouseAsk::ForSale(kind),
            );
            x += w + 6.0;
        }
        y += 46.0;
        if let Some(s) = &f.small {
            text::draw_centred(
                rc,
                s,
                "Apartments have no place on the map; the server says how many are free.",
                cx,
                y,
                text::TextStyle::plain(MUTED),
            );
        }
    }
}

impl Widget for HouseFrame {
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
                self.hits.hover = Some((x, y));
                false
            }
            kind::HOVER_LEAVE => {
                self.hits.hover = None;
                false
            }
            kind::CLICK => {
                if let Some(Some(ask)) = self.hits.at(x, y) {
                    self.asks.push(ask);
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let mut h = Hits::new(self.hits.hover);
        let theme = self.theme;
        kit::window_back(rc, &theme, WIDTH, HEIGHT, HEADER, true);
        let right = kit::title(
            rc,
            &theme,
            self.fonts.title.as_ref(),
            HOUSE,
            "House",
            HEADER,
        );
        put_mid(
            rc,
            self.fonts.small.as_ref(),
            "H",
            right + 10.0,
            0.0,
            HEADER,
            theme.palette.muted,
        );
        let close = kit::close_rect(WIDTH, HEADER);
        let lit = h.add(close, Some(HouseAsk::Close));
        kit::close(rc, &theme, close, lit);
        match self.screen.owned.clone() {
            Some(o) => self.draw_owned(rc, &mut h, &o),
            None => self.draw_none(rc, &mut h),
        }
        self.hits = h;
    }
}
