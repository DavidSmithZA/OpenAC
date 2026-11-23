//! The modern item screens: the inventory (equipment, packs and the item
//! details), the Inspect sheet and the Loot window.

pub mod cell;
pub mod glyphs;
pub mod grid;
pub mod inspect;
pub mod inventory;
pub mod loot;
pub mod menu;
pub mod model;
pub mod plugin_notes;
pub mod run;
pub mod scribe;
pub mod sheet;
pub mod sheet_view;
pub mod sheets;
pub mod split;

use ac_ui::UiFont;

/// `words` as they are when they fit `width`, else cut short with an
/// ellipsis.
#[must_use]
pub fn fit(font: &UiFont, words: &str, width: f32) -> String {
    ac_ui::modern::kit::fit(font, words, width)
}
