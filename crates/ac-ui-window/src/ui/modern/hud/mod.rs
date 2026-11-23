//! The modern HUD: the pieces over the world. Each piece is a window of its
//! own (so its place is saved, and quick move can carry it), drawn by one
//! widget and fed from the session by the window.

pub mod combat;
pub mod connection;
pub mod edge_glow;
pub mod effects;
pub mod fellows;
pub mod lines;
pub mod panels;
pub mod radar;
pub mod recall;
pub mod recall_dialog;
pub mod target;
pub mod target_guide;
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

/// The panel strip's window name.
pub const PANELS: &str = "modern-panels";

/// The combat strip's (and the jump meter's) window name.
pub const COMBAT: &str = "modern-combat";

/// The fellows on screen's window name.
pub const FELLOWS: &str = "modern-fellows";

/// The quests on screen's window name.
pub const QUESTS: &str = "modern-quests";

/// Every HUD piece's window name (not the bars: they keep their own lock
/// and hiding).
pub const PIECES: [&str; 8] = [
    VITALS, TARGET, EFFECTS, RADAR, COMBAT, PANELS, FELLOWS, QUESTS,
];

/// What the player calls piece `name` (its window name).
#[must_use]
pub fn piece_words(name: &str) -> &str {
    match name {
        VITALS => "Vitals",
        TARGET => "Target",
        EFFECTS => "Effects",
        RADAR => "Radar",
        COMBAT => "Combat strip",
        PANELS => "Panel strip",
        FELLOWS => "Fellows",
        QUESTS => "Quests",
        other => other.strip_prefix("modern-").unwrap_or(other),
    }
}

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
