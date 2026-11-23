//! The toolkit pieces OpenAC's modern interface is built from: its look
//! ([`theme`]), easing motion ([`tween`]), short numbers ([`numbers`]),
//! flex layout over elements ([`flex`]), a pane that opens on hover
//! ([`hover`]), the outlines quick move shows ([`quick_move`]), hover
//! labels for pieces made in code ([`tip`]), text with spacing and
//! shadows ([`text`]), the pieces windows draw alike ([`kit`]: backs,
//! tabs, buttons, chips, search boxes, scrollbars, wrapped text), icons
//! drawn as lines ([`lines`]) and numbered picks laid out in a column
//! ([`picks`]).
//! The modern screens themselves live with the game window
//! (`ac-ui-window`); what is here knows nothing of the game.

pub mod cursors;
pub mod flex;
pub mod hover;
pub mod kit;
pub mod lines;
pub mod numbers;
pub mod picks;
pub mod quick_move;
pub mod text;
pub mod theme;
pub mod tip;
pub mod tone;
pub mod tween;

pub use theme::Horizon;
pub use tween::{Ease, Tween};
