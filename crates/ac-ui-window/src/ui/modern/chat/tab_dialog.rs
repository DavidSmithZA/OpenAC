//! The dialog that makes a chat tab or changes one: its name, the message
//! types it shows, the channel it talks on, whether it opens as its own
//! window and that window's background.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_runtime::communication::ChatChannelKind;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::geometry::AnchorEdges;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::widgets::Field;
use ac_ui::{UiFont, UiRoot};

use super::channels::{channel_color, channel_name};
use super::model::{ChatTab, DEFAULT_OPACITY, GROUPS, group_mask};

/// The dialog's size.
pub const WIDTH: f32 = 380.0;
pub const HEIGHT: f32 = 400.0;
const PAD: f32 = 16.0;
const NAME_TOP: f32 = 58.0;
const SHOWS_TOP: f32 = 112.0;
const CELL: f32 = 22.0;
const TALK_TOP: f32 = SHOWS_TOP + 18.0 + 5.0 * CELL + 12.0;
const WINDOW_TOP: f32 = TALK_TOP + 34.0;
const OPACITY_TOP: f32 = WINDOW_TOP + 30.0;
const FOOT_TOP: f32 = HEIGHT - 44.0;

/// The channels a tab can talk on, in the order the chip steps through
/// them (none: the tab keeps the last channel used).
const TALKS: [Option<ChatChannelKind>; 13] = [
    None,
    Some(ChatChannelKind::Say),
    Some(ChatChannelKind::Fellowship),
    Some(ChatChannelKind::Allegiance),
    Some(ChatChannelKind::Monarch),
    Some(ChatChannelKind::Patron),
    Some(ChatChannelKind::Vassals),
    Some(ChatChannelKind::General),
    Some(ChatChannelKind::Trade),
    Some(ChatChannelKind::Lfg),
    Some(ChatChannelKind::Roleplay),
    Some(ChatChannelKind::Society),
    Some(ChatChannelKind::Tell),
];

/// What the dialog ended with.
#[derive(Debug, Clone, PartialEq)]
pub enum TabDialogDone {
    /// Make the tab (none) or change tab `n`.
    Save(Option<usize>, ChatTab),
    /// Delete tab `n`.
    Delete(usize),
}

/// The fonts the dialog uses.
#[derive(Debug, Clone, Default)]
pub struct TabDialogFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// Labels and buttons.
    pub body: Option<Arc<UiFont>>,
    /// Headings and notes.
    pub small: Option<Arc<UiFont>>,
}

/// A clickable part of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Group(usize),
    Talk,
    Window,
    Opacity,
    Delete,
    Cancel,
    Save,
}

/// The tab dialog.
#[derive(Debug)]
pub struct TabDialog {
    theme: Horizon,
    fonts: TabDialogFonts,
    /// The name field.
    pub field: ElementId,
    /// The tab being changed (none: a new one).
    editing: Option<usize>,
    /// The tab as it was (its id, and filter bits no group names).
    base: Option<ChatTab>,
    groups: [bool; GROUPS.len()],
    talk: usize,
    window: bool,
    opacity: f32,
    /// The tab being changed is Main (it cannot be deleted or popped out).
    main: bool,
    hovered: Option<Part>,
    dragging: bool,
    /// How it ended (the window acts, then clears it).
    pub done: Option<TabDialogDone>,
}

impl TabDialog {
    /// Mounts it, hidden, under the root.
    pub fn mount(ui: &mut UiRoot, theme: Horizon, fonts: TabDialogFonts) -> ElementId {
        let root = ui.root();
        let body = fonts.body.clone();
        let me = ui.add(
            root,
            Element::at(0.0, 0.0, WIDTH, HEIGHT)
                .with_visible(false)
                .with_anchors(AnchorEdges::NONE),
            Self {
                theme,
                fonts,
                field: root,
                editing: None,
                base: None,
                groups: [false; GROUPS.len()],
                talk: 0,
                window: false,
                opacity: DEFAULT_OPACITY,
                main: false,
                hovered: None,
                dragging: false,
                done: None,
            },
        );
        let mut e = Element::at(PAD + 1.0, NAME_TOP + 19.0, WIDTH - 2.0 * PAD - 2.0, 24.0)
            .with_anchors(AnchorEdges::NONE);
        Field::configure(&mut e);
        e.tab_stop = true;
        let mut f = Field::default();
        f.font = body;
        f.one_line = true;
        f.selectable = true;
        f.record_history = false;
        f.max_characters = 20;
        f.padding = 6.0;
        f.text_color = Horizon::current().palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = tone::gold(0.35);
        f.set_editable_detached(&mut e, true);
        let field = ui.add(me, e, f);
        if let Some(d) = ui.widget_mut::<Self>(me) {
            d.field = field;
        }
        me
    }

    /// Shows it for a new tab (`editing` none) or to change `tab`, `n`.
    pub fn show(ui: &mut UiRoot, me: ElementId, editing: Option<(usize, &ChatTab)>) {
        let screen = ui.size();
        let Some(field) = ui.widget_mut::<Self>(me).map(|d| {
            d.editing = editing.map(|(n, _)| n);
            d.base = editing.map(|(_, t)| t.clone());
            d.main = editing.is_some_and(|(n, _)| n == 0);
            let filter = editing.map_or(0, |(_, t)| t.filter);
            for (i, on) in d.groups.iter_mut().enumerate() {
                *on = filter & group_mask(i) != 0;
            }
            let talk = editing.and_then(|(_, t)| t.talk);
            d.talk = TALKS.iter().position(|t| *t == talk).unwrap_or(0);
            d.window = editing.is_some_and(|(_, t)| t.window);
            d.opacity = editing.map_or(DEFAULT_OPACITY, |(_, t)| t.opacity);
            d.done = None;
            d.field
        }) else {
            return;
        };
        if let Some(f) = ui.widget_mut::<Field>(field) {
            f.set_text(editing.map_or("", |(_, t)| t.name.as_str()));
        }
        if let Some(e) = ui.element_mut(me) {
            e.left = ((screen.0 - WIDTH) * 0.5).round();
            e.top = ((screen.1 - HEIGHT) * 0.5).round();
        }
        ui.set_visible(me, true);
        ui.bring_to_front(me);
        ui.set_keyboard_focus(Some(field));
    }

    /// Hides it.
    pub fn hide(ui: &mut UiRoot, me: ElementId) {
        ui.set_keyboard_focus(None);
        ui.set_visible(me, false);
    }

    fn column_width() -> f32 {
        (WIDTH - 2.0 * PAD) / 3.0
    }

    fn buttons(&self) -> Vec<(Part, &'static str, f32, f32)> {
        let body = self.fonts.body.as_deref();
        let w = |s: &str| body.map_or(70.0, |f| text::measure(f, s, 0.0)) + 24.0;
        let save = if self.editing.is_some() {
            "Save"
        } else {
            "Make tab"
        };
        let right = WIDTH - PAD;
        let (sw, cw) = (w(save), w("Cancel"));
        let mut out = vec![
            (Part::Save, save, right - sw, sw),
            (Part::Cancel, "Cancel", right - sw - 8.0 - cw, cw),
        ];
        if self.editing.is_some() && !self.main {
            out.push((Part::Delete, "Delete tab", PAD, w("Delete tab")));
        }
        out
    }

    fn part_at(&self, x: f32, y: f32) -> Option<Part> {
        let cw = Self::column_width();
        let grid_top = SHOWS_TOP + 18.0;
        if (grid_top..grid_top + 5.0 * CELL).contains(&y) && (PAD..WIDTH - PAD).contains(&x) {
            let col = ((x - PAD) / cw).floor() as usize;
            let row = ((y - grid_top) / CELL).floor() as usize;
            let i = col * 5 + row;
            return (i < GROUPS.len()).then_some(Part::Group(i));
        }
        if (TALK_TOP..TALK_TOP + 26.0).contains(&y) && x >= PAD {
            return Some(Part::Talk);
        }
        if (WINDOW_TOP..WINDOW_TOP + 22.0).contains(&y) && x >= PAD && !self.main {
            return Some(Part::Window);
        }
        if (OPACITY_TOP..OPACITY_TOP + 22.0).contains(&y) && (PAD..WIDTH - PAD).contains(&x) {
            return Some(Part::Opacity);
        }
        if (FOOT_TOP..FOOT_TOP + 28.0).contains(&y) {
            return self
                .buttons()
                .into_iter()
                .find(|(_, _, left, w)| (*left..left + w).contains(&x))
                .map(|(p, ..)| p);
        }
        None
    }

    fn opacity_at(x: f32) -> f32 {
        let left = PAD + 90.0;
        let right = WIDTH - PAD - 40.0;
        ((x - left) / (right - left)).clamp(0.1, 1.0)
    }

    fn finish(&mut self, ui: &mut UiRoot, me: ElementId) {
        let name = ui
            .widget::<Field>(self.field)
            .map(|f| f.text().trim().to_owned())
            .unwrap_or_default();
        if name.is_empty() {
            ui.set_keyboard_focus(Some(self.field));
            return;
        }
        let named: u64 = (0..GROUPS.len()).map(group_mask).fold(0, |m, g| m | g);
        let kept = self.base.as_ref().map_or(0, |t| t.filter & !named);
        let chosen = self
            .groups
            .iter()
            .enumerate()
            .filter(|(_, on)| **on)
            .fold(0, |m, (i, _)| m | group_mask(i));
        let tab = ChatTab {
            id: self.base.as_ref().map_or(0, |t| t.id),
            name,
            filter: kept | chosen,
            talk: TALKS[self.talk],
            window: self.window && !self.main,
            opacity: self.opacity,
            place: self.base.as_ref().and_then(|t| t.place),
        };
        self.done = Some(TabDialogDone::Save(self.editing, tab));
        Self::hide(ui, me);
    }

    fn talk_words(&self) -> String {
        match TALKS[self.talk] {
            None => "The last channel used".to_owned(),
            Some(ChatChannelKind::Tell) => "Tells to the last teller".to_owned(),
            Some(k) => channel_name(k).to_owned(),
        }
    }
}

impl Widget for TabDialog {
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
        if e.kind == kind::KEY_DOWN {
            match e.key {
                ac_ui::Key::Enter => {
                    self.finish(ui, me);
                    return true;
                }
                ac_ui::Key::Escape => {
                    Self::hide(ui, me);
                    return true;
                }
                _ => {}
            }
        }
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                if self.dragging {
                    self.opacity = Self::opacity_at(x);
                }
                self.hovered = self.part_at(x, y);
            }
            kind::HOVER_LEAVE => self.hovered = None,
            kind::MOUSE_DOWN if self.part_at(x, y) == Some(Part::Opacity) => {
                self.dragging = true;
                self.opacity = Self::opacity_at(x);
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => self.dragging = false,
            kind::RIGHT_CLICK if self.part_at(x, y) == Some(Part::Talk) => {
                self.talk = (self.talk + TALKS.len() - 1) % TALKS.len();
            }
            kind::CLICK => match self.part_at(x, y) {
                Some(Part::Group(i)) => self.groups[i] = !self.groups[i],
                Some(Part::Talk) => self.talk = (self.talk + 1) % TALKS.len(),
                Some(Part::Window) => self.window = !self.window,
                Some(Part::Delete) => {
                    if let Some(n) = self.editing {
                        self.done = Some(TabDialogDone::Delete(n));
                    }
                    Self::hide(ui, me);
                }
                Some(Part::Cancel) => Self::hide(ui, me),
                Some(Part::Save) => self.finish(ui, me),
                _ => {}
            },
            _ => {}
        }
        true
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, WIDTH, HEIGHT, 6.0, 14.0, 34.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, WIDTH, HEIGHT, round, tone::raised(0.98));
        rc.stroke_rounded_rect(0.0, 0.0, WIDTH, HEIGHT, round, tone::bronze(0.8), 1.0);
        let (Some(title), Some(body), Some(small)) = (
            self.fonts.title.clone(),
            self.fonts.body.clone(),
            self.fonts.small.clone(),
        ) else {
            return;
        };
        let heading = |rc: &mut RenderContext<'_>, words: &str, y: f32| {
            text::draw(
                rc,
                &small,
                words,
                PAD,
                y,
                TextStyle::plain(tone::soft(1.0)).tracked(1.1),
            );
        };
        let words = if self.editing.is_some() {
            "Change chat tab"
        } else {
            "New chat tab"
        };
        text::draw(rc, &title, words, PAD, 16.0, TextStyle::plain(p.text));
        heading(rc, "NAME", NAME_TOP);
        let r4 = CornerRadii::all(4.0);
        rc.fill_rounded_rect(
            PAD,
            NAME_TOP + 18.0,
            WIDTH - 2.0 * PAD,
            26.0,
            r4,
            hex(0, 0.4),
        );
        rc.stroke_rounded_rect(
            PAD,
            NAME_TOP + 18.0,
            WIDTH - 2.0 * PAD,
            26.0,
            r4,
            tone::bronze(0.6),
            1.0,
        );
        heading(rc, "SHOWS", SHOWS_TOP);
        let cw = Self::column_width();
        for (i, (label, _)) in GROUPS.iter().enumerate() {
            let x = PAD + (i / 5) as f32 * cw;
            let y = SHOWS_TOP + 18.0 + (i % 5) as f32 * CELL;
            let on = self.groups[i];
            let box_y = y + (CELL - 14.0) * 0.5;
            let r3 = CornerRadii::all(3.0);
            if on {
                rc.fill_rounded_rect(
                    x,
                    box_y,
                    14.0,
                    14.0,
                    r3,
                    Color {
                        a: 0.85,
                        ..tone::gold(1.0)
                    },
                );
                rc.smooth_line(
                    x + 3.5,
                    box_y + 7.0,
                    x + 6.0,
                    box_y + 10.0,
                    tone::ink(1.0),
                    1.8,
                );
                rc.smooth_line(
                    x + 6.0,
                    box_y + 10.0,
                    x + 10.5,
                    box_y + 4.0,
                    tone::ink(1.0),
                    1.8,
                );
            } else {
                rc.stroke_rounded_rect(x, box_y, 14.0, 14.0, r3, tone::bronze(0.45), 1.0);
            }
            let ty = (y + (CELL - small.line_height) * 0.5).round();
            text::draw(
                rc,
                &small,
                label,
                x + 21.0,
                ty,
                TextStyle::plain(if on { p.text } else { tone::soft(1.0) }),
            );
        }
        // Talks on: a chip that steps through the channels.
        let ty = (TALK_TOP + (26.0 - body.line_height) * 0.5).round();
        text::draw(
            rc,
            &body,
            "Talks on",
            PAD,
            ty,
            TextStyle::plain(tone::soft(1.0)),
        );
        let talk = self.talk_words();
        let c = TALKS[self.talk].map_or(p.text, channel_color);
        let cx = PAD + 90.0;
        let w = text::measure(&body, &talk, 0.0) + 30.0;
        rc.fill_rounded_rect(cx, TALK_TOP, w, 26.0, r4, Color { a: 0.12, ..c });
        rc.stroke_rounded_rect(cx, TALK_TOP, w, 26.0, r4, Color { a: 0.45, ..c }, 1.0);
        text::draw(rc, &body, &talk, cx + 10.0, ty, TextStyle::plain(c));
        text::draw(
            rc,
            &small,
            "click to change",
            cx + w + 8.0,
            (TALK_TOP + (26.0 - small.line_height) * 0.5).round(),
            TextStyle::plain(tone::muted(1.0)),
        );
        // Own window: a switch.
        rc.push_alpha(if self.main { 0.4 } else { 1.0 });
        let sy = WINDOW_TOP + 3.0;
        let fill = if self.window {
            Color {
                a: 0.85,
                ..tone::gold(1.0)
            }
        } else {
            hex(0, 0.45)
        };
        rc.fill_rounded_rect(PAD, sy, 30.0, 16.0, CornerRadii::all(8.0), fill);
        rc.stroke_rounded_rect(
            PAD,
            sy,
            30.0,
            16.0,
            CornerRadii::all(8.0),
            tone::bronze(0.6),
            1.0,
        );
        let knob = if self.window { PAD + 16.0 } else { PAD + 2.0 };
        rc.fill_ellipse(knob, sy + 2.0, 12.0, 12.0, p.text);
        let ty = (WINDOW_TOP + (22.0 - body.line_height) * 0.5).round();
        text::draw(
            rc,
            &body,
            "Own window",
            PAD + 40.0,
            ty,
            TextStyle::plain(p.text),
        );
        rc.pop_alpha();
        // The window's background.
        let ty = (OPACITY_TOP + (22.0 - small.line_height) * 0.5).round();
        text::draw(
            rc,
            &small,
            "Background",
            PAD,
            ty,
            TextStyle::plain(tone::soft(1.0)),
        );
        let (left, right) = (PAD + 90.0, WIDTH - PAD - 40.0);
        let mid = OPACITY_TOP + 11.0;
        let r2 = CornerRadii::all(2.0);
        rc.fill_rounded_rect(left, mid - 2.0, right - left, 4.0, r2, hex(0xFF_FFFF, 0.12));
        let at = left + (right - left) * ((self.opacity - 0.1) / 0.9).clamp(0.0, 1.0);
        rc.fill_rounded_rect(left, mid - 2.0, at - left, 4.0, r2, tone::gold(1.0));
        rc.fill_ellipse(at - 6.0, mid - 6.0, 12.0, 12.0, tone::gold(1.0));
        let pct = format!("{}%", (self.opacity * 100.0).round() as i32);
        text::draw(rc, &small, &pct, right + 8.0, ty, TextStyle::plain(p.text));
        // The foot.
        for (part, words, left, w) in self.buttons() {
            let main = part == Part::Save;
            let hovered = self.hovered == Some(part);
            let (fill, ink) = match (main, part) {
                (true, _) => (
                    Color {
                        a: 0.85,
                        ..tone::gold(1.0)
                    },
                    tone::ink(1.0),
                ),
                (false, Part::Delete) => (hex(0, 0.3), hex(0xFF_8A8E, 1.0)),
                _ => (hex(0, 0.3), p.text),
            };
            rc.fill_rounded_rect(left, FOOT_TOP, w, 28.0, r4, fill);
            let edge = if hovered {
                tone::gold(1.0)
            } else {
                tone::bronze(0.5)
            };
            rc.stroke_rounded_rect(left, FOOT_TOP, w, 28.0, r4, edge, 1.0);
            let ty = (FOOT_TOP + (28.0 - body.line_height) * 0.5).round();
            text::draw_centred(rc, &body, words, left + w * 0.5, ty, TextStyle::plain(ink));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changed_tab_keeps_its_id_and_the_bits_no_group_names() {
        let mut ui = UiRoot::new();
        let me = TabDialog::mount(&mut ui, Horizon::current(), TabDialogFonts::default());
        let mut tab = ChatTab::defaults().remove(0);
        tab.filter |= 1 << 40;
        TabDialog::show(&mut ui, me, Some((0, &tab)));
        ui.with::<TabDialog, _>(me, |d, ui| {
            assert!(d.main);
            d.groups[11] = false; // Combat off.
            d.finish(ui, me);
        });
        let done = ui.widget_mut::<TabDialog>(me).unwrap().done.take();
        let Some(TabDialogDone::Save(Some(0), saved)) = done else {
            panic!("{done:?}");
        };
        assert_eq!(saved.id, tab.id);
        assert_ne!(saved.filter & (1 << 40), 0);
        assert_eq!(saved.filter & group_mask(11), 0);
        assert!(!saved.window);
    }

    #[test]
    fn a_tab_needs_a_name() {
        let mut ui = UiRoot::new();
        let me = TabDialog::mount(&mut ui, Horizon::current(), TabDialogFonts::default());
        TabDialog::show(&mut ui, me, None);
        ui.with::<TabDialog, _>(me, |d, ui| d.finish(ui, me));
        assert!(ui.widget::<TabDialog>(me).unwrap().done.is_none());
    }
}
