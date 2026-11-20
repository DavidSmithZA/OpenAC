//! The modern item screens: the inventory (equipment, packs and the item
//! details), the Inspect sheet and the Loot window.

pub mod cell;
pub mod grid;
pub mod inspect;
pub mod inventory;
pub mod loot;
pub mod menu;
pub mod model;
pub mod run;
pub mod sheet;
pub mod sheet_view;
pub mod sheets;

use ac_ui::UiFont;
use ac_ui::modern::text;

/// `words` as they are when they fit `width`, else cut short with an
/// ellipsis.
#[must_use]
pub fn fit(font: &UiFont, words: &str, width: f32) -> String {
    if text::measure(font, words, 0.0) <= width {
        words.to_owned()
    } else {
        crate::ui::modern::chat::panel::shorten(font, words, width)
    }
}
