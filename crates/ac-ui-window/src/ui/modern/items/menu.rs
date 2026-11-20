//! A small pop-up menu for the item screens: headings, entries (with a
//! note beside or under them, a tick, greyed when they cannot be chosen)
//! and lines. Each entry carries a number the window gives meaning to; the
//! menu only notes which was chosen.

use std::sync::Arc;

use ac_ui::draw::RenderContext;
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

const PAD: f32 = 6.0;

/// One row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// Small capitals over a run of entries.
    Header(String),
    /// Something to choose.
    Entry {
        /// What choosing it means (the window's number).
        tag: u32,
        /// Its words.
        words: String,
        /// A note on the right.
        note: String,
        /// A line under it.
        under: String,
        /// It can be chosen.
        on: bool,
        /// Ticked.
        ticked: bool,
    },
    /// A dividing line.
    Line,
}

impl Row {
    /// An entry `words` meaning `tag`.
    #[must_use]
    pub fn entry(tag: u32, words: impl Into<String>) -> Self {
        Self::Entry {
            tag,
            words: words.into(),
            note: String::new(),
            under: String::new(),
            on: true,
            ticked: false,
        }
    }

    /// With a note on the right.
    #[must_use]
    pub fn note(mut self, words: impl Into<String>) -> Self {
        if let Self::Entry { note, .. } = &mut self {
            *note = words.into();
        }
        self
    }

    /// With a line under it.
    #[must_use]
    pub fn under(mut self, words: impl Into<String>) -> Self {
        if let Self::Entry { under, .. } = &mut self {
            *under = words.into();
        }
        self
    }

    /// Ticked or not.
    #[must_use]
    pub fn ticked(mut self, yes: bool) -> Self {
        if let Self::Entry { ticked, .. } = &mut self {
            *ticked = yes;
        }
        self
    }

    /// Greyed unless `yes`.
    #[must_use]
    pub fn on(mut self, yes: bool) -> Self {
        if let Self::Entry { on, .. } = &mut self {
            *on = yes;
        }
        self
    }

    fn height(&self) -> f32 {
        match self {
            Self::Header(_) => 24.0,
            Self::Entry { under, .. } if !under.is_empty() => 40.0,
            Self::Entry { .. } => 28.0,
            Self::Line => 9.0,
        }
    }
}

/// The menu's fonts.
#[derive(Debug, Clone, Default)]
pub struct MenuFonts {
    /// Entries.
    pub body: Option<Arc<UiFont>>,
    /// Notes and headings.
    pub small: Option<Arc<UiFont>>,
}

/// The menu.
#[derive(Debug)]
pub struct PopMenu {
    theme: Horizon,
    fonts: MenuFonts,
    rows: Vec<Row>,
    width: f32,
    /// What it is about (the window's number: an item, a pack).
    pub about: u32,
    /// What it was opened for (the window's number).
    pub kind: u32,
    /// The tag chosen since the window last looked.
    pub chosen: Option<u32>,
    hovered: Option<usize>,
}

impl PopMenu {
    /// An empty, hidden menu.
    #[must_use]
    pub fn new(theme: Horizon, fonts: MenuFonts) -> Self {
        Self {
            theme,
            fonts,
            rows: Vec::new(),
            width: 220.0,
            about: 0,
            kind: 0,
            chosen: None,
            hovered: None,
        }
    }

    /// A hidden menu on the root.
    pub fn mount(ui: &mut UiRoot, theme: Horizon, fonts: MenuFonts) -> ElementId {
        let root = ui.root();
        ui.add(
            root,
            Element::at(0.0, 0.0, 220.0, 40.0)
                .with_visible(false)
                .with_anchors(ac_ui::geometry::AnchorEdges::NONE),
            Self::new(theme, fonts),
        )
    }

    fn height(&self) -> f32 {
        PAD * 2.0 + self.rows.iter().map(Row::height).sum::<f32>()
    }

    /// The top and height of the entry with words `words`.
    #[must_use]
    pub fn entry_rows(&self, words: &str) -> Option<(f32, f32)> {
        let mut top = PAD;
        for row in &self.rows {
            if matches!(row, Row::Entry { words: w, .. } if w == words) {
                return Some((top, row.height()));
            }
            top += row.height();
        }
        None
    }

    fn row_at(&self, y: f32) -> Option<usize> {
        let mut top = PAD;
        for (i, row) in self.rows.iter().enumerate() {
            let h = row.height();
            if y >= top && y < top + h {
                return Some(i);
            }
            top += h;
        }
        None
    }

    /// Opens menu `me` (`kind`, about `about`) with `rows`, `width` wide,
    /// its top left at (`x`, `y`) kept on the screen.
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        ui: &mut UiRoot,
        me: ElementId,
        kind: u32,
        about: u32,
        rows: Vec<Row>,
        width: f32,
        (x, y): (f32, f32),
    ) {
        let screen = ui.size();
        let Some(height) = ui.widget_mut::<Self>(me).map(|m| {
            m.rows = rows;
            m.kind = kind;
            m.about = about;
            m.width = width;
            m.hovered = None;
            m.chosen = None;
            m.height()
        }) else {
            return;
        };
        if let Some(e) = ui.element_mut(me) {
            e.left = x.min(screen.0 - width).max(0.0).round();
            e.top = y.min(screen.1 - height).max(0.0).round();
            e.width = width;
            e.height = height;
        }
        ui.set_visible(me, true);
        ui.bring_to_front(me);
        ui.set_active_popup(me, me);
    }

    /// Closes menu `me`.
    pub fn close(ui: &mut UiRoot, me: ElementId) {
        ui.clear_active_popup(me);
        ui.set_visible(me, false);
    }

    /// What was chosen in menu `me` since the window last looked: (kind,
    /// about, tag).
    pub fn take(ui: &mut UiRoot, me: ElementId) -> Option<(u32, u32, u32)> {
        let m = ui.widget_mut::<Self>(me)?;
        let tag = m.chosen.take()?;
        Some((m.kind, m.about, tag))
    }
}

impl Widget for PopMenu {
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
                    && let Some(Row::Entry { tag, on: true, .. }) = self.rows.get(i)
                {
                    self.chosen = Some(*tag);
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
        let w = self.width;
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 12.0, 30.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, hex(0x18_130E, 0.98));
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, p.trim, 1.0);
        let mut top = PAD;
        for (i, row) in self.rows.iter().enumerate() {
            let rh = row.height();
            match row {
                Row::Header(words) => {
                    if let Some(f) = &self.fonts.small {
                        text::draw(
                            rc,
                            f,
                            words,
                            14.0,
                            top + ((rh - f.line_height) * 0.5).round() + 2.0,
                            TextStyle::plain(p.text_soft).tracked(1.0),
                        );
                    }
                }
                Row::Line => rc.fill(PAD + 2.0, top + 4.0, w - 2.0 * PAD - 4.0, 1.0, p.trim_faint),
                Row::Entry {
                    words,
                    note,
                    under,
                    on,
                    ticked,
                    ..
                } => {
                    let r = CornerRadii::all(self.theme.metrics.slot_radius);
                    if *ticked {
                        rc.fill_rounded_rect(PAD, top, w - 2.0 * PAD, rh, r, p.accent_tint);
                    } else if self.hovered == Some(i) && *on {
                        rc.fill_rounded_rect(PAD, top, w - 2.0 * PAD, rh, r, hex(0x2A_2116, 0.9));
                    }
                    let ink = if *on { p.text } else { p.muted };
                    if *ticked {
                        let (cx, cy) = (PAD + 8.0, top + rh * 0.5);
                        rc.smooth_line(cx, cy, cx + 3.0, cy + 3.0, p.accent, 1.6);
                        rc.smooth_line(cx + 3.0, cy + 3.0, cx + 9.0, cy - 3.0, p.accent, 1.6);
                    }
                    if let Some(f) = &self.fonts.body {
                        let ty = if under.is_empty() {
                            top + ((rh - f.line_height) * 0.5).round()
                        } else {
                            top + 5.0
                        };
                        text::draw(rc, f, words, PAD + 22.0, ty, TextStyle::plain(ink));
                        if let Some(s) = &self.fonts.small {
                            if !note.is_empty() {
                                let nw = text::measure(s, note, 0.0);
                                text::draw(
                                    rc,
                                    s,
                                    note,
                                    w - PAD - 8.0 - nw,
                                    top + ((rh - s.line_height) * 0.5).round(),
                                    TextStyle::plain(if *on { p.text_soft } else { p.bad_text }),
                                );
                            }
                            if !under.is_empty() {
                                text::draw(
                                    rc,
                                    s,
                                    under,
                                    PAD + 22.0,
                                    ty + f.line_height + 1.0,
                                    TextStyle::plain(p.text_soft),
                                );
                            }
                        }
                    }
                }
            }
            top += rh;
        }
    }
}
