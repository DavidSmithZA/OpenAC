//! The split pop-up: part of a stack taken off into a stack of its own
//! beside it. A track and a count box choose how many (1 to one less than
//! the stack). The pop-up takes the keyboard: digits type the count, the
//! arrows step it, Enter splits, Escape closes; so do Split and Cancel,
//! and a click elsewhere closes it.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_ui::draw::{Color, ColorVertex, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{Key, Modifiers, UiFont, UiRoot};

use super::fit;
use crate::ui::modern::bars::edit::{Look, button, button_width};

/// The pop-up's size.
pub const WIDTH: f32 = 260.0;
pub const HEIGHT: f32 = 164.0;
const PAD: f32 = 14.0;
/// The track's row.
const TRACK_TOP: f32 = 58.0;
const TRACK_H: f32 = 18.0;
/// The count row.
const ROW_TOP: f32 = 90.0;
const ROW_H: f32 = 28.0;
const COUNT_LEFT: f32 = PAD + 42.0;
const COUNT_W: f32 = 56.0;
/// Where the tip starts.
const TIP_TOP: f32 = 130.0;
/// The most digits the count takes.
const DIGITS: usize = 9;

/// The fonts it draws with.
#[derive(Debug, Clone, Default)]
pub struct SplitFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// Labels, notes and the tip.
    pub small: Option<Arc<UiFont>>,
    /// The buttons and the count.
    pub body: Option<Arc<UiFont>>,
}

/// What the pop-up asks of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitRequest {
    /// Take `amount` off stack `item`.
    Split { item: u32, amount: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Track,
    Count,
    Cancel,
    Split,
}

/// The split pop-up.
#[derive(Debug)]
pub struct SplitPopover {
    fonts: SplitFonts,
    /// The stack.
    pub item: u32,
    /// Its name.
    pub name: String,
    /// Its size.
    pub stack: u32,
    /// How many to take, 1..stack.
    pub amount: u32,
    /// What was typed in the count box (none: it shows the amount, and
    /// the next digit starts afresh).
    typed: Option<String>,
    /// It has the keyboard.
    focused: bool,
    dragging: bool,
    hovered: Option<Part>,
    /// What it asked for since the window last looked.
    pub requests: Vec<SplitRequest>,
}

impl SplitPopover {
    /// The pop-up, hidden.
    pub fn mount(ui: &mut UiRoot, fonts: SplitFonts) -> ElementId {
        let root = ui.root();
        let mut e = Element::at(0.0, 0.0, WIDTH, HEIGHT)
            .with_visible(false)
            .with_anchors(AnchorEdges::NONE);
        // It takes the keyboard and typed characters, and a drag on its
        // track moves the knob, not anything under it.
        e.accepts_focus = true;
        e.is_edit_control = true;
        e.captures_pointer_drag = true;
        e.click_through = false;
        ui.add(
            root,
            e,
            Self {
                fonts,
                item: 0,
                name: String::new(),
                stack: 0,
                amount: 1,
                typed: None,
                focused: false,
                dragging: false,
                hovered: None,
                requests: Vec::new(),
            },
        )
    }

    /// Opens pop-up `me` on stack `item` (`name`, `stack` of them), its
    /// top left near (`x`, `y`) kept on the screen, half the stack chosen
    /// and the keyboard on it. Nothing for a single item.
    pub fn open(
        ui: &mut UiRoot,
        me: ElementId,
        item: u32,
        name: &str,
        stack: u32,
        (x, y): (f32, f32),
    ) {
        if stack < 2 {
            return;
        }
        let screen = ui.size();
        let Some(p) = ui.widget_mut::<Self>(me) else {
            return;
        };
        p.item = item;
        name.clone_into(&mut p.name);
        p.stack = stack;
        p.amount = (stack / 2).max(1);
        p.typed = None;
        p.dragging = false;
        p.hovered = None;
        p.requests.clear();
        if let Some(e) = ui.element_mut(me) {
            e.left = x.min(screen.0 - WIDTH).max(0.0).round();
            e.top = y.min(screen.1 - HEIGHT).max(0.0).round();
        }
        ui.set_visible(me, true);
        ui.bring_to_front(me);
        ui.set_active_popup(me, me);
        ui.set_keyboard_focus(Some(me));
    }

    /// Closes pop-up `me`.
    pub fn close(ui: &mut UiRoot, me: ElementId) {
        if ui.keyboard_focus() == Some(me) {
            ui.set_keyboard_focus(None);
        }
        ui.clear_active_popup(me);
        ui.set_visible(me, false);
    }

    /// What pop-up `me` asked for since the window last looked.
    pub fn take(ui: &mut UiRoot, me: ElementId) -> Vec<SplitRequest> {
        ui.widget_mut::<Self>(me)
            .map(|p| std::mem::take(&mut p.requests))
            .unwrap_or_default()
    }

    /// The most that can be taken (one stays behind).
    fn most(&self) -> u32 {
        self.stack.saturating_sub(1).max(1)
    }

    fn track(&self) -> (f32, f32) {
        (PAD + 6.0, WIDTH - PAD - 6.0)
    }

    fn at_track(&self, x: f32) -> u32 {
        let (l, r) = self.track();
        let share = ((x - l) / (r - l)).clamp(0.0, 1.0);
        let span = self.most().saturating_sub(1) as f32;
        (1.0 + (share * span).round()) as u32
    }

    /// The amount as it reads now: what is typed, else the one chosen.
    fn shown(&self) -> u32 {
        self.typed
            .as_deref()
            .and_then(|t| t.parse::<u64>().ok())
            .map_or(self.amount, |n| n.clamp(1, u64::from(self.most())) as u32)
    }

    fn share_of(&self, amount: u32) -> f32 {
        let most = self.most();
        if most <= 1 {
            1.0
        } else {
            amount.saturating_sub(1) as f32 / (most - 1) as f32
        }
    }

    fn buttons(&self) -> [(Part, f32, f32, &'static str, Look); 2] {
        let body = self.fonts.body.as_deref();
        let split_w = button_width(body, "Split");
        let cancel_w = button_width(body, "Cancel");
        let split_x = WIDTH - PAD - split_w;
        let cancel_x = split_x - 4.0 - cancel_w;
        [
            (Part::Cancel, cancel_x, cancel_w, "Cancel", Look::Quiet),
            (Part::Split, split_x, split_w, "Split", Look::Main),
        ]
    }

    fn part_at(&self, x: f32, y: f32) -> Option<Part> {
        if (TRACK_TOP - 4.0..TRACK_TOP + TRACK_H + 4.0).contains(&y) {
            return Some(Part::Track);
        }
        if (ROW_TOP..ROW_TOP + ROW_H).contains(&y) {
            if (COUNT_LEFT..COUNT_LEFT + COUNT_W).contains(&x) {
                return Some(Part::Count);
            }
            return self
                .buttons()
                .into_iter()
                .find(|(_, left, w, ..)| (*left..left + w).contains(&x))
                .map(|(p, ..)| p);
        }
        None
    }

    /// The amount becomes `amount`, kept within the stack; what was typed
    /// gives way to it.
    fn choose(&mut self, amount: i64) {
        self.amount = amount.clamp(1, i64::from(self.most())) as u32;
        self.typed = None;
    }

    /// What was typed, read as an amount within the stack.
    fn take_typed(&mut self) {
        if let Some(t) = self.typed.take()
            && let Ok(n) = t.parse::<u64>()
        {
            self.amount = n.clamp(1, u64::from(self.most())) as u32;
        }
    }

    fn split(&mut self, ui: &mut UiRoot, me: ElementId) {
        self.take_typed();
        self.requests.push(SplitRequest::Split {
            item: self.item,
            amount: self.amount,
        });
        Self::close(ui, me);
    }

    fn key(&mut self, ui: &mut UiRoot, me: ElementId, key: Key, modifiers: Modifiers) {
        let step = if modifiers.contains(Modifiers::SHIFT) {
            10
        } else {
            1
        };
        match key {
            Key::Enter | Key::KeypadEnter => self.split(ui, me),
            Key::Escape => Self::close(ui, me),
            Key::Backspace => {
                let mut t = self.typed.take().unwrap_or_default();
                t.pop();
                self.typed = Some(t);
            }
            Key::Up | Key::Right => {
                self.take_typed();
                self.choose(i64::from(self.amount) + step);
            }
            Key::Down | Key::Left => {
                self.take_typed();
                self.choose(i64::from(self.amount) - step);
            }
            _ => {}
        }
    }
}

impl Widget for SplitPopover {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_popup_dismissed(&mut self, ui: &mut UiRoot, me: ElementId, _popup: ElementId) {
        Self::close(ui, me);
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            // Every key is the pop-up's while it has the keyboard.
            kind::KEY_DOWN => {
                self.key(ui, me, e.key, e.modifiers);
                true
            }
            kind::CHAR => {
                if let Some(c) = char::from_u32(e.data0 as u32).filter(char::is_ascii_digit) {
                    let t = self.typed.get_or_insert_with(String::new);
                    if t.len() < DIGITS {
                        t.push(c);
                    }
                }
                true
            }
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                if self.dragging {
                    let n = self.at_track(x);
                    self.choose(i64::from(n));
                }
                self.hovered = self.part_at(x, y);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::MOUSE_DOWN => {
                ui.set_keyboard_focus(Some(me));
                if self.part_at(x, y) == Some(Part::Track) {
                    self.dragging = true;
                    let n = self.at_track(x);
                    self.choose(i64::from(n));
                }
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.dragging = false;
                true
            }
            kind::SCROLL => {
                let step = if e.modifiers.contains(Modifiers::SHIFT) {
                    10
                } else {
                    1
                };
                self.take_typed();
                self.choose(i64::from(self.amount) + i64::from(e.data0.signum()) * step);
                true
            }
            kind::CLICK => {
                match self.part_at(x, y) {
                    Some(Part::Split) => self.split(ui, me),
                    Some(Part::Cancel) => Self::close(ui, me),
                    _ => {}
                }
                true
            }
            _ => true,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, me: ElementId, _dt: f64) {
        self.focused = ui.keyboard_focus() == Some(me);
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let (w, h) = (WIDTH, HEIGHT);
        let fill = tone::raised(0.98);
        let edge = tone::bronze(0.8);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 12.0, 30.0, p.shadow);
        // The pointer up to the stack.
        let v = |x: f32, y: f32, color: Color| ColorVertex { x, y, color };
        let ax = 28.0;
        let _ = rc.fill_convex_polygon(&[
            v(ax - 8.0, 0.5, edge),
            v(ax + 8.0, 0.5, edge),
            v(ax, -7.0, edge),
        ]);
        let round = CornerRadii::all(6.0);
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, fill);
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, edge, 1.0);
        let _ = rc.fill_convex_polygon(&[
            v(ax - 6.5, 1.5, fill),
            v(ax + 6.5, 1.5, fill),
            v(ax, -5.0, fill),
        ]);
        rc.fill(ax - 6.5, 0.0, 13.0, 1.0, fill);
        // What is typed counts as soon as it reads.
        let shown = self.shown();
        // The title and the stack.
        let stack_words = format!(
            "stack of {}",
            ac_ui::panels::appraisal::group_thousands(i64::from(self.stack))
        );
        let stack_w = self
            .fonts
            .small
            .as_deref()
            .map_or(0.0, |f| text::measure(f, &stack_words, 0.0));
        if let Some(f) = &self.fonts.title {
            let title = fit(
                f,
                &format!("Split {}", self.name),
                w - 2.0 * PAD - stack_w - 8.0,
            );
            text::draw(rc, f, &title, PAD, PAD, TextStyle::plain(p.text));
        }
        if let Some(f) = &self.fonts.small {
            text::draw(
                rc,
                f,
                &stack_words,
                w - PAD - stack_w,
                PAD + 2.0,
                TextStyle::plain(p.text_soft),
            );
            let ty = TRACK_TOP - f.line_height - 2.0;
            text::draw(rc, f, "Take", PAD, ty, TextStyle::plain(p.text_soft));
            let leaves = format!(
                "leaves {}",
                ac_ui::panels::appraisal::group_thousands(i64::from(
                    self.stack.saturating_sub(shown)
                ))
            );
            let lw = text::measure(f, &leaves, 0.0);
            text::draw(
                rc,
                f,
                &leaves,
                w - PAD - lw,
                ty,
                TextStyle::plain(p.text_soft),
            );
        }
        // The track.
        let (l, r) = self.track();
        let mid = TRACK_TOP + TRACK_H * 0.5;
        let bar = CornerRadii::all(2.0);
        rc.fill_rounded_rect(l, mid - 2.0, r - l, 4.0, bar, hex(0, 0.5));
        let knob = l + (r - l) * self.share_of(shown);
        rc.fill_rounded_rect(l, mid - 2.0, knob - l, 4.0, bar, p.accent);
        let held = if self.dragging || self.hovered == Some(Part::Track) {
            7.0
        } else {
            6.0
        };
        rc.fill_ellipse(
            knob - held - 1.0,
            mid - held - 1.0,
            (held + 1.0) * 2.0,
            (held + 1.0) * 2.0,
            tone::ink(0.9),
        );
        rc.fill_ellipse(knob - held, mid - held, held * 2.0, held * 2.0, p.accent);
        // The count row.
        if let Some(f) = &self.fonts.small {
            let ty = ROW_TOP + ((ROW_H - f.line_height) * 0.5).round();
            text::draw(rc, f, "Count", PAD, ty, TextStyle::plain(p.text_soft));
        }
        let box_edge = if self.focused {
            p.accent
        } else {
            tone::bronze(0.6)
        };
        let r4 = CornerRadii::all(4.0);
        rc.fill_rounded_rect(COUNT_LEFT, ROW_TOP, COUNT_W, ROW_H, r4, hex(0, 0.4));
        rc.stroke_rounded_rect(COUNT_LEFT, ROW_TOP, COUNT_W, ROW_H, r4, box_edge, 1.0);
        if let Some(f) = &self.fonts.body {
            let words = self
                .typed
                .clone()
                .unwrap_or_else(|| self.amount.to_string());
            let tw = text::measure(f, &words, 0.0);
            let tx = COUNT_LEFT + COUNT_W - 8.0 - tw;
            let ty = ROW_TOP + ((ROW_H - f.line_height) * 0.5).round();
            if self.focused && self.typed.is_none() {
                // The amount stands selected: the next digit replaces it.
                rc.fill_rounded_rect(
                    tx - 2.0,
                    ty - 1.0,
                    tw + 4.0,
                    f.line_height + 2.0,
                    CornerRadii::all(2.0),
                    tone::gold(0.35),
                );
            }
            text::draw(rc, f, &words, tx, ty, TextStyle::plain(p.text));
            if self.focused && self.typed.is_some() {
                rc.fill(tx + tw + 1.0, ty, 1.0, f.line_height, p.accent);
            }
        }
        let body = self.fonts.body.as_deref();
        for (part, left, bw, words, look) in self.buttons() {
            button(
                rc,
                &Horizon::current(),
                body,
                (left, ROW_TOP, bw, ROW_H),
                words,
                look,
                self.hovered == Some(part),
            );
        }
        if let Some(f) = &self.fonts.small {
            let tip = "The target line's Split amount moves part of a stack on any drag.";
            for (i, line) in crate::ui::modern::bars::edit::wrap(f, tip, w - 2.0 * PAD)
                .iter()
                .take(2)
                .enumerate()
            {
                text::draw(
                    rc,
                    f,
                    line,
                    PAD,
                    TIP_TOP + i as f32 * (f.line_height + 1.0),
                    TextStyle::plain(p.muted),
                );
            }
        }
    }
}
