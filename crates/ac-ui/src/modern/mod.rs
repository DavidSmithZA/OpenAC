//! The toolkit pieces OpenAC's modern interface is built from: its look
//! ([`theme`]), easing motion ([`tween`]), short numbers ([`numbers`]) and
//! flex layout over elements ([`flex`]) and a pane that opens on hover
//! ([`hover`]).
//! The modern screens themselves live with the game window
//! (`ac-ui-window`); what is here knows nothing of the game.

pub mod flex;
pub mod hover;
pub mod numbers;
pub mod theme;
pub mod tween;

pub use theme::Horizon;
pub use tween::{Ease, Tween};
