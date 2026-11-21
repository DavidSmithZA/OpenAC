//! The Barber window, in place of retail's: the figure on the left with
//! turn buttons and Hold for before (OPENAC: the old look while held);
//! part tabs (Hair, Eyes, Nose, Mouth, Skin, or the heritage's own three),
//! each with a style stepper and numbered picks, colour swatches for hair
//! and eyes, a shade slider for hair and skin; the heritage's one option;
//! a line of what changed; Cancel and Accept.

use std::sync::Arc;

use ac_core::chargen::{ChargenGenderOptions, ChargenPalSetSource, ChargenSwatchRgb};
use ac_runtime::gameplay::{BarberCommand, BarberView, GameplayCommand};
use ac_runtime::session::SessionCommand;
use ac_ui::bundled_font::Weight;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::barber::{CAPTION_TABLE, TOGGLE_TABLE};
use ac_ui::panels::barber_rules::{color_slot, shade_slot, style_on_pick, style_on_step};
use ac_ui::panels::items::emit;
use ac_ui::pregame::chargen::appearance::{Part, PreviewRequest, SwatchSourcesDyn};
use ac_ui::shapes::CornerRadii;
use ac_ui::strings::compute_hash;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::{BarberFacts, BarberScreen, Before, barber_screen, shade_indices, shade_source};
use crate::scene::Scene;
use crate::ui::barber_window::BARBER_WINDOW;
use crate::ui::modern::hud;
use crate::ui::modern::hud::lines::{circle, stroke};
use crate::ui::modern::kit::{self, Hits, MUTED, Rect, SOFT, bronze, gold, put, put_mid};
use crate::ui::window::WindowUi;

/// The window's name.
pub const BARBER: &str = "modern-barber";

/// The window's size.
pub const WIDTH: f32 = 760.0;
pub const HEIGHT: f32 = 560.0;
const HEADER: f32 = 52.0;
const FOOT: f32 = 68.0;
/// The figure's box.
const PREVIEW: Rect = (20.0, 72.0, 320.0, HEIGHT - 72.0 - 84.0);
/// The right column.
const COL_X: f32 = 356.0;
const COL_W: f32 = WIDTH - 20.0 - COL_X;
const TAB_H: f32 = 54.0;
const PICK_H: f32 = 34.0;
const SWATCH: f32 = 34.0;

/// Scissors, the window's glyph.
const GLYPH: &[&[(f32, f32)]] = &[&[(8.5, 7.5), (20.0, 18.0)], &[(8.5, 16.5), (20.0, 6.0)]];
const LEFT: &[&[(f32, f32)]] = &[&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]];
const RIGHT: &[&[(f32, f32)]] = &[&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]];
/// A turn to the left: an open circle with its arrowhead.
const TURN_LEFT: &[&[(f32, f32)]] = &[
    &[
        (3.0, 12.0),
        (4.2, 16.5),
        (7.5, 19.8),
        (12.0, 21.0),
        (16.5, 19.8),
        (19.8, 16.5),
        (21.0, 12.0),
        (19.8, 7.5),
        (16.5, 4.2),
        (12.0, 3.0),
        (6.0, 5.3),
    ],
    &[(3.0, 4.0), (3.0, 9.0), (8.0, 9.0)],
];
const TURN_RIGHT: &[&[(f32, f32)]] = &[
    &[
        (21.0, 12.0),
        (19.8, 16.5),
        (16.5, 19.8),
        (12.0, 21.0),
        (7.5, 19.8),
        (4.2, 16.5),
        (3.0, 12.0),
        (4.2, 7.5),
        (7.5, 4.2),
        (12.0, 3.0),
        (18.0, 5.3),
    ],
    &[(21.0, 4.0), (21.0, 9.0), (16.0, 9.0)],
];

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct BarberFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// Tabs, picks, buttons.
    pub strong: Option<Arc<UiFont>>,
    /// Notes and the changed line.
    pub body: Option<Arc<UiFont>>,
    /// The tabs' second lines, the header's note.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals and the OPENAC tag.
    pub label: Option<Arc<UiFont>>,
}

/// What the window asks of the barber.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BarberAsk {
    /// Pick a part.
    Part(Part),
    /// Step the picked part's style.
    Step(i32),
    /// Pick a style (from 0).
    Style(usize),
    /// Pick a colour.
    Swatch(usize),
    /// Set the picked part's shade (0 to 1).
    Shade(f32),
    /// Flip the heritage's option.
    Toggle,
    /// Turn the figure (the same way again stops it).
    Turn(PreviewRequest),
    /// Keep the new look.
    Accept,
    /// Leave the look as it was.
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Ask(BarberAsk),
    Hold,
    Track,
}

/// The Barber window's frame.
#[derive(Debug)]
pub struct BarberFrame {
    theme: Horizon,
    fonts: BarberFonts,
    /// What it shows.
    pub screen: BarberScreen,
    /// "Hold for before" is held down.
    pub holding: bool,
    /// The way the figure turns, if it turns.
    pub turning: Option<PreviewRequest>,
    /// What it asked for since the window last looked.
    pub asks: Vec<BarberAsk>,
    hits: Hits<Hit>,
    /// The shade slider is held.
    sliding: bool,
}

impl BarberFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: BarberFonts) -> Self {
        Self {
            theme,
            fonts,
            screen: BarberScreen::default(),
            holding: false,
            turning: None,
            asks: Vec::new(),
            hits: Hits::default(),
            sliding: false,
        }
    }

    /// Where the figure is drawn (the viewport over the frame).
    #[must_use]
    pub fn figure_rect() -> Rect {
        (
            PREVIEW.0 + 1.0,
            PREVIEW.1 + 1.0,
            PREVIEW.2 - 2.0,
            PREVIEW.3 - 58.0,
        )
    }

    /// The turn buttons and Hold for before.
    #[must_use]
    pub fn preview_buttons(&self) -> (Rect, Rect, Rect) {
        let y = PREVIEW.1 + PREVIEW.3 - 12.0 - 34.0;
        let words = self
            .fonts
            .strong
            .as_ref()
            .map_or(100.0, |f| text::measure(f, "Hold for before", 0.0));
        let tag = self
            .fonts
            .label
            .as_ref()
            .map_or(50.0, |f| text::measure(f, "OPENAC", 0.6) + 12.0);
        let hold_w = 24.0 + words + 6.0 + tag;
        let total = 34.0 + 8.0 + hold_w + 8.0 + 34.0;
        let x = PREVIEW.0 + (PREVIEW.2 - total) * 0.5;
        (
            (x, y, 34.0, 34.0),
            (x + 42.0, y, hold_w, 34.0),
            (x + 42.0 + hold_w + 8.0, y, 34.0, 34.0),
        )
    }

    /// Cancel's and Accept's places.
    #[must_use]
    pub fn foot_buttons(&self) -> (Rect, Rect) {
        let y = HEIGHT - FOOT + (FOOT - 38.0) * 0.5;
        let accept_w = kit::button_width("Accept", self.fonts.strong.as_ref()) + 12.0;
        let cancel_w = kit::button_width("Cancel", self.fonts.strong.as_ref()) + 4.0;
        let accept = (WIDTH - 20.0 - accept_w, y, accept_w, 38.0);
        ((accept.0 - 10.0 - cancel_w, y, cancel_w, 38.0), accept)
    }

    /// The shade track, when the part has one; where the column's pieces
    /// stand is worked out as they are drawn, so the track is kept here.
    fn shade_value(track: Rect, x: f32) -> f32 {
        ((x - track.0) / track.2).clamp(0.0, 1.0)
    }

    fn draw_preview(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let r = PREVIEW;
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0, 0.35));
        rc.fill_ellipse(
            r.0 + r.2 * 0.5 - 150.0,
            r.1 + r.3 * 0.4 - 150.0,
            300.0,
            300.0,
            gold(0.06),
        );
        rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.3), 1.0);
        if self.holding {
            put(
                rc,
                self.fonts.small.as_ref(),
                "Your look as it was",
                r.0 + 12.0,
                r.1 + 10.0,
                gold(1.0),
            );
        }
        let (left, hold, right) = self.preview_buttons();
        for (b, glyph, way) in [
            (left, TURN_LEFT, PreviewRequest::RotateCounterClockwise),
            (right, TURN_RIGHT, PreviewRequest::RotateClockwise),
        ] {
            let lit = h.add(b, Hit::Ask(BarberAsk::Turn(way)));
            let on = self.turning == Some(way);
            rc.fill_ellipse(
                b.0,
                b.1,
                b.2,
                b.3,
                if on { gold(0.2) } else { hex(0, 0.45) },
            );
            rc.stroke_ellipse(
                b.0,
                b.1,
                b.2,
                b.3,
                if on || lit { gold(1.0) } else { bronze(0.6) },
                1.0,
            );
            stroke(rc, b.0 + 9.5, b.1 + 9.5, 15.0, glyph, kit::WARM, 2.0);
        }
        let lit = h.add(hold, Hit::Hold);
        let pill = CornerRadii::all(17.0);
        rc.fill_rounded_rect(
            hold.0,
            hold.1,
            hold.2,
            hold.3,
            pill,
            if self.holding {
                gold(0.2)
            } else {
                hex(0, 0.45)
            },
        );
        rc.stroke_rounded_rect(
            hold.0,
            hold.1,
            hold.2,
            hold.3,
            pill,
            if self.holding || lit {
                gold(1.0)
            } else {
                bronze(0.6)
            },
            1.0,
        );
        put_mid(
            rc,
            self.fonts.strong.as_ref(),
            "Hold for before",
            hold.0 + 12.0,
            hold.1,
            hold.3,
            self.theme.palette.text,
        );
        let words = self
            .fonts
            .strong
            .as_ref()
            .map_or(100.0, |f| text::measure(f, "Hold for before", 0.0));
        kit::openac_tag(
            rc,
            self.fonts.label.as_ref(),
            hold.0 + 12.0 + words + 6.0,
            hold.1 + hold.3 * 0.5,
        );
    }

    #[allow(clippy::too_many_lines)]
    fn draw_column(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let p = self.theme.palette;
        let f = self.fonts.clone();
        let s = self.screen.clone();
        let mut y = PREVIEW.1;
        // The part tabs.
        let n = s.parts.len().max(1) as f32;
        let tw = (COL_W - (n - 1.0) * 4.0) / n;
        for (i, tab) in s.parts.iter().enumerate() {
            let r = (COL_X + i as f32 * (tw + 4.0), y, tw, TAB_H);
            let lit = h.add(r, Hit::Ask(BarberAsk::Part(tab.part)));
            let round = CornerRadii::all(4.0);
            let (fill, edge) = if tab.on {
                (gold(0.14), gold(1.0))
            } else if lit {
                (hex(0xFF_FFFF, 0.05), bronze(0.5))
            } else {
                (hex(0, 0.25), bronze(0.3))
            };
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, fill);
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, edge, 1.0);
            centred(
                rc,
                f.strong.as_ref(),
                &tab.label,
                r.0,
                r.2,
                r.1 + 11.0,
                p.text,
            );
            centred(rc, f.small.as_ref(), &tab.sub, r.0, r.2, r.1 + 31.0, MUTED);
        }
        y += TAB_H + 14.0;
        // The style.
        if let Some((now, count)) = s.style {
            kit::spaced(rc, f.label.as_ref(), "STYLE", COL_X, y + 8.0);
            let next = (COL_X + COL_W - 26.0, y, 26.0, 26.0);
            let words = if now == 0 {
                format!("None of {count}")
            } else {
                format!("{now} of {count}")
            };
            let ww = 72.0;
            let prev = (next.0 - 8.0 - ww - 8.0 - 26.0, y, 26.0, 26.0);
            for (b, glyph, step) in [(prev, LEFT, -1), (next, RIGHT, 1)] {
                let lit = h.add(b, Hit::Ask(BarberAsk::Step(step)));
                kit::button(
                    rc,
                    &self.theme,
                    b,
                    "",
                    None,
                    kit::ButtonLook::Plain,
                    lit,
                    true,
                );
                stroke(rc, b.0 + 6.0, b.1 + 6.0, 14.0, glyph, kit::WARM, 2.0);
            }
            centred(
                rc,
                f.strong.as_ref(),
                &words,
                prev.0 + 34.0,
                ww,
                y + 6.0,
                p.text,
            );
            y += 26.0 + 8.0;
            let below = below_style(&s);
            let room = PREVIEW.1 + PREVIEW.3 - y - below;
            let grid = PickGrid::fit(count, now, room);
            for i in grid.first..grid.last {
                let k = i - grid.first;
                let r = (
                    COL_X + (k % grid.cols) as f32 * (grid.width + 4.0),
                    y + (k / grid.cols) as f32 * (grid.height + 4.0),
                    grid.width,
                    grid.height,
                );
                let lit = h.add(r, Hit::Ask(BarberAsk::Style(i)));
                kit::pick_row(rc, r, i + 1 == now, lit);
                let ty = f
                    .strong
                    .as_ref()
                    .map_or(r.1, |sf| r.1 + ((r.3 - sf.line_height) * 0.5).round());
                centred(
                    rc,
                    f.strong.as_ref(),
                    &(i + 1).to_string(),
                    r.0,
                    r.2,
                    ty,
                    p.text,
                );
            }
            y += grid.rows as f32 * (grid.height + 4.0) - 4.0 + 14.0;
        }
        // The colours.
        if !s.swatches.is_empty() {
            kit::spaced(rc, f.label.as_ref(), "COLOUR", COL_X, y);
            y += 20.0;
            for (i, w) in s.swatches.iter().enumerate() {
                let r = (COL_X + i as f32 * (SWATCH + 6.0), y, SWATCH, SWATCH);
                let lit = h.add(r, Hit::Ask(BarberAsk::Swatch(i)));
                rc.fill_ellipse(r.0, r.1, r.2, r.3, w.color);
                rc.stroke_ellipse(
                    r.0 + 2.0,
                    r.1 + 2.0,
                    r.2 - 4.0,
                    r.3 - 4.0,
                    hex(0, 0.35),
                    2.0,
                );
                let ring = if w.on {
                    gold(1.0)
                } else if lit {
                    bronze(0.8)
                } else {
                    hex(0, 0.5)
                };
                rc.stroke_ellipse(r.0, r.1, r.2, r.3, ring, 2.0);
            }
            y += SWATCH + 14.0;
        }
        // The shade.
        if let Some((share, stops)) = &s.shade {
            kit::spaced(rc, f.label.as_ref(), "SHADE", COL_X, y);
            let pct = format!("{}%", (share * 100.0).round());
            if let Some(sf) = &f.strong {
                let w = text::measure(sf, &pct, 0.0);
                text::draw(
                    rc,
                    sf,
                    &pct,
                    COL_X + COL_W - w,
                    y - 2.0,
                    TextStyle::plain(p.text),
                );
            }
            y += 20.0;
            let track = (COL_X + 7.0, y, COL_W - 14.0, 8.0);
            let hit = (COL_X, y - 8.0, COL_W, 24.0);
            let lit = h.add(hit, Hit::Track);
            draw_shade_track(rc, track, stops);
            let kx = track.0 + track.2 * share;
            let size = if lit || self.sliding { 18.0 } else { 16.0 };
            rc.fill_ellipse(kx - size * 0.5, y + 4.0 - size * 0.5, size, size, gold(1.0));
            rc.stroke_ellipse(
                kx - size * 0.5,
                y + 4.0 - size * 0.5,
                size,
                size,
                hex(0x1A_140D, 0.8),
                1.5,
            );
            y += 8.0 + 18.0;
        }
        if let Some(note) = s.note {
            put(rc, f.body.as_ref(), note, COL_X, y - 6.0, MUTED);
            y += 22.0;
        }
        // The heritage's option.
        if let Some((words, on)) = &s.toggle {
            let r = (COL_X, y, COL_W, 36.0);
            let lit = h.add(r, Hit::Ask(BarberAsk::Toggle));
            kit::pick_row(rc, r, false, lit);
            put_mid(rc, f.strong.as_ref(), words, r.0 + 12.0, r.1, r.3, p.text);
            kit::switch(
                rc,
                (r.0 + r.2 - 12.0 - 36.0, r.1 + 8.0, 36.0, 20.0),
                *on,
                lit,
            );
        }
    }
}

/// How tall the sections under the styles are.
fn below_style(s: &BarberScreen) -> f32 {
    let mut h = 14.0;
    if !s.swatches.is_empty() {
        h += 20.0 + SWATCH + 14.0;
    }
    if s.shade.is_some() {
        h += 20.0 + 26.0;
    }
    if s.note.is_some() {
        h += 22.0;
    }
    if s.toggle.is_some() {
        h += 36.0;
    }
    h
}

/// How the numbered style picks are laid out in the room they have: at
/// least eight to a row, up to twelve, and paged round the picked one
/// when even that is not enough (the stepper reaches the rest).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickGrid {
    /// Picks to a row.
    pub cols: usize,
    /// Rows shown.
    pub rows: usize,
    /// The first pick shown (from 0) and one past the last.
    pub first: usize,
    pub last: usize,
    /// A pick's size.
    pub width: f32,
    pub height: f32,
}

impl PickGrid {
    /// The grid for `count` picks, `now` picked (from 1; 0: none), in
    /// `room` units of height.
    #[must_use]
    pub fn fit(count: usize, now: usize, room: f32) -> Self {
        let height = if count > 16 { 28.0 } else { PICK_H };
        let rows_fit = (((room + 4.0) / (height + 4.0)).floor() as usize).max(1);
        let cols = count.div_ceil(rows_fit).clamp(8, 12);
        let page = (rows_fit * cols).max(1);
        // A pick past the last style (the view not caught up yet) pages
        // as the last.
        let now = now.clamp(1, count.max(1));
        let first = (now - 1) / page * page;
        let last = (first + page).min(count).max(first);
        let rows = (last - first).div_ceil(cols).max(1);
        Self {
            cols,
            rows,
            first,
            last,
            width: (COL_W - (cols - 1) as f32 * 4.0) / cols as f32,
            height,
        }
    }
}

/// `words` centred across a band from `x`, `w` wide, top at `y`.
fn centred(
    rc: &mut RenderContext<'_>,
    font: Option<&Arc<UiFont>>,
    words: &str,
    x: f32,
    w: f32,
    y: f32,
    c: Color,
) {
    if let Some(f) = font {
        let tw = text::measure(f, words, 0.0);
        text::draw(
            rc,
            f,
            words,
            (x + (w - tw) * 0.5).round(),
            y,
            TextStyle::plain(c),
        );
    }
}

/// The shade's track: the colours along it, or dark to light when the
/// dats gave none.
fn draw_shade_track(rc: &mut RenderContext<'_>, r: Rect, stops: &[Color]) {
    let round = CornerRadii::all(4.0);
    match stops {
        [] | [_] => {
            let one = stops.first().copied().unwrap_or(hex(0x8C_6A44, 1.0));
            rc.fill_horizontal_gradient(
                r.0,
                r.1,
                r.2,
                r.3,
                round,
                Color {
                    r: one.r * 0.3,
                    g: one.g * 0.3,
                    b: one.b * 0.3,
                    a: 1.0,
                },
                one,
            );
        }
        _ => {
            let seg = r.2 / (stops.len() - 1) as f32;
            for (i, pair) in stops.windows(2).enumerate() {
                let x = r.0 + i as f32 * seg;
                let radii = CornerRadii::new(
                    if i == 0 { 4.0 } else { 0.0 },
                    if i + 2 == stops.len() { 4.0 } else { 0.0 },
                    if i + 2 == stops.len() { 4.0 } else { 0.0 },
                    if i == 0 { 4.0 } else { 0.0 },
                );
                rc.fill_horizontal_gradient(x, r.1, seg + 0.5, r.3, radii, pair[0], pair[1]);
            }
        }
    }
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0, 0.4), 1.0);
}

impl Widget for BarberFrame {
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
        let track = || (COL_X + 7.0, 0.0, COL_W - 14.0, 0.0);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hits.hover = Some((x, y));
                if self.sliding {
                    self.asks
                        .push(BarberAsk::Shade(Self::shade_value(track(), x)));
                    return true;
                }
                false
            }
            kind::HOVER_LEAVE => {
                self.hits.hover = None;
                false
            }
            kind::MOUSE_DOWN => {
                match self.hits.at(x, y) {
                    Some(Hit::Hold) => {
                        self.holding = true;
                        ui.set_capture(me);
                    }
                    Some(Hit::Track) => {
                        self.sliding = true;
                        ui.set_capture(me);
                        self.asks
                            .push(BarberAsk::Shade(Self::shade_value(track(), x)));
                    }
                    _ => {}
                }
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.holding = false;
                if self.sliding && e.kind == kind::MOUSE_UP {
                    self.asks
                        .push(BarberAsk::Shade(Self::shade_value(track(), x)));
                }
                self.sliding = false;
                true
            }
            kind::CLICK => {
                if let Some(Hit::Ask(a)) = self.hits.at(x, y) {
                    if let BarberAsk::Turn(way) = a {
                        self.turning = if self.turning == Some(way) {
                            None
                        } else {
                            Some(way)
                        };
                    }
                    self.asks.push(a);
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let f = self.fonts.clone();
        let mut h = Hits::new(self.hits.hover);
        kit::window_back(rc, &self.theme, WIDTH, HEIGHT, HEADER, true);
        let end = kit::title(rc, &self.theme, f.title.as_ref(), GLYPH, "Barber", HEADER);
        let top = (HEADER - 18.0) * 0.5;
        for at in [(6.0, 6.0), (6.0, 18.0)] {
            circle(rc, 20.0, top, 18.0, at, 3.0, gold(1.0), 2.0);
        }
        put_mid(
            rc,
            f.small.as_ref(),
            &self.screen.subtitle,
            end + 12.0,
            0.0,
            HEADER,
            MUTED,
        );
        let close = kit::close_rect(WIDTH, HEADER);
        let lit = h.add(close, Hit::Ask(BarberAsk::Cancel));
        kit::close(rc, &self.theme, close, lit);
        self.draw_preview(rc, &mut h);
        self.draw_column(rc, &mut h);
        // The foot.
        let fy = HEIGHT - FOOT;
        rc.fill_vertical_gradient(
            1.0,
            fy,
            WIDTH - 2.0,
            FOOT - 1.0,
            CornerRadii::new(0.0, 0.0, 5.0, 5.0),
            hex(0, 0.2),
            hex(0, 0.2),
        );
        rc.fill(0.0, fy, WIDTH, 1.0, bronze(0.35));
        let (cancel, accept) = self.foot_buttons();
        put_mid(
            rc,
            f.body.as_ref(),
            &self.screen.changed,
            20.0,
            fy,
            FOOT,
            SOFT,
        );
        let lit = h.add(cancel, Hit::Ask(BarberAsk::Cancel));
        kit::button(
            rc,
            &self.theme,
            cancel,
            "Cancel",
            f.strong.as_ref(),
            kit::ButtonLook::Plain,
            lit,
            true,
        );
        let lit = h.add(accept, Hit::Ask(BarberAsk::Accept));
        kit::button(
            rc,
            &self.theme,
            accept,
            "Accept",
            f.strong.as_ref(),
            kit::ButtonLook::Primary,
            lit,
            true,
        );
        self.hits = h;
    }
}

/// The Barber window's elements and what it remembers.
#[derive(Debug)]
pub struct BarberUi {
    /// The window.
    pub frame: ElementId,
    /// The figure.
    pub figure: ElementId,
    /// The part picked.
    part: Part,
    /// The barber as it opened.
    pub before: Before,
    /// The opening last set up.
    openings: Option<u64>,
    /// The barber's revision and the part last shown.
    shown: Option<(u64, Part, u64)>,
    /// The last shade sent, to send no repeats while sliding.
    last_shade: Option<f32>,
    /// Turns of the figure asked for since the viewports last looked.
    pub turns: Vec<PreviewRequest>,
}

fn barber_command(ui: &mut UiRoot, command: BarberCommand) {
    emit(
        ui,
        PanelOutput::Session(SessionCommand::Gameplay(GameplayCommand::Barber(command))),
    );
}

impl WindowUi {
    /// The Barber window, hidden, in place of retail's.
    pub(in crate::ui) fn mount_modern_barber(&mut self) {
        let theme = Horizon::BRONZE;
        let fonts = BarberFonts {
            title: self.modern_font(Weight::SemiBold, 15.0),
            strong: self.modern_font(Weight::SemiBold, 12.0),
            body: self.modern_font(Weight::Regular, 11.5),
            small: self.modern_font(Weight::Regular, 10.5),
            label: self.modern_font(Weight::SemiBold, 9.5),
        };
        let frame = match hud::mount_piece(
            &mut self.host.root,
            BARBER,
            (WIDTH, HEIGHT),
            BarberFrame::new(theme, fonts),
        ) {
            Ok(id) => id,
            Err(e) => {
                self.notices.push(format!("ui: modern barber: {e}"));
                return;
            }
        };
        let ui = &mut self.host.root;
        if let Some(e) = ui.element_mut(frame) {
            e.draggable = true;
        }
        let r = BarberFrame::figure_rect();
        let figure = ui.add(
            frame,
            Element::at(r.0, r.1, r.2, r.3).with_anchors(AnchorEdges::NONE),
            ac_ui::widgets::Viewport::default(),
        );
        ui.hide_window(BARBER);
        let Some(modern) = self.modern.as_mut() else {
            return;
        };
        modern.barber = Some(BarberUi {
            frame,
            figure,
            part: Part::Hair,
            before: Before::default(),
            openings: None,
            shown: None,
            last_shade: None,
            turns: Vec::new(),
        });
        modern.replace(BARBER_WINDOW);
    }

    /// What the barber window reads besides the barber: the gender's
    /// choices, names and words, and the picked part's colours.
    fn barber_facts<'a>(
        &self,
        view: &BarberView,
        part: Part,
        gender: Option<&'a ChargenGenderOptions>,
        heritage_name: String,
    ) -> BarberFacts<'a> {
        let strings = self.content.strings();
        let text = |table: u32, key: &str| strings.resolve(table, compute_hash(key), 0);
        let captions = if super::model::three_parts(view.heritage) {
            let keys = match ac_core::chargen::ChargenHeritageGroup(view.heritage) {
                ac_core::chargen::ChargenHeritageGroup::GEARKNIGHT => [
                    "ID_CharGen_GearText_HairButton",
                    "ID_CharGen_GearText_EyesButton",
                    "ID_CharGen_GearText_SkinButton",
                ],
                _ => [
                    "ID_CharGen_OlthoiText_HairButton",
                    "ID_CharGen_OlthoiText_EyesButton",
                    "ID_CharGen_OlthoiText_SkinButton",
                ],
            };
            let words = keys.map(|k| text(CAPTION_TABLE, k).unwrap_or_default());
            words.iter().all(|w| !w.is_empty()).then_some(words)
        } else {
            None
        };
        let toggle_words = view.toggle.and_then(|t| text(TOGGLE_TABLE, t.string_key()));
        let catalog = self.barber.catalog();
        let colors = match (gender, catalog) {
            (Some(g), Some(c)) => ac_ui::pregame::chargen::appearance::swatch_colors(
                part,
                g,
                &view.appearance,
                c as &dyn SwatchSourcesDyn,
            ),
            _ => [None; 9],
        };
        let shade_stops: Vec<ChargenSwatchRgb> = match (gender, catalog) {
            (Some(g), Some(c)) => shade_source(part, view, g)
                .and_then(|(set, sample)| {
                    let palettes = c.pal_set(set)?.palette_ids.clone();
                    Some(
                        shade_indices(palettes.len(), 8)
                            .into_iter()
                            .filter_map(|i| {
                                (c as &dyn SwatchSourcesDyn).direct(palettes[i], sample)
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        BarberFacts {
            gender,
            heritage_name,
            captions,
            toggle_words,
            colors,
            shade_stops,
        }
    }

    /// Each frame: the window shows while the barber is open and follows
    /// it; its picks change the look by commands.
    #[allow(clippy::too_many_lines)]
    pub(in crate::ui) fn tick_modern_barber(&mut self, scene: &Scene) {
        let Some(b) = self.modern.as_ref().and_then(|m| m.barber.as_ref()) else {
            return;
        };
        let (frame, mut part, openings, shown, mut last_shade) =
            (b.frame, b.part, b.openings, b.shown, b.last_shade);
        let view = self.facts.barber.clone().unwrap_or_default();
        let ui = &mut self.host.root;
        let visible = ui.is_window_visible(BARBER);
        if view.open != visible {
            if view.open {
                ui.show_window(BARBER);
            } else {
                ui.hide_window(BARBER);
            }
        }
        if !view.open {
            if let Some(f) = ui.widget_mut::<BarberFrame>(frame) {
                f.asks.clear();
                f.holding = false;
                f.turning = None;
            }
            return;
        }
        let mut before = self
            .modern
            .as_ref()
            .and_then(|m| m.barber.as_ref())
            .map(|b| b.before)
            .unwrap_or_default();
        if openings != Some(view.openings) {
            before = Before::of(&view);
            part = Part::Hair;
            last_shade = None;
            if let Some(f) = self.host.root.widget_mut::<BarberFrame>(frame) {
                f.turning = None;
            }
        }
        let options = scene.pregame().options.clone();
        let heritage = options
            .as_deref()
            .and_then(|o| o.try_get_heritage(view.heritage));
        let gender = heritage.and_then(|h| h.genders_by_key.get(&i32::try_from(view.gender).ok()?));
        let heritage_name = heritage.map(|h| h.name.clone()).unwrap_or_default();
        let asks = self
            .host
            .root
            .widget_mut::<BarberFrame>(frame)
            .map(|f| std::mem::take(&mut f.asks))
            .unwrap_or_default();
        let mut sent = 0u64;
        let mut turns = Vec::new();
        for ask in asks {
            let ui = &mut self.host.root;
            match ask {
                BarberAsk::Part(p) => {
                    if let Some(g) = gender
                        && let Some((slot, n)) = style_on_pick(p, &view, g)
                    {
                        barber_command(ui, BarberCommand::SetAppearance(slot, n));
                    }
                    part = p;
                    // Another part's slider starts afresh.
                    last_shade = None;
                }
                BarberAsk::Step(step) => {
                    if let Some((slot, n)) =
                        gender.and_then(|g| style_on_step(part, &view, g, step))
                    {
                        barber_command(ui, BarberCommand::SetAppearance(slot, n));
                    }
                }
                BarberAsk::Style(i) => {
                    if let Some(slot) = ac_ui::panels::barber_rules::style_slot(part) {
                        barber_command(
                            ui,
                            BarberCommand::SetAppearance(slot, u32::try_from(i).unwrap_or(0)),
                        );
                    }
                }
                BarberAsk::Swatch(i) => {
                    if let Some(slot) = color_slot(part) {
                        barber_command(
                            ui,
                            BarberCommand::SetAppearance(slot, u32::try_from(i).unwrap_or(0)),
                        );
                    }
                }
                BarberAsk::Shade(v) => {
                    if last_shade.is_none_or(|l| (l - v).abs() > 0.002)
                        && let Some(slot) = shade_slot(part)
                    {
                        last_shade = Some(v);
                        barber_command(ui, BarberCommand::SetShade(slot, f64::from(v)));
                    }
                }
                BarberAsk::Toggle => {
                    if view.toggle.is_some() {
                        barber_command(ui, BarberCommand::SetToggle(!view.toggle_on));
                    }
                }
                BarberAsk::Turn(way) => turns.push(way),
                BarberAsk::Accept => barber_command(ui, BarberCommand::Finish),
                BarberAsk::Cancel => barber_command(ui, BarberCommand::Cancel),
            }
            sent += 1;
        }
        let key = (view.revision, part, view.openings);
        if shown != Some(key) || sent > 0 {
            let facts = self.barber_facts(&view, part, gender, heritage_name);
            let screen = barber_screen(&view, part, &before, &facts);
            if let Some(f) = self.host.root.widget_mut::<BarberFrame>(frame) {
                f.screen = screen;
            }
        }
        if let Some(b) = self.modern.as_mut().and_then(|m| m.barber.as_mut()) {
            b.turns.extend(turns);
            b.part = part;
            b.before = before;
            b.openings = Some(view.openings);
            b.shown = Some(key);
            b.last_shade = last_shade;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn many_styles_take_more_to_a_row_and_page_when_they_must() {
        let g = PickGrid::fit(12, 4, 200.0);
        assert_eq!((g.cols, g.rows, g.first, g.last), (8, 2, 0, 12));
        let g = PickGrid::fit(50, 6, 170.0);
        assert_eq!(g.cols, 10);
        assert_eq!((g.first, g.last), (0, 50));
        let g = PickGrid::fit(50, 40, 60.0);
        assert_eq!(g.cols, 12);
        assert_eq!((g.first, g.last), (24, 48), "the page holding 40");
    }
}
