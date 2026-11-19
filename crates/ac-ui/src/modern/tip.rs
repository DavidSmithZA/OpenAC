//! Hover labels for the modern pieces. The retail tooltip is built from a
//! dat layout the element names; a piece made in code names none, so its
//! [`Widget::tooltip_text`] would never show. This layer shows it instead:
//! when the pointer rests on an element inside a piece (an element marked
//! [`Element::quick_move`], or anything in one) for a moment, the element's
//! text shows in a small Horizon label beside the pointer, wrapped, kept on
//! the screen, and follows the text as it changes (a strip whose parts
//! each have their own words).

use std::sync::Arc;

use crate::draw::RenderContext;
use crate::font::UiFont;
use crate::root::UiRoot;
use crate::shapes::CornerRadii;
use crate::text::wrap_words;
use crate::tree::{Element, ElementId, Widget};

use super::theme::Horizon;

/// How long the pointer rests before the label shows, seconds.
pub const DELAY_SECONDS: f32 = 0.5;
/// The widest a label grows before its words wrap, units.
pub const MAX_WIDTH: f32 = 280.0;
/// The label's inside margins, across and down.
const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 5.0;
/// How far from the pointer the label sits.
const OFFSET: f32 = 18.0;

/// The labels' layer: a click-through element over the whole screen.
#[derive(Debug)]
pub struct TipLayer {
    theme: Horizon,
    font: Option<Arc<UiFont>>,
    /// The element the pointer rests on and its text.
    over: Option<(ElementId, String)>,
    /// How long it has rested there.
    rested: f32,
}

impl TipLayer {
    /// The layer, made a child of the root; its words in `font`.
    pub fn mount(ui: &mut UiRoot, theme: Horizon, font: Option<Arc<UiFont>>) -> ElementId {
        let (w, h) = ui.size();
        let mut element = Element::at(0.0, 0.0, w, h);
        element.click_through = true;
        element.name = Some("modern-tips".to_owned());
        let root = ui.root();
        ui.add(
            root,
            element,
            Self {
                theme,
                font,
                over: None,
                rested: 0.0,
            },
        )
    }

    /// The text showing now, if any.
    #[must_use]
    pub fn showing(&self) -> Option<&str> {
        self.over
            .as_ref()
            .filter(|_| self.rested >= DELAY_SECONDS)
            .map(|(_, t)| t.as_str())
    }

    /// The text of what the pointer rests on, when it is in a piece.
    fn hovered_text(ui: &UiRoot) -> Option<(ElementId, String)> {
        let hovered = ui.hovered()?;
        if ui.drag_payload().is_some() || !in_piece(ui, hovered) {
            return None;
        }
        let element = ui.element(hovered)?;
        let text = ui.widget_dyn(hovered)?.tooltip_text(element)?;
        (!text.trim().is_empty()).then_some((hovered, text))
    }
}

/// Whether `id` is a piece or inside one.
fn in_piece(ui: &UiRoot, id: ElementId) -> bool {
    let mut at = Some(id);
    while let Some(c) = at {
        if ui.element(c).is_some_and(|e| e.quick_move) {
            return true;
        }
        at = ui.parent(c);
    }
    false
}

impl Widget for TipLayer {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        false
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, dt: f64) {
        let now = Self::hovered_text(ui);
        match (&mut self.over, now) {
            (Some((id, text)), Some((next, words))) if *id == next => {
                // The same element: its words may change (another part).
                *text = words;
                self.rested += dt as f32;
            }
            (over, now) => {
                *over = now;
                self.rested = 0.0;
            }
        }
    }

    fn draw_overlay(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let (sw, sh) = ui.size();
        if let Some(e) = ui.element_mut(me) {
            e.width = sw;
            e.height = sh;
        }
        let (Some(text), Some(font)) = (self.showing(), self.font.clone()) else {
            return;
        };
        let lines = wrap_words(text, |s| font.measure_width(s), MAX_WIDTH);
        let widest = lines
            .iter()
            .map(|l| font.measure_width(l))
            .fold(0.0_f32, f32::max);
        let (w, h) = (
            (widest + PAD_X * 2.0).ceil(),
            font.line_height * lines.len() as f32 + PAD_Y * 2.0,
        );
        let (mx, my) = ui.mouse();
        let (mx, my) = (mx as f32, my as f32);
        // Below and right of the pointer; above or left where the screen
        // ends.
        let x = if mx + OFFSET + w <= sw {
            mx + OFFSET * 0.5
        } else {
            (mx - w - 4.0).max(0.0)
        };
        let y = if my + OFFSET + h <= sh {
            my + OFFSET
        } else {
            (my - h - 6.0).max(0.0)
        };
        let (x, y) = (x.round(), y.round());
        let p = self.theme.palette;
        let round = CornerRadii::all(self.theme.metrics.radius);
        rc.soft_shadow(x, y, w, h, 3.0, 4.0, 10.0, p.shadow);
        rc.fill_rounded_rect(x, y, w, h, round, p.popup);
        rc.stroke_rounded_rect(x, y, w, h, round, p.trim_soft, 1.0);
        for (i, line) in lines.iter().enumerate() {
            let ty = y + PAD_Y + font.line_height * i as f32;
            rc.string_dat(&font, line, x + PAD_X, ty, p.text, false, p.text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::Panel;

    #[derive(Debug)]
    struct Says(&'static str);

    impl Widget for Says {
        fn tooltip_text(&self, _element: &Element) -> Option<String> {
            Some(self.0.to_owned())
        }
    }

    fn layer(ui: &UiRoot, id: ElementId) -> &TipLayer {
        ui.widget::<TipLayer>(id).unwrap()
    }

    #[test]
    fn a_piece_shows_its_words_after_a_rest_and_nothing_else_does() {
        let mut ui = UiRoot::new();
        ui.set_size(800.0, 600.0);
        let root = ui.root();
        let mut piece = Element::at(100.0, 100.0, 100.0, 40.0);
        piece.quick_move = true;
        let piece = ui.add(root, piece, Panel::default());
        ui.add(
            piece,
            Element::at(0.0, 0.0, 40.0, 40.0),
            Says("Inventory (F12)"),
        );
        ui.add(root, Element::at(400.0, 400.0, 50.0, 50.0), Says("retail"));
        let tips = TipLayer::mount(&mut ui, Horizon::BRONZE, None);

        ui.on_mouse_move(110, 110);
        ui.tick(0.2, 200);
        assert_eq!(layer(&ui, tips).showing(), None);
        ui.tick(0.3, 500);
        assert_eq!(layer(&ui, tips).showing(), None, "not yet rested");
        ui.tick(0.3, 800);
        assert_eq!(layer(&ui, tips).showing(), Some("Inventory (F12)"));

        // Off the piece, onto an element with a retail tooltip: none here.
        ui.on_mouse_move(410, 410);
        ui.tick(1.0, 1600);
        ui.tick(1.0, 2600);
        assert_eq!(layer(&ui, tips).showing(), None);
    }
}
