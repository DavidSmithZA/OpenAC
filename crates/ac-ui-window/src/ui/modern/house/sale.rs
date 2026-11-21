//! The house sale window's frame (retail's slumlord window): the house's
//! type and owner, Buy and Maintenance tabs; who may buy it (each rule
//! ticked against the player) or whose it is; a row for each price line
//! with what the player carries, how much is on the row and Add; what is
//! on the rows (click to take one off); Buy or Pay maintenance.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_runtime::inventory::HousePurchaseTab;
use ac_ui::draw::RenderContext;
use ac_ui::event::{ItemDragPayload, ItemDragSource, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text;
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::SaleScreen;
use super::window::HOUSE;
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, ButtonLook, Hits, Rect, bronze, gold, put, put_mid, put_right};

/// The window's size.
pub const WIDTH: f32 = 640.0;
pub const HEIGHT: f32 = 560.0;
const HEADER: f32 = 52.0;
const TABS: f32 = 40.0;
const PAD: f32 = 20.0;
const FOOT: f32 = 64.0;
const ROW: f32 = 50.0;
const TICK: &[&[(f32, f32)]] = &[&[(5.0, 12.0), (10.0, 17.0), (19.0, 7.0)]];
const CROSS: &[&[(f32, f32)]] = &[&[(7.0, 7.0), (17.0, 17.0)], &[(17.0, 7.0), (7.0, 17.0)]];

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum SaleAsk {
    /// Close the window.
    Close,
    /// Put what the player carries for a price line (its weenie class) on
    /// the rows.
    Add(u32),
    /// Take an item off the rows.
    TakeOff(u32),
    /// Buy, or pay the maintenance (asks first when the game does).
    Send,
    /// An item let go on the rows.
    Dropped(ItemDragPayload),
}

#[derive(Debug, Clone, PartialEq)]
enum Hit {
    Ask(SaleAsk),
    Tab(HousePurchaseTab),
}

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct SaleFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// Names, buttons, values.
    pub strong: Option<Arc<UiFont>>,
    /// Words.
    pub body: Option<Arc<UiFont>>,
    /// Quiet lines.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals.
    pub label: Option<Arc<UiFont>>,
}

/// The house sale window's frame.
#[derive(Debug)]
pub struct SaleFrame {
    fonts: SaleFonts,
    /// What it shows.
    pub screen: SaleScreen,
    /// The tab shown.
    pub tab: HousePurchaseTab,
    /// What it asked for since the window last looked.
    pub asks: Vec<SaleAsk>,
    hits: Hits<Hit>,
    over: bool,
}

impl SaleFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(fonts: SaleFonts) -> Self {
        Self {
            fonts,
            screen: SaleScreen::default(),
            tab: HousePurchaseTab::Buy,
            asks: Vec::new(),
            hits: Hits::default(),
            over: false,
        }
    }

    fn rows_area() -> Rect {
        (
            PAD,
            HEADER + TABS,
            WIDTH - 2.0 * PAD,
            HEIGHT - FOOT - HEADER - TABS,
        )
    }
}

fn from_packs(p: &ItemDragPayload) -> bool {
    matches!(
        p.source_kind,
        ItemDragSource::Inventory | ItemDragSource::Ground
    )
}

impl Widget for SaleFrame {
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
                match self.hits.at(x, y) {
                    Some(Hit::Ask(a)) => self.asks.push(a),
                    Some(Hit::Tab(t)) => self.tab = t,
                    None => {}
                }
                true
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
                    self.asks.push(SaleAsk::Dropped(p.clone()));
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

    #[allow(clippy::too_many_lines)]
    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let theme = Horizon::current();
        let p = Horizon::current().palette;
        let f = self.fonts.clone();
        let s = self.screen.clone();
        let mut h = Hits::new(self.hits.hover);
        kit::window_back(rc, &theme, WIDTH, HEIGHT, HEADER, false);
        let right = kit::title(rc, &theme, f.title.as_ref(), HOUSE, s.kind, HEADER);
        let (ink, edge) = if s.for_sale {
            (kit::GOOD, hex(0x7F_D08A, 0.5))
        } else if s.yours {
            (gold(1.0), gold(0.6))
        } else {
            (tone::soft(1.0), bronze(0.4))
        };
        if let Some(st) = &f.strong {
            let w = text::measure(st, &s.owner, 0.0) + 30.0;
            kit::pill(
                rc,
                Some(st),
                right + 12.0 + w,
                HEADER * 0.5,
                &s.owner,
                ink,
                edge,
            );
        }
        let close = kit::close_rect(WIDTH, HEADER);
        let lit = h.add(close, Hit::Ask(SaleAsk::Close));
        kit::close(rc, &theme, close, lit);
        // The tabs.
        rc.fill(0.0, HEADER + TABS - 1.0, WIDTH, 1.0, bronze(0.35));
        let mut x = 16.0;
        for (tab, words) in [
            (HousePurchaseTab::Buy, "Buy"),
            (HousePurchaseTab::Rent, "Maintenance"),
        ] {
            let t = kit::Tab {
                words,
                count: None,
                on: self.tab == tab,
                red: false,
            };
            let w = kit::tab_width(&t, f.strong.as_ref(), f.label.as_ref());
            let r = (x, HEADER, w, TABS);
            let lit = h.add(r, Hit::Tab(tab));
            kit::draw_tab(rc, &theme, &t, r, f.strong.as_ref(), f.label.as_ref(), lit);
            x += w + 4.0;
        }
        // The rules.
        let mut y = HEADER + TABS + 12.0;
        let heading = if self.tab == HousePurchaseTab::Buy {
            "WHO MAY BUY IT"
        } else {
            "THIS HOUSE"
        };
        kit::group_heading(
            rc,
            &theme,
            f.label.as_ref(),
            PAD,
            y,
            20.0,
            WIDTH - PAD,
            heading,
            None,
            tone::soft(1.0),
        );
        y += 24.0;
        for (k, v, ok) in &s.rules {
            put_mid(rc, f.body.as_ref(), k, PAD, y, 24.0, tone::warm(1.0));
            let ink = match ok {
                Some(true) => kit::GOOD,
                Some(false) => kit::HARM,
                None => p.text,
            };
            put_right(rc, f.strong.as_ref(), v, WIDTH - PAD - 20.0, y, 24.0, ink);
            match ok {
                Some(true) => stroke(rc, WIDTH - PAD - 14.0, y + 5.0, 14.0, TICK, ink, 1.8),
                Some(false) => stroke(rc, WIDTH - PAD - 14.0, y + 5.0, 14.0, CROSS, ink, 1.8),
                None => {}
            }
            y += 24.0;
        }
        // The price rows.
        y += 10.0;
        let heading = if self.tab == HousePurchaseTab::Buy {
            "PRICE"
        } else {
            "MAINTENANCE THIS PERIOD"
        };
        kit::group_heading(
            rc,
            &theme,
            f.label.as_ref(),
            PAD,
            y,
            20.0,
            WIDTH - PAD,
            heading,
            None,
            tone::soft(1.0),
        );
        y += 26.0;
        let area = Self::rows_area();
        if self.over {
            rc.fill_rounded_rect(
                area.0 - 6.0,
                y - 4.0,
                area.2 + 12.0,
                s.rows.len() as f32 * (ROW + 4.0) + 8.0,
                CornerRadii::all(4.0),
                gold(0.06),
            );
        }
        for row in &s.rows {
            let r = (PAD, y, WIDTH - 2.0 * PAD, ROW);
            let full = row.full();
            let edge = if full {
                hex(0x7F_D08A, 0.5)
            } else if self.over {
                gold(0.9)
            } else {
                bronze(0.45)
            };
            dashed(rc, r, edge);
            if full {
                rc.fill_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(4.0),
                    hex(0x7F_D08A, 0.06),
                );
            }
            let add_w = 64.0;
            let status_x = r.0 + r.2 - 10.0 - add_w - 10.0;
            put(
                rc,
                f.strong.as_ref(),
                &row.name,
                r.0 + 12.0,
                y + 7.0,
                p.text,
            );
            let nw = f
                .strong
                .as_ref()
                .map_or(80.0, |st| text::measure(st, &row.name, 0.0));
            put(
                rc,
                f.small.as_ref(),
                &row.carry,
                r.0 + 20.0 + nw,
                y + 9.0,
                tone::muted(1.0),
            );
            let share = if row.num > 0 {
                ((row.paid + row.staged) as f32 / row.num as f32).min(1.0)
            } else {
                1.0
            };
            kit::meter(
                rc,
                (r.0 + 12.0, y + 32.0, status_x - 110.0 - r.0, 4.0),
                share,
                if full { kit::GOOD } else { gold(1.0) },
            );
            let status = if full {
                "Covered".to_owned()
            } else {
                format!(
                    "{} of {}",
                    ac_ui::modern::numbers::group_thousands(i64::from(row.paid + row.staged)),
                    ac_ui::modern::numbers::group_thousands(i64::from(row.num))
                )
            };
            put_right(
                rc,
                f.strong.as_ref(),
                &status,
                status_x,
                y,
                ROW,
                if full { kit::GOOD } else { tone::warm(1.0) },
            );
            let b = (
                r.0 + r.2 - 10.0 - add_w,
                y + (ROW - 28.0) * 0.5,
                add_w,
                28.0,
            );
            let on = row.carries && !full;
            let lit = on && h.add(b, Hit::Ask(SaleAsk::Add(row.wcid)));
            kit::button(
                rc,
                &theme,
                b,
                "Add",
                f.strong.as_ref(),
                ButtonLook::Plain,
                lit,
                on,
            );
            y += ROW + 4.0;
        }
        // What is on the rows.
        if !s.staged.is_empty() {
            y += 6.0;
            kit::group_heading(
                rc,
                &theme,
                f.label.as_ref(),
                PAD,
                y,
                18.0,
                WIDTH - PAD,
                "ON THE ROWS",
                None,
                tone::soft(1.0),
            );
            y += 24.0;
            let mut x = PAD;
            for (id, name) in &s.staged {
                let Some(sm) = &f.small else {
                    break;
                };
                let w = text::measure(sm, name, 0.0) + 30.0;
                if x + w > WIDTH - PAD {
                    x = PAD;
                    y += 28.0;
                }
                let r = (x, y, w, 24.0);
                let lit = h.add(r, Hit::Ask(SaleAsk::TakeOff(*id)));
                rc.fill_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(12.0),
                    if lit { gold(0.18) } else { bronze(0.14) },
                );
                put_mid(rc, Some(sm), name, r.0 + 10.0, r.1, r.3, p.text);
                put_mid(
                    rc,
                    Some(sm),
                    "×",
                    r.0 + r.2 - 16.0,
                    r.1,
                    r.3,
                    if lit { kit::HARM } else { tone::soft(1.0) },
                );
                x += w + 6.0;
            }
            y += 30.0;
        }
        if let Some(sm) = &f.small {
            put(rc, Some(sm), &s.hint, PAD, y + 4.0, tone::muted(1.0));
        }
        // The foot.
        let fy = HEIGHT - FOOT;
        rc.fill_rounded_rect(
            1.0,
            fy,
            WIDTH - 2.0,
            FOOT - 1.0,
            CornerRadii::new(0.0, 0.0, 5.0, 5.0),
            hex(0, 0.2),
        );
        rc.fill(0.0, fy, WIDTH, 1.0, bronze(0.35));
        let bw = kit::button_width(s.action, f.strong.as_ref()) + 12.0;
        let b = (WIDTH - PAD - bw, fy + (FOOT - 40.0) * 0.5, bw, 40.0);
        let lit = s.can && h.add(b, Hit::Ask(SaleAsk::Send));
        kit::button(
            rc,
            &theme,
            b,
            s.action,
            f.strong.as_ref(),
            ButtonLook::Primary,
            lit,
            s.can,
        );
        if let Some(sm) = &f.body {
            let lines = kit::wrap(sm, &s.foot, b.0 - PAD - 12.0);
            let lh = sm.line_height + 3.0;
            let top = fy + (FOOT - lines.len() as f32 * lh) * 0.5;
            kit::lines(rc, sm, &lines, PAD, top, lh, tone::soft(1.0));
        }
        self.hits = h;
    }
}

/// A dashed edge round `r`.
fn dashed(rc: &mut RenderContext<'_>, r: Rect, color: ac_ui::draw::Color) {
    let dash: f32 = 6.0;
    let gap = 4.0;
    let mut x = r.0 + 3.0;
    while x < r.0 + r.2 - 3.0 {
        let w = dash.min(r.0 + r.2 - 3.0 - x);
        rc.fill(x, r.1, w, 1.0, color);
        rc.fill(x, r.1 + r.3 - 1.0, w, 1.0, color);
        x += dash + gap;
    }
    let mut y = r.1 + 3.0;
    while y < r.1 + r.3 - 3.0 {
        let h = dash.min(r.1 + r.3 - 3.0 - y);
        rc.fill(r.0, y, 1.0, h, color);
        rc.fill(r.0 + r.2 - 1.0, y, 1.0, h, color);
        y += dash + gap;
    }
}
