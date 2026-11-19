//! The toolkit pieces OpenAC's modern interface is built from: its look
//! ([`theme`]), easing motion ([`tween`]), short numbers ([`numbers`]),
//! flex layout over elements ([`flex`]), a pane that opens on hover
//! ([`hover`]), the outlines quick move shows ([`quick_move`]) and text
//! with spacing and shadows ([`text`]).
//! The modern screens themselves live with the game window
//! (`ac-ui-window`); what is here knows nothing of the game.

pub mod flex;
pub mod hover;
pub mod numbers;
pub mod quick_move;
pub mod text;
pub mod theme;
pub mod tween;

pub use theme::Horizon;
pub use tween::{Ease, Tween};
