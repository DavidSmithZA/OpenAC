//! The combat strip above the shortcut bars. In Peace and Magic it is a
//! small button naming the mode; in Melee and Missile it grows into one
//! slim strip: the mode, High, Mid and Low (hold one to charge, let go to
//! attack), the meter (left is faster, right is stronger; a notch marks
//! the setting and a click moves it; holding an attack fills it, it glows
//! when full and flashes when let go), the word Power or Accuracy, and in
//! Missile a quiet ammunition count. Charging a jump shows the same meter
//! alone, in any mode, while the jump key is held.
//!
//! Clicking the mode opens a small menu of the four modes. Repeat attacks,
//! Auto target and Keep in view are not here: they are options.

use std::f32::consts::TAU;
use std::sync::Arc;

use ac_core::combat::{AttackHeight, CombatMode};
use ac_runtime::gameplay::{CombatAttackCommand, GameplayCommand, InputActivation};
use ac_runtime::session::SessionCommand;
use ac_runtime::world::AttackBarView;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::numbers::group_thousands;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::lines::stroke;

/// The piece's width: room for the widest strip (Missile with its
/// ammunition); what shows is centred in it.
pub const WIDTH: f32 = 600.0;
/// Its height: the strip's.
pub const HEIGHT: f32 = 30.0;
/// The small button's height (Peace and Magic).
const PILL: f32 = 24.0;
/// The mode button's height inside the strip.
const MODE: f32 = 24.0;
/// A height button's height.
const HEIGHT_BUTTON: f32 = 22.0;
/// The gap between the strip's parts.
const GAP: f32 = 10.0;
/// The meter's width, and its trough's height.
const METER: f32 = 170.0;
const TROUGH: f32 = 8.0;
/// The notch's height.
const NOTCH: f32 = 12.0;
/// An icon's side in the mode button, a height button and the ammunition.
const MODE_ICON: f32 = 13.0;
const HEIGHT_ICON: f32 = 10.0;
const AMMO_ICON: f32 = 11.0;
/// Under this many, the ammunition count turns amber.
pub const LOW_AMMO: i32 = 20;
/// How long the meter flashes when an attack or a jump is let go, seconds.
const FLASH_SECONDS: f32 = 0.3;
/// One glow of the full meter, seconds.
const FULL_PULSE_SECONDS: f32 = 0.7;
/// The meter's fill at nothing and at full, its full end, the notch and
/// the flash.
const FILL_START: Color = hex(0xE0_B068, 0.5);
const FULL_END: Color = hex(0xFF_E2A8, 1.0);
const NOTCH_COLOR: Color = hex(0xFF_E2A8, 1.0);
const FULL_GLOW: Color = hex(0xFF_D68C, 0.75);
const FLASH: Color = hex(0xFF_F4DC, 0.9);
/// The ammunition count when low and when out.
const AMMO_LOW: Color = hex(0xFF_C870, 1.0);
const AMMO_OUT: Color = hex(0xFF_8A8E, 1.0);
/// The menu's width, its rows' height and its inside margin.
pub const MENU_WIDTH: f32 = 150.0;
const MENU_ROW: f32 = 28.0;
const MENU_PAD: f32 = 4.0;

/// What is in the ammunition slot, in Missile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ammo {
    /// Nothing: the slot is empty.
    Out,
    /// A stack.
    Loaded {
        /// How many.
        count: i32,
        /// What they are ("Deadly Arrows").
        name: String,
    },
}

/// What the strip shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CombatView {
    /// The combat mode; none outside the world (nothing shows).
    pub mode: Option<CombatMode>,
    /// The attack: its height, the setting and the meter's level.
    pub attack: AttackBarView,
    /// A jump's charge, 0..=1, while the jump key is held.
    pub jump: Option<f32>,
    /// In Missile, the ammunition.
    pub ammo: Option<Ammo>,
}

/// The four modes the menu offers, in its order, with their words.
pub const MODES: [(CombatMode, &str); 4] = [
    (CombatMode::NON_COMBAT, "Peace"),
    (CombatMode::MELEE, "Melee"),
    (CombatMode::MISSILE, "Missile"),
    (CombatMode::MAGIC, "Magic"),
];

/// The word for `mode`.
#[must_use]
pub fn mode_word(mode: CombatMode) -> &'static str {
    MODES
        .iter()
        .find(|(m, _)| *m == mode)
        .map_or("Peace", |(_, w)| w)
}

/// What the piece draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    /// Nothing (outside the world).
    Hidden,
    /// The small button: Peace or Magic.
    Pill,
    /// The strip: Melee or Missile.
    Strip,
    /// The jump meter.
    Jump,
}

/// A part of the strip the pointer can be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Mode,
    Height(usize),
    Meter,
    Ammo,
}

/// The three heights, top to bottom, with their words, the command that
/// charges them and their icon.
const HEIGHTS: [(AttackHeight, &str, &str, CombatAttackCommand); 3] = [
    (
        AttackHeight::HIGH,
        "High",
        "High attack",
        CombatAttackCommand::HighAttack,
    ),
    (
        AttackHeight::MEDIUM,
        "Mid",
        "Medium attack",
        CombatAttackCommand::MediumAttack,
    ),
    (
        AttackHeight::LOW,
        "Low",
        "Low attack",
        CombatAttackCommand::LowAttack,
    ),
];

/// Where the parts are, across the piece (left, width).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Parts {
    /// The strip (or the small button).
    body: (f32, f32),
    mode: (f32, f32),
    heights: [(f32, f32); 3],
    meter: (f32, f32),
    /// The word's left.
    word: f32,
    ammo: (f32, f32),
}

/// The strip's fonts.
#[derive(Debug, Clone, Default)]
pub struct StripFonts {
    /// The mode's word.
    pub mode: Option<Arc<UiFont>>,
    /// The heights' words and the ammunition count.
    pub small: Option<Arc<UiFont>>,
    /// Power or Accuracy.
    pub word: Option<Arc<UiFont>>,
}

/// The combat strip piece.
#[derive(Debug)]
pub struct CombatStrip {
    theme: Horizon,
    fonts: StripFonts,
    /// What it shows.
    pub view: CombatView,
    /// The menu the mode opens (a child of the piece).
    pub menu: Option<ElementId>,
    /// The pointer is on this part.
    hovered: Option<Part>,
    /// This height button is held down (its release lets the attack go).
    held: Option<usize>,
    /// The meter is being dragged: the setting shown meanwhile.
    dragging: Option<f32>,
    /// Seconds since the piece was made (the glow's clock).
    clock: f32,
    /// Seconds left of the flash.
    flash: f32,
    /// The meter filled last frame (an attack or a jump being charged).
    was_charging: bool,
    /// The last jump charge, kept on show while its flash lasts.
    last_jump: Option<f32>,
    /// Quick move is held: the whole piece answers the pointer.
    quick_move: bool,
}

impl CombatStrip {
    /// The strip in `theme`, its words in `fonts`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: StripFonts) -> Self {
        Self {
            theme,
            fonts,
            view: CombatView::default(),
            menu: None,
            hovered: None,
            held: None,
            dragging: None,
            clock: 0.0,
            flash: 0.0,
            was_charging: false,
            last_jump: None,
            quick_move: false,
        }
    }

    /// Shows `view`: a charge that ended starts the flash.
    pub fn set_view(&mut self, view: CombatView) {
        let charging = view.jump.is_some() || view.attack.charging;
        if self.was_charging && !charging {
            self.flash = FLASH_SECONDS;
        }
        self.was_charging = charging;
        if view.jump.is_some() {
            self.last_jump = view.jump;
        }
        self.view = view;
    }

    fn form(&self) -> Form {
        let Some(mode) = self.view.mode else {
            return Form::Hidden;
        };
        if self.view.jump.is_some() || (self.flash > 0.0 && self.last_jump.is_some()) {
            return Form::Jump;
        }
        if mode == CombatMode::MELEE || mode == CombatMode::MISSILE {
            Form::Strip
        } else {
            Form::Pill
        }
    }

    fn missile(&self) -> bool {
        self.view.mode == Some(CombatMode::MISSILE)
    }

    fn mode_label(&self) -> &'static str {
        if self.form() == Form::Jump {
            "Jump"
        } else {
            mode_word(self.view.mode.unwrap_or(CombatMode::NON_COMBAT))
        }
    }

    /// Power in Melee, Accuracy in Missile.
    fn word(&self) -> &'static str {
        if self.missile() { "Accuracy" } else { "Power" }
    }

    fn ammo_text(&self) -> Option<String> {
        match self.view.ammo.as_ref()? {
            Ammo::Out => Some("No ammo".to_owned()),
            Ammo::Loaded { count, .. } => Some(group_thousands(i64::from(*count))),
        }
    }

    fn measure(font: Option<&Arc<UiFont>>, words: &str) -> f32 {
        font.map_or(words.len() as f32 * 7.0, |f| text::measure(f, words, 0.0))
    }

    /// Where everything goes, centred in the piece.
    fn parts(&self) -> Parts {
        let form = self.form();
        let mode_width = 8.0
            + MODE_ICON
            + 6.0
            + Self::measure(self.fonts.mode.as_ref(), self.mode_label())
            + 10.0;
        let mut parts = Parts::default();
        match form {
            Form::Hidden => {}
            Form::Pill => {
                let w = 16.0
                    + MODE_ICON
                    + 6.0
                    + Self::measure(self.fonts.mode.as_ref(), self.mode_label())
                    + 16.0;
                let left = ((WIDTH - w) * 0.5).round();
                parts.body = (left, w);
                parts.mode = (left, w);
            }
            Form::Strip | Form::Jump => {
                let mut x = 4.0;
                parts.mode = (x, mode_width);
                x += mode_width + GAP;
                if form == Form::Strip {
                    for (i, (_, word, ..)) in HEIGHTS.iter().enumerate() {
                        let w = 6.0
                            + HEIGHT_ICON
                            + 4.0
                            + Self::measure(self.fonts.small.as_ref(), word)
                            + 8.0;
                        parts.heights[i] = (x, w);
                        x += w + 1.0;
                    }
                    x += GAP - 1.0;
                }
                parts.meter = (x, METER);
                x += METER;
                if form == Form::Strip {
                    x += GAP;
                    parts.word = x;
                    x += Self::measure(self.fonts.word.as_ref(), self.word());
                    if let Some(words) = self.ammo_text() {
                        x += GAP;
                        let w = AMMO_ICON + 4.0 + Self::measure(self.fonts.small.as_ref(), &words);
                        parts.ammo = (x, w);
                        x += w;
                    }
                }
                let w = x + 12.0;
                let shift = ((WIDTH - w) * 0.5).round();
                parts.body = (shift, w);
                let mv = |r: &mut (f32, f32)| r.0 += shift;
                mv(&mut parts.mode);
                parts.heights.iter_mut().for_each(mv);
                mv(&mut parts.meter);
                parts.word += shift;
                mv(&mut parts.ammo);
            }
        }
        parts
    }

    fn part_at(&self, x: f32, y: f32) -> Option<Part> {
        let form = self.form();
        if form == Form::Hidden || !(0.0..HEIGHT).contains(&y) {
            return None;
        }
        let parts = self.parts();
        let on = |(left, w): (f32, f32)| w > 0.0 && (left..left + w).contains(&x);
        if form != Form::Jump && on(parts.mode) {
            return Some(Part::Mode);
        }
        if form == Form::Strip {
            if let Some(i) = parts.heights.iter().position(|r| on(*r)) {
                return Some(Part::Height(i));
            }
            if on(parts.meter) {
                return Some(Part::Meter);
            }
            if on(parts.ammo) {
                return Some(Part::Ammo);
            }
        }
        None
    }

    /// Whether the pointer at `x` lies on the piece's drawn body.
    fn on_body(&self, x: f32, y: f32) -> bool {
        let (left, w) = self.parts().body;
        self.form() != Form::Hidden
            && w > 0.0
            && (left..left + w).contains(&x)
            && (0.0..HEIGHT).contains(&y)
    }

    /// The meter's setting at `x`, 0..=1.
    fn setting_at(&self, x: f32) -> f32 {
        let (left, w) = self.parts().meter;
        ((x - left) / w).clamp(0.0, 1.0)
    }

    fn set_power(&mut self, ui: &mut UiRoot, x: f32) {
        let power = self.setting_at(x);
        self.dragging = Some(power);
        send(ui, GameplayCommand::SetAttackPower(power));
    }

    fn let_go(&mut self, ui: &mut UiRoot) {
        if let Some(i) = self.held.take() {
            send(
                ui,
                GameplayCommand::AttackInput(HEIGHTS[i].3, InputActivation::Release),
            );
        }
        self.dragging = None;
    }

    /// Opens the mode menu over the mode button (under it when the piece
    /// is too near the top of the screen).
    fn open_menu(&self, ui: &mut UiRoot, me: ElementId) {
        let Some(menu) = self.menu else {
            return;
        };
        let current = self.view.mode.unwrap_or(CombatMode::NON_COMBAT);
        let height = ModeMenu::height();
        let (_, y) = ui.screen_position(me);
        let left = self.parts().mode.0;
        let top = if y >= height + 6.0 {
            -height - 6.0
        } else {
            HEIGHT + 6.0
        };
        if let Some(e) = ui.element_mut(menu) {
            e.left = left;
            e.top = top;
            e.height = height;
        }
        ui.with::<ModeMenu, _>(menu, |m, ui| m.open(ui, menu, current));
        ui.bring_to_front(me);
    }

    fn draw_pill(&self, rc: &mut RenderContext<'_>, parts: &Parts) {
        let p = self.theme.palette;
        let (left, w) = parts.body;
        let top = (HEIGHT - PILL) * 0.5;
        let round = CornerRadii::all(PILL * 0.5);
        let hovered = self.hovered == Some(Part::Mode);
        let fill = if hovered {
            hex(0x2A_2116, 0.8)
        } else {
            hex(0, 0.55)
        };
        rc.fill_rounded_rect(left, top, w, PILL, round, fill);
        let edge = if hovered { p.accent } else { p.trim };
        rc.stroke_rounded_rect(left, top, w, PILL, round, edge, 1.0);
        let mode = self.view.mode.unwrap_or(CombatMode::NON_COMBAT);
        mode_icon(
            rc,
            mode,
            left + 16.0,
            top + (PILL - MODE_ICON) * 0.5,
            MODE_ICON,
            p.accent,
        );
        if let Some(f) = &self.fonts.mode {
            let ty = (top + (PILL - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                self.mode_label(),
                left + 16.0 + MODE_ICON + 6.0,
                ty,
                TextStyle::plain(p.text).shadowed(p.text_shadow),
            );
        }
    }

    fn draw_strip(&self, rc: &mut RenderContext<'_>, parts: &Parts, form: Form) {
        let p = self.theme.palette;
        let shadow = p.text_shadow;
        let (left, w) = parts.body;
        let round = CornerRadii::all(HEIGHT * 0.5);
        rc.fill_rounded_rect(left, 0.0, w, HEIGHT, round, hex(0, 0.55));
        rc.stroke_rounded_rect(left, 0.0, w, HEIGHT, round, p.trim, 1.0);
        // The mode.
        let (mx, mw) = parts.mode;
        let my = (HEIGHT - MODE) * 0.5;
        let lit = form == Form::Strip && self.hovered == Some(Part::Mode);
        let fill = if lit {
            hex(0xE0_B068, 0.24)
        } else {
            hex(0xE0_B068, 0.12)
        };
        rc.fill_rounded_rect(mx, my, mw, MODE, CornerRadii::all(MODE * 0.5), fill);
        let icon_y = (HEIGHT - MODE_ICON) * 0.5;
        if form == Form::Jump {
            jump_icon(rc, mx + 8.0, icon_y, MODE_ICON, p.accent);
        } else {
            let mode = self.view.mode.unwrap_or(CombatMode::MELEE);
            mode_icon(rc, mode, mx + 8.0, icon_y, MODE_ICON, p.accent);
        }
        if let Some(f) = &self.fonts.mode {
            let ty = ((HEIGHT - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                self.mode_label(),
                mx + 8.0 + MODE_ICON + 6.0,
                ty,
                TextStyle::plain(p.text).shadowed(shadow),
            );
        }
        if form == Form::Strip {
            self.draw_heights(rc, parts);
        }
        self.draw_meter(rc, parts, form);
        if form != Form::Strip {
            return;
        }
        if let Some(f) = &self.fonts.word {
            let ty = ((HEIGHT - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                self.word(),
                parts.word,
                ty,
                TextStyle::plain(p.muted).shadowed(shadow),
            );
        }
        if let (Some(words), Some(f)) = (self.ammo_text(), &self.fonts.small) {
            let color = match &self.view.ammo {
                Some(Ammo::Out) => AMMO_OUT,
                Some(Ammo::Loaded { count, .. }) if *count < LOW_AMMO => AMMO_LOW,
                _ => p.muted,
            };
            let (ax, _) = parts.ammo;
            ammo_icon(rc, ax, (HEIGHT - AMMO_ICON) * 0.5, AMMO_ICON, color);
            let ty = ((HEIGHT - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                &words,
                ax + AMMO_ICON + 4.0,
                ty,
                TextStyle::plain(color).shadowed(shadow),
            );
        }
    }

    fn draw_heights(&self, rc: &mut RenderContext<'_>, parts: &Parts) {
        let p = self.theme.palette;
        let top = (HEIGHT - HEIGHT_BUTTON) * 0.5;
        let round = CornerRadii::all(HEIGHT_BUTTON * 0.5);
        for (i, (height, word, ..)) in HEIGHTS.iter().enumerate() {
            let (x, w) = parts.heights[i];
            let on = self.view.attack.height == *height;
            let held = self.held == Some(i);
            let hovered = self.hovered == Some(Part::Height(i));
            let fill = if on {
                if held { p.accent } else { p.accent_fill }
            } else if held {
                hex(0xE0_B068, 0.3)
            } else if hovered {
                hex(0xE0_B068, 0.14)
            } else {
                hex(0, 0.0)
            };
            if fill.a > 0.0 {
                rc.fill_rounded_rect(x, top, w, HEIGHT_BUTTON, round, fill);
            }
            let ink = if on { p.accent_ink } else { p.text_soft };
            height_icon(
                rc,
                i,
                x + 6.0,
                (HEIGHT - HEIGHT_ICON) * 0.5,
                HEIGHT_ICON,
                ink,
            );
            if let Some(f) = &self.fonts.small {
                let ty = ((HEIGHT - f.line_height) * 0.5).round();
                let style = if on {
                    TextStyle::plain(ink)
                } else {
                    TextStyle::plain(ink).shadowed(p.text_shadow)
                };
                text::draw(rc, f, word, x + 6.0 + HEIGHT_ICON + 4.0, ty, style);
            }
        }
    }

    fn draw_meter(&self, rc: &mut RenderContext<'_>, parts: &Parts, form: Form) {
        let p = self.theme.palette;
        let (x, w) = parts.meter;
        let y = (HEIGHT - TROUGH) * 0.5;
        let round = CornerRadii::all(TROUGH * 0.5);
        let level = if form == Form::Jump {
            self.view.jump.or(self.last_jump).unwrap_or(0.0)
        } else {
            self.view.attack.power_bar
        }
        .clamp(0.0, 1.0);
        let charging = if form == Form::Jump {
            self.view.jump.is_some()
        } else {
            self.view.attack.charging
        };
        let full = level >= 0.999;
        if full && charging {
            let pulse = 0.5 - 0.5 * (self.clock / FULL_PULSE_SECONDS * TAU).cos();
            rc.soft_shadow(
                x,
                y,
                w,
                TROUGH,
                TROUGH * 0.5,
                0.0,
                9.0,
                Color {
                    a: FULL_GLOW.a * pulse,
                    ..FULL_GLOW
                },
            );
        }
        rc.fill_rounded_rect(x, y, w, TROUGH, round, hex(0, 0.6));
        if level > 0.0 {
            let end = if full { FULL_END } else { p.accent };
            let start = if full { p.accent } else { FILL_START };
            rc.fill_horizontal_gradient(x, y, w * level, TROUGH, round, start, end);
        }
        if self.flash > 0.0 {
            let a = FLASH.a * (self.flash / FLASH_SECONDS);
            rc.fill_rounded_rect(x, y, w, TROUGH, round, Color { a, ..FLASH });
        }
        rc.stroke_rounded_rect(x, y, w, TROUGH, round, p.trim_soft, 1.0);
        if form == Form::Strip {
            let setting = self
                .dragging
                .unwrap_or(self.view.attack.desired_power)
                .clamp(0.0, 1.0);
            let nx = (x + w * setting - 1.0).round();
            let ny = (HEIGHT - NOTCH) * 0.5;
            rc.fill_rounded_rect(
                nx - 1.0,
                ny - 1.0,
                4.0,
                NOTCH + 2.0,
                CornerRadii::all(1.5),
                hex(0, 0.55),
            );
            rc.fill_rounded_rect(nx, ny, 2.0, NOTCH, CornerRadii::all(1.0), NOTCH_COLOR);
        }
    }
}

fn send(ui: &mut UiRoot, command: GameplayCommand) {
    emit(ui, PanelOutput::Session(SessionCommand::Gameplay(command)));
}

impl Widget for CombatStrip {
    fn hit_test(&self, _element: &Element, x: f32, y: f32) -> bool {
        // The drawn strip takes the pointer; the room round it lets clicks
        // through to the world (all of it while quick move is held).
        self.quick_move || self.on_body(x, y)
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        // The parts sit side by side: a move from one to the next must
        // reach the strip.
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                if self.dragging.is_some() {
                    self.set_power(ui, x);
                } else if self.held.is_none() {
                    self.hovered = self.part_at(x, y);
                }
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::MOUSE_DOWN => {
                match self.part_at(x, y) {
                    Some(Part::Height(i)) => {
                        self.held = Some(i);
                        send(
                            ui,
                            GameplayCommand::AttackInput(HEIGHTS[i].3, InputActivation::Press),
                        );
                    }
                    Some(Part::Meter) => self.set_power(ui, x),
                    _ => {}
                }
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.let_go(ui);
                true
            }
            kind::CLICK => {
                if self.part_at(x, y) == Some(Part::Mode) {
                    self.open_menu(ui, me);
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, dt: f64) {
        self.quick_move = ui.quick_move_held();
        let dt = dt as f32;
        self.clock = (self.clock + dt) % FULL_PULSE_SECONDS;
        if self.flash > 0.0 {
            self.flash = (self.flash - dt).max(0.0);
            if self.flash == 0.0 && self.view.jump.is_none() {
                self.last_jump = None;
            }
        }
        // A mode that is not Melee or Missile any more lets a held attack go.
        if self.held.is_some() && self.form() != Form::Strip {
            self.let_go(ui);
        }
    }

    fn clips_children(&self) -> bool {
        false
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        match self.hovered? {
            Part::Mode => Some(
                "Combat mode (` switches): click for Peace, Melee, Missile or Magic".to_owned(),
            ),
            Part::Height(i) => Some(format!(
                "{} \u{b7} hold to charge, let go to attack",
                HEIGHTS[i].2
            )),
            Part::Meter => Some(format!(
                "{} setting, {}%. Left is faster, right is stronger. Click to set it; holding an attack fills the bar.",
                self.word(),
                (self.view.attack.desired_power * 100.0).round() as i32
            )),
            Part::Ammo => Some(match self.view.ammo.as_ref()? {
                Ammo::Out => "Nothing in your ammunition slot".to_owned(),
                Ammo::Loaded { count, name } if *count < LOW_AMMO => {
                    format!("{name} in your ammunition slot, running low")
                }
                Ammo::Loaded { name, .. } => format!("{name} in your ammunition slot"),
            }),
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let form = self.form();
        let parts = self.parts();
        match form {
            Form::Hidden => {}
            Form::Pill => self.draw_pill(rc, &parts),
            Form::Strip | Form::Jump => self.draw_strip(rc, &parts, form),
        }
    }
}

/// The small menu of the four modes the mode button opens.
#[derive(Debug)]
pub struct ModeMenu {
    theme: Horizon,
    font: Option<Arc<UiFont>>,
    /// The mode in use, marked.
    current: CombatMode,
    hovered: Option<usize>,
}

impl ModeMenu {
    /// The menu in `theme`, its words in `font`.
    #[must_use]
    pub fn new(theme: Horizon, font: Option<Arc<UiFont>>) -> Self {
        Self {
            theme,
            font,
            current: CombatMode::NON_COMBAT,
            hovered: None,
        }
    }

    /// Its height.
    #[must_use]
    pub fn height() -> f32 {
        MENU_PAD * 2.0 + MENU_ROW * MODES.len() as f32
    }

    fn row_at(y: f32) -> Option<usize> {
        let i = ((y - MENU_PAD) / MENU_ROW).floor();
        (y >= MENU_PAD && (i as usize) < MODES.len()).then_some(i as usize)
    }

    /// Opens it (the live pop-up: a press outside closes it), `current`
    /// marked.
    pub fn open(&mut self, ui: &mut UiRoot, me: ElementId, current: CombatMode) {
        self.current = current;
        self.hovered = None;
        ui.set_visible(me, true);
        ui.set_active_popup(me, me);
    }

    fn close(ui: &mut UiRoot, me: ElementId) {
        ui.clear_active_popup(me);
        ui.set_visible(me, false);
    }
}

impl Widget for ModeMenu {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_popup_dismissed(&mut self, ui: &mut UiRoot, me: ElementId, _popup: ElementId) {
        ui.set_visible(me, false);
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let y = e.data2 as f32;
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = Self::row_at(y);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::CLICK => {
                if let Some(i) = Self::row_at(y) {
                    let mode = MODES[i].0;
                    if mode != self.current {
                        send(ui, GameplayCommand::SetCombatMode(mode));
                    }
                    Self::close(ui, me);
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let (w, h) = (MENU_WIDTH, Self::height());
        let radius = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 12.0, 30.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, w, h, radius, p.popup);
        rc.stroke_rounded_rect(0.0, 0.0, w, h, radius, p.trim, 1.0);
        for (i, (mode, word)) in MODES.iter().enumerate() {
            let top = MENU_PAD + MENU_ROW * i as f32;
            let current = *mode == self.current;
            let row = CornerRadii::all(self.theme.metrics.radius);
            if current {
                rc.fill_rounded_rect(
                    MENU_PAD,
                    top,
                    w - MENU_PAD * 2.0,
                    MENU_ROW,
                    row,
                    p.accent_tint,
                );
            } else if self.hovered == Some(i) {
                rc.fill_rounded_rect(
                    MENU_PAD,
                    top,
                    w - MENU_PAD * 2.0,
                    MENU_ROW,
                    row,
                    hex(0x2A_2116, 0.9),
                );
            }
            let color = if current { p.accent } else { p.icon };
            mode_icon(
                rc,
                *mode,
                14.0,
                top + (MENU_ROW - MODE_ICON) * 0.5,
                MODE_ICON,
                color,
            );
            if let Some(f) = &self.font {
                let ty = (top + (MENU_ROW - f.line_height) * 0.5).round();
                let ink = if current { p.accent } else { p.text };
                text::draw(
                    rc,
                    f,
                    word,
                    14.0 + MODE_ICON + 8.0,
                    ty,
                    TextStyle::plain(ink),
                );
            }
            if current {
                // A tick at the right.
                let (cx, cy) = (w - 22.0, top + MENU_ROW * 0.5);
                rc.smooth_line(cx, cy, cx + 3.5, cy + 3.5, p.accent, 1.6);
                rc.smooth_line(cx + 3.5, cy + 3.5, cx + 10.0, cy - 3.5, p.accent, 1.6);
            }
        }
    }
}

/// The icon of `mode` in a `size` box.
fn mode_icon(
    rc: &mut RenderContext<'_>,
    mode: CombatMode,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) {
    let lines: &[&[(f32, f32)]] = if mode == CombatMode::MELEE {
        // A sword.
        &[
            &[
                (14.5, 17.5),
                (3.0, 6.0),
                (3.0, 3.0),
                (6.0, 3.0),
                (17.5, 14.5),
            ],
            &[(13.0, 19.0), (19.0, 13.0)],
            &[(16.0, 16.0), (20.0, 20.0)],
            &[(19.0, 21.0), (21.0, 19.0)],
        ]
    } else if mode == CombatMode::MISSILE {
        // An arrow off a bow.
        &[
            &[(3.0, 21.0), (17.0, 7.0)],
            &[(12.0, 7.0), (17.0, 7.0), (17.0, 12.0)],
            &[(14.0, 3.0), (17.6, 4.4), (19.9, 6.6), (21.0, 10.0)],
        ]
    } else if mode == CombatMode::MAGIC {
        // A wand and a spark.
        &[
            &[(4.0, 20.0), (16.0, 8.0)],
            &[
                (14.0, 4.0),
                (15.5, 7.0),
                (19.0, 8.5),
                (15.5, 10.0),
                (14.0, 13.0),
                (12.5, 10.0),
                (9.0, 8.5),
                (12.5, 7.0),
                (14.0, 4.0),
            ],
        ]
    } else {
        // Peace: a sword sheathed, point down.
        &[
            &[(12.0, 2.0), (12.0, 6.0)],
            &[(7.0, 6.0), (17.0, 6.0)],
            &[
                (9.5, 6.0),
                (9.5, 19.0),
                (12.0, 22.0),
                (14.5, 19.0),
                (14.5, 6.0),
            ],
        ]
    };
    stroke(rc, x, y, size, lines, color, 1.5);
}

/// An arrow up: a jump.
fn jump_icon(rc: &mut RenderContext<'_>, x: f32, y: f32, size: f32, color: Color) {
    stroke(
        rc,
        x,
        y,
        size,
        &[
            &[(12.0, 19.0), (12.0, 5.0)],
            &[(5.0, 12.0), (12.0, 5.0), (19.0, 12.0)],
        ],
        color,
        1.5,
    );
}

/// High (a chevron up), Mid (a dash) or Low (a chevron down).
fn height_icon(rc: &mut RenderContext<'_>, index: usize, x: f32, y: f32, size: f32, color: Color) {
    let lines: &[&[(f32, f32)]] = match index {
        0 => &[&[(6.0, 15.0), (12.0, 9.0), (18.0, 15.0)]],
        1 => &[&[(5.0, 12.0), (19.0, 12.0)]],
        _ => &[&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]],
    };
    stroke(rc, x, y, size, lines, color, 1.5);
}

/// Two plain shafts: ammunition, whatever it is.
fn ammo_icon(rc: &mut RenderContext<'_>, x: f32, y: f32, size: f32, color: Color) {
    stroke(
        rc,
        x,
        y,
        size,
        &[
            &[
                (7.0, 21.0),
                (7.0, 8.0),
                (9.0, 3.0),
                (11.0, 8.0),
                (11.0, 21.0),
            ],
            &[
                (15.0, 21.0),
                (15.0, 8.0),
                (17.0, 3.0),
                (19.0, 8.0),
                (19.0, 21.0),
            ],
        ],
        color,
        1.3,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(mode: CombatMode) -> CombatStrip {
        let mut s = CombatStrip::new(Horizon::BRONZE, StripFonts::default());
        s.set_view(CombatView {
            mode: Some(mode),
            ..CombatView::default()
        });
        s
    }

    #[test]
    fn peace_and_magic_keep_the_small_button() {
        assert_eq!(strip(CombatMode::NON_COMBAT).form(), Form::Pill);
        assert_eq!(strip(CombatMode::MAGIC).form(), Form::Pill);
        assert_eq!(strip(CombatMode::MELEE).form(), Form::Strip);
        assert_eq!(strip(CombatMode::MISSILE).form(), Form::Strip);
        let outside = CombatStrip::new(Horizon::BRONZE, StripFonts::default());
        assert_eq!(outside.form(), Form::Hidden);
    }

    #[test]
    fn charging_a_jump_shows_the_meter_alone_in_any_mode() {
        for mode in [CombatMode::NON_COMBAT, CombatMode::MELEE, CombatMode::MAGIC] {
            let mut s = strip(mode);
            s.set_view(CombatView {
                mode: Some(mode),
                jump: Some(0.4),
                ..CombatView::default()
            });
            assert_eq!(s.form(), Form::Jump);
            assert_eq!(s.mode_label(), "Jump");
            assert_eq!(s.part_at(s.parts().meter.0 + 5.0, 15.0), None);
        }
    }

    #[test]
    fn a_let_go_jump_flashes_then_goes() {
        let mut s = strip(CombatMode::NON_COMBAT);
        s.set_view(CombatView {
            mode: Some(CombatMode::NON_COMBAT),
            jump: Some(0.8),
            ..CombatView::default()
        });
        s.set_view(CombatView {
            mode: Some(CombatMode::NON_COMBAT),
            ..CombatView::default()
        });
        assert_eq!(s.form(), Form::Jump);
        assert!(s.flash > 0.0);
        let mut ui = UiRoot::new();
        let root = ui.root();
        s.tick(&mut ui, root, 0.5);
        assert_eq!(s.form(), Form::Pill);
    }

    #[test]
    fn an_attack_let_go_flashes_the_meter() {
        let mut s = strip(CombatMode::MELEE);
        let charging = |c| CombatView {
            mode: Some(CombatMode::MELEE),
            attack: AttackBarView {
                power_bar: 1.0,
                charging: c,
                ..AttackBarView::default()
            },
            ..CombatView::default()
        };
        s.set_view(charging(true));
        s.set_view(charging(true));
        assert_eq!(s.flash, 0.0, "held at full: no flash yet");
        s.set_view(charging(false));
        assert!(s.flash > 0.0);
    }

    #[test]
    fn the_parts_sit_in_order_inside_the_centred_strip() {
        let mut s = strip(CombatMode::MISSILE);
        s.view.ammo = Some(Ammo::Loaded {
            count: 214,
            name: "Deadly Arrows".to_owned(),
        });
        let parts = s.parts();
        let (left, w) = parts.body;
        assert!(left > 0.0 && left + w < WIDTH, "{parts:?}");
        assert!(((left + w * 0.5) - WIDTH * 0.5).abs() <= 1.0);
        assert!(parts.mode.0 < parts.heights[0].0);
        assert!(parts.heights[2].0 < parts.meter.0);
        assert!(parts.meter.0 + METER < parts.word);
        assert!(parts.word < parts.ammo.0);
        assert!(parts.ammo.0 + parts.ammo.1 <= left + w);
        assert_eq!(
            s.part_at(parts.heights[1].0 + 2.0, 15.0),
            Some(Part::Height(1))
        );
        assert_eq!(s.part_at(parts.meter.0 + 2.0, 15.0), Some(Part::Meter));
        assert_eq!(s.part_at(parts.ammo.0 + 2.0, 15.0), Some(Part::Ammo));
        assert_eq!(s.part_at(2.0, 15.0), None);
        assert!(!s.on_body(2.0, 15.0));
    }

    #[test]
    fn the_meter_reads_left_faster_right_stronger() {
        let s = strip(CombatMode::MELEE);
        let (x, w) = s.parts().meter;
        assert_eq!(s.setting_at(x - 20.0), 0.0);
        assert!((s.setting_at(x + w * 0.25) - 0.25).abs() < 1e-4);
        assert_eq!(s.setting_at(x + w + 20.0), 1.0);
    }

    #[test]
    fn the_word_and_the_ammunition_follow_missile() {
        let mut s = strip(CombatMode::MELEE);
        assert_eq!(s.word(), "Power");
        assert_eq!(s.ammo_text(), None);
        s = strip(CombatMode::MISSILE);
        assert_eq!(s.word(), "Accuracy");
        s.view.ammo = Some(Ammo::Out);
        assert_eq!(s.ammo_text().as_deref(), Some("No ammo"));
        s.view.ammo = Some(Ammo::Loaded {
            count: 1234,
            name: "Arrows".to_owned(),
        });
        assert_eq!(s.ammo_text().as_deref(), Some("1,234"));
    }

    #[test]
    fn the_menu_rows_are_the_four_modes() {
        assert_eq!(ModeMenu::row_at(MENU_PAD + 1.0), Some(0));
        assert_eq!(ModeMenu::row_at(MENU_PAD + MENU_ROW * 3.5), Some(3));
        assert_eq!(ModeMenu::row_at(ModeMenu::height() + 4.0), None);
        assert_eq!(ModeMenu::row_at(1.0), None);
        assert_eq!(mode_word(CombatMode::MISSILE), "Missile");
    }
}
