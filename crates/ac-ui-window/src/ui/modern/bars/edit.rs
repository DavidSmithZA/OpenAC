//! Edit layout: the world dimmed under a guide down the middle, the
//! toolbar over it (which layout, a new bar, reset, done), the pop-up that
//! sets a slot's keys, the small menus (a bar's, the layout's, quick
//! move's) and the box that renames a bar.
//!
//! Like the bars' widgets, these only note what was asked.

use ac_ui::draw::{Color, ColorVertex, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::geometry::AnchorEdges;
use ac_ui::input::KeyChord;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::widgets::Field;
use ac_ui::{UiFont, UiRoot};

use super::keys;
use super::model::KeyMode;
use super::widgets::BarFonts;

/// How a button looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Look {
    /// Outlined.
    Plain,
    /// Words only.
    Quiet,
    /// The main one: filled.
    Main,
}

/// Draws a button `w` wide at (`x`, `y`), 28 or 30 high, with `words`.
#[allow(clippy::too_many_arguments)]
fn button(
    rc: &mut RenderContext<'_>,
    theme: &Horizon,
    font: Option<&UiFont>,
    (x, y, w, h): (f32, f32, f32, f32),
    words: &str,
    look: Look,
    hovered: bool,
) {
    let p = theme.palette;
    let round = CornerRadii::all(4.0);
    let ink = match look {
        Look::Plain => {
            let fill = if hovered { hex(0x2A_2116, 0.9) } else { hex(0, 0.35) };
            rc.fill_rounded_rect(x, y, w, h, round, fill);
            rc.stroke_rounded_rect(x, y, w, h, round, hex(0xC9_A062, 0.6), 1.0);
            p.text
        }
        Look::Quiet => {
            if hovered {
                rc.fill_rounded_rect(x, y, w, h, round, hex(0x2A_2116, 0.7));
            }
            p.text_soft
        }
        Look::Main => {
            let fill = if hovered {
                hex(0xE0_B068, 1.0)
            } else {
                p.accent_fill
            };
            rc.fill_rounded_rect(x, y, w, h, round, fill);
            rc.stroke_rounded_rect(x, y, w, h, round, p.accent, 1.0);
            p.accent_ink
        }
    };
    if let Some(f) = font {
        let ty = y + ((h - f.line_height) * 0.5).round();
        text::draw_centred(rc, f, words, x + w * 0.5, ty, TextStyle::plain(ink));
    }
}

/// The width a button needs for `words`.
fn button_width(font: Option<&UiFont>, words: &str) -> f32 {
    font.map_or(60.0, |f| text::measure(f, words, 0.0)).ceil() + 24.0
}

/// A small triangle pointing down, centred on (`cx`, `cy`).
fn caret(rc: &mut RenderContext<'_>, cx: f32, cy: f32, color: Color) {
    let v = |x: f32, y: f32| ColorVertex { x, y, color };
    let _ = rc.fill_convex_polygon(&[
        v(cx - 3.5, cy - 2.0),
        v(cx + 3.5, cy - 2.0),
        v(cx, cy + 2.5),
    ]);
}

/// Splits `words` into lines no wider than `width` in `font`.
#[must_use]
pub fn wrap(font: &UiFont, words: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in words.split_whitespace() {
        let next = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && text::measure(font, &next, 0.0) > width {
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The world dimmed while editing, with a guide down the middle. Clicks go
/// through it.
#[derive(Debug)]
pub struct Dim;

impl Widget for Dim {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        false
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        rc.fill(0.0, 0.0, w, h, hex(0x0A_0806, 0.45));
        // A dashed line down the middle, under the toolbar.
        let x = (w * 0.5).floor();
        let mut y = 120.0;
        while y < h {
            rc.fill(x, y, 1.0, 6.0, hex(0xE0_B068, 0.55));
            y += 10.0;
        }
    }
}

/// A button of the edit toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarPart {
    /// Which layout (this character's own or the shared one; copy one).
    Layout,
    /// A new bar.
    NewBar,
    /// The layout made again from the retail bars.
    Reset,
    /// Edit layout ends.
    Done,
}

/// The toolbar across the top while editing.
#[derive(Debug)]
pub struct EditToolbar {
    theme: Horizon,
    fonts: BarFonts,
    /// What the Layout button says the layout is ("Aerin Vale only",
    /// "Shared").
    pub layout_words: String,
    /// Reset was pressed once: the next press resets.
    pub reset_armed: bool,
    /// What it asked for since the window last looked.
    pub pressed: Vec<ToolbarPart>,
    hovered: Option<ToolbarPart>,
}

/// The toolbar's height.
pub const TOOLBAR_HEIGHT: f32 = 46.0;
const TITLE: &str = "Editing your HUD";
const HINT: &str = "Drag a bar by its tab \u{b7} click a slot to set its keys";

impl EditToolbar {
    /// The toolbar in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: BarFonts) -> Self {
        Self {
            theme,
            fonts,
            layout_words: String::new(),
            reset_armed: false,
            pressed: Vec::new(),
            hovered: None,
        }
    }

    fn reset_words(&self) -> &'static str {
        if self.reset_armed {
            "Click again to reset"
        } else {
            "Reset layout"
        }
    }

    /// Each part with its left edge and width, and the whole width.
    fn parts(&self) -> (Vec<(ToolbarPart, f32, f32)>, f32) {
        let title = self.fonts.title.as_deref();
        let small = self.fonts.small.as_deref();
        let body = self.fonts.body.as_deref();
        let mut x = 16.0;
        x += title.map_or(120.0, |f| text::measure(f, TITLE, 0.0)) + 10.0;
        x += small.map_or(200.0, |f| text::measure(f, HINT, 0.0)) + 10.0;
        let layout = body.map_or(120.0, |f| {
            text::measure(f, "Layout", 0.0) + 6.0 + text::measure(f, &self.layout_words, 0.0)
        }) + 20.0
            + 14.0;
        let mut parts = vec![(ToolbarPart::Layout, x, layout.ceil())];
        x += layout.ceil() + 10.0;
        for (part, words) in [
            (ToolbarPart::NewBar, "+ New bar"),
            (ToolbarPart::Reset, self.reset_words()),
            (ToolbarPart::Done, "Done"),
        ] {
            let w = button_width(body, words);
            parts.push((part, x, w));
            x += w + 10.0;
        }
        (parts, x)
    }

    /// Its width.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.parts().1
    }

    fn part_at(&self, x: f32) -> Option<ToolbarPart> {
        self.parts()
            .0
            .into_iter()
            .find(|(_, left, w)| x >= *left && x < left + w)
            .map(|(p, _, _)| p)
    }
}

impl Widget for EditToolbar {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let x = e.data1 as f32;
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = self.part_at(x);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::CLICK => {
                if let Some(part) = self.part_at(x) {
                    self.pressed.push(part);
                }
                true
            }
            _ => true,
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        Some(
            match self.hovered? {
                ToolbarPart::Layout => {
                    "This character only, a layout your characters share, or copy one from another character"
                }
                ToolbarPart::NewBar => "A new empty bar in the middle of the screen",
                ToolbarPart::Reset => "Make the bars again from your retail toolbar and spell bars",
                ToolbarPart::Done => "Leave Edit layout",
            }
            .to_owned(),
        )
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 10.0, 30.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, hex(0x18_130E, 0.96));
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, hex(0xC9_A062, 0.8), 1.0);
        let mut x = 16.0;
        if let Some(f) = &self.fonts.title {
            text::draw(rc, f, TITLE, x, ((h - f.line_height) * 0.5).round(), TextStyle::plain(p.text));
            x += text::measure(f, TITLE, 0.0) + 10.0;
        }
        if let Some(f) = &self.fonts.small {
            text::draw(rc, f, HINT, x, ((h - f.line_height) * 0.5).round(), TextStyle::plain(p.text_soft));
        }
        let (parts, _) = self.parts();
        let body = self.fonts.body.as_deref();
        for (part, left, bw) in parts {
            let hovered = self.hovered == Some(part);
            let top = (h - 30.0) * 0.5;
            match part {
                ToolbarPart::Layout => {
                    button(rc, &self.theme, None, (left, top, bw, 30.0), "", Look::Plain, hovered);
                    if let Some(f) = body {
                        let ty = top + ((30.0 - f.line_height) * 0.5).round();
                        text::draw(rc, f, "Layout", left + 10.0, ty, TextStyle::plain(p.text_soft));
                        let lx = left + 10.0 + text::measure(f, "Layout", 0.0) + 6.0;
                        text::draw(rc, f, &self.layout_words, lx, ty, TextStyle::plain(p.text));
                    }
                    caret(rc, left + bw - 13.0, top + 15.0, p.text_soft);
                }
                ToolbarPart::NewBar => {
                    button(rc, &self.theme, body, (left, top, bw, 30.0), "+ New bar", Look::Plain, hovered);
                }
                ToolbarPart::Reset => button(
                    rc,
                    &self.theme,
                    body,
                    (left, top, bw, 30.0),
                    self.reset_words(),
                    if self.reset_armed { Look::Plain } else { Look::Quiet },
                    hovered,
                ),
                ToolbarPart::Done => {
                    button(rc, &self.theme, body, (left, top, bw, 30.0), "Done", Look::Main, hovered);
                }
            }
        }
    }
}

/// What the key pop-up asks of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverRequest {
    /// Listen for the key of this mode (or stop: none).
    Listen(Option<KeyMode>),
    /// Both keys cleared.
    ClearKeys,
    /// The slot emptied.
    Empty,
    /// Closed.
    Done,
}

/// The key pop-up's width and height.
pub const POPOVER_WIDTH: f32 = 300.0;
pub const POPOVER_HEIGHT: f32 = 222.0;
const POP_PAD: f32 = 14.0;
const ROWS_TOP: f32 = 56.0;
const ROW: f32 = 28.0;
const ROW_GAP: f32 = 8.0;
const KEY_BOX: f32 = 96.0;
const NOTE_TOP: f32 = ROWS_TOP + ROW * 2.0 + ROW_GAP + 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PopPart {
    Key(KeyMode),
    Clear,
    Empty,
    Done,
}

/// The pop-up that sets a slot's two keys.
#[derive(Debug)]
pub struct KeyPopover {
    theme: Horizon,
    fonts: BarFonts,
    /// What the slot holds ("Health Potion", "Empty slot").
    pub title: String,
    /// Which slot ("Main bar, slot 1").
    pub where_words: String,
    /// The slot's keys.
    pub keys: [Option<KeyChord>; 2],
    /// The key being listened for.
    pub listening: Option<KeyMode>,
    /// What the last change did, or a hint.
    pub note: String,
    /// The slot holds something (it can be emptied).
    pub filled: bool,
    /// Where the pointer to the slot sits along the bottom (or top) edge,
    /// and whether it is on top (the pop-up is under the slot).
    pub arrow: (f32, bool),
    /// What it asked for since the window last looked.
    pub requests: Vec<PopoverRequest>,
    hovered: Option<PopPart>,
}

impl KeyPopover {
    /// The pop-up in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: BarFonts) -> Self {
        Self {
            theme,
            fonts,
            title: String::new(),
            where_words: String::new(),
            keys: [None; 2],
            listening: None,
            note: String::new(),
            filled: false,
            arrow: (POPOVER_WIDTH * 0.5, false),
            requests: Vec::new(),
            hovered: None,
        }
    }

    fn buttons(&self) -> Vec<(PopPart, f32, f32, &'static str, Look)> {
        let body = self.fonts.body.as_deref();
        let mut x = POPOVER_WIDTH - POP_PAD;
        let mut out = Vec::new();
        for (part, words, look) in [
            (PopPart::Done, "Done", Look::Main),
            (PopPart::Empty, "Empty slot", Look::Quiet),
            (PopPart::Clear, "Clear keys", Look::Quiet),
        ] {
            if part == PopPart::Empty && !self.filled {
                continue;
            }
            let w = button_width(body, words);
            x -= w;
            out.push((part, x, w, words, look));
            x -= 6.0;
        }
        out
    }

    fn part_at(&self, x: f32, y: f32) -> Option<PopPart> {
        let box_left = POPOVER_WIDTH - POP_PAD - KEY_BOX;
        for (i, mode) in KeyMode::ALL.into_iter().enumerate() {
            let top = ROWS_TOP + i as f32 * (ROW + ROW_GAP);
            if x >= box_left && x < box_left + KEY_BOX && y >= top && y < top + ROW {
                return Some(PopPart::Key(mode));
            }
        }
        let top = POPOVER_HEIGHT - POP_PAD - 28.0;
        if y >= top && y < top + 28.0 {
            return self
                .buttons()
                .into_iter()
                .find(|(_, left, w, _, _)| x >= *left && x < left + w)
                .map(|(p, ..)| p);
        }
        None
    }
}

impl Widget for KeyPopover {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn clips_children(&self) -> bool {
        false
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = self.part_at(x, y);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::CLICK => {
                match self.part_at(x, y) {
                    Some(PopPart::Key(mode)) => {
                        let next = (self.listening != Some(mode)).then_some(mode);
                        self.requests.push(PopoverRequest::Listen(next));
                    }
                    Some(PopPart::Clear) => self.requests.push(PopoverRequest::ClearKeys),
                    Some(PopPart::Empty) => self.requests.push(PopoverRequest::Empty),
                    Some(PopPart::Done) => self.requests.push(PopoverRequest::Done),
                    None => {}
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let (w, h) = (POPOVER_WIDTH, POPOVER_HEIGHT);
        let fill = hex(0x18_130E, 0.98);
        let edge = hex(0xC9_A062, 0.9);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 12.0, 30.0, p.shadow);
        // The pointer to the slot.
        let (ax, on_top) = self.arrow;
        let ax = ax.clamp(16.0, w - 16.0);
        let v = |x: f32, y: f32, color: Color| ColorVertex { x, y, color };
        let (base, tip) = if on_top { (0.5, -7.0) } else { (h - 0.5, h + 7.0) };
        let _ = rc.fill_convex_polygon(&[v(ax - 8.0, base, edge), v(ax + 8.0, base, edge), v(ax, tip, edge)]);
        let inner = if on_top { (1.5, -5.0) } else { (h - 1.5, h + 5.0) };
        let _ = rc.fill_convex_polygon(&[
            v(ax - 6.5, inner.0, fill),
            v(ax + 6.5, inner.0, fill),
            v(ax, inner.1, fill),
        ]);
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, fill);
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, edge, 1.0);
        // Cover the edge where the pointer joins.
        rc.fill(ax - 6.5, if on_top { 0.0 } else { h - 1.0 }, 13.0, 1.0, fill);
        if let Some(f) = &self.fonts.title {
            let title = crate::ui::modern::hud::target::fit(f, &self.title, w - 2.0 * POP_PAD);
            text::draw(rc, f, &title, POP_PAD, POP_PAD, TextStyle::plain(p.text));
        }
        if let Some(f) = &self.fonts.small {
            text::draw(rc, f, &self.where_words, POP_PAD, POP_PAD + 19.0, TextStyle::plain(p.text_soft));
        }
        let box_left = w - POP_PAD - KEY_BOX;
        for (i, mode) in KeyMode::ALL.into_iter().enumerate() {
            let top = ROWS_TOP + i as f32 * (ROW + ROW_GAP);
            if let Some(f) = &self.fonts.body {
                let ty = top + ((ROW - f.line_height) * 0.5).round();
                text::draw(rc, f, mode.words(), POP_PAD, ty, TextStyle::plain(p.text));
            }
            let listening = self.listening == Some(mode);
            let hovered = self.hovered == Some(PopPart::Key(mode));
            let round = CornerRadii::all(4.0);
            if listening {
                rc.fill_rounded_rect(box_left, top, KEY_BOX, ROW, round, hex(0xE0_B068, 0.12));
                rc.stroke_rounded_rect(box_left, top, KEY_BOX, ROW, round, p.accent, 2.0);
            } else {
                let fill = if hovered { hex(0x2A_2116, 0.9) } else { hex(0, 0.4) };
                rc.fill_rounded_rect(box_left, top, KEY_BOX, ROW, round, fill);
                rc.stroke_rounded_rect(box_left, top, KEY_BOX, ROW, round, hex(0xC9_A062, 0.5), 1.0);
            }
            let Some(f) = &self.fonts.body else {
                continue;
            };
            let ty = top + ((ROW - f.line_height) * 0.5).round();
            let cx = box_left + KEY_BOX * 0.5;
            if listening {
                let small = self.fonts.small.as_deref().unwrap_or(f);
                let ty = top + ((ROW - small.line_height) * 0.5).round();
                text::draw_centred(rc, small, "Press a key\u{2026}", cx, ty, TextStyle::plain(p.accent));
            } else if let Some(chord) = self.keys[i] {
                let kw = keys::drawn_width(f, chord);
                keys::draw(rc, f, chord, (cx - kw * 0.5).round(), ty, p.text, None);
            } else {
                text::draw_centred(rc, f, "None", cx, ty, TextStyle::plain(p.muted));
            }
        }
        if let Some(f) = &self.fonts.small {
            for (i, line) in wrap(f, &self.note, w - 2.0 * POP_PAD).iter().take(3).enumerate() {
                text::draw(
                    rc,
                    f,
                    line,
                    POP_PAD,
                    NOTE_TOP + i as f32 * (f.line_height + 1.0),
                    TextStyle::plain(p.text_soft),
                );
            }
        }
        let top = h - POP_PAD - 28.0;
        let body = self.fonts.body.as_deref();
        for (part, left, bw, words, look) in self.buttons() {
            button(rc, &self.theme, body, (left, top, bw, 28.0), words, look, self.hovered == Some(part));
        }
    }
}

/// A menu entry's meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    /// Turn the bar.
    Turn,
    /// One slot more.
    AddSlot,
    /// One slot fewer.
    RemoveSlot,
    /// Lock it in place (or unlock it).
    Lock(bool),
    /// Hide it (or show it).
    Hide(bool),
    /// Back to its default place.
    DefaultPlace,
    /// Edit layout.
    OpenEdit,
    /// Rename the bar.
    Rename,
    /// Delete the bar (asks once more).
    Delete,
    /// Delete the bar: asked once more and said yes.
    DeleteNow,
    /// This character's own layout.
    UseOwn,
    /// The layout the characters share.
    UseShared,
    /// Copy this character's layout (by its saved name).
    CopyFrom(String),
}

/// A menu row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuRow {
    /// Small capitals over the rows.
    Header(String),
    /// An entry: its words, a hint under them, whether it can be chosen,
    /// whether it is ticked.
    Entry {
        /// Its meaning.
        action: MenuAction,
        /// Its words.
        words: String,
        /// Smaller words under them.
        hint: Option<String>,
        /// It can be chosen.
        enabled: bool,
        /// It is ticked.
        ticked: bool,
    },
    /// A line between groups.
    Line,
}

impl MenuRow {
    /// An entry that can be chosen.
    #[must_use]
    pub fn entry(action: MenuAction, words: &str, hint: Option<&str>) -> Self {
        Self::Entry {
            action,
            words: words.to_owned(),
            hint: hint.map(str::to_owned),
            enabled: true,
            ticked: false,
        }
    }

    /// The same, ticked or not.
    #[must_use]
    pub fn ticked(self, on: bool) -> Self {
        match self {
            Self::Entry {
                action,
                words,
                hint,
                enabled,
                ..
            } => Self::Entry {
                action,
                words,
                hint,
                enabled,
                ticked: on,
            },
            other => other,
        }
    }

    fn height(&self) -> f32 {
        match self {
            Self::Header(_) => 24.0,
            Self::Entry { hint: Some(_), .. } => 40.0,
            Self::Entry { .. } => 30.0,
            Self::Line => 9.0,
        }
    }
}

/// The menu's width.
pub const MENU_WIDTH: f32 = 252.0;
const MENU_PAD: f32 = 6.0;

/// What a menu is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuFor {
    /// A bar.
    Bar(usize),
    /// Another HUD piece (quick move).
    Piece(ElementId),
    /// The layout (the toolbar's Layout button).
    Layout,
}

/// A small menu.
#[derive(Debug)]
pub struct Menu {
    theme: Horizon,
    fonts: BarFonts,
    /// Its rows.
    pub rows: Vec<MenuRow>,
    /// What it is about.
    pub about: Option<MenuFor>,
    /// What was chosen since the window last looked.
    pub chosen: Option<(MenuFor, MenuAction)>,
    hovered: Option<usize>,
}

impl Menu {
    /// An empty menu in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: BarFonts) -> Self {
        Self {
            theme,
            fonts,
            rows: Vec::new(),
            about: None,
            chosen: None,
            hovered: None,
        }
    }

    /// Its height for its rows.
    #[must_use]
    pub fn height(&self) -> f32 {
        MENU_PAD * 2.0 + self.rows.iter().map(MenuRow::height).sum::<f32>()
    }

    fn row_at(&self, y: f32) -> Option<usize> {
        let mut top = MENU_PAD;
        for (i, row) in self.rows.iter().enumerate() {
            let h = row.height();
            if y >= top && y < top + h {
                return Some(i);
            }
            top += h;
        }
        None
    }

    /// Opens it about `about` with `rows`, at (`x`, `y`) kept on a screen
    /// of `screen` (a press outside closes it).
    pub fn open(ui: &mut UiRoot, me: ElementId, about: MenuFor, rows: Vec<MenuRow>, (x, y): (f32, f32)) {
        let screen = ui.size();
        let Some(height) = ui.widget_mut::<Menu>(me).map(|m| {
            m.rows = rows;
            m.about = Some(about);
            m.hovered = None;
            m.height()
        }) else {
            return;
        };
        if let Some(e) = ui.element_mut(me) {
            e.left = x.min(screen.0 - MENU_WIDTH).max(0.0).round();
            e.top = y.min(screen.1 - height).max(0.0).round();
            e.height = height;
        }
        ui.set_visible(me, true);
        ui.bring_to_front(me);
        ui.set_active_popup(me, me);
    }

    /// Closes it.
    pub fn close(ui: &mut UiRoot, me: ElementId) {
        ui.clear_active_popup(me);
        ui.set_visible(me, false);
    }
}

impl Widget for Menu {
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
        ui.set_visible(me, false);
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let y = e.data2 as f32;
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = self.row_at(y);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::CLICK => {
                if let Some(i) = self.row_at(y)
                    && let Some(MenuRow::Entry {
                        action,
                        enabled: true,
                        ..
                    }) = self.rows.get(i)
                    && let Some(about) = self.about
                {
                    self.chosen = Some((about, action.clone()));
                    Self::close(ui, me);
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let h = ui.element(me).map_or(0.0, |e| e.height);
        let w = MENU_WIDTH;
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 12.0, 30.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, p.popup);
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, p.trim, 1.0);
        let mut top = MENU_PAD;
        for (i, row) in self.rows.iter().enumerate() {
            let rh = row.height();
            match row {
                MenuRow::Header(words) => {
                    if let Some(f) = &self.fonts.note {
                        text::draw(
                            rc,
                            f,
                            &words.to_uppercase(),
                            14.0,
                            top + ((rh - f.line_height) * 0.5).round(),
                            TextStyle::plain(p.muted).tracked(0.8),
                        );
                    }
                }
                MenuRow::Line => rc.fill(MENU_PAD + 4.0, top + 4.0, w - 2.0 * MENU_PAD - 8.0, 1.0, p.trim_faint),
                MenuRow::Entry {
                    words,
                    hint,
                    enabled,
                    ticked,
                    ..
                } => {
                    if self.hovered == Some(i) && *enabled {
                        rc.fill_rounded_rect(
                            MENU_PAD,
                            top,
                            w - 2.0 * MENU_PAD,
                            rh,
                            CornerRadii::all(self.theme.metrics.radius),
                            hex(0x2A_2116, 0.9),
                        );
                    }
                    let ink = if *enabled { p.text } else { p.muted };
                    if let Some(f) = &self.fonts.body {
                        let ty = if hint.is_some() {
                            top + 5.0
                        } else {
                            top + ((rh - f.line_height) * 0.5).round()
                        };
                        text::draw(rc, f, words, 14.0, ty, TextStyle::plain(ink));
                        if let (Some(hint), Some(small)) = (hint, &self.fonts.small) {
                            text::draw(
                                rc,
                                small,
                                hint,
                                14.0,
                                ty + f.line_height + 1.0,
                                TextStyle::plain(p.muted),
                            );
                        }
                    }
                    if *ticked {
                        let (cx, cy) = (w - 24.0, top + rh * 0.5);
                        rc.smooth_line(cx, cy, cx + 3.5, cy + 3.5, p.accent, 1.6);
                        rc.smooth_line(cx + 3.5, cy + 3.5, cx + 10.0, cy - 3.5, p.accent, 1.6);
                    }
                }
            }
            top += rh;
        }
    }
}

/// The box that renames a bar.
#[derive(Debug)]
pub struct RenameBox {
    theme: Horizon,
    fonts: BarFonts,
    /// The field.
    pub field: ElementId,
    /// The bar being renamed.
    pub bar: Option<usize>,
    /// The name chosen (the window renames the bar, then clears it).
    pub done: Option<(usize, String)>,
    hovered: Option<bool>,
}

/// The rename box's width and height.
pub const RENAME_WIDTH: f32 = 260.0;
pub const RENAME_HEIGHT: f32 = 100.0;

impl RenameBox {
    /// Mounts it, hidden, under the root.
    pub fn mount(ui: &mut UiRoot, theme: Horizon, fonts: BarFonts) -> ElementId {
        let root = ui.root();
        let element = Element::at(0.0, 0.0, RENAME_WIDTH, RENAME_HEIGHT)
            .with_visible(false)
            .with_anchors(AnchorEdges::NONE);
        let me = ui.add(
            root,
            element,
            RenameBox {
                theme,
                fonts: fonts.clone(),
                field: root,
                bar: None,
                done: None,
                hovered: None,
            },
        );
        let mut e = Element::at(13.0, 35.0, RENAME_WIDTH - 26.0, 26.0).with_anchors(AnchorEdges::NONE);
        Field::configure(&mut e);
        e.tab_stop = true;
        let mut f = Field::default();
        f.font = fonts.body.clone();
        f.one_line = true;
        f.selectable = true;
        f.record_history = false;
        f.max_characters = 24;
        f.padding = 6.0;
        f.text_color = theme.palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = hex(0xE0_B068, 0.35);
        f.set_editable_detached(&mut e, true);
        let field = ui.add(me, e, f);
        if let Some(r) = ui.widget_mut::<RenameBox>(me) {
            r.field = field;
        }
        me
    }

    /// Shows it for bar `bar` named `name`, at (`x`, `y`).
    pub fn show(ui: &mut UiRoot, me: ElementId, bar: usize, name: &str, (x, y): (f32, f32)) {
        let screen = ui.size();
        let Some(field) = ui.widget_mut::<RenameBox>(me).map(|r| {
            r.bar = Some(bar);
            r.done = None;
            r.field
        }) else {
            return;
        };
        if let Some(f) = ui.widget_mut::<Field>(field) {
            f.set_text(name);
        }
        if let Some(e) = ui.element_mut(me) {
            e.left = x.min(screen.0 - RENAME_WIDTH).max(0.0).round();
            e.top = y.min(screen.1 - RENAME_HEIGHT).max(0.0).round();
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

    fn buttons(&self) -> [(bool, f32, f32); 2] {
        let body = self.fonts.body.as_deref();
        let done = button_width(body, "Done");
        let cancel = button_width(body, "Cancel");
        let right = RENAME_WIDTH - 13.0;
        [(true, right - done, done), (false, right - done - 6.0 - cancel, cancel)]
    }

    fn button_at(&self, x: f32, y: f32) -> Option<bool> {
        if !(RENAME_HEIGHT - 34.0..RENAME_HEIGHT - 8.0).contains(&y) {
            return None;
        }
        self.buttons()
            .into_iter()
            .find(|(_, left, w)| x >= *left && x < left + w)
            .map(|(done, ..)| done)
    }

    fn finish(&mut self, ui: &mut UiRoot, me: ElementId) {
        let name = ui
            .widget::<Field>(self.field)
            .map(|f| f.text().trim().to_owned())
            .unwrap_or_default();
        if let Some(bar) = self.bar
            && !name.is_empty()
        {
            self.done = Some((bar, name));
        }
        Self::hide(ui, me);
    }
}

impl Widget for RenameBox {
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
        if e.kind == kind::KEY_DOWN && e.key == ac_ui::Key::Enter {
            self.finish(ui, me);
            return true;
        }
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = self.button_at(x, y);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::CLICK => {
                match self.button_at(x, y) {
                    Some(true) => self.finish(ui, me),
                    Some(false) => Self::hide(ui, me),
                    None => {}
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let (w, h) = (RENAME_WIDTH, RENAME_HEIGHT);
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 12.0, 30.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, hex(0x18_130E, 0.98));
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, p.trim, 1.0);
        if let Some(f) = &self.fonts.body {
            text::draw(rc, f, "Rename bar", 13.0, 11.0, TextStyle::plain(p.text));
        }
        rc.fill_rounded_rect(12.0, 34.0, w - 24.0, 28.0, CornerRadii::all(4.0), hex(0, 0.4));
        rc.stroke_rounded_rect(12.0, 34.0, w - 24.0, 28.0, CornerRadii::all(4.0), hex(0xC9_A062, 0.5), 1.0);
        let body = self.fonts.body.as_deref();
        for (done, left, bw) in self.buttons() {
            let words = if done { "Done" } else { "Cancel" };
            let look = if done { Look::Main } else { Look::Quiet };
            button(
                rc,
                &self.theme,
                body,
                (left, h - 34.0, bw, 26.0),
                words,
                look,
                self.hovered == Some(done),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_rows_are_found_by_height() {
        let mut menu = Menu::new(Horizon::BRONZE, BarFonts::default());
        menu.rows = vec![
            MenuRow::Header("Main bar".to_owned()),
            MenuRow::entry(MenuAction::Turn, "Turn upright", None),
            MenuRow::Line,
            MenuRow::entry(MenuAction::OpenEdit, "Open Edit layout", Some("keys, new bars")),
        ];
        assert_eq!(menu.height(), MENU_PAD * 2.0 + 24.0 + 30.0 + 9.0 + 40.0);
        assert_eq!(menu.row_at(MENU_PAD + 1.0), Some(0));
        assert_eq!(menu.row_at(MENU_PAD + 24.0 + 1.0), Some(1));
        assert_eq!(menu.row_at(MENU_PAD + 24.0 + 30.0 + 9.0 + 39.0), Some(3));
        assert_eq!(menu.row_at(menu.height()), None);
    }

    #[test]
    fn the_popover_offers_emptying_only_a_filled_slot() {
        let mut pop = KeyPopover::new(Horizon::BRONZE, BarFonts::default());
        assert_eq!(pop.buttons().len(), 2);
        pop.filled = true;
        assert_eq!(pop.buttons().len(), 3);
        let box_left = POPOVER_WIDTH - POP_PAD - KEY_BOX;
        assert_eq!(
            pop.part_at(box_left + 2.0, ROWS_TOP + ROW + ROW_GAP + 2.0),
            Some(PopPart::Key(KeyMode::Magic))
        );
    }
}
