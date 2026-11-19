//! The radar, top right: the bearing ring. Heading up, as retail: the
//! player always faces the top, and a compass ring with ticks turns round
//! them, N in bronze, with the heading in a pill above it ("NE 42°").
//! Retail's blip shapes and colours, drawn larger; faint rings at a third
//! and two thirds of the range; a soft cone for the view. Hovering a blip
//! names it with its distance, clicking selects it. A chip on the rim
//! shows the range and, outdoors, steps it in (75, 40, 20 m); indoors the
//! range is the game's 25 m. Under the disc, the coordinates while the
//! "show coordinates on the radar" option is on.

use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use ac_ui::draw::{Color, ColorVertex, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::panels::radar::{RadarScene, build_snapshot_at};
use ac_ui::radar::{
    BlipShape, INDOOR_RANGE_METERS, OUTDOOR_RANGE_METERS, RadarBlip, RadarCoordinates,
};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};
use glam::Vec2;

/// The piece's width and height, units.
pub const WIDTH: f32 = 214.0;
pub const HEIGHT: f32 = 252.0;
/// The status line's band at the top (the connection line).
pub const STATUS_HEIGHT: f32 = 16.0;
/// The heading pill.
const PILL_TOP: f32 = STATUS_HEIGHT + 6.0;
const PILL_HEIGHT: f32 = 20.0;
/// The disc: its box's left and top, and its outer and inner radii.
const DISC_LEFT: f32 = 14.0;
const DISC_TOP: f32 = PILL_TOP + PILL_HEIGHT + 6.0;
const OUTER: f32 = 91.0;
const INNER: f32 = 77.0;
/// Where the compass letters sit, from the centre.
const LETTERS: f32 = 84.0;
/// The coordinates' line.
const COORDS_TOP: f32 = DISC_TOP + 2.0 * (OUTER + 1.0) + 6.0;
/// The range chip.
const CHIP_HEIGHT: f32 = 22.0;
/// How near the pointer must be to a blip to name it, units.
const HOVER_REACH: f32 = 7.0;
/// Half the view cone, radians.
const CONE: f32 = PI / 6.0;
/// The outdoor ranges the chip steps through, meters.
pub const OUTDOOR_RANGES: [f32; 3] = [OUTDOOR_RANGE_METERS, 40.0, 20.0];

/// The disc's centre in the piece.
#[must_use]
pub fn centre() -> Vec2 {
    Vec2::new(DISC_LEFT + OUTER + 1.0, DISC_TOP + OUTER + 1.0)
}

/// The compass word for a heading in degrees (north 0, east 90).
#[must_use]
pub fn compass_word(heading: f32) -> &'static str {
    const WORDS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let index = ((heading.rem_euclid(360.0) + 22.5) / 45.0) as usize % 8;
    WORDS[index]
}

/// The ring distances (a third and two thirds of `range`), whole meters.
#[must_use]
pub fn rings(range: f32) -> [f32; 2] {
    [(range / 3.0).round(), (range * 2.0 / 3.0).round()]
}

/// The fonts the radar uses.
#[derive(Debug, Clone, Default)]
pub struct RadarFonts {
    /// The heading pill, the compass letters, the chip.
    pub bold: Option<Arc<UiFont>>,
    /// The ring labels.
    pub tiny: Option<Arc<UiFont>>,
    /// The coordinates and the hovered blip's name.
    pub small: Option<Arc<UiFont>>,
}

/// The radar piece.
#[derive(Debug)]
pub struct RadarRing {
    theme: Horizon,
    fonts: RadarFonts,
    /// The scene, as last fed.
    scene: RadarScene,
    /// The blips at the range shown.
    blips: Vec<RadarBlip>,
    /// The heading, degrees.
    heading: f32,
    /// Outdoors (the range can be stepped).
    outdoors: bool,
    /// Which outdoor range is shown.
    outdoor_range: usize,
    /// The coordinates, when the option shows them.
    coordinates: Option<String>,
    /// The blip under the pointer.
    hovered: Option<u32>,
    /// The pointer is on the range chip.
    on_chip: bool,
    /// Quick move is held.
    quick_move: bool,
    /// The status line above the pill.
    pub status: super::connection::StatusLine,
    /// The connection card should show (its button toggles it).
    pub card_open: bool,
}

impl RadarRing {
    /// An empty radar.
    #[must_use]
    pub fn new(theme: Horizon, fonts: RadarFonts) -> Self {
        Self {
            theme,
            fonts,
            scene: RadarScene::default(),
            blips: Vec::new(),
            heading: 0.0,
            outdoors: true,
            outdoor_range: 0,
            coordinates: None,
            hovered: None,
            on_chip: false,
            quick_move: false,
            status: super::connection::StatusLine::default(),
            card_open: false,
        }
    }

    /// The range shown, meters.
    #[must_use]
    pub fn range(&self) -> f32 {
        if self.outdoors {
            OUTDOOR_RANGES[self.outdoor_range]
        } else {
            INDOOR_RANGE_METERS
        }
    }

    /// Shows `scene`.
    pub fn set_scene(&mut self, scene: &RadarScene) {
        if *scene != self.scene {
            self.scene = scene.clone();
            self.project();
        }
    }

    fn project(&mut self) {
        let cell = self
            .scene
            .player
            .and_then(|p| RadarCoordinates::from_cell(p.cell_id));
        if self.scene.player.is_some() {
            self.outdoors = cell.is_some();
        }
        let range = Some(self.range());
        let snapshot = build_snapshot_at(&self.scene, centre(), INNER as i32, range);
        self.heading = snapshot.player_heading_degrees;
        self.blips = snapshot.blips;
        self.coordinates = cell
            .filter(|_| self.scene.coordinates_on_radar)
            .map(|c| format!("{} {}", c.y_text(), c.x_text()));
        if let Some(id) = self.hovered
            && !self.blips.iter().any(|b| b.object_id == id)
        {
            self.hovered = None;
        }
    }

    /// Steps the outdoor range in (back out from the nearest).
    pub fn step_range(&mut self) {
        if self.outdoors {
            self.outdoor_range = (self.outdoor_range + 1) % OUTDOOR_RANGES.len();
            self.project();
        }
    }

    fn chip_words(&self) -> String {
        format!("{} m", self.range().round() as i32)
    }

    fn chip_rect(&self) -> (f32, f32, f32, f32) {
        let w = self
            .fonts
            .bold
            .as_ref()
            .map_or(40.0, |f| text::measure(f, &self.chip_words(), 0.0) + 16.0);
        let right = DISC_LEFT + 2.0 * (OUTER + 1.0) + 10.0;
        let bottom = DISC_TOP + 2.0 * (OUTER + 1.0) - 2.0;
        (right - w, bottom - CHIP_HEIGHT, w, CHIP_HEIGHT)
    }

    fn on_chip_at(&self, x: f32, y: f32) -> bool {
        let (cx, cy, w, h) = self.chip_rect();
        (cx..cx + w).contains(&x) && (cy..cy + h).contains(&y)
    }

    fn on_disc(x: f32, y: f32) -> bool {
        Vec2::new(x, y).distance(centre()) <= OUTER + 1.0
    }

    fn blip_at(&self, x: f32, y: f32) -> Option<&RadarBlip> {
        let at = Vec2::new(x, y);
        self.blips
            .iter()
            .filter(|b| b.object_id != 0)
            .map(|b| (b, Vec2::new(b.pixel_x, b.pixel_y).distance(at)))
            .filter(|(_, d)| *d <= HOVER_REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(b, _)| b)
    }

    fn hover(&mut self, ui: &mut UiRoot, next: Option<u32>) {
        if self.hovered != next {
            self.hovered = next;
            emit(ui, PanelOutput::HoveredObject(next));
        }
    }

    /// Where a bearing (degrees from north, clockwise) falls on screen, as
    /// an angle for the drawing calls (radians clockwise from the right).
    fn screen_angle(&self, bearing: f32) -> f32 {
        (bearing - self.heading - 90.0).to_radians()
    }

    fn draw_ring(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let c = centre();
        rc.fill_ellipse(
            c.x - OUTER,
            c.y - OUTER,
            2.0 * OUTER,
            2.0 * OUTER,
            hex(0, 0.5),
        );
        rc.stroke_ellipse(
            c.x - OUTER,
            c.y - OUTER,
            2.0 * OUTER,
            2.0 * OUTER,
            hex(0xC9_A062, 0.8),
            1.5,
        );
        rc.fill_ellipse(
            c.x - INNER,
            c.y - INNER,
            2.0 * INNER,
            2.0 * INNER,
            hex(0, 0.22),
        );
        rc.stroke_ellipse(
            c.x - INNER,
            c.y - INNER,
            2.0 * INNER,
            2.0 * INNER,
            p.trim_soft,
            1.0,
        );
        // Ticks every 15 degrees, longer and brighter between the letters.
        for step in 0..24 {
            if step % 6 == 0 {
                continue;
            }
            let major = step % 3 == 0;
            let a = self.screen_angle(step as f32 * 15.0);
            let (from, to) = if major { (85.0, 89.0) } else { (86.8, 88.9) };
            let alpha = if major { 0.75 } else { 0.4 };
            let (s, co) = a.sin_cos();
            rc.smooth_line(
                c.x + co * from,
                c.y + s * from,
                c.x + co * to,
                c.y + s * to,
                Color { a: alpha, ..p.text },
                1.2,
            );
        }
        if let Some(f) = &self.fonts.bold {
            for (bearing, letter) in [(0.0, "N"), (90.0, "E"), (180.0, "S"), (270.0, "W")] {
                let (s, co) = self.screen_angle(bearing).sin_cos();
                let (x, y) = (c.x + co * LETTERS, c.y + s * LETTERS);
                let color = if letter == "N" {
                    p.accent
                } else {
                    Color { a: 0.8, ..p.text }
                };
                let w = text::measure(f, letter, 0.0);
                text::draw(
                    rc,
                    f,
                    letter,
                    (x - w * 0.5).round(),
                    (y - f.line_height * 0.5).round(),
                    TextStyle::plain(color),
                );
            }
        }
        // The rings at a third and two thirds of the range, dashed.
        let range = self.range();
        let faint = hex(0xFF_FFFF, 0.13);
        for meters in rings(range) {
            let r = INNER * meters / range;
            let dashes = (TAU * r / 5.0).floor().max(8.0);
            let step = TAU / dashes;
            for i in 0..dashes as i32 {
                rc.stroke_arc(c.x, c.y, r, i as f32 * step, step * 0.4, faint, 1.0);
            }
            if let Some(f) = &self.fonts.tiny {
                text::draw(
                    rc,
                    f,
                    &format!("{}", meters as i32),
                    c.x + 3.0,
                    c.y - r - f.line_height + 3.0,
                    TextStyle::plain(hex(0xF4_EBDC, 0.45)),
                );
            }
        }
        // The view cone, straight ahead.
        let fill = hex(0xFF_FFFF, 0.05);
        let mut fan = vec![ColorVertex::new(c.x, c.y, fill)];
        let steps = 12;
        for i in 0..=steps {
            let a = -PI / 2.0 - CONE + 2.0 * CONE * i as f32 / steps as f32;
            fan.push(ColorVertex::new(
                c.x + a.cos() * INNER,
                c.y + a.sin() * INNER,
                fill,
            ));
        }
        let _ = rc.fill_convex_polygon(&fan);
    }

    fn draw_blip(rc: &mut RenderContext<'_>, blip: &RadarBlip) {
        let (x, y, c) = (blip.pixel_x, blip.pixel_y, blip.color);
        let w = 2.2;
        match blip.shape {
            BlipShape::Undef => {}
            BlipShape::Circle => rc.fill_ellipse(x - 2.5, y - 2.5, 5.0, 5.0, c),
            BlipShape::Plus => {
                rc.smooth_line(x - 4.0, y, x + 4.0, y, c, w);
                rc.smooth_line(x, y - 4.0, x, y + 4.0, c, w);
            }
            BlipShape::X => {
                rc.smooth_line(x - 3.5, y - 3.5, x + 3.5, y + 3.5, c, w);
                rc.smooth_line(x - 3.5, y + 3.5, x + 3.5, y - 3.5, c, w);
            }
            BlipShape::Box => {
                rc.stroke_rounded_rect(x - 4.0, y - 4.0, 8.0, 8.0, CornerRadii::default(), c, 2.0);
            }
            BlipShape::XBox => {
                rc.stroke_rounded_rect(x - 4.0, y - 4.0, 8.0, 8.0, CornerRadii::default(), c, 2.0);
                rc.smooth_line(x - 4.0, y - 4.0, x + 4.0, y + 4.0, c, 1.6);
                rc.smooth_line(x - 4.0, y + 4.0, x + 4.0, y - 4.0, c, 1.6);
            }
            BlipShape::Triangle | BlipShape::InvertedTriangle => {
                let up = blip.shape == BlipShape::Triangle;
                let (tip, base) = if up { (-4.5, 4.5) } else { (4.5, -4.5) };
                let corners = [
                    ColorVertex::new(x, y + tip, c),
                    ColorVertex::new(x + 5.0, y + base, c),
                    ColorVertex::new(x - 5.0, y + base, c),
                ];
                let _ = rc.fill_convex_polygon(&corners);
            }
        }
    }

    fn draw_player(rc: &mut RenderContext<'_>) {
        let c = centre();
        let white = Color::WHITE;
        // An arrow pointing up, notched at the back.
        let left = [
            ColorVertex::new(c.x, c.y - 9.0, white),
            ColorVertex::new(c.x, c.y + 3.0, white),
            ColorVertex::new(c.x - 5.6, c.y + 7.0, white),
        ];
        let right = [
            ColorVertex::new(c.x, c.y - 9.0, white),
            ColorVertex::new(c.x + 5.6, c.y + 7.0, white),
            ColorVertex::new(c.x, c.y + 3.0, white),
        ];
        let edge = hex(0, 0.6);
        let outline = [
            (c.x, c.y - 9.0),
            (c.x + 5.6, c.y + 7.0),
            (c.x, c.y + 3.0),
            (c.x - 5.6, c.y + 7.0),
            (c.x, c.y - 9.0),
        ];
        for pair in outline.windows(2) {
            rc.smooth_line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, edge, 2.0);
        }
        let _ = rc.fill_convex_polygon(&left);
        let _ = rc.fill_convex_polygon(&right);
    }

    fn draw_pill(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let Some(f) = &self.fonts.bold else {
            return;
        };
        let word = compass_word(self.heading);
        let degrees = format!("{}\u{b0}", self.heading.round() as i32 % 360);
        let (ww, dw) = (text::measure(f, word, 0.0), text::measure(f, &degrees, 0.0));
        let w = 10.0 + ww + 6.0 + dw + 10.0;
        let x = (centre().x - w * 0.5).round();
        let radius = CornerRadii::all(PILL_HEIGHT * 0.5);
        rc.fill_rounded_rect(x, PILL_TOP, w, PILL_HEIGHT, radius, hex(0, 0.55));
        rc.stroke_rounded_rect(
            x,
            PILL_TOP,
            w,
            PILL_HEIGHT,
            radius,
            hex(0xC9_A062, 0.8),
            1.0,
        );
        let ty = (PILL_TOP + (PILL_HEIGHT - f.line_height) * 0.5).round();
        text::draw(rc, f, word, x + 10.0, ty, TextStyle::plain(p.accent));
        text::draw(
            rc,
            f,
            &degrees,
            x + 10.0 + ww + 6.0,
            ty,
            TextStyle::plain(p.text),
        );
        // The pointer from the pill to the ring.
        let cx = centre().x;
        let top = PILL_TOP + PILL_HEIGHT + 2.0;
        let corners = [
            ColorVertex::new(cx - 5.0, top, p.accent),
            ColorVertex::new(cx + 5.0, top, p.accent),
            ColorVertex::new(cx, top + 6.0, p.accent),
        ];
        let _ = rc.fill_convex_polygon(&corners);
    }

    fn draw_chip(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let Some(f) = &self.fonts.bold else {
            return;
        };
        let (x, y, w, h) = self.chip_rect();
        let radius = CornerRadii::all(h * 0.5);
        let fill = if self.on_chip && self.outdoors {
            hex(0x2A_2116, 0.85)
        } else {
            hex(0, 0.62)
        };
        rc.fill_rounded_rect(x, y, w, h, radius, fill);
        let edge = if self.on_chip && self.outdoors {
            p.accent
        } else {
            hex(0xC9_A062, 0.8)
        };
        rc.stroke_rounded_rect(x, y, w, h, radius, edge, 1.0);
        let words = self.chip_words();
        let ty = (y + (h - f.line_height) * 0.5).round();
        text::draw(rc, f, &words, x + 8.0, ty, TextStyle::plain(p.text));
    }

    fn draw_hovered(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let (Some(id), Some(f)) = (self.hovered, &self.fonts.small) else {
            return;
        };
        let Some(blip) = self.blips.iter().find(|b| b.object_id == id) else {
            return;
        };
        let meters =
            Vec2::new(blip.pixel_x, blip.pixel_y).distance(centre()) / INNER * self.range();
        let mut rest = format!(" \u{b7} {} m", meters.round() as i32);
        if blip.selected {
            rest.push_str(" \u{b7} selected");
        }
        let (nw, rw) = (
            text::measure(f, &blip.name, 0.0),
            text::measure(f, &rest, 0.0),
        );
        let (w, h) = (8.0 + nw + rw + 8.0, f.line_height + 8.0);
        let screen = rc.screen_size();
        let (ox, _) = rc.origin();
        // Beside the blip, kept on the screen.
        let mut x = blip.pixel_x + 10.0;
        if ox + x + w > screen.0 {
            x = blip.pixel_x - 10.0 - w;
        }
        let y = blip.pixel_y - h - 4.0;
        rc.soft_shadow(x, y, w, h, 4.0, 6.0, 16.0, hex(0, 0.5));
        rc.fill_rounded_rect(x, y, w, h, CornerRadii::all(4.0), hex(0x14_100C, 0.94));
        rc.stroke_rounded_rect(x, y, w, h, CornerRadii::all(4.0), hex(0xC9_A062, 0.8), 1.0);
        text::draw(
            rc,
            f,
            &blip.name,
            x + 8.0,
            y + 4.0,
            TextStyle::plain(p.text),
        );
        text::draw(
            rc,
            f,
            &rest,
            x + 8.0 + nw,
            y + 4.0,
            TextStyle::plain(p.text_soft),
        );
    }
}

impl Widget for RadarRing {
    fn hit_test(&self, _element: &Element, x: f32, y: f32) -> bool {
        self.quick_move
            || Self::on_disc(x, y)
            || self.on_chip_at(x, y)
            || self.status.on_button(x, y)
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            if e.kind == kind::HOVER_LEAVE {
                self.on_chip = false;
                self.hover(ui, None);
            }
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.on_chip = self.on_chip_at(x, y);
                let next = if self.on_chip {
                    None
                } else {
                    self.blip_at(x, y).map(|b| b.object_id)
                };
                self.hover(ui, next);
                true
            }
            kind::HOVER_LEAVE => {
                self.on_chip = false;
                self.hover(ui, None);
                true
            }
            kind::CLICK => {
                if self.status.on_button(x, y) {
                    self.card_open = !self.card_open;
                } else if self.on_chip_at(x, y) {
                    self.step_range();
                } else if let Some(id) = self.blip_at(x, y).map(|b| b.object_id) {
                    emit(ui, PanelOutput::SelectObject(id));
                }
                true
            }
            kind::MOUSE_DOWN | kind::MOUSE_UP => true,
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, me: ElementId, _dt: f64) {
        self.quick_move = ui.quick_move_held();
        // Blips move under a resting pointer: look again every frame.
        if ui.hovered() == Some(me) {
            let (mx, my) = ui.mouse();
            let (sx, sy) = ui.screen_position(me);
            let (x, y) = (mx as f32 - sx, my as f32 - sy);
            let next = if self.on_chip_at(x, y) {
                None
            } else {
                self.blip_at(x, y).map(|b| b.object_id)
            };
            self.hover(ui, next);
        }
    }

    fn clips_children(&self) -> bool {
        false
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        if let Some(f) = self.fonts.small.clone() {
            self.status.draw(rc, &self.theme, &f, WIDTH);
        }
        self.draw_pill(rc);
        self.draw_ring(rc);
        for blip in &self.blips {
            Self::draw_blip(rc, blip);
            if blip.selected {
                rc.stroke_ellipse(
                    blip.pixel_x - 9.0,
                    blip.pixel_y - 9.0,
                    18.0,
                    18.0,
                    p.accent,
                    1.5,
                );
            }
        }
        Self::draw_player(rc);
        self.draw_chip(rc);
        if let (Some(words), Some(f)) = (&self.coordinates, &self.fonts.small) {
            let w = text::measure(f, words, 0.0);
            text::draw(
                rc,
                f,
                words,
                (centre().x - w * 0.5).round(),
                COORDS_TOP,
                TextStyle::plain(p.text).shadowed(p.text_shadow),
            );
        }
        self.draw_hovered(rc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_heading_is_named_by_the_nearest_of_eight_points() {
        assert_eq!(compass_word(0.0), "N");
        assert_eq!(compass_word(42.0), "NE");
        assert_eq!(compass_word(359.0), "N");
        assert_eq!(compass_word(200.0), "S");
        assert_eq!(compass_word(-90.0), "W");
    }

    #[test]
    fn the_rings_sit_at_thirds_of_the_range() {
        assert_eq!(rings(75.0), [25.0, 50.0]);
        assert_eq!(rings(25.0), [8.0, 17.0]);
    }

    #[test]
    fn the_chip_steps_the_range_in_outdoors_only() {
        let mut radar = RadarRing::new(Horizon::BRONZE, RadarFonts::default());
        assert_eq!(radar.range(), 75.0);
        radar.step_range();
        assert_eq!(radar.range(), 40.0);
        radar.step_range();
        radar.step_range();
        assert_eq!(radar.range(), 75.0);
        radar.outdoors = false;
        radar.step_range();
        assert_eq!(radar.range(), INDOOR_RANGE_METERS);
    }

    #[test]
    fn north_is_at_the_top_when_facing_north() {
        let radar = RadarRing::new(Horizon::BRONZE, RadarFonts::default());
        let a = radar.screen_angle(0.0);
        assert!((a - (-PI / 2.0)).abs() < 1e-5);
    }
}
