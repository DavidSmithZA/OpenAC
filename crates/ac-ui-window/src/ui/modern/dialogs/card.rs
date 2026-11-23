//! The question card: one card for every kind of question the game asks
//! (yes or no, a message with OK, please wait, a typed answer, a choice
//! from a list), in the middle of the screen over a slight dimming of the
//! world. Its icon and title say what kind of question it is, the game's
//! words follow as sent, and the buttons are named for what they do, with
//! Enter and Esc shown on them. When more questions wait, the card counts
//! them ("1 of 3"); they come one at a time.
//!
//! The card is the whole screen (the part outside the card dims the world
//! and takes no clicks); while it shows, the window makes it modal.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{UiEvent, kind};
use ac_ui::geometry::AnchorEdges;

use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::widgets::Field;
use ac_ui::{Key, UiFont, UiRoot};

use super::look::{Glyph, Look};
use crate::ui::modern::hud::lines::{arc, circle, stroke};
use crate::ui::modern::kit::{self, Rect, bronze, gold};

/// The card's width, and a please-wait card's.
pub const WIDTH: f32 = 420.0;
pub const WAIT_WIDTH: f32 = 320.0;
const PAD: f32 = 18.0;
const HEAD: f32 = 46.0;
const BUTTON: f32 = 34.0;
const ROW: f32 = 34.0;
const FIELD: f32 = 34.0;
const TILE: f32 = 34.0;
/// The most characters a typed answer takes.
const MAX_CHARACTERS: usize = 64;

/// What form the answer takes.
#[derive(Debug, Clone, PartialEq)]
pub enum Form {
    /// Yes or no.
    YesNo,
    /// Read, then OK.
    Ok,
    /// Nothing to answer: it goes when the game is done.
    Wait,
    /// Yes with what was typed, or no.
    Typed,
    /// Yes with one of these, or no.
    Choice(Vec<String>),
}

/// One of the things a crafting question combines.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// A line under it.
    pub sub: String,
}

/// What the card shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Shown {
    /// The question's number (the window's).
    pub context: u32,
    /// How it looks.
    pub look: Look,
    /// The game's words.
    pub text: String,
    /// How it is answered.
    pub form: Form,
    /// The things combined (crafting), tool first.
    pub parts: Vec<Part>,
    /// How many questions there are, this one included.
    pub of: usize,
}

/// How the player answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answer {
    /// The question's number.
    pub context: u32,
    /// Yes (or OK).
    pub yes: bool,
    /// The choice picked (-1: none).
    pub chosen: i32,
}

/// The fonts the card draws with.
#[derive(Debug, Clone, Default)]
pub struct CardFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// The question.
    pub body: Option<Arc<UiFont>>,
    /// Notes, the count, the field's label.
    pub small: Option<Arc<UiFont>>,
    /// Buttons and choices.
    pub strong: Option<Arc<UiFont>>,
    /// The keys on the buttons.
    pub hint: Option<Arc<UiFont>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    Yes,
    No,
    Choice(usize),
}

/// The card (see the module documentation).
#[derive(Debug)]
pub struct QuestionCard {
    theme: Horizon,
    fonts: CardFonts,
    /// The typed answer's field.
    pub field: ElementId,
    /// What it shows (none: hidden).
    pub shown: Option<Shown>,
    /// The choice picked.
    chosen: usize,
    hovered: Option<Hit>,
    /// The please-wait spinner's turn, radians.
    spin: f32,
    /// What was answered since the window last looked.
    pub answers: Vec<Answer>,
    /// The card's place on the screen, last drawn.
    rect: Rect,
}

impl QuestionCard {
    /// The card, hidden, over the whole screen, with its field.
    pub fn mount(ui: &mut UiRoot, theme: Horizon, fonts: CardFonts) -> ElementId {
        let root = ui.root();
        let (w, h) = ui.size();
        let mut e = Element::at(0.0, 0.0, w, h)
            .with_visible(false)
            .with_anchors(AnchorEdges::ALL);
        e.accepts_focus = true;
        e.click_through = false;
        let body = fonts.body.clone();
        let me = ui.add(
            root,
            e,
            Self {
                theme,
                fonts,
                field: root,
                shown: None,
                chosen: 0,
                hovered: None,
                spin: 0.0,
                answers: Vec::new(),
                rect: (0.0, 0.0, WIDTH, 200.0),
            },
        );
        let mut fe = Element::at(0.0, 0.0, WIDTH - 2.0 * PAD - 2.0, FIELD - 2.0)
            .with_anchors(AnchorEdges::NONE)
            .with_visible(false);
        Field::configure(&mut fe);
        let mut f = Field::default();
        f.font = body;
        f.one_line = true;
        f.selectable = true;
        f.record_history = false;
        f.clear_on_submit = false;
        f.stay_focused_after_submit = true;
        f.max_characters = MAX_CHARACTERS;
        f.padding = 8.0;
        f.text_color = Horizon::current().palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = gold(0.35);
        f.set_editable_detached(&mut fe, true);
        let field = ui.add(me, fe, f);
        if let Some(c) = ui.widget_mut::<Self>(me) {
            c.field = field;
        }
        me
    }

    /// Shows `shown` on card `me` (its typed answer starting as `typed`,
    /// its choice at `chosen`), modal, with the keyboard on it.
    pub fn show(ui: &mut UiRoot, me: ElementId, shown: Shown, typed: &str, chosen: usize) {
        let size = Some(ui.size());
        let typed_form = shown.form == Form::Typed;
        let Some(c) = ui.widget_mut::<Self>(me) else {
            return;
        };
        c.shown = Some(shown);
        c.chosen = chosen;
        c.hovered = None;
        let field = c.field;
        if let Some((w, h)) = size
            && let Some(e) = ui.element_mut(me)
        {
            e.width = w;
            e.height = h;
        }
        ui.set_visible(me, true);
        ui.bring_to_front(me);
        ui.modal = Some(me);
        if let Some(f) = ui.widget_mut::<Field>(field) {
            f.set_text(typed);
            f.select_all_text();
        }
        ui.set_visible(field, typed_form);
        Self::place_field(ui, me);
        ui.set_keyboard_focus(Some(if typed_form { field } else { me }));
    }

    /// Hides card `me` and lets go of the screen.
    pub fn hide(ui: &mut UiRoot, me: ElementId) {
        if let Some(c) = ui.widget_mut::<Self>(me) {
            c.shown = None;
        }
        let field = ui.widget::<Self>(me).map(|c| c.field);
        if ui.modal == Some(me) {
            ui.modal = None;
        }
        if let Some(f) = field
            && ui.keyboard_focus() == Some(f)
        {
            ui.set_keyboard_focus(None);
        }
        if ui.keyboard_focus() == Some(me) {
            ui.set_keyboard_focus(None);
        }
        ui.set_visible(me, false);
    }

    /// What was typed in card `me`'s field.
    #[must_use]
    pub fn typed(ui: &UiRoot, me: ElementId) -> String {
        ui.widget::<Self>(me)
            .map(|c| kit::field_text(ui, c.field).trim().to_owned())
            .unwrap_or_default()
    }

    /// The answers given on card `me` since the last call.
    pub fn take(ui: &mut UiRoot, me: ElementId) -> Vec<Answer> {
        ui.widget_mut::<Self>(me)
            .map(|c| std::mem::take(&mut c.answers))
            .unwrap_or_default()
    }

    /// Answers the question shown on card `me` (`yes`), as its buttons do.
    pub fn answer(ui: &mut UiRoot, me: ElementId, yes: bool) {
        if let Some(c) = ui.widget_mut::<Self>(me) {
            c.give(yes);
        }
    }

    /// Keeps the field over its well whatever the screen's size.
    pub fn place_field(ui: &mut UiRoot, me: ElementId) {
        let screen = ui.size();
        let Some(c) = ui.widget::<Self>(me) else {
            return;
        };
        let field = c.field;
        let r = c.layout(screen);
        if let Some(well) = r.field
            && let Some(e) = ui.element_mut(field)
        {
            e.left = well.0 + 1.0;
            e.top = well.1 + 1.0;
            e.width = well.2 - 2.0;
            e.height = well.3 - 2.0;
        }
    }

    fn give(&mut self, yes: bool) {
        let Some(s) = &self.shown else {
            return;
        };
        if s.form == Form::Wait {
            return;
        }
        let chosen = match (&s.form, yes) {
            (Form::Choice(items), true) if !items.is_empty() => {
                i32::try_from(self.chosen.min(items.len() - 1)).unwrap_or(0)
            }
            _ => -1,
        };
        self.answers.push(Answer {
            context: s.context,
            yes: yes || s.form == Form::Ok,
            chosen,
        });
    }

    fn width(&self) -> f32 {
        match self.shown.as_ref().map(|s| &s.form) {
            Some(Form::Wait) => WAIT_WIDTH,
            _ => WIDTH,
        }
    }

    /// Where everything goes on a screen `screen` big.
    fn layout(&self, screen: (f32, f32)) -> Layout {
        let w = self.width();
        let inner = w - 2.0 * PAD;
        let mut y = HEAD;
        let mut out = Layout::default();
        let Some(s) = &self.shown else {
            return out;
        };
        if !s.parts.is_empty() {
            y += 14.0;
            out.parts = Some((PAD, y, inner, TILE + 20.0));
            y += TILE + 20.0;
        }
        y += 12.0;
        let body_step = self
            .fonts
            .body
            .as_ref()
            .map_or(20.0, |f| f.line_height + 6.0);
        let lines = self
            .fonts
            .body
            .as_ref()
            .map_or(2, |f| kit::wrap(f, &s.text, inner).len().max(1));
        if s.form == Form::Wait {
            out.text_top = y + 4.0;
            y += (lines as f32 * body_step).max(24.0) + 8.0;
        } else {
            out.text_top = y;
            y += lines as f32 * body_step;
        }
        if let Some(note) = &s.look.note {
            let step = self
                .fonts
                .small
                .as_ref()
                .map_or(17.0, |f| f.line_height + 5.0);
            let n = self
                .fonts
                .small
                .as_ref()
                .map_or(1, |f| kit::wrap(f, note, inner).len().max(1));
            y += 6.0;
            out.note_top = y;
            y += n as f32 * step;
        }
        match &s.form {
            Form::Typed => {
                y += 10.0;
                out.label_top = y;
                y += 18.0;
                out.field = Some((PAD, y, inner, FIELD));
                y += FIELD;
            }
            Form::Choice(items) => {
                y += 10.0;
                for i in 0..items.len() {
                    out.rows.push((PAD, y + i as f32 * (ROW + 2.0), inner, ROW));
                }
                y += items.len() as f32 * (ROW + 2.0);
            }
            _ => {}
        }
        if s.form == Form::Wait {
            y += 8.0;
        } else {
            y += 16.0;
            let yes_w = self.button_width(&s.look.yes, true);
            out.yes = Some((w - PAD - yes_w, y, yes_w, BUTTON));
            if s.form != Form::Ok {
                let no_w = self.button_width(&s.look.no, false);
                out.no = Some((w - PAD - yes_w - 8.0 - no_w, y, no_w, BUTTON));
            }
            y += BUTTON + 16.0;
        }
        let h = y;
        let left = ((screen.0 - w) * 0.5).round().max(0.0);
        let top = ((screen.1 - h) * 0.42).round().max(0.0);
        out.card = (left, top, w, h);
        let shift = |r: Rect| (r.0 + left, r.1 + top, r.2, r.3);
        out.parts = out.parts.map(shift);
        out.field = out.field.map(shift);
        out.yes = out.yes.map(shift);
        out.no = out.no.map(shift);
        for r in &mut out.rows {
            *r = shift(*r);
        }
        out.text_top += top;
        out.note_top += top;
        out.label_top += top;
        out
    }

    fn button_width(&self, words: &str, yes: bool) -> f32 {
        let w = self
            .fonts
            .strong
            .as_ref()
            .map_or(60.0, |f| text::measure(f, words, 0.0));
        let key = self.fonts.hint.as_ref().map_or(30.0, |f| {
            text::measure(f, if yes { "Enter" } else { "Esc" }, 0.0)
        });
        (w + 8.0 + key + 28.0).max(84.0)
    }

    fn hit(&self, screen: (f32, f32), x: f32, y: f32) -> Option<Hit> {
        let r = self.layout(screen);
        if r.yes.is_some_and(|b| kit::inside(b, x, y)) {
            return Some(Hit::Yes);
        }
        if r.no.is_some_and(|b| kit::inside(b, x, y)) {
            return Some(Hit::No);
        }
        r.rows
            .iter()
            .position(|row| kit::inside(*row, x, y))
            .map(Hit::Choice)
    }

    fn key(&mut self, key: Key) {
        let count = match self.shown.as_ref().map(|s| &s.form) {
            Some(Form::Choice(items)) => items.len(),
            _ => 0,
        };
        match key {
            Key::Enter | Key::KeypadEnter => self.give(true),
            Key::Escape => {
                if self.shown.as_ref().is_some_and(|s| s.form == Form::Ok) {
                    self.give(true);
                } else {
                    self.give(false);
                }
            }
            Key::Up if count > 0 => self.chosen = self.chosen.saturating_sub(1),
            Key::Down if count > 0 => self.chosen = (self.chosen + 1).min(count - 1),
            _ => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_card(&self, rc: &mut RenderContext<'_>, r: &Layout) {
        let Some(s) = &self.shown else {
            return;
        };
        let p = Horizon::current().palette;
        let (x, y, w, h) = r.card;
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(x, y, w, h, 6.0, 18.0, 50.0, hex(0, 0.6));
        rc.fill_rounded_rect(x, y, w, h, round, tone::raised(0.97));
        let edge = if s.look.danger {
            hex(0xE5_484D, 0.7)
        } else {
            bronze(0.8)
        };
        rc.stroke_rounded_rect(x, y, w, h, round, edge, 1.0);
        // The head: icon, title, count.
        let icon_y = y + 16.0;
        rc.fill_ellipse(
            x + PAD,
            icon_y,
            30.0,
            30.0,
            Color {
                a: 0.16,
                ..s.look.tint
            },
        );
        draw_glyph(
            rc,
            s.look.glyph,
            x + PAD + 7.0,
            icon_y + 7.0,
            16.0,
            s.look.tint,
        );
        let mut right = x + w - PAD;
        if s.of > 1
            && let Some(f) = &self.fonts.small
        {
            let words = format!("1 of {}", s.of);
            let cw = text::measure(f, &words, 0.0) + 18.0;
            let cy = icon_y + 4.0;
            let pill = (right - cw, cy, cw, 22.0);
            rc.fill_rounded_rect(
                pill.0,
                pill.1,
                pill.2,
                pill.3,
                CornerRadii::all(11.0),
                hex(0, 0.2),
            );
            rc.stroke_rounded_rect(
                pill.0,
                pill.1,
                pill.2,
                pill.3,
                CornerRadii::all(11.0),
                bronze(0.5),
                1.0,
            );
            let ty = cy + ((22.0 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                &words,
                pill.0 + 9.0,
                ty,
                TextStyle::plain(tone::warm(1.0)),
            );
            right = pill.0 - 8.0;
        }
        if let Some(f) = &self.fonts.title {
            let ty = icon_y + ((30.0 - f.line_height) * 0.5).round();
            let left = x + PAD + 40.0;
            let title = kit::fit(f, &s.look.title, right - left);
            text::draw(rc, f, &title, left, ty, TextStyle::plain(p.text));
        }
        // What is combined, tool to target.
        if let Some(band) = r.parts {
            rc.fill_rounded_rect(
                band.0,
                band.1,
                band.2,
                band.3,
                CornerRadii::all(4.0),
                hex(0, 0.3),
            );
            let half = (band.2 - 40.0) / 2.0;
            for (i, part) in s.parts.iter().take(2).enumerate() {
                let px = band.0 + 12.0 + i as f32 * (half + 28.0);
                let py = band.1 + 10.0;
                rc.fill_rounded_rect(px, py, TILE, TILE, CornerRadii::all(4.0), hex(0, 0.45));
                if let Some(icon) = part.icon {
                    rc.sprite(
                        icon,
                        px + 1.0,
                        py + 1.0,
                        32.0,
                        32.0,
                        (0.0, 0.0, 1.0, 1.0),
                        Color::WHITE,
                    );
                }
                let tx = px + TILE + 8.0;
                let room = half - TILE - 12.0;
                if let Some(f) = &self.fonts.strong {
                    let name = kit::fit(f, &part.name, room);
                    text::draw(rc, f, &name, tx, py + 1.0, TextStyle::plain(p.text));
                }
                if let Some(f) = &self.fonts.small {
                    let sub = kit::fit(f, &part.sub, room);
                    text::draw(rc, f, &sub, tx, py + 18.0, TextStyle::plain(p.muted));
                }
                if i == 0 && s.parts.len() > 1 {
                    let ax = band.0 + 12.0 + half + 2.0;
                    let ay = band.1 + band.3 * 0.5 - 8.0;
                    stroke(
                        rc,
                        ax,
                        ay,
                        16.0,
                        &[
                            &[(4.0, 12.0), (20.0, 12.0)],
                            &[(14.0, 6.0), (20.0, 12.0), (14.0, 18.0)],
                        ],
                        p.muted,
                        1.6,
                    );
                }
            }
        }
        // The game's words.
        let inner = w - 2.0 * PAD;
        if let Some(f) = &self.fonts.body {
            let step = f.line_height + 6.0;
            let mut tx = x + PAD;
            if s.form == Form::Wait {
                arc(
                    rc,
                    tx,
                    r.text_top,
                    20.0,
                    (12.0, 12.0),
                    9.0,
                    (0.0, std::f32::consts::TAU),
                    gold(0.25),
                    2.5,
                );
                arc(
                    rc,
                    tx,
                    r.text_top,
                    20.0,
                    (12.0, 12.0),
                    9.0,
                    (self.spin, 1.4),
                    gold(1.0),
                    2.5,
                );
                tx += 32.0;
            }
            let lines = kit::wrap(f, &s.text, inner - (tx - x - PAD));
            kit::lines(rc, f, &lines, tx, r.text_top, step, tone::warm(1.0));
        }
        if let (Some(note), Some(f)) = (&s.look.note, &self.fonts.small) {
            let color = if s.look.note_red {
                kit::HARM
            } else {
                tone::muted(1.0)
            };
            let lines = kit::wrap(f, note, inner);
            kit::lines(
                rc,
                f,
                &lines,
                x + PAD,
                r.note_top,
                f.line_height + 5.0,
                color,
            );
        }
        // The answer's field.
        if let Some(well) = r.field {
            if let (Some(label), Some(f)) = (&s.look.field, &self.fonts.small) {
                text::draw(
                    rc,
                    f,
                    label,
                    x + PAD,
                    r.label_top,
                    TextStyle::plain(tone::soft(1.0)),
                );
            }
            let round = CornerRadii::all(4.0);
            rc.fill_rounded_rect(well.0, well.1, well.2, well.3, round, hex(0, 0.35));
            rc.stroke_rounded_rect(well.0, well.1, well.2, well.3, round, gold(1.0), 1.0);
        }
        // The choices.
        if let Form::Choice(items) = &s.form {
            for (i, (row, words)) in r.rows.iter().zip(items).enumerate() {
                let on = i == self.chosen;
                let lit = self.hovered == Some(Hit::Choice(i));
                let round = CornerRadii::all(4.0);
                let fill = if on {
                    gold(0.12)
                } else if lit {
                    hex(0xFF_FFFF, 0.04)
                } else {
                    Color::TRANSPARENT
                };
                rc.fill_rounded_rect(row.0, row.1, row.2, row.3, round, fill);
                let edge = if on { gold(1.0) } else { bronze(0.2) };
                rc.stroke_rounded_rect(row.0, row.1, row.2, row.3, round, edge, 1.0);
                let ring = if on { gold(1.0) } else { bronze(0.6) };
                let (cx, cy) = (row.0 + 10.0, row.1 + (ROW - 14.0) * 0.5);
                circle(rc, cx, cy, 14.0, (12.0, 12.0), 10.0, ring, 1.5);
                if on {
                    rc.fill_ellipse(cx + 4.0, cy + 4.0, 6.0, 6.0, gold(1.0));
                }
                if let Some(f) = &self.fonts.strong {
                    let ty = row.1 + ((ROW - f.line_height) * 0.5).round();
                    let words = kit::fit(f, words, row.2 - 44.0);
                    text::draw(rc, f, &words, row.0 + 34.0, ty, TextStyle::plain(p.text));
                }
            }
        }
        // The foot: how many wait, then no and yes.
        if let Some(yes) = r.yes {
            if s.of > 1
                && let Some(f) = &self.fonts.small
            {
                let more = s.of - 1;
                let words = if more == 1 {
                    "1 more waiting".to_owned()
                } else {
                    format!("{more} more waiting")
                };
                let ty = yes.1 + ((BUTTON - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    &words,
                    x + PAD,
                    ty,
                    TextStyle::plain(tone::muted(1.0)),
                );
            }
            if let Some(no) = r.no {
                self.draw_button(rc, no, &s.look.no, "Esc", false, false);
            }
            self.draw_button(rc, yes, &s.look.yes, "Enter", true, s.look.danger);
        }
    }

    fn draw_button(
        &self,
        rc: &mut RenderContext<'_>,
        r: Rect,
        words: &str,
        key: &str,
        yes: bool,
        danger: bool,
    ) {
        let p = Horizon::current().palette;
        let lit = self.hovered == Some(if yes { Hit::Yes } else { Hit::No });
        let round = CornerRadii::all(4.0);
        let (fill, edge, ink) = if !yes {
            (
                if lit {
                    hex(0xFF_FFFF, 0.06)
                } else {
                    Color::TRANSPARENT
                },
                bronze(0.45),
                p.text,
            )
        } else if danger {
            (
                hex(0xE5_484D, if lit { 1.0 } else { 0.9 }),
                hex(0xE5_484D, 1.0),
                hex(0xFF_FFFF, 1.0),
            )
        } else {
            (gold(if lit { 1.0 } else { 0.9 }), gold(1.0), tone::ink(1.0))
        };
        rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, fill);
        rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, edge, 1.0);
        let (Some(f), Some(k)) = (&self.fonts.strong, &self.fonts.hint) else {
            return;
        };
        let ww = text::measure(f, words, 0.0);
        let kw = text::measure(k, key, 0.0);
        let x = (r.0 + (r.2 - ww - 8.0 - kw) * 0.5).round();
        let ty = r.1 + ((r.3 - f.line_height) * 0.5).round();
        text::draw(rc, f, words, x, ty, TextStyle::plain(ink));
        let ky = r.1 + ((r.3 - k.line_height) * 0.5).round();
        let key_ink = if yes {
            Color { a: 0.7, ..ink }
        } else {
            tone::muted(1.0)
        };
        text::draw(rc, k, key, x + ww + 8.0, ky, TextStyle::plain(key_ink));
    }
}

/// Where the card's parts go, in screen units.
#[derive(Debug, Clone, Default)]
struct Layout {
    card: Rect,
    parts: Option<Rect>,
    text_top: f32,
    note_top: f32,
    label_top: f32,
    field: Option<Rect>,
    rows: Vec<Rect>,
    yes: Option<Rect>,
    no: Option<Rect>,
}

/// Draws `glyph` in a `size` box at (`x`, `y`).
pub fn draw_glyph(
    rc: &mut RenderContext<'_>,
    glyph: Glyph,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) {
    let w = 1.8;
    match glyph {
        Glyph::Hammer => stroke(
            rc,
            x,
            y,
            size,
            &[
                &[(14.0, 6.0), (18.0, 10.0)],
                &[(3.0, 21.0), (12.0, 12.0)],
                &[
                    (12.0, 12.0),
                    (15.0, 9.0),
                    (19.0, 13.0),
                    (16.0, 16.0),
                    (12.0, 12.0),
                ],
                &[(14.0, 4.0), (20.0, 10.0)],
            ],
            color,
            w,
        ),
        Glyph::Shield => stroke(
            rc,
            x,
            y,
            size,
            &[&[
                (12.0, 3.0),
                (20.0, 6.0),
                (20.0, 12.0),
                (17.0, 18.0),
                (12.0, 21.0),
                (7.0, 18.0),
                (4.0, 12.0),
                (4.0, 6.0),
                (12.0, 3.0),
            ]],
            color,
            w,
        ),
        Glyph::Users => {
            circle(rc, x, y, size, (9.0, 7.0), 4.0, color, w);
            arc(
                rc,
                x,
                y,
                size,
                (9.0, 21.0),
                7.0,
                (std::f32::consts::PI, std::f32::consts::PI),
                color,
                w,
            );
            stroke(
                rc,
                x,
                y,
                size,
                &[&[(16.0, 3.5), (18.0, 5.0), (18.0, 9.0), (16.0, 10.5)]],
                color,
                w,
            );
            stroke(
                rc,
                x,
                y,
                size,
                &[&[(17.0, 14.5), (21.0, 17.0), (22.0, 21.0)]],
                color,
                w,
            );
        }
        Glyph::Up => stroke(
            rc,
            x,
            y,
            size,
            &[
                &[(12.0, 19.0), (12.0, 5.0)],
                &[(5.0, 12.0), (12.0, 5.0), (19.0, 12.0)],
            ],
            color,
            w,
        ),
        Glyph::Star => stroke(
            rc,
            x,
            y,
            size,
            &[&[
                (12.0, 3.0),
                (14.7, 8.6),
                (20.8, 9.5),
                (16.4, 13.8),
                (17.4, 19.9),
                (12.0, 17.0),
                (6.6, 19.9),
                (7.6, 13.8),
                (3.2, 9.5),
                (9.3, 8.6),
                (12.0, 3.0),
            ]],
            color,
            w,
        ),
        Glyph::Question => {
            arc(
                rc,
                x,
                y,
                size,
                (12.0, 9.0),
                3.5,
                (std::f32::consts::PI, 4.0),
                color,
                w,
            );
            stroke(
                rc,
                x,
                y,
                size,
                &[&[(13.4, 11.6), (12.0, 13.0), (12.0, 15.0)]],
                color,
                w,
            );
            rc.fill_ellipse(
                x + size * 0.46,
                y + size * 0.72,
                size * 0.09,
                size * 0.09,
                color,
            );
        }
        Glyph::Info => {
            circle(rc, x, y, size, (12.0, 12.0), 10.0, color, w);
            rc.fill_ellipse(
                x + size * 0.46,
                y + size * 0.3,
                size * 0.09,
                size * 0.09,
                color,
            );
            stroke(
                rc,
                x,
                y,
                size,
                &[&[(11.0, 11.0), (12.0, 11.0), (12.0, 17.0)]],
                color,
                w,
            );
        }
        Glyph::Warn => {
            stroke(
                rc,
                x,
                y,
                size,
                &[
                    &[(12.0, 3.0), (2.0, 20.0), (22.0, 20.0), (12.0, 3.0)],
                    &[(12.0, 10.0), (12.0, 14.0)],
                ],
                color,
                w,
            );
            rc.fill_ellipse(
                x + size * 0.46,
                y + size * 0.68,
                size * 0.09,
                size * 0.09,
                color,
            );
        }
    }
}

impl Widget for QuestionCard {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn tick(&mut self, _ui: &mut UiRoot, _me: ElementId, dt: f64) {
        if self.shown.as_ref().is_some_and(|s| s.form == Form::Wait) {
            self.spin = (self.spin + dt as f32 * 7.0) % std::f32::consts::TAU;
        }
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let screen = ui.size();
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::KEY_DOWN => {
                self.key(e.key);
                true
            }
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = self.hit(screen, x, y);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::CLICK => {
                match self.hit(screen, x, y) {
                    Some(Hit::Yes) => self.give(true),
                    Some(Hit::No) => self.give(false),
                    Some(Hit::Choice(i)) => self.chosen = i,
                    None => {}
                }
                true
            }
            kind::DOUBLE_CLICK => {
                if let Some(Hit::Choice(i)) = self.hit(screen, x, y) {
                    self.chosen = i;
                    self.give(true);
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        if self.shown.is_none() {
            return;
        }
        let (w, h) = ui.size();
        if let Some(e) = ui.element_mut(me) {
            e.width = w;
            e.height = h;
        }
        rc.fill(0.0, 0.0, w, h, hex(0, 0.22));
        let r = self.layout((w, h));
        self.rect = r.card;
        self.draw_card(rc, &r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::modern::dialogs::look::Look;

    fn card(form: Form) -> QuestionCard {
        let ui = UiRoot::new();
        QuestionCard {
            theme: Horizon::current(),
            fonts: CardFonts::default(),
            field: ui.root(),
            shown: Some(Shown {
                context: 7,
                look: Look::question(),
                text: "Proceed?".to_owned(),
                form,
                parts: Vec::new(),
                of: 1,
            }),
            chosen: 0,
            hovered: None,
            spin: 0.0,
            answers: Vec::new(),
            rect: (0.0, 0.0, 0.0, 0.0),
        }
    }

    #[test]
    fn enter_says_yes_and_escape_no() {
        let mut c = card(Form::YesNo);
        c.key(Key::Enter);
        c.key(Key::Escape);
        let yes: Vec<bool> = c.answers.iter().map(|a| a.yes).collect();
        assert_eq!(yes, [true, false]);
    }

    #[test]
    fn a_message_is_closed_either_way_and_a_wait_not_at_all() {
        let mut c = card(Form::Ok);
        c.key(Key::Escape);
        assert!(c.answers[0].yes);
        let mut w = card(Form::Wait);
        w.key(Key::Enter);
        w.key(Key::Escape);
        assert!(w.answers.is_empty());
    }

    #[test]
    fn a_choice_is_moved_with_the_arrows_and_sent_with_yes() {
        let mut c = card(Form::Choice(vec!["a".into(), "b".into(), "c".into()]));
        c.key(Key::Down);
        c.key(Key::Down);
        c.key(Key::Down);
        c.key(Key::Up);
        c.key(Key::Enter);
        c.key(Key::Escape);
        assert_eq!(c.answers[0].chosen, 1);
        assert_eq!(c.answers[1].chosen, -1);
    }

    #[test]
    fn the_card_sits_in_the_middle_with_its_buttons_inside() {
        let c = card(Form::YesNo);
        let r = c.layout((1000.0, 800.0));
        assert!((r.card.0 - (1000.0 - WIDTH) * 0.5).abs() < 1.0);
        let yes = r.yes.unwrap();
        let no = r.no.unwrap();
        assert!(no.0 + no.2 <= yes.0);
        assert!(yes.0 + yes.2 <= r.card.0 + r.card.2);
        assert!(yes.1 + yes.3 <= r.card.1 + r.card.3);
    }
}
