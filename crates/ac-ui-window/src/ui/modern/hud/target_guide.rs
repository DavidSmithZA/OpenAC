//! While an item waits for a target (a lockpick, a key, a kit, a salvage
//! tool), the play windows step aside: they fade and let the pointer
//! through to the world behind them (the toolkit's target see-through).
//! This guide is drawn over everything meanwhile: a dashed edge where each
//! faded window stands, a pill above the bars naming the item with a
//! button that cancels, and "Use <item> on <target>" beside the pointer
//! while it is over something the item can be used on.

use std::sync::Arc;

use ac_ui::draw::RenderContext;
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::quick_move::dashed_rect;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::modern::tone;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

/// The pill's height.
pub const PILL_HEIGHT: f32 = 38.0;

/// The room between the pill and what it sits above.
pub const PILL_GAP: f32 = 12.0;

/// How far above the screen's bottom the pill ends when nothing else
/// places it.
pub const PILL_ABOVE_BOTTOM: f32 = 196.0;

/// The cancel button's height.
const BUTTON_HEIGHT: f32 = 24.0;

/// The label's room right of the pointer: half the target cursor and a
/// gap.
const LABEL_FROM_POINTER: f32 = 24.0;

/// The fonts the guide draws with.
#[derive(Debug, Clone, Default)]
pub struct GuideFonts {
    /// The item's name in the pill.
    pub name: Option<Arc<UiFont>>,
    /// The rest of the pill's words.
    pub body: Option<Arc<UiFont>>,
    /// The cancel button and the label at the pointer.
    pub small: Option<Arc<UiFont>>,
}

/// What the guide shows this frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GuideView {
    /// The item waiting for a target.
    pub item: String,
    /// The faded windows' rectangles, screen units.
    pub faded: Vec<(f32, f32, f32, f32)>,
    /// The bottom of the pill, screen units.
    pub pill_bottom: f32,
    /// What the pointer is over and the item can be used on, and where the
    /// pointer is.
    pub over: Option<(String, (f32, f32))>,
}

/// The guide: the whole screen, taking the pointer only over its cancel
/// button.
#[derive(Debug, Default)]
pub struct TargetGuide {
    fonts: GuideFonts,
    /// What it shows.
    pub view: GuideView,
    /// The cancel button, as last drawn (screen units).
    button: Option<(f32, f32, f32, f32)>,
    hovered: bool,
    pressed: bool,
    /// The cancel button was clicked since the window last looked.
    pub cancelled: bool,
}

impl TargetGuide {
    /// The guide, hidden, made a child of the root.
    pub fn mount(ui: &mut UiRoot, fonts: GuideFonts) -> ElementId {
        let (w, h) = ui.size();
        let mut element = Element::at(0.0, 0.0, w, h).with_visible(false);
        element.name = Some("modern-target-guide".to_owned());
        let root = ui.root();
        ui.add(
            root,
            element,
            Self {
                fonts,
                ..Self::default()
            },
        )
    }

    fn over_button(&self, x: f32, y: f32) -> bool {
        self.button
            .is_some_and(|(bx, by, bw, bh)| x >= bx && x < bx + bw && y >= by && y < by + bh)
    }

    /// The pill's words: the item's name, then what to do.
    const ASK: &'static str = ": pick what to use it on";

    /// The cancel button's words.
    const CANCEL: &'static str = "Esc cancels";

    fn draw_pill(&mut self, rc: &mut RenderContext<'_>, screen_w: f32) {
        let (Some(name), Some(body), Some(small)) =
            (&self.fonts.name, &self.fonts.body, &self.fonts.small)
        else {
            self.button = None;
            return;
        };
        let name_w = text::measure(name, &self.view.item, 0.0);
        let ask_w = text::measure(body, Self::ASK, 0.0);
        let button_w = text::measure(small, Self::CANCEL, 0.0) + 20.0;
        let w = (14.0 + name_w + ask_w + 10.0 + button_w + 8.0).min(screen_w - 16.0);
        let x = ((screen_w - w) * 0.5).round();
        let y = (self.view.pill_bottom - PILL_HEIGHT).round();
        let round = CornerRadii::all(PILL_HEIGHT * 0.5);
        rc.fill_rounded_rect(x, y + 4.0, w, PILL_HEIGHT, round, hex(0, 0.3));
        rc.fill_rounded_rect(x, y, w, PILL_HEIGHT, round, tone::deep(0.9));
        rc.stroke_rounded_rect(x, y, w, PILL_HEIGHT, round, tone::gold(1.0), 1.0);
        let ty = y + ((PILL_HEIGHT - body.line_height) * 0.5).round();
        text::draw(
            rc,
            name,
            &self.view.item,
            x + 14.0,
            ty,
            TextStyle::plain(tone::words(1.0)),
        );
        text::draw(
            rc,
            body,
            Self::ASK,
            x + 14.0 + name_w,
            ty,
            TextStyle::plain(tone::words(1.0)),
        );
        let b = (
            x + w - 8.0 - button_w,
            y + ((PILL_HEIGHT - BUTTON_HEIGHT) * 0.5).round(),
            button_w,
            BUTTON_HEIGHT,
        );
        let lit = self.hovered || self.pressed;
        let face = if self.pressed {
            tone::gold(0.3)
        } else if self.hovered {
            tone::gold(0.15)
        } else {
            hex(0, 0.4)
        };
        let round = CornerRadii::all(BUTTON_HEIGHT * 0.5);
        rc.fill_rounded_rect(b.0, b.1, b.2, b.3, round, face);
        rc.stroke_rounded_rect(
            b.0,
            b.1,
            b.2,
            b.3,
            round,
            tone::bronze(if lit { 0.9 } else { 0.6 }),
            1.0,
        );
        text::draw(
            rc,
            small,
            Self::CANCEL,
            b.0 + 10.0,
            b.1 + ((BUTTON_HEIGHT - small.line_height) * 0.5).round(),
            TextStyle::plain(tone::soft(1.0)),
        );
        self.button = Some(b);
    }

    fn draw_label(&self, rc: &mut RenderContext<'_>, screen: (f32, f32)) {
        let (Some((target, (px, py))), Some(small)) = (&self.view.over, &self.fonts.small) else {
            return;
        };
        let words = format!("Use {} on {target}", self.view.item);
        let w = text::measure(small, &words, 0.0) + 20.0;
        let h = small.line_height + 10.0;
        // Right of the pointer, or left of it when the screen runs out.
        let mut x = px + LABEL_FROM_POINTER;
        if x + w > screen.0 - 4.0 {
            x = px - LABEL_FROM_POINTER - w;
        }
        let x = x.max(4.0).round();
        let y = (py - h * 0.5)
            .clamp(4.0, (screen.1 - h - 4.0).max(4.0))
            .round();
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(x, y, w, h, round, tone::deep(0.92));
        rc.stroke_rounded_rect(x, y, w, h, round, tone::bronze(0.8), 1.0);
        text::draw(
            rc,
            small,
            &words,
            x + 10.0,
            y + 5.0,
            TextStyle::plain(tone::words(1.0)),
        );
    }
}

impl Widget for TargetGuide {
    fn hit_test(&self, _element: &Element, x: f32, y: f32) -> bool {
        self.over_button(x, y)
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = true;
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = false;
                true
            }
            kind::MOUSE_DOWN => {
                self.pressed = true;
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.pressed = false;
                true
            }
            kind::CLICK => {
                self.cancelled = true;
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let screen = ui.size();
        if let Some(e) = ui.element_mut(me) {
            e.width = screen.0;
            e.height = screen.1;
        }
        for &(x, y, w, h) in &self.view.faded {
            dashed_rect(rc, x, y, w, h, tone::gold(0.55));
        }
        self.draw_pill(rc, screen.0);
        self.draw_label(rc, screen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_takes_the_pointer_only_over_its_button() {
        let mut guide = TargetGuide::default();
        let e = Element::at(0.0, 0.0, 800.0, 600.0);
        assert!(!guide.hit_test(&e, 400.0, 300.0));
        guide.button = Some((100.0, 100.0, 80.0, 24.0));
        assert!(guide.hit_test(&e, 120.0, 110.0));
        assert!(!guide.hit_test(&e, 400.0, 300.0));
    }
}
