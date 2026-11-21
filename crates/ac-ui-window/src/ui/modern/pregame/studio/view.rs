//! The Studio's full-screen view: the figure on a lit plinth with Turn and
//! Zoom under it and Back · Step N of 6 · Next (Finish on the last step);
//! a numbered step rail across the top; each step's choices in a glass
//! panel on the left with a quiet Random; a live "So far" card on the
//! right; Randomize all and Exit (both ask) top right; and the questions
//! as cards over the stage.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_runtime::session::creation::{ChargenAttribute, UNSET};
use ac_ui::draw::{Color, ColorVertex, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::{fill_glass, hex};
use ac_ui::pregame::chargen::appearance::{Part, PreviewRequest};
use ac_ui::pregame::chargen::skills::Bucket;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::{Group, SkillRow, Step, StudioScreen};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, GOOD, HARM, Hits, Rect, bronze, gold, put, put_mid, put_right};
use crate::ui::modern::pregame::scenery::{self, Sky};
use crate::ui::modern::pregame::stage::spinner;

/// The fonts the Studio draws with.
#[derive(Debug, Clone, Default)]
pub struct StudioFonts {
    /// The page title.
    pub title: Option<Arc<UiFont>>,
    /// The step's heading, the name field.
    pub heading: Option<Arc<UiFont>>,
    /// Buttons, names, values.
    pub strong: Option<Arc<UiFont>>,
    /// Descriptions.
    pub body: Option<Arc<UiFont>>,
    /// Second lines, notes.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals, the OPENAC tag.
    pub label: Option<Arc<UiFont>>,
}

/// A question over the stage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StudioCard {
    /// None.
    #[default]
    None,
    /// Leave creation?
    Exit,
    /// Randomize everything?
    RandomAll,
    /// Attribute points left over.
    Unspent(i32),
    /// No name typed.
    NoName,
    /// The server refused the name: its words and near names.
    Refused(String, Vec<String>),
    /// Waiting for the server.
    Creating(String),
}

/// What the Studio asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum StudioAsk {
    /// Show a step.
    Go(Step),
    /// The step before (or ask to leave on the first).
    Back,
    /// The next step.
    Next,
    /// Create the character.
    Finish,
    /// Create it with attribute points left over.
    FinishAnyway,
    /// Pick a heritage.
    Heritage(u32),
    /// Pick a gender.
    Gender(u32),
    /// Pick a profession.
    Template(u32),
    /// Set an attribute.
    Attribute(ChargenAttribute, i32),
    /// Lock or unlock an attribute.
    Lock(ChargenAttribute, bool),
    /// Show a skill's details.
    PickSkill(u32),
    /// Train or specialize a skill.
    Raise(u32),
    /// Untrain a skill or take its specialization back.
    Lower(u32),
    /// Show the face or the clothes.
    Group(Group),
    /// Pick a part.
    Part(Part),
    /// Step the part's style.
    StepStyle(i32),
    /// Pick a style (from 0, or none).
    Style(u32),
    /// Pick a colour.
    Color(usize),
    /// Set the part's shade.
    Shade(f32),
    /// Pick a starting town.
    Town(i32),
    /// Use a suggested or near name.
    Name(String),
    /// Use a near name and finish at once.
    TryName(String),
    /// Pick at random for the step.
    RandomStep,
    /// Ask to randomize everything.
    AskRandomAll,
    /// Randomize everything.
    RandomAll,
    /// Ask to leave.
    AskExit,
    /// Leave.
    Exit,
    /// Close the card.
    CloseCard,
    /// Turn or zoom the figure.
    Preview(PreviewRequest),
}

#[derive(Debug, Clone, PartialEq)]
enum Hit {
    Ask(StudioAsk),
    Slider(Slider),
    Swallow,
}

/// A slider being dragged.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Slider {
    Attribute(ChargenAttribute, Rect),
    Shade(Rect),
}

/// A line drawing in the 24-unit box.
type Glyph = &'static [&'static [(f32, f32)]];
/// A card's words and buttons: title, text, yes, no, whether it warns.
type CardParts<'a> = (
    String,
    String,
    Option<(&'a str, StudioAsk)>,
    Option<(&'a str, StudioAsk)>,
    bool,
);

const PANEL_W: f32 = 420.0;
const CARD_W: f32 = 300.0;
const TOP: f32 = 84.0;
const EDGE: f32 = 24.0;

const TICK: &[&[(f32, f32)]] = &[&[(5.0, 12.5), (9.5, 17.0), (19.0, 7.5)]];
const LEFT: &[&[(f32, f32)]] = &[&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]];
const RIGHT: &[&[(f32, f32)]] = &[&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]];
const DICE: &[&[(f32, f32)]] = &[
    &[
        (4.0, 4.0),
        (20.0, 4.0),
        (20.0, 20.0),
        (4.0, 20.0),
        (4.0, 4.0),
    ],
    &[(8.5, 8.5), (8.6, 8.6)],
    &[(15.5, 15.5), (15.6, 15.6)],
    &[(12.0, 12.0), (12.1, 12.1)],
];
const CROSS: &[&[(f32, f32)]] = &[&[(6.0, 6.0), (18.0, 18.0)], &[(18.0, 6.0), (6.0, 18.0)]];
const PLUS: &[&[(f32, f32)]] = &[&[(12.0, 5.0), (12.0, 19.0)], &[(5.0, 12.0), (19.0, 12.0)]];
const MINUS: &[&[(f32, f32)]] = &[&[(5.0, 12.0), (19.0, 12.0)]];
const UP: &[&[(f32, f32)]] = &[&[(6.0, 15.0), (12.0, 9.0), (18.0, 15.0)]];
const DOWN: &[&[(f32, f32)]] = &[&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]];
const LOCK: &[&[(f32, f32)]] = &[
    &[
        (7.0, 11.0),
        (7.0, 8.0),
        (9.0, 4.5),
        (12.0, 3.0),
        (15.0, 4.5),
        (17.0, 8.0),
        (17.0, 11.0),
    ],
    &[
        (5.0, 11.0),
        (19.0, 11.0),
        (19.0, 21.0),
        (5.0, 21.0),
        (5.0, 11.0),
    ],
];
const OPEN: &[&[(f32, f32)]] = &[
    &[
        (7.0, 11.0),
        (7.0, 8.0),
        (9.0, 4.5),
        (12.0, 3.0),
        (15.0, 4.0),
        (16.6, 6.0),
    ],
    &[
        (5.0, 11.0),
        (19.0, 11.0),
        (19.0, 21.0),
        (5.0, 21.0),
        (5.0, 11.0),
    ],
];
const TURN_LEFT: &[&[(f32, f32)]] = &[
    &[
        (3.0, 12.0),
        (5.6, 18.4),
        (12.0, 21.0),
        (18.4, 18.4),
        (21.0, 12.0),
        (18.4, 5.6),
        (12.0, 3.0),
        (6.0, 5.3),
    ],
    &[(3.0, 4.0), (3.0, 9.0), (8.0, 9.0)],
];
const TURN_RIGHT: &[&[(f32, f32)]] = &[
    &[
        (21.0, 12.0),
        (18.4, 18.4),
        (12.0, 21.0),
        (5.6, 18.4),
        (3.0, 12.0),
        (5.6, 5.6),
        (12.0, 3.0),
        (18.0, 5.3),
    ],
    &[(21.0, 4.0), (21.0, 9.0), (16.0, 9.0)],
];

/// The Studio.
#[derive(Debug)]
pub struct StudioView {
    fonts: StudioFonts,
    /// What it shows.
    pub screen: StudioScreen,
    /// The pictures behind it (none: the drawn sky).
    pub backdrop: crate::ui::modern::pregame::backdrops::Backdrop,
    /// The card over the stage.
    pub card: StudioCard,
    /// The screen's size.
    pub size: (f32, f32),
    /// The picked part's colours.
    pub swatches: Vec<Color>,
    /// The colours along the picked part's shade.
    pub shade_stops: Vec<Color>,
    /// The picked skill.
    pub skill: Option<u32>,
    /// What it asked for since the window last looked.
    pub asks: Vec<StudioAsk>,
    /// The window's clock (the spinner).
    pub clock: f64,
    /// The skills list's scroll.
    pub skills_scroll: kit::Scroll,
    hits: Hits<Hit>,
    sliding: Option<Slider>,
    /// The figure is being dragged round: the pointer's last x.
    turning: Option<f32>,
    /// Where the skills list is (for the wheel).
    skills_room: Rect,
}

impl StudioView {
    /// A Studio in `theme`.
    #[must_use]
    pub fn new(fonts: StudioFonts) -> Self {
        Self {
            fonts,
            screen: StudioScreen::default(),
            backdrop: crate::ui::modern::pregame::backdrops::Backdrop::default(),
            card: StudioCard::None,
            size: (1440.0, 900.0),
            swatches: Vec::new(),
            shade_stops: Vec::new(),
            skill: None,
            asks: Vec::new(),
            clock: 0.0,
            skills_scroll: kit::Scroll::default(),
            hits: Hits::default(),
            sliding: None,
            turning: None,
            skills_room: (0.0, 0.0, 0.0, 0.0),
        }
    }

    /// The left panel.
    #[must_use]
    pub fn panel_rect(&self) -> Rect {
        (EDGE, TOP, PANEL_W, self.size.1 - TOP - EDGE)
    }

    /// The "So far" card's column.
    #[must_use]
    pub fn card_column(&self) -> Rect {
        (
            self.size.0 - EDGE - CARD_W,
            TOP,
            CARD_W,
            self.size.1 - TOP - EDGE,
        )
    }

    /// The middle of the stage, across.
    #[must_use]
    pub fn stage_centre(&self) -> f32 {
        let left = EDGE + PANEL_W;
        let right = self.size.0 - EDGE - CARD_W;
        (left + right) * 0.5
    }

    /// Where the figure is drawn.
    #[must_use]
    pub fn figure_rect(&self) -> Rect {
        let cx = self.stage_centre();
        let top = 120.0;
        let bottom = self.size.1 - 128.0;
        let w = (self.size.0 - 2.0 * (EDGE + 8.0) - PANEL_W - CARD_W).clamp(200.0, 520.0);
        (cx - w * 0.5, top, w, (bottom - top).max(80.0))
    }

    /// The name field's box on the name step.
    #[must_use]
    pub fn name_rect(&self) -> Rect {
        let p = self.panel_rect();
        (p.0 + 18.0, p.1 + 124.0, p.2 - 36.0, 46.0)
    }

    /// Back, the step count and Next (or Finish).
    #[must_use]
    pub fn foot_buttons(&self) -> (Rect, Rect) {
        let cx = self.stage_centre();
        let y = self.size.1 - 20.0 - 40.0;
        (
            (cx - 70.0 - 110.0, y, 110.0, 40.0),
            (cx + 70.0, y, 130.0, 40.0),
        )
    }

    fn controls(&self) -> [Rect; 4] {
        let cx = self.stage_centre();
        let y = self.size.1 - 76.0 - 34.0 + 8.0;
        let w = 34.0 * 4.0 + 8.0 * 3.0;
        let x = cx - w * 0.5;
        [0.0, 1.0, 2.0, 3.0].map(|i| (x + i * 42.0, y, 34.0, 34.0))
    }

    fn top_buttons(&self) -> (Rect, Rect) {
        let f = self.fonts.strong.as_ref();
        let exit_w = kit::button_width("Exit", f) + 22.0;
        let rand_w = kit::button_width("Randomize all", f) + 22.0;
        let exit = (self.size.0 - EDGE - exit_w, 16.0, exit_w, 34.0);
        ((exit.0 - 8.0 - rand_w, 16.0, rand_w, 34.0), exit)
    }

    fn card_rect(&self, height: f32) -> Rect {
        let cx = self.stage_centre();
        (
            cx - 220.0,
            ((self.size.1 - height) * 0.45).round(),
            440.0,
            height,
        )
    }

    /// The card's two buttons (left: the quiet one).
    #[must_use]
    pub fn card_buttons(&self, height: f32) -> (Rect, Rect) {
        let c = self.card_rect(height);
        let y = c.1 + c.3 - 18.0 - 34.0;
        let right = (c.0 + c.2 - 18.0 - 150.0, y, 150.0, 34.0);
        ((right.0 - 8.0 - 150.0, y, 150.0, 34.0), right)
    }

    fn draw_top(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = &self.fonts;
        let p = Horizon::current().palette;
        rc.fill_vertical_gradient(
            0.0,
            0.0,
            self.size.0,
            64.0,
            CornerRadii::all(0.0),
            tone::deep(0.75),
            tone::deep(0.0),
        );
        put(rc, f.title.as_ref(), "New character", EDGE, 13.0, p.text);
        put(
            rc,
            f.small.as_ref(),
            &self.screen.subtitle,
            EDGE,
            38.0,
            tone::muted(1.0),
        );
        let (rand, exit) = self.top_buttons();
        for (r, words, glyph, ask) in [
            (rand, "Randomize all", DICE, StudioAsk::AskRandomAll),
            (exit, "Exit", CROSS, StudioAsk::AskExit),
        ] {
            let lit = h.add(r, Hit::Ask(ask));
            quiet_button(rc, r, lit);
            stroke(
                rc,
                r.0 + 11.0,
                r.1 + 10.0,
                15.0,
                glyph,
                tone::warm(1.0),
                2.0,
            );
            put_mid(rc, f.strong.as_ref(), words, r.0 + 33.0, r.1, r.3, p.text);
        }
        // The rail.
        let steps = Step::ALL;
        let widths: Vec<f32> = steps
            .iter()
            .map(|s| {
                f.strong
                    .as_ref()
                    .map_or(70.0, |ft| text::measure(ft, s.word(), 0.0))
                    + 46.0
            })
            .collect();
        let joint = 18.0;
        let total: f32 = widths.iter().sum::<f32>() + joint * (steps.len() - 1) as f32;
        let mut x = (self.stage_centre() - total * 0.5).max(EDGE + 200.0);
        let y = 16.0;
        let at = self
            .screen
            .steps
            .iter()
            .position(|s| *s == self.screen.step)
            .unwrap_or(0);
        for (i, (step, w)) in steps.iter().zip(&widths).enumerate() {
            let pos = self.screen.steps.iter().position(|s| s == step);
            let off = pos.is_none();
            let current = *step == self.screen.step;
            let done = pos.is_some_and(|p| p < at);
            if i > 0 {
                let c = if done || current {
                    gold(0.7)
                } else {
                    bronze(0.25)
                };
                rc.fill(x - joint + 3.0, y + 16.0, joint - 6.0, 1.0, c);
            }
            let r = (x, y, *w, 32.0);
            let lit = !off && h.add(r, Hit::Ask(StudioAsk::Go(*step)));
            let round = CornerRadii::all(16.0);
            let bg = if current {
                gold(0.16)
            } else {
                tone::deep(0.55)
            };
            let edge = if current {
                gold(1.0)
            } else if off {
                bronze(0.15)
            } else {
                bronze(if lit { 0.6 } else { 0.35 })
            };
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, bg);
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, edge, 1.0);
            let dot = (r.0 + 6.0, r.1 + 5.0, 22.0, 22.0);
            let dot_bg = if done {
                gold(1.0)
            } else if current {
                gold(0.9)
            } else {
                hex(0xFF_FFFF, 0.08)
            };
            rc.fill_ellipse(dot.0, dot.1, dot.2, dot.3, dot_bg);
            let fg = if off {
                hex(0x6E_665C, 1.0)
            } else if current {
                p.text
            } else {
                tone::warm(1.0)
            };
            if done {
                stroke(
                    rc,
                    dot.0 + 5.0,
                    dot.1 + 5.0,
                    12.0,
                    TICK,
                    tone::ink(1.0),
                    2.6,
                );
            } else if let Some(sm) = &f.small {
                let n = (i + 1).to_string();
                let nw = text::measure(sm, &n, 0.0);
                let ink = if current { tone::ink(1.0) } else { fg };
                text::draw(
                    rc,
                    sm,
                    &n,
                    (dot.0 + (dot.2 - nw) * 0.5).round(),
                    dot.1 + ((dot.3 - sm.line_height) * 0.5).round(),
                    TextStyle::plain(ink),
                );
            }
            put_mid(
                rc,
                f.strong.as_ref(),
                step.word(),
                dot.0 + 30.0,
                r.1,
                r.3,
                fg,
            );
            x += w + joint;
        }
    }

    fn draw_stage(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = &self.fonts;
        let p = Horizon::current().palette;
        let cx = self.stage_centre();
        scenery::plinth(rc, cx, self.size.1 - 134.0, 440.0, 76.0);
        let glyphs: [(Glyph, PreviewRequest); 4] = [
            (TURN_LEFT, PreviewRequest::RotateCounterClockwise),
            (MINUS, PreviewRequest::ZoomOut),
            (PLUS, PreviewRequest::ZoomIn),
            (TURN_RIGHT, PreviewRequest::RotateClockwise),
        ];
        for (r, (glyph, ask)) in self.controls().into_iter().zip(glyphs) {
            let lit = h.add(r, Hit::Ask(StudioAsk::Preview(ask)));
            quiet_button(rc, r, lit);
            stroke(
                rc,
                r.0 + 10.0,
                r.1 + 10.0,
                14.0,
                glyph,
                tone::warm(1.0),
                2.2,
            );
        }
        let (back, next) = self.foot_buttons();
        let first = self.screen.steps.first() == Some(&self.screen.step);
        let lit = h.add(back, Hit::Ask(StudioAsk::Back));
        quiet_button(rc, back, lit);
        stroke(
            rc,
            back.0 + 16.0,
            back.1 + 13.0,
            14.0,
            LEFT,
            if first {
                tone::muted(1.0)
            } else {
                tone::warm(1.0)
            },
            2.4,
        );
        put_mid(
            rc,
            f.strong.as_ref(),
            "Back",
            back.0 + 40.0,
            back.1,
            back.3,
            if first { tone::muted(1.0) } else { p.text },
        );
        let at = self
            .screen
            .steps
            .iter()
            .position(|s| *s == self.screen.step)
            .unwrap_or(0);
        let count = format!("Step {} of {}", at + 1, self.screen.steps.len());
        if let Some(sm) = &f.small {
            let w = text::measure(sm, &count, 0.0);
            put_mid(
                rc,
                Some(sm),
                &count,
                (cx - w * 0.5).round(),
                back.1,
                back.3,
                tone::soft(1.0),
            );
        }
        let last = self.screen.step == Step::Name;
        let words = if last { "Finish" } else { "Next" };
        let ask = if last {
            StudioAsk::Finish
        } else {
            StudioAsk::Next
        };
        let lit = h.add(next, Hit::Ask(ask));
        let round = CornerRadii::all(5.0);
        rc.fill_rounded_rect(
            next.0,
            next.1,
            next.2,
            next.3,
            round,
            gold(if lit { 1.0 } else { 0.92 }),
        );
        let ink = tone::ink(1.0);
        if let Some(ft) = &f.strong {
            let w = text::measure(ft, words, 0.0) + 22.0;
            let x = (next.0 + (next.2 - w) * 0.5).round();
            put_mid(rc, Some(ft), words, x, next.1, next.3, ink);
            stroke(rc, x + w - 16.0, next.1 + 13.0, 14.0, RIGHT, ink, 2.6);
        }
    }

    fn panel_head(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>, p: Rect) -> f32 {
        let f = &self.fonts;
        let pal = Horizon::current().palette;
        let (kicker, title, sub) = match self.screen.step {
            Step::Heritage => (
                "",
                "Who are you?",
                "Heritage changes how you look and your two starting masteries.",
            ),
            Step::Profession => (
                "",
                "Profession and attributes",
                "A profession is a starting set. Change anything you like.",
            ),
            Step::Skills => (
                "",
                "Skills",
                "Spend credits to train and specialize. What you keep carries into the game.",
            ),
            Step::Appearance => (
                "",
                "Appearance",
                "Only looks. A barber can change the face later.",
            ),
            Step::Town => (
                "",
                "Where do you start?",
                "You arrive at the training academy, then step out into this town.",
            ),
            Step::Name => (
                "LAST STEP",
                "Name your character",
                "Others will see this name above you.",
            ),
        };
        let at = self
            .screen
            .steps
            .iter()
            .position(|s| *s == self.screen.step)
            .unwrap_or(0);
        let kicker = if kicker.is_empty() {
            format!("STEP {}", at + 1)
        } else {
            kicker.to_owned()
        };
        kit::spaced(rc, f.label.as_ref(), &kicker, p.0 + 18.0, p.1 + 16.0);
        put(
            rc,
            f.heading.as_ref(),
            title,
            p.0 + 18.0,
            p.1 + 30.0,
            pal.text,
        );
        let mut y = p.1 + 60.0;
        if let Some(sm) = &f.small {
            for l in kit::wrap(sm, sub, p.2 - 36.0 - 96.0) {
                put(rc, Some(sm), &l, p.0 + 18.0, y, tone::muted(1.0));
                y += 16.0;
            }
        }
        // Skills have nothing to pick at random (retail's neither).
        if self.screen.step != Step::Skills {
            let r = (p.0 + p.2 - 16.0 - 86.0, p.1 + 16.0, 86.0, 28.0);
            let lit = h.add(r, Hit::Ask(StudioAsk::RandomStep));
            quiet_button(rc, r, lit);
            stroke(rc, r.0 + 9.0, r.1 + 7.5, 13.0, DICE, tone::warm(1.0), 2.0);
            put_mid(
                rc,
                f.small.as_ref(),
                "Random",
                r.0 + 29.0,
                r.1,
                r.3,
                pal.text,
            );
        }
        rc.fill(p.0, y + 10.0, p.2, 1.0, bronze(0.2));
        y + 22.0
    }

    #[allow(clippy::too_many_lines)]
    fn draw_heritage(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Hit>,
        x: f32,
        mut y: f32,
        w: f32,
    ) {
        let f = &self.fonts;
        let s = &self.screen;
        let pal = Horizon::current().palette;
        // The gender, beside the heritages.
        let seg = (x, y, w, 34.0);
        rc.fill_rounded_rect(
            seg.0,
            seg.1,
            seg.2,
            seg.3,
            CornerRadii::all(5.0),
            hex(0, 0.3),
        );
        rc.stroke_rounded_rect(
            seg.0,
            seg.1,
            seg.2,
            seg.3,
            CornerRadii::all(5.0),
            bronze(0.35),
            1.0,
        );
        let n = s.genders.len().max(1) as f32;
        for (i, (key, name)) in s.genders.iter().enumerate() {
            let r = (
                seg.0 + 3.0 + i as f32 * (seg.2 - 6.0) / n,
                seg.1 + 3.0,
                (seg.2 - 6.0) / n,
                28.0,
            );
            let on = *key == s.gender;
            let lit = h.add(r, Hit::Ask(StudioAsk::Gender(*key)));
            if on {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), gold(0.9));
            } else if lit {
                rc.fill_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(4.0),
                    hex(0xFF_FFFF, 0.05),
                );
            }
            if let Some(ft) = &f.strong {
                let tw = text::measure(ft, name, 0.0);
                put_mid(
                    rc,
                    Some(ft),
                    name,
                    (r.0 + (r.2 - tw) * 0.5).round(),
                    r.1,
                    r.3,
                    if on { tone::ink(1.0) } else { tone::warm(1.0) },
                );
            }
        }
        y += 46.0;
        let col = (w - 8.0) * 0.5;
        for (i, tile) in s.heritages.iter().enumerate() {
            let r = (
                x + (i % 2) as f32 * (col + 8.0),
                y + (i / 2) as f32 * 50.0,
                col,
                44.0,
            );
            let on = tile.id == s.heritage;
            let lit = h.add(r, Hit::Ask(StudioAsk::Heritage(tile.id)));
            kit::pick_row(rc, r, on, lit);
            if !on {
                rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), bronze(0.3), 1.0);
            }
            put(
                rc,
                f.strong.as_ref(),
                &tile.name,
                r.0 + 10.0,
                r.1 + 6.0,
                pal.text,
            );
            if let Some(sm) = &f.small {
                put(
                    rc,
                    Some(sm),
                    &kit::fit(sm, &tile.sub, r.2 - 20.0),
                    r.0 + 10.0,
                    r.1 + 24.0,
                    if tile.pk { HARM } else { tone::muted(1.0) },
                );
            }
        }
        y += s.heritages.len().div_ceil(2) as f32 * 50.0 + 6.0;
        // The heritage's card.
        let card = (
            x,
            y,
            w,
            self.panel_rect().1 + self.panel_rect().3 - 18.0 - y,
        );
        kit::panel(rc, card, 0.35);
        let mut cy = card.1 + 12.0;
        put(rc, f.strong.as_ref(), &s.who, card.0 + 12.0, cy, pal.text);
        if s.olthoi
            && let Some(ft) = &f.strong
        {
            let tx = card.0 + 12.0 + text::measure(ft, &s.who, 0.0) + 10.0;
            kit::pill(
                rc,
                f.label.as_ref(),
                tx + 74.0,
                cy + 9.0,
                "ALWAYS PK",
                HARM,
                hex(0xFF_6B70, 0.6),
            );
        }
        cy += 24.0;
        if let Some(b) = &f.small {
            let mut lines: Vec<(String, Color)> = Vec::new();
            for para in &s.heritage_text {
                for l in kit::wrap(b, para, card.2 - 24.0) {
                    lines.push((l, tone::warm(1.0)));
                }
                lines.push((String::new(), tone::warm(1.0)));
            }
            if !s.starting_skills.is_empty() && !s.olthoi {
                for l in kit::wrap(
                    b,
                    &format!("Trained from the start: {}", s.starting_skills),
                    card.2 - 24.0,
                ) {
                    lines.push((l, tone::soft(1.0)));
                }
                lines.push((String::new(), tone::warm(1.0)));
            }
            if let Some(note) = s.heritage_note {
                for l in kit::wrap(b, note, card.2 - 24.0) {
                    lines.push((l, gold(0.9)));
                }
            }
            rc.push_clip(card.0, card.1, card.2, card.3 - 8.0);
            for (l, c) in lines {
                if !l.is_empty() {
                    put(rc, Some(b), &l, card.0 + 12.0, cy, c);
                }
                cy += if l.is_empty() { 6.0 } else { 15.5 };
            }
            rc.pop_clip();
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_profession(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Hit>,
        x: f32,
        mut y: f32,
        w: f32,
    ) {
        let f = &self.fonts;
        let s = &self.screen;
        let pal = Horizon::current().palette;
        let col = (w - 8.0) * 0.5;
        for (i, (index, name)) in s.templates.iter().enumerate() {
            let r = (
                x + (i % 2) as f32 * (col + 8.0),
                y + (i / 2) as f32 * 40.0,
                col,
                34.0,
            );
            let on = *index == s.template;
            let lit = h.add(r, Hit::Ask(StudioAsk::Template(*index)));
            kit::pick_row(rc, r, on, lit);
            if !on {
                rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), bronze(0.3), 1.0);
            }
            put_mid(
                rc,
                f.strong.as_ref(),
                name,
                r.0 + 12.0,
                r.1,
                r.3,
                if on { pal.text } else { tone::warm(1.0) },
            );
        }
        y += s.templates.len().div_ceil(2) as f32 * 40.0 + 4.0;
        if let Some(b) = &f.small {
            for l in kit::wrap(b, &s.template_text, w).iter().take(3) {
                put(rc, Some(b), l, x, y, tone::soft(1.0));
                y += 15.5;
            }
        }
        y += 8.0;
        kit::spaced(rc, f.label.as_ref(), "ATTRIBUTES", x, y + 4.0);
        let (left, total) = s.points;
        let words = if left == 0 {
            format!("All {total} points spent")
        } else {
            format!("{left} of {total} points left")
        };
        put_right(
            rc,
            f.small.as_ref(),
            &words,
            x + w,
            y,
            16.0,
            if left == 0 {
                tone::muted(1.0)
            } else {
                gold(1.0)
            },
        );
        y += 20.0;
        let spent = total as f32 - left as f32;
        kit::meter(
            rc,
            (x, y, w, 4.0),
            if total > 0 { spent / total as f32 } else { 0.0 },
            gold(1.0),
        );
        y += 12.0;
        for row in &s.attributes {
            let r = (x, y, w, 30.0);
            let lock = (r.0, r.1 + 4.0, 22.0, 22.0);
            let lit = h.add(lock, Hit::Ask(StudioAsk::Lock(row.attribute, !row.locked)));
            let round = CornerRadii::all(4.0);
            if row.locked {
                rc.fill_rounded_rect(lock.0, lock.1, lock.2, lock.3, round, gold(0.18));
            }
            rc.stroke_rounded_rect(
                lock.0,
                lock.1,
                lock.2,
                lock.3,
                round,
                if row.locked {
                    gold(1.0)
                } else {
                    bronze(if lit { 0.6 } else { 0.35 })
                },
                1.0,
            );
            stroke(
                rc,
                lock.0 + 5.0,
                lock.1 + 5.0,
                12.0,
                if row.locked { LOCK } else { OPEN },
                if row.locked {
                    gold(1.0)
                } else {
                    tone::muted(1.0)
                },
                2.0,
            );
            put_mid(
                rc,
                f.strong.as_ref(),
                row.name,
                r.0 + 32.0,
                r.1,
                r.3,
                pal.text,
            );
            let track = (r.0 + 132.0, r.1 + 13.0, w - 132.0 - 92.0, 4.0);
            let share = ((row.value - 10) as f32 / 90.0).clamp(0.0, 1.0);
            rc.fill_rounded_rect(
                track.0,
                track.1,
                track.2,
                track.3,
                CornerRadii::all(2.0),
                hex(0xFF_FFFF, 0.1),
            );
            rc.fill_rounded_rect(
                track.0,
                track.1,
                track.2 * share,
                track.3,
                CornerRadii::all(2.0),
                if row.locked { bronze(0.6) } else { gold(1.0) },
            );
            let knob = (track.0 + track.2 * share - 7.0, r.1 + 8.0, 14.0, 14.0);
            rc.fill_ellipse(
                knob.0,
                knob.1,
                knob.2,
                knob.3,
                if row.locked {
                    tone::soft(1.0)
                } else {
                    gold(1.0)
                },
            );
            if !row.locked {
                h.add(
                    (track.0 - 8.0, r.1, track.2 + 16.0, r.3),
                    Hit::Slider(Slider::Attribute(row.attribute, track)),
                );
            }
            // The value with a step down and up.
            let minus = (r.0 + w - 84.0, r.1 + 4.0, 22.0, 22.0);
            let plus = (r.0 + w - 22.0, r.1 + 4.0, 22.0, 22.0);
            for (b, glyph, value) in [(minus, MINUS, row.value - 1), (plus, PLUS, row.value + 1)] {
                let lit =
                    !row.locked && h.add(b, Hit::Ask(StudioAsk::Attribute(row.attribute, value)));
                rc.stroke_rounded_rect(
                    b.0,
                    b.1,
                    b.2,
                    b.3,
                    CornerRadii::all(4.0),
                    bronze(if lit { 0.7 } else { 0.3 }),
                    1.0,
                );
                stroke(
                    rc,
                    b.0 + 5.0,
                    b.1 + 5.0,
                    12.0,
                    glyph,
                    if row.locked {
                        tone::muted(1.0)
                    } else {
                        tone::warm(1.0)
                    },
                    2.2,
                );
            }
            if let Some(ft) = &f.strong {
                let v = row.value.to_string();
                let vw = text::measure(ft, &v, 0.0);
                put_mid(
                    rc,
                    Some(ft),
                    &v,
                    (r.0 + w - 42.0 - vw * 0.5).round(),
                    r.1,
                    r.3,
                    pal.text,
                );
            }
            y += 32.0;
        }
        if let Some(sm) = &f.small {
            for l in kit::wrap(
                sm,
                "Moving one attribute takes from the others. Lock the ones you want to keep.",
                w,
            ) {
                put(rc, Some(sm), &l, x, y + 2.0, tone::muted(1.0));
                y += 15.0;
            }
        }
        y += 10.0;
        vitals(rc, f, (x, y, w, 46.0), s.vitals, s.olthoi);
    }

    #[allow(clippy::too_many_lines)]
    fn draw_skills(
        &mut self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Hit>,
        x: f32,
        mut y: f32,
        w: f32,
    ) {
        let f = self.fonts.clone();
        let pal = Horizon::current().palette;
        let (left, total) = self.screen.credits;
        kit::spaced(rc, f.label.as_ref(), "SKILL CREDITS", x, y + 4.0);
        put_right(
            rc,
            f.small.as_ref(),
            &format!("{left} of {total} left"),
            x + w,
            y,
            16.0,
            gold(1.0),
        );
        y += 20.0;
        kit::meter(
            rc,
            (x, y, w, 4.0),
            if total > 0 {
                (total as f32 - left as f32) / total as f32
            } else {
                0.0
            },
            gold(1.0),
        );
        y += 10.0;
        put(
            rc,
            f.small.as_ref(),
            &format!(
                "Specialized: {} of {} credits at most",
                self.screen.specialized,
                super::model::SPECIALIZED_LIMIT
            ),
            x,
            y,
            tone::muted(1.0),
        );
        y += 22.0;
        let info_h = 150.0;
        let bottom = self.panel_rect().1 + self.panel_rect().3 - 18.0;
        let room = (x, y, w, (bottom - info_h - 10.0 - y).max(60.0));
        self.skills_room = room;
        let total_h: f32 = self
            .screen
            .skills
            .iter()
            .map(|r| match r {
                SkillRow::Head(..) => 26.0,
                SkillRow::Skill(_) => 30.0,
            })
            .sum();
        self.skills_scroll.hold(total_h, room.3);
        rc.push_clip(room.0, room.1, room.2 + 2.0, room.3);
        let mut ry = room.1 - self.skills_scroll.at;
        let picked = self.skill;
        for row in &self.screen.skills {
            match row {
                SkillRow::Head(words, count) => {
                    if ry + 26.0 > room.1 && ry < room.1 + room.3 {
                        let c = match *words {
                            "SPECIALIZED" => gold(1.0),
                            "TRAINED" => tone::warm(1.0),
                            "USABLE UNTRAINED" => tone::soft(1.0),
                            _ => hex(0x8E_8577, 1.0),
                        };
                        if let Some(lf) = &f.label {
                            text::draw(
                                rc,
                                lf,
                                words,
                                x,
                                ry + 9.0,
                                TextStyle::plain(c).tracked(kit::LABEL_TRACKING),
                            );
                        }
                        put_right(
                            rc,
                            f.small.as_ref(),
                            count,
                            x + w - 6.0,
                            ry + 4.0,
                            18.0,
                            tone::muted(1.0),
                        );
                    }
                    ry += 26.0;
                }
                SkillRow::Skill(line) => {
                    let r = (x, ry, w - 6.0, 28.0);
                    if ry + 30.0 > room.1 && ry < room.1 + room.3 {
                        let on = picked == Some(line.id);
                        let lit = kit::inside(
                            room,
                            h.hover.map_or(-1.0, |p| p.0),
                            h.hover.map_or(-1.0, |p| p.1),
                        ) && h.add(
                            (r.0, r.1, r.2 - 120.0, r.3),
                            Hit::Ask(StudioAsk::PickSkill(line.id)),
                        );
                        if on {
                            rc.fill_rounded_rect(
                                r.0,
                                r.1,
                                r.2,
                                r.3,
                                CornerRadii::all(4.0),
                                gold(0.1),
                            );
                            rc.stroke_rounded_rect(
                                r.0,
                                r.1,
                                r.2,
                                r.3,
                                CornerRadii::all(4.0),
                                gold(0.6),
                                1.0,
                            );
                        } else if lit {
                            rc.fill_rounded_rect(
                                r.0,
                                r.1,
                                r.2,
                                r.3,
                                CornerRadii::all(4.0),
                                hex(0xFF_FFFF, 0.04),
                            );
                        }
                        let unusable = line.bucket == Bucket::UnuseableUntrained;
                        put_mid(
                            rc,
                            f.strong.as_ref(),
                            &line.name,
                            r.0 + 10.0,
                            r.1,
                            r.3,
                            if unusable { tone::muted(1.0) } else { pal.text },
                        );
                        let score = line.score.map_or_else(|| "—".to_owned(), |v| v.to_string());
                        put_right(
                            rc,
                            f.strong.as_ref(),
                            &score,
                            r.0 + r.2 - 122.0,
                            r.1,
                            r.3,
                            if line.score.is_some() {
                                pal.text
                            } else {
                                hex(0x6E_665C, 1.0)
                            },
                        );
                        let lower = (r.0 + r.2 - 112.0, r.1 + 3.0, 52.0, 22.0);
                        let raise = (r.0 + r.2 - 56.0, r.1 + 3.0, 52.0, 22.0);
                        if let Some(refund) = line.lower {
                            let lit = h.add(lower, Hit::Ask(StudioAsk::Lower(line.id)));
                            small_step(
                                rc,
                                f.small.as_ref(),
                                lower,
                                DOWN,
                                &format!("+{refund}"),
                                tone::warm(1.0),
                                lit,
                                true,
                            );
                        }
                        if let Some((cost, can)) = line.raise {
                            let lit = can && h.add(raise, Hit::Ask(StudioAsk::Raise(line.id)));
                            let words = if cost == 0 {
                                "free".to_owned()
                            } else {
                                cost.to_string()
                            };
                            small_step(
                                rc,
                                f.small.as_ref(),
                                raise,
                                UP,
                                &words,
                                gold(1.0),
                                lit,
                                can,
                            );
                        }
                    }
                    ry += 30.0;
                }
            }
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            room.0 + room.2 + 2.0,
            room.1,
            room.3,
            total_h,
            self.skills_scroll.at,
        );
        // The picked skill.
        let info = (x, bottom - info_h, w, info_h);
        kit::panel(rc, info, 0.35);
        let si = &self.screen.skill_info;
        put(
            rc,
            f.strong.as_ref(),
            &si.name,
            info.0 + 12.0,
            info.1 + 10.0,
            pal.text,
        );
        if let Some(ft) = &f.strong {
            let nx = info.0 + 12.0 + text::measure(ft, &si.name, 0.0) + 10.0;
            put(
                rc,
                f.small.as_ref(),
                si.state,
                nx,
                info.1 + 12.0,
                tone::muted(1.0),
            );
        }
        put_right(
            rc,
            f.strong.as_ref(),
            &si.score,
            info.0 + info.2 - 12.0,
            info.1 + 6.0,
            24.0,
            pal.text,
        );
        let mut iy = info.1 + 32.0;
        if let Some(sm) = &f.small {
            for l in kit::wrap(sm, &si.text, info.2 - 24.0).iter().take(3) {
                put(rc, Some(sm), l, info.0 + 12.0, iy, tone::warm(1.0));
                iy += 15.0;
            }
            iy += 4.0;
            for (label, value) in [
                ("Formula", si.formula.as_str()),
                ("Cost", si.cost.as_str()),
                ("Training bonus", "+5 trained, +10 specialized"),
            ] {
                if iy + 15.0 > info.1 + info.3 {
                    break;
                }
                put(rc, Some(sm), label, info.0 + 12.0, iy, tone::muted(1.0));
                put(rc, Some(sm), value, info.0 + 112.0, iy, tone::soft(1.0));
                iy += 16.0;
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_appearance(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Hit>,
        x: f32,
        mut y: f32,
        w: f32,
    ) {
        let f = &self.fonts;
        let s = &self.screen;
        let pal = Horizon::current().palette;
        if s.has_clothes {
            let seg = (x, y, w, 34.0);
            rc.fill_rounded_rect(
                seg.0,
                seg.1,
                seg.2,
                seg.3,
                CornerRadii::all(5.0),
                hex(0, 0.3),
            );
            rc.stroke_rounded_rect(
                seg.0,
                seg.1,
                seg.2,
                seg.3,
                CornerRadii::all(5.0),
                bronze(0.35),
                1.0,
            );
            for (i, (g, words)) in [(Group::Face, "Face"), (Group::Clothes, "Clothes")]
                .into_iter()
                .enumerate()
            {
                let r = (
                    seg.0 + 3.0 + i as f32 * (seg.2 - 6.0) * 0.5,
                    seg.1 + 3.0,
                    (seg.2 - 6.0) * 0.5,
                    28.0,
                );
                let on = s.group == g;
                let lit = h.add(r, Hit::Ask(StudioAsk::Group(g)));
                if on {
                    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), gold(0.9));
                } else if lit {
                    rc.fill_rounded_rect(
                        r.0,
                        r.1,
                        r.2,
                        r.3,
                        CornerRadii::all(4.0),
                        hex(0xFF_FFFF, 0.05),
                    );
                }
                if let Some(ft) = &f.strong {
                    let tw = text::measure(ft, words, 0.0);
                    put_mid(
                        rc,
                        Some(ft),
                        words,
                        (r.0 + (r.2 - tw) * 0.5).round(),
                        r.1,
                        r.3,
                        if on { tone::ink(1.0) } else { tone::warm(1.0) },
                    );
                }
            }
            y += 46.0;
        } else if let Some(sm) = &f.small {
            let note = if s.olthoi {
                "Olthoi pick a body style and colour only."
            } else {
                "Gear Knights have no face and wear no clothes. The look you pick is the whole body, and armour does not show on it."
            };
            for l in kit::wrap(sm, note, w) {
                put(rc, Some(sm), &l, x, y, gold(0.9));
                y += 15.0;
            }
            y += 10.0;
        }
        // The parts.
        let n = s.parts.len().max(1) as f32;
        let tw = (w - 4.0 * (n - 1.0)) / n;
        for (i, tab) in s.parts.iter().enumerate() {
            let r = (x + i as f32 * (tw + 4.0), y, tw, 50.0);
            let on = s.part == Some(tab.part);
            let lit = h.add(r, Hit::Ask(StudioAsk::Part(tab.part)));
            kit::pick_row(rc, r, on, lit);
            if !on {
                rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(4.0), bronze(0.3), 1.0);
            }
            if let Some(ft) = &f.strong {
                let lw = text::measure(ft, &tab.label, 0.0);
                put(
                    rc,
                    Some(ft),
                    &tab.label,
                    (r.0 + (r.2 - lw) * 0.5).round(),
                    r.1 + 9.0,
                    pal.text,
                );
            }
            if let Some(sm) = &f.small {
                let sub = kit::fit(sm, &tab.sub, r.2 - 8.0);
                let sw = text::measure(sm, &sub, 0.0);
                put(
                    rc,
                    Some(sm),
                    &sub,
                    (r.0 + (r.2 - sw) * 0.5).round(),
                    r.1 + 28.0,
                    tone::muted(1.0),
                );
            }
        }
        y += 64.0;
        let c = &s.choices;
        if let Some((now, count)) = c.style {
            kit::spaced(rc, f.label.as_ref(), "STYLE", x, y + 9.0);
            let prev = (x + w - 160.0, y, 28.0, 28.0);
            let next = (x + w - 28.0, y, 28.0, 28.0);
            for (b, glyph, step) in [(prev, LEFT, -1), (next, RIGHT, 1)] {
                let lit = h.add(b, Hit::Ask(StudioAsk::StepStyle(step)));
                quiet_button(rc, b, lit);
                stroke(rc, b.0 + 7.0, b.1 + 7.0, 14.0, glyph, tone::warm(1.0), 2.4);
            }
            let words = if now == 0 {
                "None".to_owned()
            } else {
                format!("{now} of {count}")
            };
            if let Some(ft) = &f.strong {
                let ww = text::measure(ft, &words, 0.0);
                put_mid(
                    rc,
                    Some(ft),
                    &words,
                    (prev.0 + 28.0 + (next.0 - prev.0 - 28.0 - ww) * 0.5).round(),
                    y,
                    28.0,
                    pal.text,
                );
            }
            y += 38.0;
            // Eight to a row, up to three rows, paged round the picked one
            // (the stepper reaches the rest).
            let per_row = 8usize;
            let page = per_row * 3;
            let first = now.saturating_sub(1) / page * page;
            let pick_w = (w - 4.0 * (per_row as f32 - 1.0)) / per_row as f32;
            let mut shown: Vec<Option<usize>> =
                (first..(first + page).min(count)).map(Some).collect();
            if c.may_be_none {
                shown.push(None);
            }
            for (i, pick) in shown.iter().enumerate() {
                let r = (
                    x + (i % per_row) as f32 * (pick_w + 4.0),
                    y + (i / per_row) as f32 * 38.0,
                    pick_w,
                    34.0,
                );
                let on = match pick {
                    Some(n) => *n + 1 == now,
                    None => now == 0,
                };
                let value = pick.map_or(UNSET, |n| u32::try_from(n).unwrap_or(0));
                let lit = h.add(r, Hit::Ask(StudioAsk::Style(value)));
                kit::pick_row(rc, r, on, lit);
                if !on {
                    rc.stroke_rounded_rect(
                        r.0,
                        r.1,
                        r.2,
                        r.3,
                        CornerRadii::all(4.0),
                        bronze(0.3),
                        1.0,
                    );
                }
                let words = pick.map_or_else(|| "None".to_owned(), |n| (n + 1).to_string());
                if let Some(ft) = &f.strong {
                    let ww = text::measure(ft, &words, 0.0);
                    put_mid(
                        rc,
                        Some(ft),
                        &words,
                        (r.0 + (r.2 - ww) * 0.5).round(),
                        r.1,
                        r.3,
                        if on { pal.text } else { tone::warm(1.0) },
                    );
                }
            }
            y += shown.len().div_ceil(per_row) as f32 * 38.0 + 10.0;
        }
        if let Some((count, picked)) = c.colors {
            kit::spaced(rc, f.label.as_ref(), "COLOUR", x, y + 4.0);
            y += 20.0;
            for i in 0..count {
                let r = (x + i as f32 * 42.0, y, 34.0, 34.0);
                let color = self
                    .swatches
                    .get(i)
                    .copied()
                    .unwrap_or(Color::rgba(0.5, 0.5, 0.5, 1.0));
                let on = picked == Some(i);
                let lit = h.add(r, Hit::Ask(StudioAsk::Color(i)));
                rc.fill_ellipse(r.0 + 3.0, r.1 + 3.0, r.2 - 6.0, r.3 - 6.0, color);
                let ring = if on {
                    gold(1.0)
                } else if lit {
                    bronze(0.8)
                } else {
                    hex(0, 0.5)
                };
                rc.stroke_ellipse(
                    r.0 + 1.0,
                    r.1 + 1.0,
                    r.2 - 2.0,
                    r.3 - 2.0,
                    ring,
                    if on { 2.0 } else { 1.0 },
                );
            }
            y += 46.0;
        }
        if let Some(value) = c.shade {
            kit::spaced(rc, f.label.as_ref(), "SHADE", x, y + 4.0);
            put_right(
                rc,
                f.small.as_ref(),
                &format!("{}%", (value * 100.0).round()),
                x + w,
                y,
                16.0,
                tone::soft(1.0),
            );
            y += 22.0;
            let track = (x + 7.0, y, w - 14.0, 12.0);
            shade_track(rc, track, &self.shade_stops);
            let kx = track.0 + track.2 * value;
            rc.fill_ellipse(kx - 8.0, track.1 - 2.0, 16.0, 16.0, tone::words(1.0));
            rc.stroke_ellipse(kx - 8.0, track.1 - 2.0, 16.0, 16.0, hex(0, 0.5), 1.0);
            h.add(
                (track.0 - 8.0, track.1 - 8.0, track.2 + 16.0, track.3 + 16.0),
                Hit::Slider(Slider::Shade(track)),
            );
            y += 26.0;
        }
        if let (Some(note), Some(sm)) = (c.note, f.small.as_ref()) {
            for l in kit::wrap(sm, note, w) {
                put(rc, Some(sm), &l, x, y, tone::muted(1.0));
                y += 15.0;
            }
        }
    }

    fn draw_town(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>, x: f32, mut y: f32, w: f32) {
        let f = &self.fonts;
        let s = &self.screen;
        let pal = Horizon::current().palette;
        let bottom = self.panel_rect().1 + self.panel_rect().3 - 60.0;
        let card_h = ((bottom - y - 3.0 * 10.0) / 4.0).clamp(96.0, 140.0);
        for town in &s.towns {
            let r = (x, y, w, card_h);
            let on = town.index == s.town;
            let lit = h.add(r, Hit::Ask(StudioAsk::Town(town.index)));
            let round = CornerRadii::all(5.0);
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0, 0.28));
            let band = (r.0 + 1.0, r.1 + 1.0, r.2 - 2.0, 44.0);
            let (sky, land) = town_colors(town.index);
            rc.fill_vertical_gradient(
                band.0,
                band.1,
                band.2,
                band.3,
                CornerRadii::new(4.0, 4.0, 0.0, 0.0),
                sky.0,
                sky.1,
            );
            skyline(rc, band, town.index, land);
            rc.stroke_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                round,
                if on {
                    gold(1.0)
                } else {
                    bronze(if lit { 0.6 } else { 0.3 })
                },
                1.0,
            );
            if on {
                rc.fill_ellipse(r.0 + r.2 - 32.0, r.1 + 10.0, 22.0, 22.0, gold(1.0));
                stroke(
                    rc,
                    r.0 + r.2 - 27.0,
                    r.1 + 15.0,
                    12.0,
                    TICK,
                    tone::ink(1.0),
                    2.6,
                );
            }
            let ty = band.1 + band.3 + 8.0;
            put(rc, f.strong.as_ref(), &town.name, r.0 + 12.0, ty, pal.text);
            if let Some(ft) = &f.strong {
                let nx = r.0 + 12.0 + text::measure(ft, &town.name, 0.0) + 8.0;
                put(
                    rc,
                    f.small.as_ref(),
                    town.home,
                    nx,
                    ty + 2.0,
                    tone::muted(1.0),
                );
            }
            if let Some(sm) = &f.small {
                let room = ((r.1 + r.3 - (ty + 20.0)) / 15.0).floor().max(0.0) as usize;
                for (i, l) in kit::wrap(sm, &town.text, r.2 - 24.0)
                    .iter()
                    .take(room)
                    .enumerate()
                {
                    put(
                        rc,
                        Some(sm),
                        l,
                        r.0 + 12.0,
                        ty + 20.0 + i as f32 * 15.0,
                        tone::soft(1.0),
                    );
                }
            }
            y += card_h + 10.0;
        }
        if let Some(sm) = &f.small {
            for l in kit::wrap(
                sm,
                "Any heritage may start in any town. The towns are joined by portals, so the others are a short trip away.",
                w,
            ) {
                put(rc, Some(sm), &l, x, y + 2.0, tone::muted(1.0));
                y += 15.0;
            }
        }
    }

    fn draw_name(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Hit>,
        x: f32,
        w: f32,
        field_empty: bool,
    ) {
        let f = &self.fonts;
        let s = &self.screen;
        let pal = Horizon::current().palette;
        let nr = self.name_rect();
        let mut y = nr.1 - 20.0;
        kit::spaced(rc, f.label.as_ref(), "NAME", x, y + 4.0);
        put_right(
            rc,
            f.small.as_ref(),
            &format!("{} / {}", s.name.chars().count(), super::model::NAME_LIMIT),
            x + w,
            y,
            16.0,
            tone::muted(1.0),
        );
        kit::field_box(
            rc,
            &Horizon::current(),
            nr,
            "Type a name",
            field_empty,
            true,
            f.heading.as_ref(),
        );
        y = nr.1 + nr.3 + 6.0;
        if let Some(sm) = &f.small {
            for l in kit::wrap(
                sm,
                "Letters, spaces, apostrophes and hyphens, up to 32. The server says whether it is free when you finish.",
                w,
            ) {
                put(rc, Some(sm), &l, x, y, tone::muted(1.0));
                y += 15.0;
            }
        }
        y += 12.0;
        if !s.suggestions.is_empty() {
            kit::spaced(
                rc,
                f.label.as_ref(),
                &format!(
                    "{} NAMES, AS THE GAME GIVES THEM",
                    s.who.split_whitespace().last().unwrap_or("").to_uppercase()
                ),
                x,
                y + 4.0,
            );
            y += 22.0;
            let mut cx = x;
            for name in &s.suggestions {
                let cw = f
                    .small
                    .as_ref()
                    .map_or(80.0, |sm| text::measure(sm, name, 0.0))
                    + 20.0;
                if cx + cw > x + w {
                    cx = x;
                    y += 32.0;
                }
                let r = (cx, y, cw, 26.0);
                let lit = h.add(r, Hit::Ask(StudioAsk::Name(name.clone())));
                rc.fill_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(13.0),
                    if lit { gold(0.14) } else { hex(0, 0.3) },
                );
                rc.stroke_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(13.0),
                    bronze(if lit { 0.8 } else { 0.4 }),
                    1.0,
                );
                put_mid(rc, f.small.as_ref(), name, r.0 + 10.0, r.1, r.3, pal.text);
                cx += cw + 6.0;
            }
            y += 42.0;
        }
        kit::spaced(rc, f.label.as_ref(), "BEFORE YOU FINISH", x, y + 4.0);
        y += 24.0;
        for check in &s.checks {
            let dot = (x, y + 2.0, 20.0, 20.0);
            rc.fill_ellipse(
                dot.0,
                dot.1,
                dot.2,
                dot.3,
                if check.ok {
                    hex(0x7F_D08A, 0.9)
                } else {
                    tone::gold_light(0.2)
                },
            );
            if check.ok {
                stroke(
                    rc,
                    dot.0 + 4.0,
                    dot.1 + 4.0,
                    12.0,
                    TICK,
                    hex(0x10_200F, 1.0),
                    2.6,
                );
            } else {
                rc.fill(dot.0 + 9.3, dot.1 + 5.0, 1.6, 7.0, tone::gold_light(1.0));
                rc.fill(dot.0 + 9.3, dot.1 + 14.0, 1.6, 1.6, tone::gold_light(1.0));
            }
            put_mid(
                rc,
                f.body.as_ref(),
                &check.text,
                x + 30.0,
                y,
                24.0,
                if check.ok {
                    pal.text
                } else {
                    tone::gold_light(1.0)
                },
            );
            if !check.ok
                && let Some(step) = check.fix
            {
                let r = (x + w - 50.0, y, 50.0, 24.0);
                let lit = h.add(r, Hit::Ask(StudioAsk::Go(step)));
                quiet_button(rc, r, lit);
                put_mid(rc, f.small.as_ref(), "Fix", r.0 + 15.0, r.1, r.3, pal.text);
            }
            y += 30.0;
        }
        let _ = GOOD;
    }

    #[allow(clippy::too_many_lines)]
    fn draw_so_far(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = &self.fonts;
        let s = &self.screen;
        let pal = Horizon::current().palette;
        let col = self.card_column();
        let name_step = s.step == Step::Name;
        let mut height = 16.0 + 30.0 + s.facts.len() as f32 * 34.0 + 14.0;
        if !s.olthoi {
            height += 24.0 + 6.0 * 20.0 + 60.0;
        }
        if name_step && !s.chosen_skills.is_empty() {
            height += 30.0 + s.chosen_skills.len() as f32 * 20.0;
        }
        if !s.olthoi {
            height += 34.0;
        } else {
            height += 40.0;
        }
        let r = (col.0, col.1, col.2, height.min(col.3));
        let round = CornerRadii::all(6.0);
        rc.soft_shadow(r.0, r.1, r.2, r.3, 6.0, 18.0, 50.0, hex(0, 0.45));
        fill_glass(rc, r, round, tone::ground(0.82));
        rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, bronze(0.5), 1.0);
        h.add(r, Hit::Swallow);
        rc.push_clip(r.0, r.1, r.2, r.3);
        let x = r.0 + 18.0;
        let w = r.2 - 36.0;
        let mut y = r.1 + 16.0;
        kit::spaced(
            rc,
            f.label.as_ref(),
            if name_step { "SUMMARY" } else { "SO FAR" },
            x,
            y + 4.0,
        );
        put_right(
            rc,
            f.small.as_ref(),
            "Click a line to change it",
            x + w,
            y,
            16.0,
            tone::muted(1.0),
        );
        y += 28.0;
        for fact in &s.facts {
            let fr = (x - 6.0, y, w + 12.0, 30.0);
            let lit = fact.go.is_some()
                && h.add(fr, Hit::Ask(StudioAsk::Go(fact.go.unwrap_or_default())));
            if lit {
                rc.fill_rounded_rect(
                    fr.0,
                    fr.1,
                    fr.2,
                    fr.3,
                    CornerRadii::all(4.0),
                    hex(0xFF_FFFF, 0.05),
                );
            }
            put_mid(
                rc,
                f.small.as_ref(),
                fact.label,
                x,
                y,
                30.0,
                tone::muted(1.0),
            );
            let dim =
                fact.go.is_none() || fact.value.starts_with("Not ") || fact.value.ends_with(" yet");
            if let Some(ft) = &f.strong {
                let v = kit::fit(ft, &fact.value, w - 90.0);
                put_right(
                    rc,
                    Some(ft),
                    &v,
                    x + w,
                    y,
                    30.0,
                    if dim { tone::muted(1.0) } else { pal.text },
                );
            }
            rc.fill(x, y + 32.0, w, 1.0, bronze(0.12));
            y += 34.0;
        }
        y += 10.0;
        if !s.olthoi {
            kit::spaced(rc, f.label.as_ref(), "ATTRIBUTES", x, y + 4.0);
            y += 22.0;
            for a in &s.attributes {
                put_mid(rc, f.small.as_ref(), a.name, x, y, 18.0, tone::soft(1.0));
                let bar = (x + 92.0, y + 7.0, w - 92.0 - 36.0, 4.0);
                rc.fill_rounded_rect(
                    bar.0,
                    bar.1,
                    bar.2,
                    bar.3,
                    CornerRadii::all(2.0),
                    hex(0xFF_FFFF, 0.1),
                );
                rc.fill_rounded_rect(
                    bar.0,
                    bar.1,
                    bar.2 * (a.value as f32 / 100.0).clamp(0.0, 1.0),
                    bar.3,
                    CornerRadii::all(2.0),
                    gold(1.0),
                );
                put_right(
                    rc,
                    f.strong.as_ref(),
                    &a.value.to_string(),
                    x + w,
                    y,
                    18.0,
                    pal.text,
                );
                y += 20.0;
            }
            y += 8.0;
            vitals(rc, f, (x, y, w, 46.0), s.vitals, false);
            y += 56.0;
        }
        if name_step && !s.chosen_skills.is_empty() {
            kit::spaced(rc, f.label.as_ref(), "SKILLS", x, y + 4.0);
            y += 22.0;
            for (name, spec, score) in &s.chosen_skills {
                rc.fill_ellipse(
                    x,
                    y + 6.0,
                    6.0,
                    6.0,
                    if *spec { gold(1.0) } else { tone::warm(1.0) },
                );
                put_mid(rc, f.small.as_ref(), name, x + 14.0, y, 18.0, pal.text);
                put_right(
                    rc,
                    f.small.as_ref(),
                    &score.to_string(),
                    x + w,
                    y,
                    18.0,
                    pal.text,
                );
                put_right(
                    rc,
                    f.small.as_ref(),
                    if *spec { "spec." } else { "trained" },
                    x + w - 40.0,
                    y,
                    18.0,
                    tone::muted(1.0),
                );
                y += 20.0;
            }
            y += 8.0;
        }
        if s.olthoi {
            if let Some(sm) = &f.small {
                for l in kit::wrap(
                    sm,
                    "Olthoi have no profession, skills or starting town to pick. The server sets them.",
                    w,
                ) {
                    put(rc, Some(sm), &l, x, y, tone::muted(1.0));
                    y += 15.0;
                }
            }
        } else {
            put_mid(
                rc,
                f.small.as_ref(),
                "Skill credits left",
                x,
                y,
                24.0,
                tone::soft(1.0),
            );
            put_right(
                rc,
                f.strong.as_ref(),
                &s.credits.0.to_string(),
                x + w,
                y,
                24.0,
                gold(1.0),
            );
        }
        rc.pop_clip();
    }

    fn draw_card(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = &self.fonts;
        let pal = Horizon::current().palette;
        let (title, words, yes, no, danger): CardParts<'_> = match &self.card {
            StudioCard::None => return,
            StudioCard::Exit => (
                "Leave character creation?".to_owned(),
                "Nothing you picked is kept.".to_owned(),
                Some(("Exit", StudioAsk::Exit)),
                Some(("Keep creating", StudioAsk::CloseCard)),
                true,
            ),
            StudioCard::RandomAll => (
                "Randomize everything?".to_owned(),
                "This picks a new heritage, profession, skills, look and town at random, resetting what you changed. Your name stays.".to_owned(),
                Some(("Randomize", StudioAsk::RandomAll)),
                Some(("Cancel", StudioAsk::CloseCard)),
                false,
            ),
            StudioCard::Unspent(points) => (
                "Attribute points left".to_owned(),
                format!("You have {points} attribute points left. If you do not spend them now you lose them."),
                Some(("Go to attributes", StudioAsk::Go(Step::Profession))),
                Some(("Finish anyway", StudioAsk::FinishAnyway)),
                false,
            ),
            StudioCard::NoName => (
                "No name".to_owned(),
                "Please type a name for your character.".to_owned(),
                Some(("OK", StudioAsk::CloseCard)),
                None,
                false,
            ),
            StudioCard::Refused(text, _) => (
                "Name not available".to_owned(),
                text.clone(),
                Some(("Change the name", StudioAsk::CloseCard)),
                None,
                true,
            ),
            StudioCard::Creating(name) => (
                format!("Creating {name}"),
                "Asking the server to make your character.".to_owned(),
                None,
                None,
                false,
            ),
        };
        let near = match &self.card {
            StudioCard::Refused(_, near) if !near.is_empty() => Some(near.clone()),
            _ => None,
        };
        let text_lines = f
            .body
            .as_ref()
            .map_or_else(Vec::new, |b| kit::wrap(b, &words, 404.0));
        let mut height = 60.0 + text_lines.len() as f32 * 20.0 + 18.0;
        if near.is_some() {
            height += 84.0;
        }
        if matches!(self.card, StudioCard::Creating(_)) {
            height += 30.0;
        } else {
            height += 52.0;
        }
        let (w, hh) = self.size;
        rc.fill(0.0, 0.0, w, hh, hex(0x06_0402, 0.55));
        h.add((0.0, 0.0, w, hh), Hit::Swallow);
        let c = self.card_rect(height);
        let round = CornerRadii::all(6.0);
        rc.soft_shadow(c.0, c.1, c.2, c.3, 6.0, 24.0, 70.0, hex(0, 0.6));
        rc.fill_rounded_rect(c.0, c.1, c.2, c.3, round, tone::raised(0.97));
        rc.stroke_rounded_rect(
            c.0,
            c.1,
            c.2,
            c.3,
            round,
            if danger {
                hex(0xFF_6B70, 0.6)
            } else {
                bronze(0.8)
            },
            1.0,
        );
        h.add(c, Hit::Swallow);
        let disc = (c.0 + 18.0, c.1 + 16.0, 30.0, 30.0);
        rc.fill_ellipse(
            disc.0,
            disc.1,
            disc.2,
            disc.3,
            if danger {
                hex(0xFF_6B70, 0.16)
            } else {
                gold(0.16)
            },
        );
        stroke(
            rc,
            disc.0 + 7.0,
            disc.1 + 7.0,
            16.0,
            if danger { CROSS } else { DICE },
            if danger { HARM } else { gold(1.0) },
            2.0,
        );
        put_mid(
            rc,
            f.strong.as_ref(),
            &title,
            disc.0 + 40.0,
            disc.1,
            disc.3,
            pal.text,
        );
        let mut y = c.1 + 60.0;
        for l in &text_lines {
            put(rc, f.body.as_ref(), l, c.0 + 18.0, y, tone::warm(1.0));
            y += 20.0;
        }
        if let Some(names) = near {
            y += 8.0;
            kit::spaced(
                rc,
                f.label.as_ref(),
                "CLOSE TO WHAT YOU TYPED",
                c.0 + 18.0,
                y + 4.0,
            );
            if let Some(lf) = &f.label {
                let tw = text::measure(lf, "CLOSE TO WHAT YOU TYPED", kit::LABEL_TRACKING);
                kit::openac_tag(rc, Some(lf), c.0 + 18.0 + tw + 10.0, y + 9.0);
            }
            y += 22.0;
            let mut x = c.0 + 18.0;
            for name in names {
                let cw = f
                    .small
                    .as_ref()
                    .map_or(80.0, |sm| text::measure(sm, &name, 0.0))
                    + 22.0;
                let r = (x, y, cw, 28.0);
                let lit = h.add(r, Hit::Ask(StudioAsk::TryName(name.clone())));
                rc.fill_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(14.0),
                    if lit { gold(0.16) } else { hex(0, 0.3) },
                );
                rc.stroke_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(14.0),
                    bronze(if lit { 0.8 } else { 0.45 }),
                    1.0,
                );
                put_mid(rc, f.small.as_ref(), &name, r.0 + 11.0, r.1, r.3, pal.text);
                x += cw + 6.0;
            }
            y += 34.0;
            put(
                rc,
                f.small.as_ref(),
                "Picking one tries it with the server at once.",
                c.0 + 18.0,
                y,
                tone::muted(1.0),
            );
        }
        if matches!(self.card, StudioCard::Creating(_)) {
            spinner(rc, c.0 + 30.0, c.1 + c.3 - 28.0, 9.0, self.clock);
            put_mid(
                rc,
                f.small.as_ref(),
                "Waiting for the server",
                c.0 + 48.0,
                c.1 + c.3 - 40.0,
                24.0,
                tone::soft(1.0),
            );
            return;
        }
        let (left, right) = self.card_buttons(height);
        if let Some((words, ask)) = no {
            let lit = h.add(left, Hit::Ask(ask));
            kit::button(
                rc,
                &Horizon::current(),
                left,
                words,
                f.strong.as_ref(),
                kit::ButtonLook::Plain,
                lit,
                true,
            );
        }
        if let Some((words, ask)) = yes {
            let lit = h.add(right, Hit::Ask(ask));
            let look = if danger && matches!(self.card, StudioCard::Exit) {
                kit::ButtonLook::Danger
            } else {
                kit::ButtonLook::Primary
            };
            kit::button(
                rc,
                &Horizon::current(),
                right,
                words,
                f.strong.as_ref(),
                look,
                lit,
                true,
            );
        }
    }
}

/// A quiet button's back.
fn quiet_button(rc: &mut RenderContext<'_>, r: Rect, lit: bool) {
    let round = CornerRadii::all(4.0);
    rc.fill_rounded_rect(
        r.0,
        r.1,
        r.2,
        r.3,
        round,
        tone::ground(if lit { 0.85 } else { 0.6 }),
    );
    rc.stroke_rounded_rect(
        r.0,
        r.1,
        r.2,
        r.3,
        round,
        bronze(if lit { 0.8 } else { 0.45 }),
        1.0,
    );
}

/// A skill's small raise or lower button.
#[allow(clippy::too_many_arguments)]
fn small_step(
    rc: &mut RenderContext<'_>,
    font: Option<&Arc<UiFont>>,
    r: Rect,
    glyph: &[&[(f32, f32)]],
    words: &str,
    ink: Color,
    lit: bool,
    enabled: bool,
) {
    let fade = if enabled { 1.0 } else { 0.35 };
    let round = CornerRadii::all(4.0);
    rc.fill_rounded_rect(
        r.0,
        r.1,
        r.2,
        r.3,
        round,
        hex(0, if lit { 0.5 } else { 0.25 }),
    );
    rc.stroke_rounded_rect(
        r.0,
        r.1,
        r.2,
        r.3,
        round,
        bronze((if lit { 0.8 } else { 0.35 }) * fade),
        1.0,
    );
    let c = Color { a: fade, ..ink };
    stroke(rc, r.0 + 4.0, r.1 + 5.0, 12.0, glyph, c, 2.6);
    put_mid(rc, font, words, r.0 + 19.0, r.1, r.3, c);
}

/// Health, stamina and mana in three small boxes.
fn vitals(rc: &mut RenderContext<'_>, f: &StudioFonts, r: Rect, v: [i32; 3], olthoi: bool) {
    let w = (r.2 - 16.0) / 3.0;
    for (i, (name, color)) in [
        ("Health", 0xFF_8A8E),
        ("Stamina", 0xF2_D27A),
        ("Mana", 0x7F_B8FF),
    ]
    .into_iter()
    .enumerate()
    {
        let b = (r.0 + i as f32 * (w + 8.0), r.1, w, r.3);
        kit::panel(rc, b, 0.25);
        put(
            rc,
            f.small.as_ref(),
            name,
            b.0 + 10.0,
            b.1 + 6.0,
            tone::soft(1.0),
        );
        let value = if olthoi {
            "—".to_owned()
        } else {
            v[i].to_string()
        };
        put(
            rc,
            f.strong.as_ref(),
            &value,
            b.0 + 10.0,
            b.1 + 22.0,
            hex(color, 1.0),
        );
    }
}

/// The shade slider's track, through the part's own colours.
fn shade_track(rc: &mut RenderContext<'_>, r: Rect, stops: &[Color]) {
    let round = CornerRadii::all(6.0);
    match stops.len() {
        0 => rc.fill_horizontal_gradient(
            r.0,
            r.1,
            r.2,
            r.3,
            round,
            hex(0x2A_1A10, 1.0),
            tone::warm(1.0),
        ),
        1 => rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, stops[0]),
        n => {
            let seg = r.2 / (n - 1) as f32;
            for (i, pair) in stops.windows(2).enumerate() {
                let radii = CornerRadii::new(
                    if i == 0 { 6.0 } else { 0.0 },
                    if i + 2 == n { 6.0 } else { 0.0 },
                    if i + 2 == n { 6.0 } else { 0.0 },
                    if i == 0 { 6.0 } else { 0.0 },
                );
                rc.fill_horizontal_gradient(
                    r.0 + i as f32 * seg,
                    r.1,
                    seg + 0.5,
                    r.3,
                    radii,
                    pair[0],
                    pair[1],
                );
            }
        }
    }
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, hex(0, 0.4), 1.0);
}

/// A starting town's sky (top, bottom) and land.
fn town_colors(index: i32) -> ((Color, Color), Color) {
    match index {
        1 => (
            (hex(0x8D_A9A0, 1.0), hex(0xC8_D1B4, 1.0)),
            hex(0x3E_5E52, 1.0),
        ),
        2 => (
            (hex(0xD3_A86E, 1.0), hex(0xE9_CF9C, 1.0)),
            hex(0x8A_6234, 1.0),
        ),
        3 => (
            (hex(0x8D_9CB0, 1.0), hex(0xD3_DAE2, 1.0)),
            hex(0x5A_6576, 1.0),
        ),
        _ => (
            (hex(0x7F_A0B6, 1.0), hex(0xB9_C79C, 1.0)),
            hex(0x3F_5A33, 1.0),
        ),
    }
}

/// A small skyline along the foot of `band`: hills and the town's shapes
/// (a tower, a pagoda, domes, walls).
fn skyline(rc: &mut RenderContext<'_>, band: Rect, index: i32, land: Color) {
    let outline: &[(f32, f32)] = match index {
        1 => &[
            (0.0, 42.0),
            (50.0, 36.0),
            (70.0, 22.0),
            (76.0, 22.0),
            (76.0, 18.0),
            (64.0, 18.0),
            (82.0, 10.0),
            (100.0, 18.0),
            (88.0, 18.0),
            (88.0, 22.0),
            (94.0, 22.0),
            (110.0, 36.0),
            (170.0, 30.0),
            (210.0, 36.0),
            (260.0, 26.0),
            (300.0, 34.0),
            (380.0, 30.0),
        ],
        2 => &[
            (0.0, 44.0),
            (60.0, 40.0),
            (80.0, 30.0),
            (86.0, 22.0),
            (94.0, 18.0),
            (102.0, 22.0),
            (108.0, 30.0),
            (120.0, 40.0),
            (170.0, 38.0),
            (190.0, 26.0),
            (200.0, 22.0),
            (210.0, 26.0),
            (222.0, 38.0),
            (290.0, 42.0),
            (380.0, 38.0),
        ],
        3 => &[
            (0.0, 38.0),
            (30.0, 38.0),
            (30.0, 24.0),
            (40.0, 24.0),
            (40.0, 38.0),
            (90.0, 38.0),
            (90.0, 20.0),
            (96.0, 14.0),
            (102.0, 20.0),
            (102.0, 38.0),
            (170.0, 38.0),
            (170.0, 26.0),
            (182.0, 26.0),
            (182.0, 38.0),
            (260.0, 36.0),
            (320.0, 30.0),
            (380.0, 34.0),
        ],
        _ => &[
            (0.0, 40.0),
            (40.0, 30.0),
            (80.0, 38.0),
            (110.0, 26.0),
            (120.0, 30.0),
            (120.0, 18.0),
            (128.0, 14.0),
            (136.0, 18.0),
            (136.0, 30.0),
            (170.0, 36.0),
            (220.0, 28.0),
            (260.0, 36.0),
            (300.0, 30.0),
            (340.0, 38.0),
            (380.0, 32.0),
        ],
    };
    let sx = band.2 / 380.0;
    let sy = band.3 / 58.0;
    let base = band.1 + band.3;
    let mut tris = Vec::new();
    let v = |x: f32, y: f32| ColorVertex { x, y, color: land };
    for pair in outline.windows(2) {
        let (x0, y0) = (band.0 + pair[0].0 * sx, band.1 + pair[0].1 * sy);
        let (x1, y1) = (band.0 + pair[1].0 * sx, band.1 + pair[1].1 * sy);
        tris.extend([
            v(x0, y0),
            v(x1, y1),
            v(x1, base),
            v(x0, y0),
            v(x1, base),
            v(x0, base),
        ]);
    }
    if let Some(&(lx, ly)) = outline.last() {
        let (x0, y0) = (band.0 + lx * sx, band.1 + ly * sy);
        let x1 = band.0 + band.2;
        tris.extend([
            v(x0, y0),
            v(x1, y0),
            v(x1, base),
            v(x0, y0),
            v(x1, base),
            v(x0, base),
        ]);
    }
    rc.triangles(&tris);
}

impl Widget for StudioView {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        let value_at = |slider: Slider, x: f32| -> StudioAsk {
            match slider {
                Slider::Attribute(attr, t) => {
                    let share = ((x - t.0) / t.2).clamp(0.0, 1.0);
                    StudioAsk::Attribute(attr, 10 + (share * 90.0).round() as i32)
                }
                Slider::Shade(t) => StudioAsk::Shade(((x - t.0) / t.2).clamp(0.0, 1.0)),
            }
        };
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hits.hover = Some((x, y));
                if let Some(s) = self.sliding {
                    self.asks.push(value_at(s, x));
                    return true;
                }
                if let Some(last) = self.turning {
                    if let Some(turn) = ac_ui::pregame::chargen::appearance::drag_turn(x - last) {
                        self.asks.push(StudioAsk::Preview(turn));
                        self.turning = Some(x);
                    }
                    return true;
                }
                false
            }
            kind::HOVER_LEAVE => {
                self.hits.hover = None;
                false
            }
            kind::MOUSE_DOWN => {
                match self.hits.at(x, y) {
                    Some(Hit::Slider(s)) => {
                        self.sliding = Some(s);
                        ui.set_capture(me);
                        self.asks.push(value_at(s, x));
                    }
                    // A drag across the figure turns it.
                    None | Some(Hit::Swallow) if kit::inside(self.figure_rect(), x, y) => {
                        self.turning = Some(x);
                        ui.set_capture(me);
                    }
                    _ => {}
                }
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.turning = None;
                if let Some(s) = self.sliding.take()
                    && e.kind == kind::MOUSE_UP
                {
                    self.asks.push(value_at(s, x));
                }
                true
            }
            kind::SCROLL => {
                if kit::inside(
                    self.skills_room,
                    self.hits.hover.map_or(-1.0, |p| p.0),
                    self.hits.hover.map_or(-1.0, |p| p.1),
                ) {
                    let total: f32 = self
                        .screen
                        .skills
                        .iter()
                        .map(|r| match r {
                            SkillRow::Head(..) => 26.0,
                            SkillRow::Skill(_) => 30.0,
                        })
                        .sum();
                    self.skills_scroll
                        .wheel(e.data0, 60.0, total, self.skills_room.3);
                }
                true
            }
            kind::CLICK => {
                if let Some(Hit::Ask(a)) = self.hits.at(x, y) {
                    if let StudioAsk::PickSkill(id) | StudioAsk::Raise(id) | StudioAsk::Lower(id) =
                        a
                    {
                        self.skill = Some(id);
                    }
                    self.asks.push(a);
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let mut h = Hits::new(self.hits.hover);
        let (w, hh) = self.size;
        h.add((0.0, 0.0, w, hh), Hit::Swallow);
        let town_sky = matches!(self.screen.step, Step::Town | Step::Name) && !self.screen.olthoi;
        let sky = if town_sky {
            match self.screen.town {
                1 => Sky::Blossom,
                2 => Sky::Desert,
                3 => Sky::Coast,
                _ => Sky::Dusk,
            }
        } else {
            Sky::Studio
        };
        // The pictures, dimmed more than the Stage's: panels stand on
        // both sides here.
        if self.backdrop.draw(rc, w, hh) {
            scenery::shade(rc, w, hh, false, 0.3);
        } else {
            scenery::draw(rc, w, hh, sky, false);
        }
        scenery::glow(
            rc,
            self.stage_centre(),
            hh * 0.42,
            450.0,
            350.0,
            hex(0xFF_D696, 0.2),
        );
        self.draw_stage(rc, &mut h);
        self.draw_top(rc, &mut h);
        // The panel.
        let p = self.panel_rect();
        let round = CornerRadii::all(6.0);
        rc.soft_shadow(p.0, p.1, p.2, p.3, 6.0, 18.0, 50.0, hex(0, 0.5));
        fill_glass(rc, p, round, tone::ground(0.9));
        rc.stroke_rounded_rect(p.0, p.1, p.2, p.3, round, bronze(0.8), 1.0);
        h.add(p, Hit::Swallow);
        rc.push_clip(p.0, p.1, p.2, p.3);
        let top = self.panel_head(rc, &mut h, p);
        let (x, cw) = (p.0 + 18.0, p.2 - 36.0);
        match self.screen.step {
            Step::Heritage => self.draw_heritage(rc, &mut h, x, top, cw),
            Step::Profession => self.draw_profession(rc, &mut h, x, top, cw),
            Step::Skills => self.draw_skills(rc, &mut h, x, top, cw),
            Step::Appearance => self.draw_appearance(rc, &mut h, x, top, cw),
            Step::Town => self.draw_town(rc, &mut h, x, top, cw),
            Step::Name => {
                let empty = ui
                    .children(me)
                    .into_iter()
                    .filter_map(|c| ui.widget::<ac_ui::widgets::Field>(c))
                    .all(|f| f.text().is_empty());
                self.draw_name(rc, &mut h, x, cw, empty);
            }
        }
        rc.pop_clip();
        self.draw_so_far(rc, &mut h);
        self.draw_card(rc, &mut h);
        self.hits = h;
    }
}
