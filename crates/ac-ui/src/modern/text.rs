//! Text as the modern interface sets it: optional letter-spacing (the small
//! capitals over a group or a bar), and a soft shadow under text drawn
//! straight over the world.

use crate::draw::{Color, RenderContext};
use crate::font::UiFont;

/// How a line of text is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    /// Its colour.
    pub color: Color,
    /// Extra room after every character but the last, units.
    pub tracking: f32,
    /// A shadow one unit below, in this colour.
    pub shadow: Option<Color>,
}

impl TextStyle {
    /// Plain text in `color`.
    #[must_use]
    pub const fn plain(color: Color) -> Self {
        Self {
            color,
            tracking: 0.0,
            shadow: None,
        }
    }

    /// The same, with `tracking` units after each character.
    #[must_use]
    pub const fn tracked(self, tracking: f32) -> Self {
        Self { tracking, ..self }
    }

    /// The same, with a shadow in `color`.
    #[must_use]
    pub const fn shadowed(self, color: Color) -> Self {
        Self {
            shadow: Some(color),
            ..self
        }
    }
}

/// The width of `text` in `font` with `tracking` after every character but
/// the last.
#[must_use]
pub fn measure(font: &UiFont, text: &str, tracking: f32) -> f32 {
    let width = font.measure_width(text);
    if tracking == 0.0 {
        return width;
    }
    let gaps = text.chars().filter(|&c| font.glyph(c).is_some()).count();
    width + tracking * gaps.saturating_sub(1) as f32
}

/// Draws `text` with its top left at (`x`, `y`).
pub fn draw(
    rc: &mut RenderContext<'_>,
    font: &UiFont,
    text: &str,
    x: f32,
    y: f32,
    style: TextStyle,
) {
    if let Some(shadow) = style.shadow {
        draw_run(rc, font, text, x, y + 1.0, shadow, style.tracking);
    }
    draw_run(rc, font, text, x, y, style.color, style.tracking);
}

/// Draws `text` centred on `centre_x`, its top at `y`; answers its width.
pub fn draw_centred(
    rc: &mut RenderContext<'_>,
    font: &UiFont,
    text: &str,
    centre_x: f32,
    y: f32,
    style: TextStyle,
) -> f32 {
    let width = measure(font, text, style.tracking);
    draw(rc, font, text, (centre_x - width * 0.5).round(), y, style);
    width
}

fn draw_run(
    rc: &mut RenderContext<'_>,
    font: &UiFont,
    text: &str,
    x: f32,
    y: f32,
    color: Color,
    tracking: f32,
) {
    if tracking == 0.0 {
        rc.string_dat(font, text, x, y, color, false, color);
        return;
    }
    // Spaced out, a character at a time: kerning gives way to the spacing.
    let mut pen = x;
    let mut buffer = [0u8; 4];
    for c in text.chars() {
        if font.glyph(c).is_none() {
            continue;
        }
        let one = c.encode_utf8(&mut buffer);
        rc.string_dat(font, one, pen, y, color, false, color);
        pen += font.advance(c) + tracking;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{DrawCmd, DrawList, NoSprites, TextureRef};

    fn font() -> UiFont {
        crate::bundled_font::bake(16.0, crate::bundled_font::Weight::Regular)
            .unwrap()
            .create_font(TextureRef::Runtime(1))
    }

    #[test]
    fn tracking_adds_room_between_characters_only() {
        let f = font();
        assert_eq!(measure(&f, "HP", 0.0), f.measure_width("HP"));
        assert_eq!(
            measure(&f, "HP", 2.0),
            f.advance('H') + f.advance('P') + 2.0
        );
        assert_eq!(measure(&f, "", 2.0), 0.0);
    }

    #[test]
    fn a_shadow_is_drawn_first_one_unit_down() {
        let f = font();
        let mut list = DrawList::default();
        let mut rc = RenderContext::new(&mut list, (200.0, 100.0), &NoSprites);
        let white = Color::WHITE;
        draw(
            &mut rc,
            &f,
            "A",
            10.0,
            10.0,
            TextStyle::plain(white).shadowed(Color::BLACK),
        );
        let runs: Vec<(Color, f32)> = list
            .main
            .iter()
            .filter_map(|c| match c {
                DrawCmd::Glyphs { tint, quads, .. } => Some((*tint, quads[0].y)),
                _ => None,
            })
            .collect();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].0, Color::BLACK);
        assert_eq!(runs[1].0, white);
        assert_eq!(runs[0].1, runs[1].1 + 1.0);
    }
}
