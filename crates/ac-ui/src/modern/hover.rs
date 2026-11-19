//! A hover pane: a small Horizon pane that opens beside a piece the pointer
//! rests on and holds whatever widgets its owner puts in it (the list of
//! every effect on you under the HUD's counts, say). It opens after a short
//! rest, moves at once to another of its pieces, stays open while the
//! pointer is over it (so it can be scrolled or clicked), and closes a
//! moment after the pointer has left both.

use crate::draw::RenderContext;
use crate::root::UiRoot;
use crate::shapes::CornerRadii;
use crate::tree::{Element, ElementId, Widget};

use super::theme::Horizon;

/// How long the pointer rests on a piece before its pane opens, seconds.
pub const OPEN_DELAY: f32 = 0.25;

/// How long the pane stays after the pointer has left it and its piece,
/// seconds: long enough to cross a gap between them.
pub const CLOSE_GRACE: f32 = 0.2;

/// The gap between a piece and its pane, units.
pub const GAP: f32 = 6.0;

/// The pane's frame: the popup fill, a soft bronze edge and a shadow.
#[derive(Debug, Clone, Copy)]
pub struct HoverFrame {
    /// The look.
    pub theme: Horizon,
}

impl Widget for HoverFrame {
    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        let p = self.theme.palette;
        let radius = self.theme.metrics.window_radius;
        rc.soft_shadow(0.0, 0.0, w, h, radius, 12.0, 30.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, w, h, CornerRadii::all(radius), p.popup);
        rc.stroke_rounded_rect(
            0.0,
            0.0,
            w,
            h,
            CornerRadii::all(radius),
            p.trim_soft,
            self.theme.metrics.edge,
        );
    }
}

/// A hover pane and where it stands.
#[derive(Debug)]
pub struct HoverPane {
    pane: ElementId,
    /// The piece it is open for.
    open_for: Option<ElementId>,
    /// The piece the pointer is resting on, and for how long.
    resting: Option<(ElementId, f32)>,
    /// How long the pointer has been off both pane and piece.
    away: f32,
}

impl HoverPane {
    /// A pane `width` by `height`, hidden, made a child of the root. Put
    /// its content in [`HoverPane::element`].
    pub fn mount(ui: &mut UiRoot, theme: Horizon, width: f32, height: f32) -> Self {
        let mut element = Element::at(0.0, 0.0, width, height).with_visible(false);
        element.name = Some("hover-pane".to_owned());
        let root = ui.root();
        let pane = ui.add(root, element, HoverFrame { theme });
        Self {
            pane,
            open_for: None,
            resting: None,
            away: 0.0,
        }
    }

    /// The pane's element: its content goes in it as children.
    #[must_use]
    pub fn element(&self) -> ElementId {
        self.pane
    }

    /// The piece the pane is open for.
    #[must_use]
    pub fn open_for(&self) -> Option<ElementId> {
        self.open_for
    }

    /// One frame of `seconds`: opens, moves or closes the pane for
    /// whichever of `pieces` the pointer is on. Answers when the pane
    /// opened for a piece or moved to another, so the owner fills it.
    pub fn tick(
        &mut self,
        ui: &mut UiRoot,
        pieces: &[ElementId],
        seconds: f32,
    ) -> Option<ElementId> {
        let hovered = ui.hovered();
        let within = |id: ElementId| hovered.is_some_and(|h| is_within(ui, h, id));
        let on_piece = pieces.iter().copied().find(|&p| within(p));
        let on_pane = self.open_for.is_some() && within(self.pane);
        match (on_piece, self.open_for) {
            (Some(piece), Some(open)) if piece == open => {
                self.away = 0.0;
                None
            }
            // Open for another piece: move straight to this one.
            (Some(piece), Some(_)) => {
                self.away = 0.0;
                self.open(ui, piece);
                Some(piece)
            }
            (Some(piece), None) => {
                let rested = match self.resting {
                    Some((p, t)) if p == piece => t + seconds,
                    _ => seconds,
                };
                self.resting = Some((piece, rested));
                if rested >= OPEN_DELAY {
                    self.open(ui, piece);
                    Some(piece)
                } else {
                    None
                }
            }
            (None, Some(_)) if on_pane => {
                self.away = 0.0;
                None
            }
            (None, Some(_)) => {
                self.away += seconds;
                if self.away >= CLOSE_GRACE {
                    self.close(ui);
                }
                None
            }
            (None, None) => {
                self.resting = None;
                None
            }
        }
    }

    /// Closes the pane at once.
    pub fn close(&mut self, ui: &mut UiRoot) {
        self.open_for = None;
        self.resting = None;
        self.away = 0.0;
        ui.set_visible(self.pane, false);
    }

    fn open(&mut self, ui: &mut UiRoot, piece: ElementId) {
        self.open_for = Some(piece);
        self.resting = None;
        self.away = 0.0;
        let (x, y) = ui.screen_position(piece);
        let (pw, ph) = ui
            .element(piece)
            .map_or((0.0, 0.0), |e| (e.width, e.height));
        let (w, h) = ui
            .element(self.pane)
            .map_or((0.0, 0.0), |e| (e.width, e.height));
        let screen = ui.size();
        let (left, top) = place_beside((x, y, pw, ph), (w, h), screen);
        if let Some(e) = ui.element_mut(self.pane) {
            e.left = left;
            e.top = top;
        }
        ui.set_visible(self.pane, true);
        ui.bring_to_front(self.pane);
    }
}

/// Where a pane `size` goes for a piece at `piece` (left, top, width,
/// height) on a screen of `screen`: under it, its left edges lined up; above
/// it when there is no room below; kept on the screen.
#[must_use]
pub fn place_beside(
    (x, y, _w, h): (f32, f32, f32, f32),
    (pane_w, pane_h): (f32, f32),
    (screen_w, screen_h): (f32, f32),
) -> (f32, f32) {
    let below = y + h + GAP;
    let top = if below + pane_h <= screen_h {
        below
    } else {
        (y - GAP - pane_h).max(0.0)
    };
    let left = x.min(screen_w - pane_w).max(0.0);
    (left, top)
}

/// Whether `id` is `ancestor` or inside it.
fn is_within(ui: &UiRoot, mut id: ElementId, ancestor: ElementId) -> bool {
    loop {
        if id == ancestor {
            return true;
        }
        match ui.parent(id) {
            Some(parent) => id = parent,
            None => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::Panel;

    fn setup() -> (UiRoot, HoverPane, ElementId, ElementId) {
        let mut ui = UiRoot::new();
        ui.set_size(800.0, 600.0);
        let root = ui.root();
        let a = ui.add(root, Element::at(16.0, 14.0, 60.0, 28.0), Panel::default());
        let b = ui.add(root, Element::at(82.0, 14.0, 60.0, 28.0), Panel::default());
        let pane = HoverPane::mount(&mut ui, Horizon::BRONZE, 320.0, 200.0);
        (ui, pane, a, b)
    }

    fn point(ui: &mut UiRoot, x: i32, y: i32) {
        ui.on_mouse_move(x, y);
    }

    #[test]
    fn it_opens_after_a_rest_under_the_piece() {
        let (mut ui, mut pane, a, b) = setup();
        point(&mut ui, 20, 20);
        assert_eq!(pane.tick(&mut ui, &[a, b], 0.1), None);
        assert_eq!(pane.tick(&mut ui, &[a, b], 0.2), Some(a));
        let e = ui.element(pane.element()).unwrap();
        assert!(e.visible());
        assert_eq!((e.left, e.top), (16.0, 14.0 + 28.0 + GAP));
    }

    #[test]
    fn it_moves_at_once_and_stays_while_the_pointer_is_over_it() {
        let (mut ui, mut pane, a, b) = setup();
        point(&mut ui, 20, 20);
        pane.tick(&mut ui, &[a, b], 0.3);
        point(&mut ui, 90, 20);
        assert_eq!(pane.tick(&mut ui, &[a, b], 0.01), Some(b));
        // Into the pane, past the gap.
        point(&mut ui, 100, 60);
        pane.tick(&mut ui, &[a, b], 0.1);
        pane.tick(&mut ui, &[a, b], 0.5);
        assert_eq!(pane.open_for(), Some(b));
        // Off everything: it goes after the grace.
        point(&mut ui, 700, 500);
        pane.tick(&mut ui, &[a, b], 0.1);
        assert_eq!(pane.open_for(), Some(b));
        pane.tick(&mut ui, &[a, b], 0.15);
        assert_eq!(pane.open_for(), None);
        assert!(!ui.element(pane.element()).unwrap().visible());
    }

    #[test]
    fn passing_over_a_piece_does_not_open_it() {
        let (mut ui, mut pane, a, b) = setup();
        point(&mut ui, 20, 20);
        pane.tick(&mut ui, &[a, b], 0.1);
        point(&mut ui, 700, 500);
        pane.tick(&mut ui, &[a, b], 0.1);
        point(&mut ui, 20, 20);
        assert_eq!(pane.tick(&mut ui, &[a, b], 0.2), None);
    }

    #[test]
    fn a_pane_goes_above_a_piece_near_the_bottom_and_stays_on_screen() {
        assert_eq!(
            place_beside((780.0, 560.0, 40.0, 30.0), (320.0, 200.0), (800.0, 600.0)),
            (480.0, 560.0 - GAP - 200.0)
        );
    }
}
