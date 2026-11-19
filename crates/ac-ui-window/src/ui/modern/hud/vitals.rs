//! The vitals, top centre: three slim bars that show what just happened
//! to them. A loss drops the bar at once, flashes the lost part white and
//! leaves a pale ghost that drains after a beat; a gain jumps a bright
//! leading edge to the new value and fills the bar up behind it. A change
//! of at least a set share of the bar also shows its number, falling off
//! the bar. Health under a quarter pulses red; stamina at nothing blinks
//! amber; a spell the mana cannot pay for shakes the mana bar and marks
//! what it needs.

use std::f32::consts::TAU;
use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::modern::numbers::compact;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::modern::{Ease, Horizon, Tween};
use ac_ui::panels::vitals::{VitalView, VitalsView};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

/// One bar's width, units.
pub const BAR_WIDTH: f32 = 230.0;
/// One bar's height, units.
pub const BAR_HEIGHT: f32 = 18.0;
/// The gap between bars, units.
pub const BAR_GAP: f32 = 10.0;
/// The three bars' width.
pub const WIDTH: f32 = BAR_WIDTH * 3.0 + BAR_GAP * 2.0;

/// A change shows its number from this share of the bar (the design's 5%;
/// Options sets it).
pub const DEFAULT_NUMBER_THRESHOLD: f32 = 0.05;

/// Health under this share pulses (the design's 25%; Options sets it).
pub const DEFAULT_LOW_HEALTH: f32 = 0.25;

/// How long the lost part flashes, seconds.
const FLASH_SECONDS: f32 = 0.35;
/// How long a number falls off the bar, seconds.
const NUMBER_SECONDS: f32 = 1.2;
/// How long the glint runs along a filling bar, seconds.
const SHEEN_SECONDS: f32 = 0.9;
/// How long a mana bar shakes, seconds.
const SHAKE_SECONDS: f32 = 0.4;
/// How long the mana a spell needs stays marked, seconds.
const NEED_SECONDS: f32 = 2.5;
/// One low-health pulse, seconds.
const LOW_PULSE_SECONDS: f32 = 1.1;
/// One out-of-stamina blink, seconds.
const TIRED_BLINK_SECONDS: f32 = 0.8;

/// Which vital a bar shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vital {
    /// Health.
    Health,
    /// Stamina.
    Stamina,
    /// Mana.
    Mana,
}

impl Vital {
    /// The word on the bar.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Health => "HEALTH",
            Self::Stamina => "STAMINA",
            Self::Mana => "MANA",
        }
    }

    /// The bar's colour.
    fn color(self, theme: &Horizon) -> Color {
        match self {
            Self::Health => theme.palette.health,
            Self::Stamina => theme.palette.stamina,
            Self::Mana => theme.palette.mana,
        }
    }

    /// The pale ghost a loss leaves.
    fn ghost(self) -> Color {
        match self {
            Self::Health => hex(0xFF_BEAA, 0.7),
            Self::Stamina => hex(0xFF_E196, 0.7),
            Self::Mana => hex(0xAA_C8FF, 0.7),
        }
    }

    /// The bright leading edge of a gain.
    fn lead(self) -> Color {
        match self {
            Self::Health => hex(0xFF_9696, 0.75),
            Self::Stamina => hex(0xFF_D778, 0.75),
            Self::Mana => hex(0x8C_B9FF, 0.8),
        }
    }
}

/// A number falling off a bar.
#[derive(Debug, Clone, PartialEq)]
pub struct FallingNumber {
    /// "−212" or "+85".
    pub text: String,
    /// A loss or a gain.
    pub gain: bool,
    /// Where along the bar, 0..=1.
    pub at: f32,
    /// Seconds since it appeared.
    pub age: f32,
}

/// One bar's value and what it is showing of its last changes.
#[derive(Debug, Clone)]
pub struct BarMotion {
    /// Which vital.
    pub vital: Vital,
    /// The value and maximum last given, when known.
    pub current: Option<u32>,
    /// The maximum last given.
    pub max: Option<u32>,
    /// Whether a value has been given yet.
    seeded: bool,
    /// The bar, 0..=1.
    pub fill: Tween,
    /// The ghost under it (a loss's pale part, a gain's leading edge).
    pub ghost: Tween,
    /// The last change was a gain (the ghost is the leading edge).
    pub gaining: bool,
    /// The lost part flashing: from, to, seconds since.
    pub flash: Option<(f32, f32, f32)>,
    /// The number of a big change.
    pub number: Option<FallingNumber>,
    /// Seconds since a big gain's glint set off.
    pub sheen: Option<f32>,
    /// The mana a refused spell needed, and seconds since.
    pub need: Option<(u32, f32)>,
}

impl BarMotion {
    /// A bar with nothing shown yet.
    #[must_use]
    pub fn new(vital: Vital) -> Self {
        Self {
            vital,
            current: None,
            max: None,
            seeded: false,
            fill: Tween::new(0.0),
            ghost: Tween::new(0.0),
            gaining: false,
            flash: None,
            number: None,
            sheen: None,
            need: None,
        }
    }

    /// Where the bar is going, 0..=1.
    #[must_use]
    pub fn share(&self) -> f32 {
        self.fill.target()
    }

    /// Takes the vital's new value; a change of at least `threshold` of
    /// the bar shows its number. Not `animate`d, the bar moves straight to
    /// it, with no flash, ghost or glint.
    pub fn apply(&mut self, view: VitalView, threshold: f32, animate: bool) {
        let share = match (view.current, view.max, view.percent) {
            (Some(c), Some(m), _) if m > 0 => c as f32 / m as f32,
            (_, _, Some(p)) => p,
            _ => 0.0,
        }
        .clamp(0.0, 1.0);
        let before = (self.share(), self.current);
        self.current = view.current;
        self.max = view.max;
        if !self.seeded {
            self.seeded = true;
            self.fill.jump_to(share);
            self.ghost.jump_to(share);
            return;
        }
        if share == before.0 {
            return;
        }
        let amount = match (before.1, view.current) {
            (Some(b), Some(c)) => b.abs_diff(c),
            _ => 0,
        };
        let big = (share - before.0).abs() >= threshold && amount > 0;
        if !animate {
            self.gaining = false;
            self.flash = None;
            self.sheen = None;
            self.fill.jump_to(share);
            self.ghost.jump_to(share);
            let gain = share > before.0;
            if big && (gain || self.vital == Vital::Health) {
                self.number = Some(FallingNumber {
                    text: if gain {
                        format!("+{}", compact(i64::from(amount)))
                    } else {
                        format!("\u{2212}{}", compact(i64::from(amount)))
                    },
                    gain,
                    at: share,
                    age: 0.0,
                });
            }
            return;
        }
        if share < before.0 {
            // The bar drops at once; the lost part stays as a pale ghost
            // that drains after a beat.
            self.gaining = false;
            self.fill.go_to(share, 0.12, Ease::Out);
            if self.ghost.value() < before.0 {
                self.ghost.jump_to(before.0);
            }
            self.ghost.go_to_after(share, 0.45, 0.55, Ease::Out);
            // A hit flashes and may show its number; spending stamina or
            // mana (running, casting) just drains.
            if self.vital == Vital::Health {
                self.flash = Some((before.0, share, 0.0));
                if big {
                    self.number = Some(FallingNumber {
                        text: format!("\u{2212}{}", compact(i64::from(amount))),
                        gain: false,
                        at: share,
                        age: 0.0,
                    });
                }
            }
        } else {
            // A bright leading edge jumps to the new value and the bar
            // fills up behind it.
            self.gaining = true;
            self.ghost.go_to(share, 0.12, Ease::Out);
            self.fill.go_to_after(share, 0.15, 0.6, Ease::Out);
            if big {
                self.sheen = Some(0.0);
                self.number = Some(FallingNumber {
                    text: format!("+{}", compact(i64::from(amount))),
                    gain: true,
                    at: share,
                    age: 0.0,
                });
            }
        }
    }

    /// A spell needing `need` mana was refused for want of it.
    pub fn refuse(&mut self, need: u32) {
        self.need = Some((need, 0.0));
    }

    /// Moves every motion on by `seconds`.
    pub fn tick(&mut self, seconds: f32) {
        self.fill.tick(seconds);
        self.ghost.tick(seconds);
        let age = |slot: &mut Option<f32>, limit: f32| {
            if let Some(a) = slot {
                *a += seconds;
                if *a >= limit {
                    *slot = None;
                }
            }
        };
        age(&mut self.sheen, SHEEN_SECONDS);
        if let Some((_, _, a)) = &mut self.flash {
            *a += seconds;
        }
        if self.flash.is_some_and(|(_, _, a)| a >= FLASH_SECONDS) {
            self.flash = None;
        }
        if let Some(n) = &mut self.number {
            n.age += seconds;
        }
        if self
            .number
            .as_ref()
            .is_some_and(|n| n.age >= NUMBER_SECONDS)
        {
            self.number = None;
        }
        if let Some((_, a)) = &mut self.need {
            *a += seconds;
        }
        if self.need.is_some_and(|(_, a)| a >= NEED_SECONDS) {
            self.need = None;
        }
    }

    /// Whether anything is still moving (the piece draws every frame while
    /// it is, and the glows pulse while their state lasts).
    #[must_use]
    pub fn is_moving(&self) -> bool {
        !self.fill.is_settled()
            || !self.ghost.is_settled()
            || self.flash.is_some()
            || self.number.is_some()
            || self.sheen.is_some()
            || self.need.is_some()
    }

    /// The value as the bar shows it: "412 / 450".
    #[must_use]
    pub fn value_text(&self) -> String {
        match (self.current, self.max) {
            (Some(c), Some(m)) => format!("{} / {}", compact(i64::from(c)), compact(i64::from(m))),
            _ => String::new(),
        }
    }
}

/// The vitals piece.
#[derive(Debug)]
pub struct VitalsHud {
    theme: Horizon,
    /// The words and values on the bars.
    label_font: Option<Arc<UiFont>>,
    /// The numbers that fall off them.
    number_font: Option<Arc<UiFont>>,
    /// Health, stamina, mana.
    pub bars: [BarMotion; 3],
    /// The share of a bar a change must reach to show its number (above 1:
    /// never).
    pub number_threshold: f32,
    /// Health under this share pulses (0: never).
    pub low_health: f32,
    /// The bars move (losses flash and drain, gains fill in).
    pub animate: bool,
    /// Seconds since the piece was made (the pulses' clock).
    clock: f32,
}

impl VitalsHud {
    /// The piece, its text in `label_font` and its falling numbers in
    /// `number_font`.
    #[must_use]
    pub fn new(
        theme: Horizon,
        label_font: Option<Arc<UiFont>>,
        number_font: Option<Arc<UiFont>>,
    ) -> Self {
        Self {
            theme,
            label_font,
            number_font,
            bars: [
                BarMotion::new(Vital::Health),
                BarMotion::new(Vital::Stamina),
                BarMotion::new(Vital::Mana),
            ],
            number_threshold: DEFAULT_NUMBER_THRESHOLD,
            low_health: DEFAULT_LOW_HEALTH,
            animate: true,
            clock: 0.0,
        }
    }

    /// Takes the vitals' new values.
    pub fn apply(&mut self, vitals: &VitalsView) {
        let (threshold, animate) = (self.number_threshold, self.animate);
        self.bars[0].apply(vitals.health, threshold, animate);
        self.bars[1].apply(vitals.stamina, threshold, animate);
        self.bars[2].apply(vitals.mana, threshold, animate);
    }

    /// A spell needing `need` mana was refused for want of it: the mana bar
    /// shakes and marks the need.
    pub fn refuse_mana(&mut self, need: u32) {
        self.bars[2].refuse(need);
    }

    /// Whether health is low enough to pulse (and the screen's edge to
    /// glow).
    #[must_use]
    pub fn health_is_low(&self) -> bool {
        self.bars[0].seeded && self.bars[0].share() < self.low_health
    }

    fn draw_bar(&self, rc: &mut RenderContext<'_>, bar: &BarMotion, x: f32) {
        let p = &self.theme.palette;
        let radius = self.theme.metrics.radius;
        let (w, h) = (BAR_WIDTH, BAR_HEIGHT);
        let x = x + bar.need.map_or(0.0, |(_, age)| shake(age / SHAKE_SECONDS));
        let low = bar.vital == Vital::Health && self.health_is_low();
        let tired = bar.vital == Vital::Stamina && bar.seeded && bar.current == Some(0);
        if low {
            let pulse = pulse(self.clock, LOW_PULSE_SECONDS);
            rc.soft_shadow(
                x,
                0.0,
                w,
                h,
                radius,
                0.0,
                14.0,
                Color {
                    a: 0.9 * pulse,
                    ..p.enemy
                },
            );
        }
        rc.soft_shadow(x, 0.0, w, h, radius, 1.0, 4.0, hex(0, 0.35));
        rc.fill_rounded_rect(x, 0.0, w, h, CornerRadii::all(radius), p.hud);
        // Inside the edge.
        let (ix, iy, iw, ih) = (x + 1.0, 1.0, w - 2.0, h - 2.0);
        let inner = (radius - 1.0).max(0.0);
        let part = |rc: &mut RenderContext<'_>, from: f32, to: f32, color: Color| {
            let (from, to) = (from.clamp(0.0, 1.0), to.clamp(0.0, 1.0));
            if to <= from {
                return;
            }
            let left = if from <= 0.0 { inner } else { 0.0 };
            let right = if to >= 1.0 { inner } else { 0.0 };
            rc.fill_rounded_rect(
                ix + iw * from,
                iy,
                iw * (to - from),
                ih,
                CornerRadii::new(left, right, right, left),
                color,
            );
        };
        let fill = bar.fill.value();
        let ghost = bar.ghost.value();
        if ghost > fill {
            let color = if bar.gaining {
                bar.vital.lead()
            } else {
                bar.vital.ghost()
            };
            part(rc, fill, ghost, color);
        }
        part(rc, 0.0, fill, bar.vital.color(&self.theme));
        if let Some(age) = bar.sheen {
            // A glint 40 units wide runs along the filled part.
            let band = 40.0;
            let t = (age / SHEEN_SECONDS).clamp(0.0, 1.0);
            let bx = ix - band * 1.2 + (band * 4.4) * t;
            rc.push_clip(ix, iy, iw * fill, ih);
            let clear = hex(0xFF_FFFF, 0.0);
            let shine = hex(0xFF_FFFF, 0.35);
            rc.fill_horizontal_gradient(
                bx,
                iy,
                band * 0.5,
                ih,
                CornerRadii::default(),
                clear,
                shine,
            );
            rc.fill_horizontal_gradient(
                bx + band * 0.5,
                iy,
                band * 0.5,
                ih,
                CornerRadii::default(),
                shine,
                clear,
            );
            rc.pop_clip();
        }
        if let Some((from, to, age)) = bar.flash {
            let alpha = 0.95 * (1.0 - age / FLASH_SECONDS).max(0.0);
            part(rc, to, from, hex(0xFF_FFFF, alpha));
        }
        // The glassy top half.
        rc.fill_rounded_rect(
            ix,
            iy,
            iw,
            ih * 0.5,
            CornerRadii::new(inner, inner, 0.0, 0.0),
            hex(0xFF_FFFF, 0.10),
        );
        let max = bar.max.unwrap_or(0);
        if let Some((need, _)) = bar.need
            && max > 0
            && need > 0
        {
            let at = ix + iw * (need as f32 / max as f32).clamp(0.0, 1.0);
            rc.fill(at - 1.0, -1.0, 2.0, h + 2.0, p.bad);
        }
        let edge = if bar.need.is_some() {
            p.bad
        } else if low {
            p.enemy
        } else if tired {
            let blink = pulse(self.clock, TIRED_BLINK_SECONDS);
            hex(0xFF_B547, 0.95 - 0.8 * blink)
        } else {
            p.trim
        };
        rc.stroke_rounded_rect(x, 0.0, w, h, CornerRadii::all(radius), edge, 1.0);
        let shadow = hex(0, 0.9);
        if let Some(font) = &self.label_font {
            let label = TextStyle::plain(Color::WHITE).tracked(1.1).shadowed(shadow);
            let value = TextStyle::plain(Color::WHITE).shadowed(shadow);
            let word = bar.vital.label();
            let numbers = bar.value_text();
            let gap = if numbers.is_empty() { 0.0 } else { 8.0 };
            let total = text::measure(font, word, 1.1) + gap + text::measure(font, &numbers, 0.0);
            let ty = ((h - font.line_height) * 0.5).round();
            let start = (x + (w - total) * 0.5).round();
            text::draw(rc, font, word, start, ty, label);
            let after = start + text::measure(font, word, 1.1) + gap;
            text::draw(rc, font, &numbers, after, ty, value);
            let note = match (bar.need, tired) {
                (Some((need, _)), _) => {
                    let text = if need > 0 {
                        format!("Needs {need} mana")
                    } else {
                        "Not enough mana".to_owned()
                    };
                    Some((text, hex(0xFF_9A9D, 1.0)))
                }
                (None, true) => Some(("Out of stamina".to_owned(), hex(0xFF_C870, 1.0))),
                _ => None,
            };
            if let Some((note, color)) = note {
                let style = TextStyle::plain(color).shadowed(hex(0, 0.95));
                text::draw_centred(rc, font, &note, x + w * 0.5, h + 4.0, style);
            }
        }
        if let (Some(n), Some(font)) = (&bar.number, &self.number_font) {
            // In fast from above, held, then down and out.
            let t = n.age / NUMBER_SECONDS;
            let (alpha, dy) = if t < 0.12 {
                (t / 0.12, -4.0 * (1.0 - t / 0.12))
            } else if t < 0.7 {
                (1.0, 14.0 * (t - 0.12) / 0.88)
            } else {
                (1.0 - (t - 0.7) / 0.3, 14.0 * (t - 0.12) / 0.88)
            };
            let color = if n.gain { hex(0x7F_E08A, 1.0) } else { p.bad };
            let style =
                TextStyle::plain(Color { a: alpha, ..color }).shadowed(hex(0, 0.95 * alpha));
            let cx = ix + iw * n.at;
            text::draw_centred(rc, font, &n.text, cx, h + 3.0 + dy, style);
        }
    }
}

/// 0 to 1 and back over `period` seconds.
fn pulse(clock: f32, period: f32) -> f32 {
    0.5 - 0.5 * (clock / period * TAU).cos()
}

/// How far a shaking bar is pushed sideways at `t` (0..=1) of the shake.
fn shake(t: f32) -> f32 {
    const KEYS: [(f32, f32); 6] = [
        (0.0, 0.0),
        (0.15, -5.0),
        (0.35, 5.0),
        (0.55, -3.0),
        (0.75, 2.0),
        (1.0, 0.0),
    ];
    if t >= 1.0 {
        return 0.0;
    }
    for pair in KEYS.windows(2) {
        let ((t0, x0), (t1, x1)) = (pair[0], pair[1]);
        if t <= t1 {
            return x0 + (x1 - x0) * (t - t0) / (t1 - t0);
        }
    }
    0.0
}

impl Widget for VitalsHud {
    fn tick(&mut self, _ui: &mut UiRoot, _me: ElementId, dt: f64) {
        let seconds = dt as f32;
        self.clock = (self.clock + seconds) % 3600.0;
        for bar in &mut self.bars {
            bar.tick(seconds);
        }
    }

    fn clips_children(&self) -> bool {
        // The numbers and notes hang below the bars.
        false
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        for (i, bar) in self.bars.iter().enumerate() {
            self.draw_bar(rc, bar, i as f32 * (BAR_WIDTH + BAR_GAP));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(current: u32, max: u32) -> VitalView {
        VitalView {
            percent: Some(current as f32 / max as f32),
            current: Some(current),
            max: Some(max),
        }
    }

    fn seeded(vital: Vital, current: u32, max: u32) -> BarMotion {
        let mut bar = BarMotion::new(vital);
        bar.apply(reading(current, max), DEFAULT_NUMBER_THRESHOLD, true);
        bar
    }

    #[test]
    fn the_first_value_shows_at_once() {
        let bar = seeded(Vital::Health, 412, 450);
        assert!(!bar.is_moving());
        assert_eq!(bar.value_text(), "412 / 450");
        assert!((bar.fill.value() - 412.0 / 450.0).abs() < 1e-6);
    }

    #[test]
    fn a_big_hit_drops_flashes_shows_its_number_and_the_ghost_drains_after() {
        let mut bar = seeded(Vital::Health, 412, 450);
        bar.apply(reading(200, 450), DEFAULT_NUMBER_THRESHOLD, true);
        let n = bar.number.clone().unwrap();
        assert_eq!(n.text, "\u{2212}212");
        assert!(!n.gain);
        assert!(bar.flash.is_some());
        bar.tick(0.2);
        // The bar is down; the ghost has not moved yet.
        assert!((bar.fill.value() - 200.0 / 450.0).abs() < 1e-6);
        assert!((bar.ghost.value() - 412.0 / 450.0).abs() < 1e-6);
        bar.tick(1.5);
        assert!((bar.ghost.value() - 200.0 / 450.0).abs() < 1e-6);
        assert!(!bar.is_moving());
    }

    #[test]
    fn a_small_change_shows_no_number() {
        let mut bar = seeded(Vital::Health, 412, 450);
        bar.apply(reading(400, 450), DEFAULT_NUMBER_THRESHOLD, true);
        assert!(bar.number.is_none());
        assert!(bar.flash.is_some());
        let mut bar = seeded(Vital::Health, 300, 450);
        bar.apply(reading(310, 450), DEFAULT_NUMBER_THRESHOLD, true);
        assert!(bar.number.is_none() && bar.sheen.is_none());
    }

    #[test]
    fn spending_stamina_or_mana_drains_without_a_flash_or_number() {
        let mut bar = seeded(Vital::Mana, 600, 620);
        bar.apply(reading(300, 620), DEFAULT_NUMBER_THRESHOLD, true);
        assert!(bar.flash.is_none() && bar.number.is_none());
        assert!(bar.is_moving());
    }

    #[test]
    fn a_big_gain_leads_and_the_bar_fills_up_behind() {
        let mut bar = seeded(Vital::Health, 200, 450);
        bar.apply(reading(285, 450), DEFAULT_NUMBER_THRESHOLD, true);
        assert!(bar.gaining);
        assert_eq!(bar.number.as_ref().unwrap().text, "+85");
        assert!(bar.sheen.is_some());
        bar.tick(0.13);
        // The leading edge is there; the bar has not moved yet.
        assert!((bar.ghost.value() - 285.0 / 450.0).abs() < 1e-6);
        assert!((bar.fill.value() - 200.0 / 450.0).abs() < 1e-6);
        bar.tick(1.0);
        assert!((bar.fill.value() - 285.0 / 450.0).abs() < 1e-6);
    }

    #[test]
    fn big_numbers_are_short() {
        let mut bar = seeded(Vital::Health, 1_500_000, 2_000_000);
        bar.apply(reading(400_000, 2_000_000), DEFAULT_NUMBER_THRESHOLD, true);
        assert_eq!(bar.number.as_ref().unwrap().text, "\u{2212}1.10M");
        assert_eq!(bar.value_text(), "400K / 2.00M");
    }

    #[test]
    fn effects_end_on_their_own() {
        let mut bar = seeded(Vital::Mana, 46, 620);
        bar.refuse(80);
        assert!(bar.is_moving());
        bar.tick(3.0);
        assert!(bar.need.is_none());
        assert!(!bar.is_moving());
    }

    #[test]
    fn low_health_is_under_a_quarter() {
        let mut hud = VitalsHud::new(Horizon::BRONZE, None, None);
        assert!(!hud.health_is_low());
        let vitals = VitalsView {
            health: reading(88, 450),
            stamina: reading(104, 410),
            mana: reading(361, 620),
        };
        hud.apply(&vitals);
        assert!(hud.health_is_low());
    }

    #[test]
    fn the_low_health_warning_follows_its_setting() {
        let mut hud = VitalsHud::new(Horizon::BRONZE, None, None);
        hud.apply(&VitalsView {
            health: reading(200, 450),
            stamina: reading(104, 410),
            mana: reading(361, 620),
        });
        assert!(!hud.health_is_low());
        hud.low_health = 0.5;
        assert!(hud.health_is_low());
        hud.low_health = 0.0;
        assert!(!hud.health_is_low());
    }

    #[test]
    fn unanimated_bars_move_at_once_and_still_count() {
        let mut bar = BarMotion::new(Vital::Health);
        bar.apply(reading(450, 450), DEFAULT_NUMBER_THRESHOLD, false);
        bar.apply(reading(200, 450), DEFAULT_NUMBER_THRESHOLD, false);
        assert!(bar.flash.is_none() && bar.fill.is_settled() && bar.ghost.is_settled());
        assert_eq!(
            bar.number.as_ref().map(|n| n.text.as_str()),
            Some("\u{2212}250")
        );
        bar.apply(reading(300, 450), 2.0, false);
        assert!(bar.sheen.is_none() && bar.fill.is_settled());
    }

    #[test]
    fn a_shake_starts_and_ends_still() {
        assert_eq!(shake(0.0), 0.0);
        assert_eq!(shake(0.15), -5.0);
        assert_eq!(shake(0.35), 5.0);
        assert_eq!(shake(1.0), 0.0);
    }
}
