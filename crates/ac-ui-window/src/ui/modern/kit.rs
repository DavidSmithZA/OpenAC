//! Pieces the modern windows draw alike: the window's back and header,
//! the close button, underlined tabs with counts, buttons, filter chips,
//! the search box (and its field), thin scrollbars and wrapped text.
//! Everything is in the window's own units, drawn by the window's widget.

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId};
use ac_ui::widgets::Field;
use ac_ui::{UiFont, UiRoot};

use super::hud::lines::{circle, stroke};

/// A rectangle: left, top, width, height.
pub type Rect = (f32, f32, f32, f32);

/// Whether (`px`, `py`) is inside `r`.
#[must_use]
pub fn inside((x, y, w, h): Rect, px: f32, py: f32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

/// Whether the pointer at `hover` is inside `r`.
#[must_use]
pub fn hot(hover: Option<(f32, f32)>, r: Rect) -> bool {
    hover.is_some_and(|(x, y)| inside(r, x, y))
}

/// The spacing of spaced capitals (group headings, labels).
pub const LABEL_TRACKING: f32 = 1.14;

/// Bronze at `alpha`.
#[must_use]
pub const fn bronze(alpha: f32) -> Color {
    hex(0xC9_A062, alpha)
}

/// The gold of what is chosen, at `alpha`.
#[must_use]
pub const fn gold(alpha: f32) -> Color {
    hex(0xE0_B068, alpha)
}

/// Words a step softer than the text.
pub const SOFT: Color = hex(0xC8_B89C, 1.0);
/// Warm words (descriptions).
pub const WARM: Color = hex(0xE9_D3AE, 1.0);
/// Helpful changes.
pub const GOOD: Color = hex(0x7F_D08A, 1.0);
/// Harmful words.
pub const HARM: Color = hex(0xFF_8A8E, 1.0);
/// Ending soon.
pub const SOON: Color = hex(0xFF_B547, 1.0);
/// Red, for what is about to run out.
pub const RED: Color = hex(0xFF_6B70, 1.0);

/// A window's back: shadow, glass, edge and the header's glow, with a rule
/// under the header when `rule`.
pub fn window_back(
    rc: &mut RenderContext<'_>,
    theme: &Horizon,
    w: f32,
    h: f32,
    header: f32,
    rule: bool,
) {
    let round = CornerRadii::all(theme.metrics.window_radius);
    rc.soft_shadow(0.0, 0.0, w, h, 6.0, 18.0, 50.0, hex(0, 0.55));
    rc.fill_rounded_rect(0.0, 0.0, w, h, round, hex(0x14_100C, 0.95));
    rc.stroke_rounded_rect(0.0, 0.0, w, h, round, bronze(0.8), 1.0);
    rc.fill_vertical_gradient(
        1.0,
        1.0,
        w - 2.0,
        header - 1.0,
        CornerRadii::new(5.0, 5.0, 0.0, 0.0),
        bronze(0.10),
        bronze(0.0),
    );
    if rule {
        rc.fill(0.0, header - 1.0, w, 1.0, bronze(0.35));
    }
}

/// The title beside its icon (`glyph` in the 24-unit box), the header
/// `header` high.
pub fn title(
    rc: &mut RenderContext<'_>,
    theme: &Horizon,
    font: Option<&Arc<UiFont>>,
    glyph: &[&[(f32, f32)]],
    words: &str,
    header: f32,
) -> f32 {
    stroke(rc, 20.0, (header - 18.0) * 0.5, 18.0, glyph, gold(1.0), 2.0);
    let Some(f) = font else {
        return 46.0;
    };
    let y = ((header - f.line_height) * 0.5).round();
    text::draw(rc, f, words, 46.0, y, TextStyle::plain(theme.palette.text));
    46.0 + text::measure(f, words, 0.0)
}

/// The close button's place in a window `w` wide with a header `header`
/// high.
#[must_use]
pub fn close_rect(w: f32, header: f32) -> Rect {
    (w - 12.0 - 30.0, (header - 30.0) * 0.5, 30.0, 30.0)
}

/// The close button.
pub fn close(rc: &mut RenderContext<'_>, theme: &Horizon, r: Rect, lit: bool) {
    let p = theme.palette;
    if lit {
        rc.fill_rounded_rect(
            r.0,
            r.1,
            r.2,
            r.3,
            CornerRadii::all(4.0),
            hex(0xFF_FFFF, 0.06),
        );
    }
    stroke(
        rc,
        r.0 + 7.0,
        r.1 + 7.0,
        16.0,
        &[&[(6.0, 6.0), (18.0, 18.0)], &[(18.0, 6.0), (6.0, 18.0)]],
        if lit { p.accent } else { p.icon },
        1.8,
    );
}

/// An underlined tab: its words and, when `count` has some, a pill with
/// it (red for harmful things).
#[derive(Debug)]
pub struct Tab<'a> {
    /// The words.
    pub words: &'a str,
    /// The pill's figure (none: no pill).
    pub count: Option<String>,
    /// It is the tab shown.
    pub on: bool,
    /// The pill is red.
    pub red: bool,
}

/// How wide `tab` is in `font` with its pill in `badge`.
#[must_use]
pub fn tab_width(tab: &Tab<'_>, font: Option<&Arc<UiFont>>, badge: Option<&Arc<UiFont>>) -> f32 {
    let words = font.map_or(60.0, |f| text::measure(f, tab.words, 0.0));
    let pill = match (&tab.count, badge) {
        (Some(c), Some(b)) => 7.0 + text::measure(b, c, 0.0) + 12.0,
        _ => 0.0,
    };
    24.0 + words + pill
}

/// Draws `tab` in `r`, its line along the bottom.
pub fn draw_tab(
    rc: &mut RenderContext<'_>,
    theme: &Horizon,
    tab: &Tab<'_>,
    r: Rect,
    font: Option<&Arc<UiFont>>,
    badge: Option<&Arc<UiFont>>,
    lit: bool,
) {
    let p = theme.palette;
    if tab.on {
        rc.fill(r.0, r.1 + r.3 - 2.0, r.2, 2.0, gold(1.0));
    }
    let Some(f) = font else {
        return;
    };
    let color = if tab.on || lit { p.text } else { p.muted };
    let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
    text::draw(rc, f, tab.words, r.0 + 12.0, y, TextStyle::plain(color));
    if let (Some(count), Some(b)) = (&tab.count, badge) {
        let x = r.0 + 12.0 + text::measure(f, tab.words, 0.0) + 7.0;
        let w = text::measure(b, count, 0.0) + 12.0;
        let py = r.1 + (r.3 - 18.0) * 0.5;
        let (fill, ink) = if tab.red {
            (hex(0xFF_6B70, 0.18), hex(0xFF_B3B5, 1.0))
        } else {
            (bronze(0.16), WARM)
        };
        rc.fill_rounded_rect(x, py, w, 18.0, CornerRadii::all(9.0), fill);
        let ty = py + ((18.0 - b.line_height) * 0.5).round();
        text::draw(rc, b, count, x + 6.0, ty, TextStyle::plain(ink));
    }
}

/// How a button looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonLook {
    /// The window's main action: gold.
    Primary,
    /// Any other.
    Plain,
    /// Asks before it does something that cannot be undone.
    Danger,
}

/// A button with `words` centred in `r`; `enabled` false draws it faded.
#[allow(clippy::too_many_arguments)]
pub fn button(
    rc: &mut RenderContext<'_>,
    theme: &Horizon,
    r: Rect,
    words: &str,
    font: Option<&Arc<UiFont>>,
    look: ButtonLook,
    lit: bool,
    enabled: bool,
) {
    let p = theme.palette;
    let radius = CornerRadii::all(4.0);
    let fade = if enabled { 1.0 } else { 0.45 };
    let (fill, edge, ink) = match look {
        ButtonLook::Primary => (
            gold(if lit && enabled { 1.0 } else { 0.9 } * fade),
            gold(fade),
            hex(0x1A_140D, fade),
        ),
        ButtonLook::Plain => (
            hex(0, if lit && enabled { 0.5 } else { 0.35 } * fade),
            bronze(if lit && enabled { 0.9 } else { 0.6 } * fade),
            Color { a: fade, ..p.text },
        ),
        ButtonLook::Danger => (
            hex(0x3C_0E10, if lit && enabled { 0.7 } else { 0.45 } * fade),
            hex(0xFF_6B70, 0.6 * fade),
            Color { a: fade, ..HARM },
        ),
    };
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, fill);
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, edge, 1.0);
    if let Some(f) = font {
        let w = text::measure(f, words, 0.0);
        let x = (r.0 + (r.2 - w) * 0.5).round();
        let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
        text::draw(rc, f, words, x, y, TextStyle::plain(ink));
    }
}

/// How wide a button with `words` in `font` is.
#[must_use]
pub fn button_width(words: &str, font: Option<&Arc<UiFont>>) -> f32 {
    font.map_or(80.0, |f| text::measure(f, words, 0.0)) + 28.0
}

/// A filter chip: its words and a quiet count, gold when `on`.
#[allow(clippy::too_many_arguments)]
pub fn chip(
    rc: &mut RenderContext<'_>,
    theme: &Horizon,
    r: Rect,
    words: &str,
    count: Option<&str>,
    font: Option<&Arc<UiFont>>,
    small: Option<&Arc<UiFont>>,
    on: bool,
    lit: bool,
) {
    let p = theme.palette;
    let radius = CornerRadii::all(r.3 * 0.5);
    let fill = if on { gold(0.14) } else { hex(0, 0.25) };
    let edge = if on {
        gold(1.0)
    } else if lit {
        bronze(0.6)
    } else {
        bronze(0.3)
    };
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, fill);
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, edge, 1.0);
    let Some(f) = font else {
        return;
    };
    let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
    text::draw(rc, f, words, r.0 + 9.0, y, TextStyle::plain(p.text));
    if let (Some(c), Some(s)) = (count, small) {
        let x = r.0 + 9.0 + text::measure(f, words, 0.0) + 6.0;
        let y = r.1 + ((r.3 - s.line_height) * 0.5).round();
        text::draw(rc, s, c, x, y, TextStyle::plain(p.muted));
    }
}

/// How wide a chip with `words` and `count` is.
#[must_use]
pub fn chip_width(
    words: &str,
    count: Option<&str>,
    font: Option<&Arc<UiFont>>,
    small: Option<&Arc<UiFont>>,
) -> f32 {
    let w = font.map_or(60.0, |f| text::measure(f, words, 0.0));
    let c = match (count, small) {
        (Some(c), Some(s)) => 6.0 + text::measure(s, c, 0.0),
        _ => 0.0,
    };
    w + c + 18.0
}

/// The magnifier's lines (and its circle, drawn apart).
const HANDLE: &[&[(f32, f32)]] = &[&[(16.2, 16.2), (21.0, 21.0)]];

/// The search box in `r` (the field goes over it, 30 units in from its
/// left); `empty` shows `hint` in it.
#[allow(clippy::too_many_arguments)]
pub fn search_box(
    rc: &mut RenderContext<'_>,
    theme: &Horizon,
    r: Rect,
    hint: &str,
    empty: bool,
    focused: bool,
    font: Option<&Arc<UiFont>>,
) {
    let p = theme.palette;
    let radius = CornerRadii::all(4.0);
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, hex(0, 0.35));
    let edge = if focused { gold(0.8) } else { bronze(0.45) };
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, edge, 1.0);
    let s = 14.0;
    let (gx, gy) = (r.0 + 10.0, r.1 + (r.3 - s) * 0.5);
    circle(rc, gx, gy, s, (11.0, 11.0), 7.0, SOFT, 1.6);
    stroke(rc, gx, gy, s, HANDLE, SOFT, 1.6);
    if empty && let Some(f) = font {
        let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
        text::draw(rc, f, hint, r.0 + 32.0, y, TextStyle::plain(p.muted));
    }
}

/// A one-line search field over the search box at `r` in `parent`.
pub fn mount_search(
    ui: &mut UiRoot,
    parent: ElementId,
    r: Rect,
    font: Option<Arc<UiFont>>,
    theme: &Horizon,
) -> ElementId {
    let mut e = Element::at(r.0 + 29.0, r.1 + (r.3 - 24.0) * 0.5, r.2 - 34.0, 24.0)
        .with_anchors(AnchorEdges::NONE);
    Field::configure(&mut e);
    let mut f = Field::default();
    f.font = font;
    f.one_line = true;
    f.selectable = true;
    f.padding = 3.0;
    f.record_history = false;
    f.clear_on_submit = false;
    f.stay_focused_after_submit = true;
    f.text_color = theme.palette.text;
    f.background_color = Color::TRANSPARENT;
    f.selection_color = gold(0.35);
    f.set_editable_detached(&mut e, true);
    ui.add(parent, e, f)
}

/// What is typed in the search field `field`.
#[must_use]
pub fn field_text(ui: &UiRoot, field: ElementId) -> String {
    ui.widget::<Field>(field)
        .map(Field::text)
        .unwrap_or_default()
}

/// An input box in `r` (a field goes over it, [`mount_field`]); `empty`
/// shows `hint` in it.
#[allow(clippy::too_many_arguments)]
pub fn field_box(
    rc: &mut RenderContext<'_>,
    theme: &Horizon,
    r: Rect,
    hint: &str,
    empty: bool,
    focused: bool,
    font: Option<&Arc<UiFont>>,
) {
    let radius = CornerRadii::all(4.0);
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, hex(0, 0.35));
    let edge = if focused { gold(0.8) } else { bronze(0.45) };
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, edge, 1.0);
    if empty && let Some(f) = font {
        let y = if r.3 > 40.0 {
            r.1 + 8.0
        } else {
            r.1 + ((r.3 - f.line_height) * 0.5).round()
        };
        text::draw(
            rc,
            f,
            hint,
            r.0 + 10.0,
            y,
            TextStyle::plain(theme.palette.muted),
        );
    }
}

/// A field over the input box at `r` in `parent` (one line unless
/// `lines`), at most `most` characters (0: no limit).
pub fn mount_field(
    ui: &mut UiRoot,
    parent: ElementId,
    r: Rect,
    font: Option<Arc<UiFont>>,
    theme: &Horizon,
    lines: bool,
) -> ElementId {
    let (y, h) = if lines {
        (r.1 + 4.0, r.3 - 8.0)
    } else {
        (r.1 + (r.3 - 24.0) * 0.5, 24.0)
    };
    let mut e = Element::at(r.0 + 6.0, y, r.2 - 12.0, h).with_anchors(AnchorEdges::NONE);
    Field::configure(&mut e);
    let mut f = Field::default();
    f.font = font;
    f.one_line = !lines;
    f.selectable = true;
    f.padding = 3.0;
    f.record_history = false;
    f.clear_on_submit = false;
    f.stay_focused_after_submit = true;
    f.text_color = theme.palette.text;
    f.background_color = Color::TRANSPARENT;
    f.selection_color = gold(0.35);
    f.set_editable_detached(&mut e, true);
    ui.add(parent, e, f)
}

/// Puts `words` in field `field` (when it holds something else).
pub fn set_field_text(ui: &mut UiRoot, field: ElementId, words: &str) {
    if let Some(f) = ui.widget_mut::<Field>(field)
        && f.text() != words
    {
        f.set_text(words);
    }
}

/// A switch in `r` (38 by 20 suits): gold with its knob right when `on`.
pub fn switch(rc: &mut RenderContext<'_>, r: Rect, on: bool, lit: bool) {
    let track = if on {
        gold(0.9)
    } else {
        hex(0xFF_FFFF, if lit { 0.2 } else { 0.14 })
    };
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(r.3 * 0.5), track);
    let knob = if on { hex(0x1A_140D, 1.0) } else { SOFT };
    let kx = if on { r.0 + r.2 - 17.0 } else { r.0 + 3.0 };
    rc.fill_rounded_rect(kx, r.1 + 3.0, 14.0, 14.0, CornerRadii::all(7.0), knob);
}

/// The OPENAC tag (what the original client does not have) at `x`,
/// centred on `cy`; returns its width.
pub fn openac_tag(rc: &mut RenderContext<'_>, font: Option<&Arc<UiFont>>, x: f32, cy: f32) -> f32 {
    let Some(f) = font else {
        return 0.0;
    };
    let w = text::measure(f, "OPENAC", 0.6) + 12.0;
    let r = (x, cy - 8.0, w, 16.0);
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(8.0), bronze(0.5), 1.0);
    text::draw(
        rc,
        f,
        "OPENAC",
        x + 6.0,
        r.1 + ((16.0 - f.line_height) * 0.5).round(),
        TextStyle::plain(gold(1.0)).tracked(0.6),
    );
    w
}

/// Someone's online dot at (`x`, `y`), `size` across: filled green when
/// online, an empty grey ring when not.
pub fn online_dot(rc: &mut RenderContext<'_>, x: f32, y: f32, size: f32, online: bool) {
    if online {
        rc.fill_ellipse(x, y, size, size, GOOD);
    } else {
        rc.stroke_ellipse(x, y, size, size, hex(0x8C_8070, 1.0), 1.5);
    }
}

/// A row picked (gold edge and wash) or not (a quiet one), lit when
/// hovered.
pub fn pick_row(rc: &mut RenderContext<'_>, r: Rect, on: bool, lit: bool) {
    let radius = CornerRadii::all(4.0);
    let (fill, edge) = if on {
        (gold(0.12), gold(1.0))
    } else if lit {
        (hex(0xFF_FFFF, 0.05), bronze(0.3))
    } else {
        (hex(0, 0.22), bronze(0.18))
    };
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, fill);
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, edge, 1.0);
}

/// A spaced-capitals heading with a count and a rule running on to
/// `right`.
#[allow(clippy::too_many_arguments)]
pub fn group_heading(
    rc: &mut RenderContext<'_>,
    theme: &Horizon,
    font: Option<&Arc<UiFont>>,
    x: f32,
    y: f32,
    h: f32,
    right: f32,
    words: &str,
    count: Option<&str>,
    color: Color,
) {
    let mut x = x;
    if let Some(f) = font {
        let ty = y + ((h - f.line_height) * 0.5).round();
        text::draw(
            rc,
            f,
            words,
            x,
            ty,
            TextStyle::plain(color).tracked(LABEL_TRACKING),
        );
        x += text::measure(f, words, LABEL_TRACKING) + 8.0;
        if let Some(c) = count {
            text::draw(
                rc,
                f,
                c,
                x,
                ty,
                TextStyle::plain(color).tracked(LABEL_TRACKING),
            );
            x += text::measure(f, c, LABEL_TRACKING) + 8.0;
        }
    }
    rc.fill(
        x,
        y + (h * 0.5).round(),
        (right - x).max(0.0),
        1.0,
        theme.palette.trim_faint,
    );
}

/// A scrolled region's offset, held within its content.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Scroll {
    /// How far it is scrolled.
    pub at: f32,
}

impl Scroll {
    /// One wheel notch (`notch` above zero scrolls up) over content
    /// `total` high in a room `room` high, by `step`.
    pub fn wheel(&mut self, notch: i32, step: f32, total: f32, room: f32) {
        let by = -(notch.signum() as f32) * step;
        self.at = (self.at + by).clamp(0.0, (total - room).max(0.0));
    }

    /// Kept within content `total` high in a room `room` high.
    pub fn hold(&mut self, total: f32, room: f32) {
        self.at = self.at.clamp(0.0, (total - room).max(0.0));
    }

    /// Scrolled so `top..top + h` shows in a room `room` high.
    pub fn reveal(&mut self, top: f32, h: f32, room: f32) {
        if top < self.at {
            self.at = top;
        } else if top + h > self.at + room {
            self.at = top + h - room;
        }
    }
}

/// A thin scrollbar at `x` beside a room from `top`, `room` high, over
/// content `total` high scrolled `at`; nothing when it all fits.
pub fn scrollbar(rc: &mut RenderContext<'_>, x: f32, top: f32, room: f32, total: f32, at: f32) {
    if total <= room + 0.5 {
        return;
    }
    let thumb = (room * room / total).max(18.0);
    let travel = room - thumb;
    let y = top + travel * (at / (total - room)).clamp(0.0, 1.0);
    rc.fill_rounded_rect(
        x,
        top,
        5.0,
        room,
        CornerRadii::all(2.5),
        hex(0xFF_FFFF, 0.05),
    );
    rc.fill_rounded_rect(x, y, 5.0, thumb, CornerRadii::all(2.5), bronze(0.7));
}

/// A thin bar `share` full (time left, a cost against what is had).
pub fn meter(rc: &mut RenderContext<'_>, r: Rect, share: f32, color: Color) {
    let radius = CornerRadii::all(r.3 * 0.5);
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, hex(0, 0.55));
    let w = (r.2 * share.clamp(0.0, 1.0)).max(0.0);
    if w > 0.5 {
        rc.fill_rounded_rect(r.0, r.1, w, r.3, radius, color);
    }
}

/// `words` split into lines no wider than `width` in `font` (a blank line
/// between paragraphs kept).
#[must_use]
pub fn wrap(font: &UiFont, words: &str, width: f32) -> Vec<String> {
    let mut out = Vec::new();
    for (i, paragraph) in words.split('\n').enumerate() {
        if paragraph.trim().is_empty() {
            if i > 0 && out.last().is_some_and(|l: &String| !l.is_empty()) {
                out.push(String::new());
            }
            continue;
        }
        out.extend(super::bars::edit::wrap(font, paragraph, width));
    }
    out
}

/// `words` as they are when they fit `width`, else cut short.
#[must_use]
pub fn fit(font: &UiFont, words: &str, width: f32) -> String {
    super::items::fit(font, words, width)
}

/// Draws `lines` from `y`, `step` apart, in `color`; returns the bottom.
#[allow(clippy::too_many_arguments)]
pub fn lines(
    rc: &mut RenderContext<'_>,
    font: &UiFont,
    lines: &[String],
    x: f32,
    y: f32,
    step: f32,
    color: Color,
) -> f32 {
    let mut y = y;
    for line in lines {
        text::draw(rc, font, line, x, y, TextStyle::plain(color));
        y += step;
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scroll_is_held_within_its_content() {
        let mut s = Scroll::default();
        s.wheel(-1, 50.0, 300.0, 200.0);
        assert!((s.at - 50.0).abs() < 1e-6);
        s.wheel(-1, 80.0, 300.0, 200.0);
        assert!((s.at - 100.0).abs() < 1e-6);
        s.wheel(1, 500.0, 300.0, 200.0);
        assert!(s.at.abs() < 1e-6);
        s.reveal(250.0, 40.0, 200.0);
        assert!((s.at - 90.0).abs() < 1e-6);
        s.reveal(10.0, 40.0, 200.0);
        assert!((s.at - 10.0).abs() < 1e-6);
    }

    #[test]
    fn a_point_is_inside_a_rectangle_up_to_its_far_edges() {
        let r = (10.0, 10.0, 20.0, 20.0);
        assert!(inside(r, 10.0, 10.0));
        assert!(!inside(r, 30.0, 15.0));
        assert!(hot(Some((15.0, 15.0)), r));
        assert!(!hot(None, r));
    }
}
