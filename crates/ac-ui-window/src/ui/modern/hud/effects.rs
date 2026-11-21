//! Top left of the HUD: small chips in place of an icon per effect. Buffs
//! easily run past forty, so the helpful effects are one count with the
//! time left on the next to end (amber once one ends within five minutes),
//! the harmful ones a red count with the first to end, and vitae its
//! percentage after a death. Beside them, chips that show only when they
//! have something to say: a burden over what the player can carry, and a
//! chess game going on.
//!
//! Resting the pointer on the helpful or harmful count opens a pane that
//! lists every effect of that kind, soonest to end first, with a
//! scrollbar. Clicking a chip opens the window behind it.

use std::sync::Arc;

use ac_core::spells::Spellbook;
use ac_runtime::gameplay::chess::ChessStage;
use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::indicators::format_seconds_left;
use ac_ui::panels::items::emit;
use ac_ui::panels::{PanelOutput, panel_id};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::target::fit;

/// By default, a helpful effect ending within this many seconds turns its
/// count amber (Options sets it)
/// (the design's default; an Options row is to follow).
pub const DEFAULT_SOON_SECONDS: f64 = 300.0;

/// A chip's height, units.
pub const CHIP_HEIGHT: f32 = 28.0;
/// The gap between chips.
pub const CHIP_GAP: f32 = 6.0;
/// The chips' icons' side.
const ICON: f32 = 15.0;
/// The pane's width.
pub const PANE_WIDTH: f32 = 320.0;
/// The pane's list at most, for the helpful and the harmful effects.
const HELPFUL_LIST_MAX: f32 = 372.0;
const HARMFUL_LIST_MAX: f32 = 260.0;
/// A group's heading and a row, in the pane.
const GROUP_HEIGHT: f32 = 22.0;
const ROW_HEIGHT: f32 = 28.0;
/// How far one wheel step scrolls the pane.
const WHEEL_STEP: f32 = 2.0 * ROW_HEIGHT;

/// Ending soon.
const SOON: Color = hex(0xFF_B547, 1.0);
/// A group heading, quietly.
const HEADING: Color = hex(0xC8_B89C, 1.0);
/// Words on the vitae chip.
const VITAE_TEXT: Color = hex(0xFF_D6D7, 1.0);

// ── What the chips show ─────────────────────────────────────────────────

/// One effect on the player.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectLine {
    /// Its spell.
    pub spell_id: u32,
    /// The spell's name.
    pub name: String,
    /// Seconds left; none for one with no end (a cantrip on worn gear).
    pub left: Option<f64>,
}

/// The effects on the player at one moment.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EffectsOnYou {
    /// The helpful ones in effect: those with an end soonest first, then
    /// those without by name.
    pub helpful: Vec<EffectLine>,
    /// The harmful ones in effect, in the same order.
    pub harmful: Vec<EffectLine>,
    /// What vitae leaves of health, stamina, mana and skills (0.94 for 6%
    /// lower); none without vitae.
    pub vitae: Option<f32>,
}

impl EffectsOnYou {
    /// The effects in `book` at `now` on the enchantments' clock: those in
    /// effect (a weaker copy waiting underneath is not counted), helpful or
    /// harmful by their spell. Spells the client cannot name are left out,
    /// as the retail effects panels leave them out.
    #[must_use]
    pub fn read(book: &Spellbook, now: f64) -> Self {
        let mut helpful = Vec::new();
        let mut harmful = Vec::new();
        for record in book.enchantments_in_effect_snapshot() {
            let Some(spell) = book.try_get_metadata(record.spell_id) else {
                continue;
            };
            let line = EffectLine {
                spell_id: record.spell_id,
                name: spell.name.clone(),
                left: (record.duration >= 0.0)
                    .then(|| (record.start_time + record.duration - now).max(0.0)),
            };
            if spell.is_beneficial() {
                helpful.push(line);
            } else {
                harmful.push(line);
            }
        }
        for list in [&mut helpful, &mut harmful] {
            list.sort_by(|a, b| match (a.left, b.left) {
                (Some(x), Some(y)) => x.total_cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });
        }
        let vitae = book
            .active_enchantments()
            .iter()
            .filter(|r| r.bucket == 4)
            .find_map(|r| r.stat_mod_value.filter(|v| *v < 1.0));
        Self {
            helpful,
            harmful,
            vitae,
        }
    }

    /// Seconds left on the helpful effect to end first.
    #[must_use]
    pub fn next_helpful(&self) -> Option<f64> {
        self.helpful.first().and_then(|l| l.left)
    }

    /// Seconds left on the harmful effect to end first.
    #[must_use]
    pub fn next_harmful(&self) -> Option<f64> {
        self.harmful.first().and_then(|l| l.left)
    }

    /// How many helpful effects end within `soon` seconds.
    #[must_use]
    pub fn helpful_ending_within(&self, soon: f64) -> usize {
        self.helpful
            .iter()
            .filter(|l| l.left.is_some_and(|t| t < soon))
            .count()
    }
}

/// Vitae as its chip says it: the share kept, a whole percentage (never
/// rounded up to 100 while some is lost).
#[must_use]
pub fn vitae_percent(kept: f32) -> u32 {
    let percent = (kept.clamp(0.0, 1.0) * 100.0).round() as u32;
    if kept < 1.0 { percent.min(99) } else { percent }
}

/// The burden chip's look, from the load (what is carried over what can
/// be): none under a full load, amber from it, red from twice it (retail's
/// three burden steps).
#[must_use]
pub fn burden_tone(load: f32) -> Option<Tone> {
    if load >= 2.0 {
        Some(Tone::VeryHeavy)
    } else if load >= 1.0 {
        Some(Tone::Heavy)
    } else {
        None
    }
}

/// The burden chip's words for a load.
#[must_use]
pub fn burden_words(load: f32) -> String {
    format!("Burden {}%", (load * 100.0).round() as i64)
}

/// The chess chip's words for where the player is in a game; none outside
/// one.
#[must_use]
pub fn chess_words(stage: ChessStage) -> Option<&'static str> {
    match stage {
        ChessStage::NotPlaying | ChessStage::Joining => None,
        ChessStage::WaitingForStart => Some("Chess \u{b7} waiting"),
        ChessStage::MyTurn => Some("Chess \u{b7} your move"),
        ChessStage::MoveSent | ChessStage::OpponentsTurn => Some("Chess \u{b7} their move"),
    }
}

// ── The chips ───────────────────────────────────────────────────────────

/// Which chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipKind {
    /// The helpful effects.
    Helpful,
    /// The harmful effects.
    Harmful,
    /// Vitae.
    Vitae,
    /// Burden.
    Burden,
    /// A chess game.
    Chess,
}

impl ChipKind {
    /// The chips, left to right.
    pub const ALL: [Self; 5] = [
        Self::Helpful,
        Self::Harmful,
        Self::Vitae,
        Self::Burden,
        Self::Chess,
    ];
}

/// A chip's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Helpful effects, none ending soon.
    Good,
    /// Helpful effects, one ending soon.
    Soon,
    /// Harmful effects, vitae.
    Bad,
    /// Over a full load.
    Heavy,
    /// Over twice a full load.
    VeryHeavy,
    /// Nothing wrong (a game going on).
    Calm,
}

struct Look {
    edge: Color,
    fill: Color,
    /// The icon, the time or the words.
    ink: Color,
}

impl Tone {
    fn look(self, open: bool, accent: Color) -> Look {
        match (self, open) {
            (Self::Good | Self::Soon, true) => Look {
                edge: accent,
                fill: hex(0x14_100C, 0.85),
                ink: if self == Self::Soon {
                    SOON
                } else {
                    hex(0xE9_D3AE, 1.0)
                },
            },
            (Self::Good, false) => Look {
                edge: hex(0x7F_D08A, 0.55),
                fill: hex(0x0A_1E0E, 0.45),
                ink: hex(0xE9_D3AE, 1.0),
            },
            (Self::Soon, false) => Look {
                edge: hex(0xFF_B547, 0.75),
                fill: hex(0x00_0000, 0.55),
                ink: SOON,
            },
            (Self::Bad, open) => Look {
                edge: if open {
                    hex(0xFF_6B70, 1.0)
                } else {
                    hex(0xFF_6B70, 0.75)
                },
                fill: hex(0x28_0A0C, if open { 0.9 } else { 0.6 }),
                ink: hex(0xFF_8A8E, 1.0),
            },
            (Self::Heavy, _) => Look {
                edge: hex(0xF0_B44C, 0.85),
                fill: hex(0x28_1C08, 0.7),
                ink: hex(0xFF_D99A, 1.0),
            },
            (Self::VeryHeavy, _) => Look {
                edge: hex(0xFF_6B70, 0.85),
                fill: hex(0x32_0C0E, 0.75),
                ink: hex(0xFF_C2C4, 1.0),
            },
            (Self::Calm, _) => Look {
                edge: hex(0xC9_A062, 0.7),
                fill: hex(0x14_100C, 0.6),
                ink: hex(0xE9_D3AE, 1.0),
            },
        }
    }
}

/// What a chip shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipView {
    /// Its colours.
    pub tone: Tone,
    /// The count (the effects chips) or the percentage (vitae), in the
    /// larger figures.
    pub figure: String,
    /// The time left, or the chip's words.
    pub words: String,
}

/// The fonts the chips use.
#[derive(Debug, Clone, Default)]
pub struct ChipFonts {
    /// Counts and percentages.
    pub figure: Option<Arc<UiFont>>,
    /// Words.
    pub words: Option<Arc<UiFont>>,
    /// Times left and the vitae word.
    pub small: Option<Arc<UiFont>>,
}

/// One chip.
#[derive(Debug)]
pub struct Chip {
    theme: Horizon,
    fonts: ChipFonts,
    /// Which.
    pub kind: ChipKind,
    /// What it shows; hidden without.
    pub view: Option<ChipView>,
    /// Its pane is open.
    pub open: bool,
    hovered: bool,
}

impl Chip {
    /// A chip of `kind`, showing nothing yet.
    #[must_use]
    pub fn new(theme: Horizon, fonts: ChipFonts, kind: ChipKind) -> Self {
        Self {
            theme,
            fonts,
            kind,
            view: None,
            open: false,
            hovered: false,
        }
    }

    fn figure_width(&self, figure: &str) -> f32 {
        self.fonts
            .figure
            .as_ref()
            .map_or(0.0, |f| text::measure(f, figure, 0.0))
    }

    fn words_font(&self) -> Option<&Arc<UiFont>> {
        match self.kind {
            ChipKind::Helpful | ChipKind::Harmful | ChipKind::Vitae => self.fonts.small.as_ref(),
            ChipKind::Burden | ChipKind::Chess => self.fonts.words.as_ref(),
        }
    }

    fn words_width(&self, words: &str) -> f32 {
        let tracking = if self.kind == ChipKind::Vitae {
            VITAE_TRACKING
        } else {
            0.0
        };
        self.words_font()
            .map_or(0.0, |f| text::measure(f, words, tracking))
    }

    /// Its width for what it shows (0 when hidden).
    #[must_use]
    pub fn width(&self) -> f32 {
        let Some(view) = &self.view else {
            return 0.0;
        };
        let figure = self.figure_width(&view.figure);
        let words = self.words_width(&view.words);
        match self.kind {
            ChipKind::Vitae => 10.0 + words + 6.0 + figure + 10.0,
            _ => {
                let mut w = 8.0 + ICON;
                if figure > 0.0 {
                    w += 7.0 + figure;
                }
                if words > 0.0 {
                    w += 7.0 + words;
                }
                w + 10.0
            }
        }
    }

    fn click(&self, ui: &mut UiRoot) {
        let panel = match self.kind {
            ChipKind::Helpful => panel_id::POSITIVE_EFFECTS,
            ChipKind::Harmful => panel_id::NEGATIVE_EFFECTS,
            ChipKind::Vitae => panel_id::VITAE,
            ChipKind::Burden => panel_id::INVENTORY,
            ChipKind::Chess => {
                // Brings the board back; it never puts it away.
                if ui.is_window_visible(crate::ui::modern::chess::window::CHESS) {
                    return;
                }
                panel_id::MINI_GAME
            }
        };
        emit(ui, PanelOutput::TogglePanel(panel));
    }
}

/// The spacing of the vitae chip's word.
const VITAE_TRACKING: f32 = 0.66;

impl Widget for Chip {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        self.view.is_some()
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
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
            kind::CLICK => {
                self.click(ui);
                true
            }
            kind::MOUSE_DOWN | kind::MOUSE_UP => true,
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let Some(view) = self.view.clone() else {
            return;
        };
        let _ = me;
        let p = self.theme.palette;
        let width = self.width();
        let look = view.tone.look(self.open, p.accent);
        let radius = CornerRadii::all(self.theme.metrics.radius);
        let fill = if self.hovered && !self.open {
            Color {
                a: (look.fill.a + 0.2).min(0.95),
                ..look.fill
            }
        } else {
            look.fill
        };
        rc.fill_rounded_rect(0.0, 0.0, width, CHIP_HEIGHT, radius, fill);
        rc.stroke_rounded_rect(0.0, 0.0, width, CHIP_HEIGHT, radius, look.edge, 1.0);
        let shadow = p.text_shadow;
        let mut x = 10.0;
        if self.kind != ChipKind::Vitae {
            let icon_y = (CHIP_HEIGHT - ICON) * 0.5;
            match self.kind {
                ChipKind::Helpful => shield(rc, 8.0, icon_y, p.good, true),
                ChipKind::Harmful => shield(rc, 8.0, icon_y, p.bad, false),
                ChipKind::Burden => sack(rc, 8.0, icon_y, look.ink),
                _ => board(rc, 8.0, icon_y, look.ink),
            }
            x = 8.0 + ICON + 7.0;
        }
        let centre = |font: &UiFont| ((CHIP_HEIGHT - font.line_height) * 0.5).round();
        let draw_figure = |rc: &mut RenderContext<'_>, x: f32| -> f32 {
            match &self.fonts.figure {
                Some(f) if !view.figure.is_empty() => {
                    text::draw(
                        rc,
                        f,
                        &view.figure,
                        x,
                        centre(f),
                        TextStyle::plain(p.text).shadowed(shadow),
                    );
                    text::measure(f, &view.figure, 0.0)
                }
                _ => 0.0,
            }
        };
        match self.kind {
            ChipKind::Vitae => {
                if let Some(f) = self.words_font() {
                    text::draw(
                        rc,
                        f,
                        &view.words,
                        x,
                        centre(f) + 0.5,
                        TextStyle::plain(VITAE_TEXT)
                            .tracked(VITAE_TRACKING)
                            .shadowed(shadow),
                    );
                }
                x += self.words_width(&view.words) + 6.0;
                draw_figure(rc, x);
            }
            ChipKind::Helpful | ChipKind::Harmful => {
                let w = draw_figure(rc, x);
                x += w + 7.0;
                if let Some(f) = self.words_font()
                    && !view.words.is_empty()
                {
                    text::draw(
                        rc,
                        f,
                        &view.words,
                        x,
                        centre(f) + 0.5,
                        TextStyle::plain(look.ink).shadowed(shadow),
                    );
                }
            }
            ChipKind::Burden | ChipKind::Chess => {
                if let Some(f) = self.words_font() {
                    text::draw(
                        rc,
                        f,
                        &view.words,
                        x,
                        centre(f),
                        TextStyle::plain(look.ink).shadowed(shadow),
                    );
                }
            }
        }
    }
}

/// The row of chips, and under it the frame rate when it is asked for
/// (Show frame rate, or /framerate). Takes the pointer only while quick
/// move is held (so the row can be carried by any chip or gap).
#[derive(Debug, Default)]
pub struct ChipRow {
    quick_move: bool,
    /// Some chip shows (the frame rate goes under them, else at the top).
    any_chip: bool,
    /// The frame rate's font.
    pub font: Option<Arc<UiFont>>,
    /// The frame rate line, when shown.
    pub frame_rate: Option<String>,
    theme: Horizon,
}

impl ChipRow {
    /// A row in `theme`, its frame rate in `font`.
    #[must_use]
    pub fn new(theme: Horizon, font: Option<Arc<UiFont>>) -> Self {
        Self {
            theme,
            font,
            ..Self::default()
        }
    }
}

/// The frame rate line's words: whole frames a second and the detail
/// level (retail's two lines, "FPS" and "DEG").
#[must_use]
pub fn frame_rate_words(frames_per_second: f32, detail: f32) -> String {
    format!(
        "{:.0} fps \u{b7} detail {detail:.2}",
        frames_per_second.max(0.0)
    )
}

impl Widget for ChipRow {
    fn hit_test(&self, element: &Element, x: f32, y: f32) -> bool {
        // It does not clip, so the toolkit leaves its bounds to it.
        self.quick_move && element.contains_local(x, y)
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, _dt: f64) {
        self.quick_move = ui.quick_move_held();
    }

    fn clips_children(&self) -> bool {
        false
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let (Some(words), Some(font)) = (&self.frame_rate, &self.font) else {
            return;
        };
        let p = self.theme.palette;
        let y = if self.any_chip {
            CHIP_HEIGHT + CHIP_GAP
        } else {
            0.0
        };
        text::draw(
            rc,
            font,
            words,
            0.0,
            y,
            TextStyle::plain(p.text).shadowed(p.text_shadow),
        );
    }
}

/// Lays out `chips` left to right in `row`, hiding those with nothing to
/// show; the row is sized to hold them (one chip's height square when all
/// are hidden).
pub fn lay_out(ui: &mut UiRoot, row: ElementId, chips: &[ElementId]) {
    let mut x = 0.0;
    for &id in chips {
        let width = ui.widget::<Chip>(id).map_or(0.0, Chip::width);
        let shown = width > 0.0;
        if let Some(e) = ui.element_mut(id) {
            e.left = x;
            e.top = 0.0;
            e.width = width;
            e.height = CHIP_HEIGHT;
        }
        if ui.element(id).is_some_and(|e| e.visible()) != shown {
            ui.set_visible(id, shown);
        }
        if shown {
            x += width + CHIP_GAP;
        }
    }
    if let Some(r) = ui.widget_mut::<ChipRow>(row) {
        r.any_chip = x > 0.0;
    }
    let width = (x - CHIP_GAP).max(CHIP_HEIGHT);
    if let Some(e) = ui.element_mut(row)
        && e.width != width
    {
        e.width = width;
    }
}

/// A shield with an arrow up (helpful) or down (harmful), in a 15-unit box.
fn shield(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color, up: bool) {
    let s = ICON / 24.0;
    let at = |px: f32, py: f32| (x + px * s, y + py * s);
    let outline = [
        (12.0, 3.0),
        (20.0, 6.0),
        (20.0, 12.0),
        (18.4, 16.6),
        (12.0, 21.0),
        (5.6, 16.6),
        (4.0, 12.0),
        (4.0, 6.0),
        (12.0, 3.0),
    ];
    let width = 2.2 * s;
    for pair in outline.windows(2) {
        let (a, b) = (at(pair[0].0, pair[0].1), at(pair[1].0, pair[1].1));
        rc.smooth_line(a.0, a.1, b.0, b.1, color, width);
    }
    let (top, bottom) = if up { (9.0, 16.0) } else { (8.0, 15.0) };
    let (a, b) = (at(12.0, top), at(12.0, bottom));
    rc.smooth_line(a.0, a.1, b.0, b.1, color, width);
    let (tip, wing) = if up {
        (top, top + 3.0)
    } else {
        (bottom, bottom - 3.0)
    };
    for side in [9.0, 15.0] {
        let (a, b) = (at(side, wing), at(12.0, tip));
        rc.smooth_line(a.0, a.1, b.0, b.1, color, width);
    }
}

/// A sack, in a 15-unit box.
fn sack(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
    let s = ICON / 24.0;
    let at = |px: f32, py: f32| (x + px * s, y + py * s);
    let width = 2.2 * s;
    let body = [
        (6.0, 8.0),
        (18.0, 8.0),
        (17.0, 20.0),
        (7.0, 20.0),
        (6.0, 8.0),
    ];
    for pair in body.windows(2) {
        let (a, b) = (at(pair[0].0, pair[0].1), at(pair[1].0, pair[1].1));
        rc.smooth_line(a.0, a.1, b.0, b.1, color, width);
    }
    let (cx, cy) = at(12.0, 6.0);
    rc.stroke_arc(
        cx,
        cy,
        3.0 * s,
        std::f32::consts::PI,
        std::f32::consts::PI,
        color,
        width,
    );
}

/// A chess board, in a 15-unit box.
fn board(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
    let s = ICON / 24.0;
    let width = 2.2 * s;
    rc.stroke_rounded_rect(
        x + 4.0 * s,
        y + 4.0 * s,
        16.0 * s,
        16.0 * s,
        CornerRadii::default(),
        color,
        width,
    );
    rc.smooth_line(
        x + 4.0 * s,
        y + 12.0 * s,
        x + 20.0 * s,
        y + 12.0 * s,
        color,
        width,
    );
    rc.smooth_line(
        x + 12.0 * s,
        y + 4.0 * s,
        x + 12.0 * s,
        y + 20.0 * s,
        color,
        width,
    );
    rc.fill_rounded_rect(
        x + 6.0 * s,
        y + 6.0 * s,
        4.0 * s,
        4.0 * s,
        CornerRadii::default(),
        color,
    );
    rc.fill_rounded_rect(
        x + 14.0 * s,
        y + 14.0 * s,
        4.0 * s,
        4.0 * s,
        CornerRadii::default(),
        color,
    );
}

/// The chips for `effects` (the times hidden when `show_times` is off).
#[must_use]
pub fn effect_chips(effects: &EffectsOnYou, show_times: bool, soon: f64) -> [Option<ChipView>; 3] {
    let time = |left: Option<f64>| match left {
        Some(t) if show_times => format_seconds_left(t),
        _ => String::new(),
    };
    let helpful = (!effects.helpful.is_empty()).then(|| ChipView {
        tone: if effects.helpful_ending_within(soon) > 0 {
            Tone::Soon
        } else {
            Tone::Good
        },
        figure: effects.helpful.len().to_string(),
        words: time(effects.next_helpful()),
    });
    let harmful = (!effects.harmful.is_empty()).then(|| ChipView {
        tone: Tone::Bad,
        figure: effects.harmful.len().to_string(),
        words: time(effects.next_harmful()),
    });
    let vitae = effects.vitae.map(|kept| ChipView {
        tone: Tone::Bad,
        figure: format!("{}%", vitae_percent(kept)),
        words: "VITAE".to_owned(),
    });
    [helpful, harmful, vitae]
}

// ── The pane ────────────────────────────────────────────────────────────

/// A row in the pane.
#[derive(Debug, Clone, PartialEq)]
pub struct PaneRow {
    /// The spell's icon.
    pub icon: Option<TextureRef>,
    /// The spell's name.
    pub name: String,
    /// Its time left, or "worn".
    pub time: String,
    /// The time's colour.
    pub time_tone: TimeTone,
}

/// How a row's time is coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeTone {
    /// Plainly.
    Plain,
    /// Ending soon.
    Soon,
    /// Harmful.
    Bad,
    /// Quietly (worn gear).
    Quiet,
}

/// A group of rows under a heading.
#[derive(Debug, Clone, PartialEq)]
pub struct PaneGroup {
    /// The heading, in capitals.
    pub title: &'static str,
    /// Its colour.
    pub tone: TimeTone,
    /// The rows.
    pub rows: Vec<PaneRow>,
}

/// What the pane shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PaneView {
    /// The harmful effects (else the helpful).
    pub harmful: bool,
    /// How many.
    pub count: usize,
    /// Vitae's share kept (the harmful pane starts with it).
    pub vitae: Option<f32>,
    /// The groups.
    pub groups: Vec<PaneGroup>,
    /// How long before a helpful effect ends it counts as ending soon,
    /// seconds (0: never).
    pub soon: f64,
}

impl PaneView {
    /// The helpful (or `harmful`) effects of `effects` as the pane lists
    /// them; `icon` finds each spell's icon.
    pub fn of(
        effects: &EffectsOnYou,
        harmful: bool,
        show_times: bool,
        soon: f64,
        mut icon: impl FnMut(u32) -> Option<TextureRef>,
    ) -> Self {
        let mut row = |line: &EffectLine, tone: TimeTone| PaneRow {
            icon: icon(line.spell_id),
            name: line.name.clone(),
            time: match line.left {
                Some(t) if show_times => format_seconds_left(t),
                Some(_) => String::new(),
                None => "worn".to_owned(),
            },
            time_tone: if line.left.is_none() {
                TimeTone::Quiet
            } else {
                tone
            },
        };
        let groups = if harmful {
            vec![PaneGroup {
                title: "ENDING FIRST",
                tone: TimeTone::Bad,
                rows: effects
                    .harmful
                    .iter()
                    .map(|l| row(l, TimeTone::Bad))
                    .collect(),
            }]
        } else {
            let ending = |l: &&EffectLine| l.left.is_some_and(|t| t < soon);
            let timed = |l: &&EffectLine| l.left.is_some_and(|t| t >= soon);
            vec![
                PaneGroup {
                    title: "ENDING SOON",
                    tone: TimeTone::Soon,
                    rows: effects
                        .helpful
                        .iter()
                        .filter(ending)
                        .map(|l| row(l, TimeTone::Soon))
                        .collect(),
                },
                PaneGroup {
                    title: "TIMED",
                    tone: TimeTone::Plain,
                    rows: effects
                        .helpful
                        .iter()
                        .filter(timed)
                        .map(|l| row(l, TimeTone::Plain))
                        .collect(),
                },
                PaneGroup {
                    title: "FROM YOUR GEAR",
                    tone: TimeTone::Plain,
                    rows: effects
                        .helpful
                        .iter()
                        .filter(|l| l.left.is_none())
                        .map(|l| row(l, TimeTone::Quiet))
                        .collect(),
                },
            ]
        };
        Self {
            harmful,
            count: if harmful {
                effects.harmful.len()
            } else {
                effects.helpful.len()
            },
            vitae: if harmful { effects.vitae } else { None },
            groups: groups.into_iter().filter(|g| !g.rows.is_empty()).collect(),
            soon,
        }
    }

    /// The height of the list's content.
    #[must_use]
    pub fn list_height(&self) -> f32 {
        self.groups
            .iter()
            .map(|g| GROUP_HEIGHT + ROW_HEIGHT * g.rows.len() as f32)
            .sum()
    }

    fn list_max(&self) -> f32 {
        if self.harmful {
            HARMFUL_LIST_MAX
        } else {
            HELPFUL_LIST_MAX
        }
    }
}

/// The fonts the pane uses.
#[derive(Debug, Clone, Default)]
pub struct PaneFonts {
    /// Headings, spaced capitals.
    pub label: Option<Arc<UiFont>>,
    /// Names.
    pub name: Option<Arc<UiFont>>,
    /// Times and figures.
    pub figure: Option<Arc<UiFont>>,
    /// The quiet words (the subtitle, the foot).
    pub caption: Option<Arc<UiFont>>,
}

/// The pane's inside margins.
const PAD_LEFT: f32 = 10.0;
const PAD_RIGHT: f32 = 6.0;
const PAD_TOP: f32 = 10.0;
const PAD_BOTTOM: f32 = 8.0;
/// The heading line, and the gap under it.
const HEAD_HEIGHT: f32 = 14.0;
const HEAD_GAP: f32 = 6.0;
/// The vitae card, and the gap under it.
const VITAE_HEIGHT: f32 = 28.0;
const VITAE_GAP: f32 = 4.0;
/// The foot: a gap, a rule, a gap, a line.
const FOOT_HEIGHT: f32 = 6.0 + 1.0 + 6.0 + 13.0;
/// The scrollbar's width.
const SCROLLBAR: f32 = 3.0;
/// The spacing of the headings' capitals.
const LABEL_TRACKING: f32 = 1.14;

/// The list of every effect of one kind, inside the hover pane.
#[derive(Debug)]
pub struct EffectsPane {
    theme: Horizon,
    fonts: PaneFonts,
    /// What it shows.
    view: PaneView,
    /// How far the list is scrolled.
    scroll: f32,
}

impl EffectsPane {
    /// An empty list in `fonts`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: PaneFonts) -> Self {
        Self {
            theme,
            fonts,
            view: PaneView::default(),
            scroll: 0.0,
        }
    }

    /// Shows `view`; the scroll goes back to the top when the pane changes
    /// kind, and stays (within the list) when only the times moved.
    pub fn show(&mut self, view: PaneView) {
        if view.harmful != self.view.harmful {
            self.scroll = 0.0;
        }
        self.view = view;
        self.scroll = self.scroll.clamp(0.0, self.scroll_max());
    }

    /// Back to the top of the list.
    pub fn scroll_to_top(&mut self) {
        self.scroll = 0.0;
    }

    /// The list's visible height.
    fn list_room(&self) -> f32 {
        self.view.list_height().min(self.view.list_max())
    }

    fn scroll_max(&self) -> f32 {
        (self.view.list_height() - self.list_room()).max(0.0)
    }

    fn list_top(&self) -> f32 {
        let mut y = PAD_TOP + HEAD_HEIGHT + HEAD_GAP;
        if self.view.vitae.is_some() {
            y += VITAE_HEIGHT + VITAE_GAP;
        }
        y
    }

    /// The pane's height for what it shows.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.list_top() + self.list_room() + FOOT_HEIGHT + PAD_BOTTOM
    }
}

impl Widget for EffectsPane {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        match e.kind {
            kind::SCROLL => {
                let step = -(e.data0.signum() as f32) * WHEEL_STEP;
                self.scroll = (self.scroll + step).clamp(0.0, self.scroll_max());
                true
            }
            kind::MOUSE_MOVE | kind::HOVER_ENTER | kind::HOVER_LEAVE => true,
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let right = PANE_WIDTH - PAD_RIGHT;
        let (label, name, figure, caption) = (
            self.fonts.label.clone(),
            self.fonts.name.clone(),
            self.fonts.figure.clone(),
            self.fonts.caption.clone(),
        );
        let bad = p.bad_text;
        // The heading: what and how many, and the order.
        if let Some(f) = &label {
            let words = format!(
                "{} \u{b7} {}",
                if self.view.harmful {
                    "HARMFUL"
                } else {
                    "HELPFUL"
                },
                self.view.count
            );
            let color = if self.view.harmful { bad } else { HEADING };
            text::draw(
                rc,
                f,
                &words,
                PAD_LEFT + 2.0,
                PAD_TOP + 2.0,
                TextStyle::plain(color).tracked(LABEL_TRACKING),
            );
        }
        if let Some(f) = &caption {
            let words = "soonest to end first";
            let w = text::measure(f, words, 0.0);
            text::draw(
                rc,
                f,
                words,
                right - 6.0 - w,
                PAD_TOP,
                TextStyle::plain(p.muted),
            );
        }
        // Vitae, at the top of the harmful pane.
        if let Some(kept) = self.view.vitae {
            let (x, y) = (PAD_LEFT, PAD_TOP + HEAD_HEIGHT + HEAD_GAP);
            let w = right - 4.0 - x;
            rc.fill_rounded_rect(
                x,
                y,
                w,
                VITAE_HEIGHT,
                CornerRadii::all(4.0),
                hex(0x3C_0E10, 0.45),
            );
            rc.stroke_rounded_rect(
                x,
                y,
                w,
                VITAE_HEIGHT,
                CornerRadii::all(4.0),
                hex(0xFF_6B70, 0.5),
                1.0,
            );
            let mut tx = x + 8.0;
            if let Some(f) = &label {
                let ty = y + ((VITAE_HEIGHT - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    "VITAE",
                    tx,
                    ty,
                    TextStyle::plain(bad).tracked(LABEL_TRACKING),
                );
                tx += text::measure(f, "VITAE", LABEL_TRACKING) + 8.0;
            }
            let percent = vitae_percent(kept);
            if let Some(f) = &figure {
                let ty = y + ((VITAE_HEIGHT - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    &format!("{percent}%"),
                    tx,
                    ty,
                    TextStyle::plain(p.text),
                );
            }
            if let Some(f) = &caption {
                let words = format!("health, mana, skills {}% lower", 100 - percent);
                let tw = text::measure(f, &words, 0.0);
                let ty = y + ((VITAE_HEIGHT - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    &words,
                    x + w - 8.0 - tw,
                    ty,
                    TextStyle::plain(hex(0xE9_D3AE, 1.0)),
                );
            }
        }
        // The list, clipped and scrolled.
        let top = self.list_top();
        let room = self.list_room();
        let scrolls = self.scroll_max() > 0.0;
        let list_right = if scrolls {
            right - 4.0 - SCROLLBAR
        } else {
            right - 4.0
        };
        rc.push_clip(0.0, top, PANE_WIDTH, room);
        let mut y = top - self.scroll;
        let tone_color = |tone: TimeTone| match tone {
            TimeTone::Plain => p.text,
            TimeTone::Soon => SOON,
            TimeTone::Bad => bad,
            TimeTone::Quiet => p.muted,
        };
        for group in &self.view.groups {
            if y + GROUP_HEIGHT > top && y < top + room {
                let heading = if group.tone == TimeTone::Plain {
                    HEADING
                } else {
                    tone_color(group.tone)
                };
                let mut x = PAD_LEFT + 2.0;
                if let Some(f) = &label {
                    let ty = y + ((GROUP_HEIGHT - f.line_height) * 0.5).round();
                    text::draw(
                        rc,
                        f,
                        group.title,
                        x,
                        ty,
                        TextStyle::plain(heading).tracked(LABEL_TRACKING),
                    );
                    x += text::measure(f, group.title, LABEL_TRACKING) + 8.0;
                    let count = group.rows.len().to_string();
                    text::draw(
                        rc,
                        f,
                        &count,
                        x,
                        ty,
                        TextStyle::plain(heading).tracked(LABEL_TRACKING),
                    );
                    x += text::measure(f, &count, LABEL_TRACKING) + 8.0;
                }
                rc.fill(
                    x,
                    y + (GROUP_HEIGHT * 0.5).round(),
                    (list_right - x).max(0.0),
                    1.0,
                    p.trim_faint,
                );
            }
            y += GROUP_HEIGHT;
            for row in &group.rows {
                if y + ROW_HEIGHT > top && y < top + room {
                    let x = PAD_LEFT + 4.0;
                    let iy = y + (ROW_HEIGHT - 22.0) * 0.5;
                    rc.fill_rounded_rect(x, iy, 22.0, 22.0, CornerRadii::all(3.0), p.well);
                    if let Some(icon) = row.icon {
                        rc.sprite(icon, x, iy, 22.0, 22.0, (0.0, 0.0, 1.0, 1.0), Color::WHITE);
                    }
                    let mut time_left = list_right - 4.0;
                    if let Some(f) = &figure
                        && !row.time.is_empty()
                    {
                        let w = text::measure(f, &row.time, 0.0);
                        time_left -= w;
                        let ty = y + ((ROW_HEIGHT - f.line_height) * 0.5).round();
                        text::draw(
                            rc,
                            f,
                            &row.time,
                            time_left,
                            ty,
                            TextStyle::plain(tone_color(row.time_tone)),
                        );
                    }
                    if let Some(f) = &name {
                        let nx = x + 22.0 + 8.0;
                        let shown = fit(f, &row.name, (time_left - 8.0 - nx).max(0.0));
                        let ty = y + ((ROW_HEIGHT - f.line_height) * 0.5).round();
                        text::draw(rc, f, &shown, nx, ty, TextStyle::plain(p.text));
                    }
                }
                y += ROW_HEIGHT;
            }
        }
        rc.pop_clip();
        if scrolls {
            let total = self.view.list_height();
            let thumb = (room * room / total).max(18.0);
            let travel = room - thumb;
            let at = top + travel * (self.scroll / self.scroll_max());
            let x = right - SCROLLBAR;
            rc.fill_rounded_rect(
                x,
                top,
                SCROLLBAR,
                room,
                CornerRadii::all(1.5),
                hex(0x00_0000, 0.3),
            );
            rc.fill_rounded_rect(x, at, SCROLLBAR, thumb, CornerRadii::all(1.5), p.trim_soft);
        }
        // The foot.
        let foot = top + room + 6.0;
        rc.fill(PAD_LEFT, foot, right - 4.0 - PAD_LEFT, 1.0, p.trim_faint);
        if let Some(f) = &caption {
            let ty = foot + 1.0 + 6.0;
            text::draw(
                rc,
                f,
                "Click the count for details",
                PAD_LEFT + 2.0,
                ty,
                TextStyle::plain(p.muted),
            );
            if !self.view.harmful && self.view.soon > 0.0 {
                let words = format!("Warns under {} min", (self.view.soon / 60.0).round() as u32);
                let w = text::measure(f, &words, 0.0);
                text::draw(
                    rc,
                    f,
                    &words,
                    right - 6.0 - w,
                    ty,
                    TextStyle::plain(p.muted),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(name: &str, left: Option<f64>) -> EffectLine {
        EffectLine {
            spell_id: 1,
            name: name.to_owned(),
            left,
        }
    }

    fn effects() -> EffectsOnYou {
        EffectsOnYou {
            helpful: vec![
                line("Focus Self", Some(220.0)),
                line("Armor Other", Some(2468.0)),
                line("Epic Strength", None),
            ],
            harmful: vec![line("Imperil Other VI", Some(22.0))],
            vitae: Some(0.94),
        }
    }

    #[test]
    fn the_warning_time_is_a_setting() {
        let e = effects();
        let [never, _, _] = effect_chips(&e, true, 0.0);
        assert_ne!(never.map(|c| c.tone), Some(Tone::Soon));
        let [long, _, _] = effect_chips(&e, true, 1.0e9);
        assert_eq!(long.map(|c| c.tone), Some(Tone::Soon));
    }

    #[test]
    fn the_counts_say_how_many_and_the_next_to_end() {
        let [helpful, harmful, vitae] = effect_chips(&effects(), true, DEFAULT_SOON_SECONDS);
        let helpful = helpful.unwrap();
        assert_eq!(
            (helpful.figure.as_str(), helpful.words.as_str()),
            ("3", "3:40")
        );
        assert_eq!(helpful.tone, Tone::Soon);
        let harmful = harmful.unwrap();
        assert_eq!(
            (harmful.figure.as_str(), harmful.words.as_str()),
            ("1", "0:22")
        );
        assert_eq!(vitae.unwrap().figure, "94%");
    }

    #[test]
    fn nothing_ending_soon_keeps_the_helpful_count_calm() {
        let mut e = effects();
        e.helpful.remove(0);
        let [helpful, _, _] = effect_chips(&e, true, DEFAULT_SOON_SECONDS);
        let helpful = helpful.unwrap();
        assert_eq!(helpful.tone, Tone::Good);
        assert_eq!(helpful.words, "41:08");
    }

    #[test]
    fn hidden_durations_leave_only_the_counts() {
        let [helpful, harmful, _] = effect_chips(&effects(), false, DEFAULT_SOON_SECONDS);
        assert_eq!(helpful.unwrap().words, "");
        assert_eq!(harmful.unwrap().words, "");
    }

    #[test]
    fn no_effects_no_chips() {
        assert_eq!(
            effect_chips(&EffectsOnYou::default(), true, DEFAULT_SOON_SECONDS),
            [None, None, None]
        );
    }

    #[test]
    fn the_helpful_pane_groups_ending_soon_timed_and_gear() {
        let pane = PaneView::of(&effects(), false, true, DEFAULT_SOON_SECONDS, |_| None);
        let titles: Vec<_> = pane.groups.iter().map(|g| g.title).collect();
        assert_eq!(titles, ["ENDING SOON", "TIMED", "FROM YOUR GEAR"]);
        assert_eq!(pane.groups[2].rows[0].time, "worn");
        assert_eq!(pane.groups[2].rows[0].time_tone, TimeTone::Quiet);
        assert_eq!(pane.vitae, None);
        assert_eq!(pane.list_height(), 3.0 * GROUP_HEIGHT + 3.0 * ROW_HEIGHT);
    }

    #[test]
    fn the_harmful_pane_starts_with_vitae_and_leaves_out_empty_groups() {
        let pane = PaneView::of(&effects(), true, true, DEFAULT_SOON_SECONDS, |_| None);
        assert_eq!(pane.vitae, Some(0.94));
        assert_eq!(pane.groups.len(), 1);
        assert_eq!(pane.groups[0].rows[0].time, "0:22");
        let mut e = effects();
        e.helpful.retain(|l| l.left.is_none());
        let pane = PaneView::of(&e, false, true, DEFAULT_SOON_SECONDS, |_| None);
        assert_eq!(pane.groups.len(), 1);
    }

    #[test]
    fn a_long_list_scrolls_within_its_room() {
        let e = EffectsOnYou {
            helpful: (0..40)
                .map(|i| line(&format!("Buff {i}"), Some(1000.0 + f64::from(i))))
                .collect(),
            ..EffectsOnYou::default()
        };
        let mut pane = EffectsPane::new(Horizon::BRONZE, PaneFonts::default());
        pane.show(PaneView::of(&e, false, true, DEFAULT_SOON_SECONDS, |_| {
            None
        }));
        assert_eq!(pane.list_room(), HELPFUL_LIST_MAX);
        assert!(pane.scroll_max() > 0.0);
        let mut ui = UiRoot::new();
        let root = ui.root();
        let wheel = |data0| {
            let mut ev = UiEvent::new(0, root, kind::SCROLL);
            ev.data0 = data0;
            ev
        };
        pane.on_event(&mut ui, root, &wheel(-1));
        assert_eq!(pane.scroll, WHEEL_STEP);
        pane.on_event(&mut ui, root, &wheel(1));
        pane.on_event(&mut ui, root, &wheel(1));
        assert_eq!(pane.scroll, 0.0);
    }

    #[test]
    fn vitae_never_rounds_up_to_whole() {
        assert_eq!(vitae_percent(0.94), 94);
        assert_eq!(vitae_percent(0.996), 99);
        assert_eq!(vitae_percent(1.0), 100);
    }

    #[test]
    fn burden_shows_past_a_full_load() {
        assert_eq!(burden_tone(0.8), None);
        assert_eq!(burden_tone(1.0), Some(Tone::Heavy));
        assert_eq!(burden_tone(1.42), Some(Tone::Heavy));
        assert_eq!(burden_tone(2.15), Some(Tone::VeryHeavy));
        assert_eq!(burden_words(1.42), "Burden 142%");
    }

    #[test]
    fn the_frame_rate_is_one_line() {
        assert_eq!(frame_rate_words(61.7, 1.0), "62 fps \u{b7} detail 1.00");
    }

    #[test]
    fn chess_says_whose_move_it_is() {
        assert_eq!(chess_words(ChessStage::NotPlaying), None);
        assert_eq!(
            chess_words(ChessStage::MyTurn),
            Some("Chess \u{b7} your move")
        );
        assert_eq!(
            chess_words(ChessStage::OpponentsTurn),
            Some("Chess \u{b7} their move")
        );
    }
}
