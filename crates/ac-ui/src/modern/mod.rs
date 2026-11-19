//! The toolkit pieces OpenAC's modern interface is built from: its look
//! ([`theme`]), easing motion ([`tween`]) and short numbers ([`numbers`]).
//! The modern screens themselves live with the game window
//! (`ac-ui-window`); what is here knows nothing of the game.

pub mod numbers;
pub mod theme;
pub mod tween;

pub use theme::Horizon;
pub use tween::{Ease, Tween};
