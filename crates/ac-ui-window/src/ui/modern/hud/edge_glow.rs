//! The screen's edge glows red, pulsing, while health is low (an OpenAC
//! addition, on by default): a warning seen without looking at the bars.

use std::f32::consts::TAU;

use ac_ui::UiRoot;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};

/// How far in from the edge the glow reaches, units.
pub const DEPTH: f32 = 120.0;

/// One pulse, seconds.
const PULSE_SECONDS: f32 = 1.1;

/// The glow at its brightest.
const GLOW: Color = hex(0xC8_1E28, 0.55);

/// The glow, over the whole screen, under every other piece, taking no
/// input.
#[derive(Debug, Default)]
pub struct EdgeGlow {
    /// Glowing now.
    pub on: bool,
    clock: f32,
}

impl EdgeGlow {
    /// The glow, made a child of the root under everything else.
    pub fn mount(ui: &mut UiRoot) -> ElementId {
        let (w, h) = ui.size();
        let mut element = Element::at(0.0, 0.0, w, h);
        element.click_through = true;
        element.name = Some("modern-edge-glow".to_owned());
        element.z_order = i32::MIN / 2;
        let root = ui.root();
        ui.add(root, element, Self::default())
    }
}

impl Widget for EdgeGlow {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        false
    }

    fn tick(&mut self, _ui: &mut UiRoot, _me: ElementId, dt: f64) {
        self.clock = if self.on {
            (self.clock + dt as f32) % PULSE_SECONDS
        } else {
            0.0
        };
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let (w, h) = ui.size();
        if let Some(e) = ui.element_mut(me) {
            e.width = w;
            e.height = h;
        }
        if !self.on {
            return;
        }
        // Between a third and full strength.
        let pulse = 0.35 + 0.65 * (0.5 - 0.5 * (self.clock / PULSE_SECONDS * TAU).cos());
        let edge = Color {
            a: GLOW.a * pulse,
            ..GLOW
        };
        let clear = Color { a: 0.0, ..GLOW };
        let d = DEPTH.min(w * 0.5).min(h * 0.5);
        let square = CornerRadii::default();
        rc.fill_vertical_gradient(0.0, 0.0, w, d, square, edge, clear);
        rc.fill_vertical_gradient(0.0, h - d, w, d, square, clear, edge);
        rc.fill_horizontal_gradient(0.0, 0.0, d, h, square, edge, clear);
        rc.fill_horizontal_gradient(w - d, 0.0, d, h, square, clear, edge);
    }
}
