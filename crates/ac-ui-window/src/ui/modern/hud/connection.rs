//! The connection, in the status line above the radar: quiet while the
//! link is good (the round trip and a green dot), amber after 5 s with no
//! word from the server ("Slow · 7 s"), red with a flashing dot after 20 s
//! ("No answer · 24 s"), and "Connection lost" after 40 s or when the
//! link drops (retail's link indicator steps). Clicking it opens a small
//! card: the state, the round trip, packets lost and when the server was
//! last heard (retail's link-status panel).

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_runtime::session::LinkStatus;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

/// How often the modern interface asks the server for a round trip,
/// seconds (the retail link panel's interval, kept whether or not the card
/// is open, so the status line has a number to show).
pub const PING_SECONDS: f64 = 120.0;

/// How often a bad link's dot flashes, seconds (as retail's light).
pub const FLASH_SECONDS: f64 = 0.75;

/// The card's width and height.
pub const CARD_WIDTH: f32 = 320.0;
pub const CARD_HEIGHT: f32 = 214.0;

/// How the link stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// A packet in the last 5 seconds.
    Good,
    /// None for 5 seconds.
    Slow,
    /// None for 20 seconds.
    Bad,
    /// None for 40 seconds, or no connection.
    Lost,
}

impl LinkState {
    /// The state of `link`.
    #[must_use]
    pub fn of(link: &LinkStatus) -> Self {
        let quiet = link.seconds_since_last_packet;
        if !link.connected || quiet >= 40.0 {
            Self::Lost
        } else if quiet >= 20.0 {
            Self::Bad
        } else if quiet >= 5.0 {
            Self::Slow
        } else {
            Self::Good
        }
    }

    /// Its name on the card.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Good => "Good",
            Self::Slow => "Slow",
            Self::Bad => "No answer",
            Self::Lost => "Connection lost",
        }
    }

    fn dot(self) -> Color {
        match self {
            Self::Good => hex(0x6B_D47A, 1.0),
            Self::Slow => hex(0xF0_B44C, 1.0),
            Self::Bad | Self::Lost => hex(0xFF_6B70, 1.0),
        }
    }

    /// The status line button's edge, fill and words' colour; none while
    /// good (the words sit plainly on the world).
    fn look(self) -> Option<(Color, Color, Color)> {
        match self {
            Self::Good => None,
            Self::Slow => Some((
                hex(0xF0_B44C, 0.8),
                hex(0x28_1C08, 0.7),
                hex(0xFF_D99A, 1.0),
            )),
            Self::Bad => Some((
                hex(0xFF_6B70, 0.85),
                hex(0x32_0C0E, 0.75),
                hex(0xFF_C2C4, 1.0),
            )),
            Self::Lost => Some((hex(0xFF_6B70, 1.0), hex(0x5A_1014, 0.85), Color::WHITE)),
        }
    }
}

/// A round trip in whole milliseconds; none before one was answered.
#[must_use]
pub fn round_trip_words(link: &LinkStatus) -> Option<String> {
    link.round_trip_seconds
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(|s| format!("{:.0} ms", s * 1000.0))
}

/// The status line's words for the link.
#[must_use]
pub fn status_words(link: &LinkStatus) -> String {
    let quiet = link.seconds_since_last_packet.max(0.0) as i64;
    match LinkState::of(link) {
        LinkState::Good => round_trip_words(link).unwrap_or_default(),
        LinkState::Slow => format!("Slow \u{b7} {quiet} s"),
        LinkState::Bad => format!("No answer \u{b7} {quiet} s"),
        LinkState::Lost => "Connection lost".to_owned(),
    }
}

/// The status line: the game's hour, then the connection button, right
/// aligned in a band `width` wide. Drawn by the radar piece, which owns
/// the band above its pill.
#[derive(Debug, Clone, Default)]
pub struct StatusLine {
    /// The game's hour ("Dawnsong").
    pub hour: Option<String>,
    /// The link.
    pub link: LinkStatus,
    /// A bad link's dot is lit (it flashes).
    pub dot_lit: bool,
    /// Where the connection button was last drawn (left, width).
    pub button: (f32, f32),
}

/// The status band's height.
pub const BAND: f32 = 16.0;
const BUTTON_HEIGHT: f32 = 18.0;

impl StatusLine {
    /// Draws the line right-aligned to `right`, in `font`.
    pub fn draw(&mut self, rc: &mut RenderContext<'_>, theme: &Horizon, font: &UiFont, right: f32) {
        let p = theme.palette;
        let state = LinkState::of(&self.link);
        let words = status_words(&self.link);
        let ww = if words.is_empty() {
            0.0
        } else {
            text::measure(font, &words, 0.0)
        };
        let gap = if ww > 0.0 { 6.0 } else { 0.0 };
        let pad = if state.look().is_some() { 8.0 } else { 2.0 };
        let width = pad + ww + gap + 7.0 + pad;
        let left = right - width;
        let top = (BAND - BUTTON_HEIGHT) * 0.5;
        let shadow = p.text_shadow;
        let mut ink = p.text;
        if let Some((edge, fill, words_ink)) = state.look() {
            let radius = CornerRadii::all(BUTTON_HEIGHT * 0.5);
            rc.fill_rounded_rect(left, top, width, BUTTON_HEIGHT, radius, fill);
            rc.stroke_rounded_rect(left, top, width, BUTTON_HEIGHT, radius, edge, 1.0);
            ink = words_ink;
        }
        let ty = ((BAND - font.line_height) * 0.5).round();
        if ww > 0.0 {
            text::draw(
                rc,
                font,
                &words,
                left + pad,
                ty,
                TextStyle::plain(ink).shadowed(shadow),
            );
        }
        let lit = state != LinkState::Bad || self.dot_lit;
        if lit {
            rc.fill_ellipse(
                left + pad + ww + gap,
                (BAND - 7.0) * 0.5,
                7.0,
                7.0,
                state.dot(),
            );
        }
        self.button = (left, width);
        if let Some(hour) = &self.hour {
            let bar = "|";
            let bw = text::measure(font, bar, 0.0);
            let bx = left - 8.0 - bw;
            text::draw(
                rc,
                font,
                bar,
                bx,
                ty,
                TextStyle::plain(Color { a: 0.5, ..p.text }).shadowed(shadow),
            );
            let hw = text::measure(font, hour, 0.0);
            text::draw(
                rc,
                font,
                hour,
                bx - 8.0 - hw,
                ty,
                TextStyle::plain(p.text).shadowed(shadow),
            );
        }
    }

    /// Whether (`x`, `y`) in the band is on the connection button.
    #[must_use]
    pub fn on_button(&self, x: f32, y: f32) -> bool {
        let (left, width) = self.button;
        width > 0.0 && (0.0..BAND).contains(&y) && (left..left + width).contains(&x)
    }
}

/// The fonts the card uses.
#[derive(Debug, Clone, Default)]
pub struct CardFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// Facts and the state.
    pub body: Option<Arc<UiFont>>,
    /// The note at the foot.
    pub caption: Option<Arc<UiFont>>,
}

/// The connection card.
#[derive(Debug)]
pub struct ConnectionCard {
    theme: Horizon,
    fonts: CardFonts,
    /// The link shown.
    pub link: LinkStatus,
    /// The close button was clicked.
    pub closed: bool,
    on_close: bool,
}

const HEAD: f32 = 40.0;
const NOTE: [&str; 3] = [
    "Slow after 5 s with no word from the server,",
    "bad after 20 s, lost after 40 s. The game keeps",
    "trying on its own.",
];

impl ConnectionCard {
    /// A card in `fonts`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: CardFonts) -> Self {
        Self {
            theme,
            fonts,
            link: LinkStatus::default(),
            closed: false,
            on_close: false,
        }
    }

    fn close_rect() -> (f32, f32, f32, f32) {
        (CARD_WIDTH - 8.0 - 28.0, (HEAD - 28.0) * 0.5, 28.0, 28.0)
    }

    fn on_close_at(x: f32, y: f32) -> bool {
        let (cx, cy, w, h) = Self::close_rect();
        (cx..cx + w).contains(&x) && (cy..cy + h).contains(&y)
    }

    /// The facts listed: what and its value.
    #[must_use]
    pub fn facts(link: &LinkStatus) -> [(&'static str, String); 3] {
        let quiet = link.seconds_since_last_packet.max(0.0) as i64;
        [
            (
                "Round trip",
                round_trip_words(link).unwrap_or_else(|| "not measured yet".to_owned()),
            ),
            ("Packets lost", format!("{:.1}%", link.packet_loss_percent)),
            (
                "Last heard",
                if link.connected {
                    format!("{quiet} s ago")
                } else {
                    "not connected".to_owned()
                },
            ),
        ]
    }
}

impl Widget for ConnectionCard {
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
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.on_close = Self::on_close_at(x, y);
                true
            }
            kind::HOVER_LEAVE => {
                self.on_close = false;
                true
            }
            kind::CLICK => {
                if Self::on_close_at(x, y) {
                    self.closed = true;
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let radius = CornerRadii::all(self.theme.metrics.window_radius);
        let (w, h) = (CARD_WIDTH, CARD_HEIGHT);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 12.0, 30.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, w, h, radius, p.popup);
        rc.stroke_rounded_rect(0.0, 0.0, w, h, radius, p.trim, 1.0);
        rc.fill(0.0, HEAD, w, 1.0, tone::bronze(0.35));
        // The head: a signal mark, the title, the close button.
        let (sx, sy) = (14.0, (HEAD - 16.0) * 0.5);
        for r in [7.5_f32, 5.0, 2.5] {
            rc.stroke_arc(
                sx + 8.0,
                sy + 13.0,
                r * 1.4,
                -std::f32::consts::PI * 0.75,
                std::f32::consts::PI * 0.5,
                p.accent,
                1.8,
            );
        }
        rc.fill_ellipse(sx + 7.0, sy + 12.0, 2.0, 2.0, p.accent);
        if let Some(f) = &self.fonts.title {
            let ty = ((HEAD - f.line_height) * 0.5).round();
            text::draw(rc, f, "Connection", 38.0, ty, TextStyle::plain(p.text));
        }
        let (cx, cy, cw, ch) = Self::close_rect();
        if self.on_close {
            rc.fill_rounded_rect(cx, cy, cw, ch, CornerRadii::all(4.0), tone::field(0.85));
        }
        let (mx, my) = (cx + cw * 0.5, cy + ch * 0.5);
        rc.smooth_line(mx - 5.0, my - 5.0, mx + 5.0, my + 5.0, p.icon, 1.6);
        rc.smooth_line(mx - 5.0, my + 5.0, mx + 5.0, my - 5.0, p.icon, 1.6);
        // The state.
        let state = LinkState::of(&self.link);
        let (bx, by, bw, bh) = (14.0, HEAD + 12.0, w - 28.0, 30.0);
        let tint = state.dot();
        rc.fill_rounded_rect(
            bx,
            by,
            bw,
            bh,
            CornerRadii::all(4.0),
            Color { a: 0.12, ..tint },
        );
        rc.stroke_rounded_rect(
            bx,
            by,
            bw,
            bh,
            CornerRadii::all(4.0),
            Color { a: 0.5, ..tint },
            1.0,
        );
        rc.fill_ellipse(bx + 10.0, by + (bh - 8.0) * 0.5, 8.0, 8.0, tint);
        let facts = Self::facts(&self.link);
        if let Some(f) = &self.fonts.body {
            let ty = (by + (bh - f.line_height) * 0.5).round();
            text::draw(rc, f, state.name(), bx + 24.0, ty, TextStyle::plain(p.text));
            let quiet = self.link.seconds_since_last_packet.max(0.0) as i64;
            let words = match state {
                LinkState::Good => "Answering".to_owned(),
                LinkState::Lost if !self.link.connected => "Not connected".to_owned(),
                _ => format!("Nothing heard for {quiet} s"),
            };
            let ww = text::measure(f, &words, 0.0);
            text::draw(
                rc,
                f,
                &words,
                bx + bw - 10.0 - ww,
                ty,
                TextStyle::plain(p.icon),
            );
            let mut y = by + bh + 10.0;
            for (key, value) in &facts {
                text::draw(rc, f, key, 14.0, y, TextStyle::plain(p.muted));
                let vw = text::measure(f, value, 0.0);
                text::draw(rc, f, value, w - 14.0 - vw, y, TextStyle::plain(p.text));
                y += f.line_height + 8.0;
            }
            if let Some(c) = &self.fonts.caption {
                y += 2.0;
                for line in NOTE {
                    text::draw(rc, c, line, 14.0, y, TextStyle::plain(p.muted));
                    y += c.line_height + 2.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(connected: bool, quiet: f64, rtt: Option<f64>) -> LinkStatus {
        LinkStatus {
            connected,
            seconds_since_last_packet: quiet,
            packet_loss_percent: 0.4,
            round_trip_seconds: rtt,
        }
    }

    #[test]
    fn the_line_is_quiet_while_good_and_loud_when_not() {
        assert_eq!(status_words(&link(true, 1.0, Some(0.038))), "38 ms");
        assert_eq!(status_words(&link(true, 1.0, None)), "");
        assert_eq!(status_words(&link(true, 7.2, None)), "Slow \u{b7} 7 s");
        assert_eq!(
            status_words(&link(true, 24.0, None)),
            "No answer \u{b7} 24 s"
        );
        assert_eq!(status_words(&link(true, 41.0, None)), "Connection lost");
        assert_eq!(status_words(&link(false, 0.0, None)), "Connection lost");
    }

    #[test]
    fn the_card_lists_the_round_trip_loss_and_last_word() {
        let facts = ConnectionCard::facts(&link(true, 7.0, Some(0.041)));
        assert_eq!(facts[0].1, "41 ms");
        assert_eq!(facts[1].1, "0.4%");
        assert_eq!(facts[2].1, "7 s ago");
        let facts = ConnectionCard::facts(&link(true, 0.0, None));
        assert_eq!(facts[0].1, "not measured yet");
    }
}
