//! The current look's colours by name, at an alpha: what the windows draw
//! their chrome in, so it follows the player's theme
//! ([`Horizon::current`](super::theme::Horizon::current)). Colours that
//! mean something (the vitals, good and bad, warnings) are not here: they
//! stay the same in every theme.

use super::theme::{Horizon, alpha};
use crate::draw::Color;

fn at(pick: fn(&super::theme::HorizonPalette) -> Color, a: f32) -> Color {
    alpha(pick(&Horizon::current().palette), a)
}

/// The trim (bronze), at `a`.
#[must_use]
pub fn bronze(a: f32) -> Color {
    at(|p| p.bronze, a)
}

/// The accent (the gold of what is chosen), at `a`.
#[must_use]
pub fn gold(a: f32) -> Color {
    at(|p| p.accent, a)
}

/// The accent, lighter (lit names, attuned items), at `a`.
#[must_use]
pub fn gold_light(a: f32) -> Color {
    at(|p| p.accent_light, a)
}

/// Text on the accent fill, at `a`.
#[must_use]
pub fn ink(a: f32) -> Color {
    at(|p| p.accent_ink, a)
}

/// A window's ground, at `a`.
#[must_use]
pub fn ground(a: f32) -> Color {
    at(|p| p.ground, a)
}

/// A raised ground (cards and pop-ups over a window), at `a`.
#[must_use]
pub fn raised(a: f32) -> Color {
    at(|p| p.raised, a)
}

/// Wells and buttons' faces, at `a`.
#[must_use]
pub fn field(a: f32) -> Color {
    at(|p| p.field, a)
}

/// The deepest shade (fades over pictures), at `a`.
#[must_use]
pub fn deep(a: f32) -> Color {
    at(|p| p.deep, a)
}

/// The text, at `a`.
#[must_use]
pub fn words(a: f32) -> Color {
    at(|p| p.text, a)
}

/// Text a step softer, at `a`.
#[must_use]
pub fn soft(a: f32) -> Color {
    at(|p| p.text_soft, a)
}

/// Quiet text (hints and notes), at `a`.
#[must_use]
pub fn muted(a: f32) -> Color {
    at(|p| p.muted, a)
}

/// Warm words and line icons, at `a`.
#[must_use]
pub fn warm(a: f32) -> Color {
    at(|p| p.icon, a)
}
