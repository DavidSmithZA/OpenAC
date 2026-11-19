//! What quick move shows while its key is held: every modern piece on the
//! screen outlined with a dashed bronze line and named in a small tag at its
//! top left, so the player sees what can be moved and where each piece's
//! edges are. The moving itself is the root's ([`UiRoot::set_quick_move_key`]).

use std::collections::HashMap;
use std::sync::Arc;

use crate::draw::RenderContext;
use crate::font::UiFont;
use crate::root::UiRoot;
use crate::shapes::CornerRadii;
use crate::tree::{Element, ElementId, Widget};

use super::theme::Horizon;

/// A dash's length along an outline, units.
pub const DASH: f32 = 6.0;

/// The gap between dashes, units.
pub const DASH_GAP: f32 = 4.0;

/// The outlines' layer: a click-through element over the whole screen that
/// draws above everything while the quick-move key is held.
#[derive(Debug)]
pub struct QuickMoveOutlines {
    theme: Horizon,
    font: Option<Arc<UiFont>>,
    names: HashMap<ElementId, String>,
}

impl QuickMoveOutlines {
    /// The layer, made a child of the root; its names are drawn in `font`.
    pub fn mount(ui: &mut UiRoot, theme: Horizon, font: Option<Arc<UiFont>>) -> ElementId {
        let (w, h) = ui.size();
        let mut element = Element::at(0.0, 0.0, w, h);
        element.click_through = true;
        element.name = Some("quick-move-outlines".to_owned());
        let root = ui.root();
        ui.add(
            root,
            element,
            Self {
                theme,
                font,
                names: HashMap::new(),
            },
        )
    }

    /// The name shown on `piece` (otherwise its element's name).
    pub fn set_name(&mut self, piece: ElementId, name: impl Into<String>) {
        self.names.insert(piece, name.into());
    }
}

impl Widget for QuickMoveOutlines {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        false
    }

    fn draw_overlay(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        // Follows the screen's size.
        let (w, h) = ui.size();
        if let Some(e) = ui.element_mut(me) {
            e.width = w;
            e.height = h;
        }
        if !ui.quick_move_held() {
            return;
        }
        let root = ui.root();
        let p = self.theme.palette;
        for piece in ui.children(root) {
            let Some(e) = ui.element(piece).filter(|e| e.quick_move && e.drawn()) else {
                continue;
            };
            let (x, y, pw, ph) = (e.left, e.top, e.width, e.height);
            dashed_rect(rc, x, y, pw, ph, p.accent);
            let name = self
                .names
                .get(&piece)
                .cloned()
                .or_else(|| e.name.clone())
                .unwrap_or_default();
            if let Some(font) = &self.font
                && !name.is_empty()
            {
                let tw = font.measure_width(&name) + 12.0;
                let th = font.line_height + 4.0;
                let ty = (y - th - 2.0).max(0.0);
                rc.fill_rounded_rect(x, ty, tw, th, CornerRadii::all(3.0), p.accent_fill);
                rc.string_dat(
                    font,
                    &name,
                    x + 6.0,
                    ty + 2.0,
                    p.accent_ink,
                    false,
                    p.accent_ink,
                );
            }
        }
    }
}

/// A rectangle's outline in dashes one unit wide.
fn dashed_rect(rc: &mut RenderContext<'_>, x: f32, y: f32, w: f32, h: f32, color: crate::Color) {
    let step = DASH + DASH_GAP;
    let mut along = |x0: f32, y0: f32, length: f32, across: bool| {
        let mut at = 0.0;
        while at < length {
            let dash = DASH.min(length - at);
            if across {
                rc.fill(x0 + at, y0, dash, 1.0, color);
            } else {
                rc.fill(x0, y0 + at, 1.0, dash, color);
            }
            at += step;
        }
    };
    along(x, y, w, true);
    along(x, y + h - 1.0, w, true);
    along(x, y, h, false);
    along(x + w - 1.0, y, h, false);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{DrawCmd, NoSprites};
    use crate::event::Modifiers;
    use crate::widgets::Panel;

    fn overlay_fills(ui: &mut UiRoot) -> usize {
        let list = ui.draw((800.0, 600.0), &NoSprites);
        list.overlay
            .iter()
            .filter(|c| matches!(c, DrawCmd::Fill { .. }))
            .count()
    }

    #[test]
    fn outlines_show_only_while_the_key_is_held_and_only_on_pieces() {
        let mut ui = UiRoot::new();
        ui.set_size(800.0, 600.0);
        ui.set_quick_move_key(Some(Modifiers::CTRL));
        let root = ui.root();
        let mut e = Element::at(100.0, 100.0, 100.0, 40.0);
        e.quick_move = true;
        ui.add(root, e, Panel::default());
        ui.add(
            root,
            Element::at(300.0, 300.0, 50.0, 50.0),
            Panel::default(),
        );
        QuickMoveOutlines::mount(&mut ui, Horizon::BRONZE, None);
        assert_eq!(overlay_fills(&mut ui), 0);
        ui.set_modifiers(Modifiers::CTRL);
        // 100 across: 10 dashes top and bottom; 40 down: 4 each side.
        assert_eq!(overlay_fills(&mut ui), 28);
    }
}
