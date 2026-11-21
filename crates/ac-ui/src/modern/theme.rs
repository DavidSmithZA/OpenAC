//! Horizon, the modern interface's look: thin bronze trim on dark glass,
//! warm text, and the world left to show through. The values are the
//! design's (`docs/modern-ui` on the design branch); a screen takes its
//! colours and sizes from here, never from numbers of its own.

use crate::draw::{Color, RenderContext};
use crate::shapes::CornerRadii;

/// The most of a glass fill's colour that stays over the blurred world:
/// a fill more solid than this lets the blur show through it; a lighter
/// one keeps its own.
pub const GLASS_MOST: f32 = 0.8;

/// A window's or pop-up's back as frosted glass: the world behind it
/// blurred under `color` (no more solid than [`GLASS_MOST`]); where glass
/// is off, `color` as it is.
pub fn fill_glass(
    rc: &mut RenderContext<'_>,
    (x, y, w, h): (f32, f32, f32, f32),
    radii: CornerRadii,
    color: Color,
) {
    let tint = Color {
        a: color.a.min(GLASS_MOST),
        ..color
    };
    rc.fill_glass_rounded_rect(x, y, w, h, radii, tint, color);
}

/// A colour from `0xRRGGBB` and an alpha from 0 to 1.
#[must_use]
pub const fn hex(packed: u32, alpha: f32) -> Color {
    Color::rgba(
        ((packed >> 16) & 0xFF) as f32 / 255.0,
        ((packed >> 8) & 0xFF) as f32 / 255.0,
        (packed & 0xFF) as f32 / 255.0,
        alpha,
    )
}

/// The colours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonPalette {
    /// Window and HUD edges: the bronze trim.
    pub trim: Color,
    /// Quieter edges: slots, fields, cards.
    pub trim_soft: Color,
    /// Dividers and the faintest edges.
    pub trim_faint: Color,
    /// A window's fill (translucent; the world shows through).
    pub window: Color,
    /// A pop-up's or hover pane's fill, a little more solid.
    pub popup: Color,
    /// A HUD piece's fill over the world (pills, the panel strip).
    pub hud: Color,
    /// Wells inside a window: lists, slots, bars' troughs.
    pub well: Color,
    /// Text.
    pub text: Color,
    /// Text a step down: values beside labels, secondary lines.
    pub text_soft: Color,
    /// Quiet text: hints, units, captions.
    pub muted: Color,
    /// Icons drawn as lines.
    pub icon: Color,
    /// Highlights: the selected tab, names, links, the main button's edge.
    pub accent: Color,
    /// The main button's fill.
    pub accent_fill: Color,
    /// Text on the accent fill.
    pub accent_ink: Color,
    /// The fill of a selected row or slot.
    pub accent_tint: Color,
    /// Good news: helpful effects, a met need, online.
    pub good: Color,
    /// Bad news: harmful effects, an unmet need, a refusal.
    pub bad: Color,
    /// Bad news as text, lighter to read on the dark fill.
    pub bad_text: Color,
    /// A warning: ending soon, heavy, low ammo.
    pub warn: Color,
    /// An enemy's health and a hostile's mark.
    pub enemy: Color,
    /// Health.
    pub health: Color,
    /// Stamina.
    pub stamina: Color,
    /// Mana.
    pub mana: Color,
    /// The shadow under windows.
    pub shadow: Color,
    /// The shadow under text drawn straight over the world.
    pub text_shadow: Color,
}

/// The sizes, in UI units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonMetrics {
    /// Corners of buttons, chips, fields and HUD pieces.
    pub radius: f32,
    /// Corners of slots and icon tiles.
    pub slot_radius: f32,
    /// Corners of windows, pop-ups and cards.
    pub window_radius: f32,
    /// Every edge's width.
    pub edge: f32,
    /// How far a window's shadow reaches.
    pub shadow_blur: f32,
    /// How far down a window's shadow sits.
    pub shadow_drop: f32,
    /// The usual gap between things.
    pub gap: f32,
    /// A window's inside margin.
    pub padding: f32,
    /// A row's height in lists.
    pub row_height: f32,
    /// A button's height.
    pub button_height: f32,
    /// A slot's side on the shortcut bars.
    pub slot: f32,
    /// A window's title bar height.
    pub title_height: f32,
}

/// The text sizes, in UI units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonType {
    /// Small capitals over a group ("HELPFUL", "SKILLS"), spaced out.
    pub label: f32,
    /// Captions and hints.
    pub caption: f32,
    /// Secondary text.
    pub small: f32,
    /// Body text.
    pub body: f32,
    /// Chat lines.
    pub chat: f32,
    /// Names in the HUD and on a card.
    pub name: f32,
    /// Window titles.
    pub title: f32,
    /// Big numbers (a level, damage falling off a bar).
    pub big: f32,
}

/// The whole look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Horizon {
    /// The colours.
    pub palette: HorizonPalette,
    /// The sizes.
    pub metrics: HorizonMetrics,
    /// The text sizes.
    pub text: HorizonType,
}

impl Horizon {
    /// Horizon with bronze trim, the chosen look.
    pub const BRONZE: Self = Self {
        palette: HorizonPalette {
            trim: hex(0xC9_A062, 0.8),
            trim_soft: hex(0xC9_A062, 0.45),
            trim_faint: hex(0xC9_A062, 0.25),
            window: hex(0x14_100C, 0.92),
            popup: hex(0x14_100C, 0.95),
            hud: hex(0x00_0000, 0.5),
            well: hex(0x00_0000, 0.35),
            text: hex(0xF4_EBDC, 1.0),
            text_soft: hex(0xC8_B89C, 1.0),
            muted: hex(0xA8_9A82, 1.0),
            icon: hex(0xE9_D3AE, 1.0),
            accent: hex(0xE0_B068, 1.0),
            accent_fill: hex(0xE0_B068, 0.85),
            accent_ink: hex(0x1A_140D, 1.0),
            accent_tint: hex(0xE0_B068, 0.14),
            good: hex(0x7F_D08A, 1.0),
            bad: hex(0xFF_6B70, 1.0),
            bad_text: hex(0xFF_8A8E, 1.0),
            warn: hex(0xFF_AB00, 1.0),
            enemy: hex(0xE5_484D, 1.0),
            health: hex(0xB8_323A, 1.0),
            stamina: hex(0xB8_8A1E, 1.0),
            mana: hex(0x2F_64C2, 1.0),
            shadow: hex(0x00_0000, 0.55),
            text_shadow: hex(0x00_0000, 0.85),
        },
        metrics: HorizonMetrics {
            radius: 3.0,
            slot_radius: 4.0,
            window_radius: 6.0,
            edge: 1.0,
            shadow_blur: 50.0,
            shadow_drop: 18.0,
            gap: 8.0,
            padding: 14.0,
            row_height: 28.0,
            button_height: 28.0,
            slot: 40.0,
            title_height: 36.0,
        },
        text: HorizonType {
            label: 9.5,
            caption: 10.5,
            small: 11.0,
            body: 12.0,
            chat: 12.5,
            name: 14.0,
            title: 15.0,
            big: 16.0,
        },
    };
}

impl Default for Horizon {
    fn default() -> Self {
        Self::BRONZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_reads_red_green_blue() {
        let c = hex(0xFF_8000, 0.5);
        assert_eq!((c.r, c.g, c.b, c.a), (1.0, 128.0 / 255.0, 0.0, 0.5));
    }

    #[test]
    fn text_stands_out_from_the_window() {
        // Light text on a dark fill: the text is far brighter than the
        // window it sits on.
        let p = Horizon::BRONZE.palette;
        let luma = |c: Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
        assert!(luma(p.text) - luma(p.window) > 0.7);
        assert!(luma(p.muted) - luma(p.window) > 0.4);
    }
}
