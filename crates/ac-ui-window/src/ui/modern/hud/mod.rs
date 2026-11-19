//! The modern HUD: the pieces over the world. Each piece is a window of its
//! own (so its place is saved, and quick move can carry it), drawn by one
//! widget and fed from the session by the window.

pub mod edge_glow;
pub mod effects;
pub mod radar;
pub mod target;
pub mod vitals;

use ac_ui::UiRoot;
use ac_ui::geometry::AnchorEdges;
use ac_ui::tree::{Element, ElementId, Widget};

use super::places;

/// The vitals' window name.
pub const VITALS: &str = "modern-vitals";

/// The effects counts' and status chips' window name.
pub const EFFECTS: &str = "modern-effects";

/// The radar's window name.
pub const RADAR: &str = "modern-radar";

/// The target line's window name.
pub const TARGET: &str = "modern-target";

/// Mounts `widget` as the HUD piece `name`, at its first place on a screen
/// of the root's size (or the top left when the table has none); a piece
/// quick move can carry.
///
/// # Errors
///
/// The window could not be registered (a piece of that name exists).
pub fn mount_piece(
    ui: &mut UiRoot,
    name: &str,
    size: (f32, f32),
    widget: impl Widget,
) -> Result<ElementId, String> {
    let screen = ui.size();
    let rect = places::default_place(name).map_or(
        ac_ui::geometry::Rect::new(0.0, 0.0, size.0, size.1),
        |place| place.rect(screen),
    );
    let mut element = Element::at(rect.x, rect.y, rect.w, rect.h).with_anchors(AnchorEdges::NONE);
    element.name = Some(name.to_owned());
    element.quick_move = true;
    let root = ui.root();
    let frame = ui.add(root, element, widget);
    ui.register_window(name, frame, None, 0)
        .map_err(|e| format!("{name}: {e:?}"))?;
    Ok(frame)
}
