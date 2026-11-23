//! What plugins say about items, as the modern item screens show it: a
//! mark beside an item, a card under each plugin's name on its details and
//! Inspect sheet, entries in its menus, and a band of summaries and buttons
//! in a window's foot. The screens only show these; a chosen entry or a
//! pressed button goes back to its plugin.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_plugin_ui::{ItemNoteEntry, ItemWindow, NoteTone, PluginUiView, WindowFootEntry};
use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::fit;
use super::menu::Row;
use super::sheet::{Entry, Section};
use crate::ui::modern::bars::edit::{Look, button, button_width};

/// The colour a mark in `tone` is drawn in.
#[must_use]
pub fn tone_color(theme: &Horizon, tone: NoteTone) -> Color {
    let p = theme.palette;
    match tone {
        NoteTone::Plain => p.text_soft,
        NoteTone::Good => p.good,
        NoteTone::Warning => p.warn,
        NoteTone::Bad => p.bad_text,
        NoteTone::Accent => p.accent,
    }
}

/// The notes on item `id`, each plugin's, by plugin id.
#[must_use]
pub fn notes_on(view: &PluginUiView, id: u32) -> &[ItemNoteEntry] {
    view.item_notes.get(&id).map_or(&[], Vec::as_slice)
}

/// The mark shown beside item `id`: the first plugin's that has one.
#[must_use]
pub fn mark_on(view: &PluginUiView, theme: &Horizon, id: u32) -> Option<(String, Color)> {
    notes_on(view, id)
        .iter()
        .find(|e| !e.note.mark.is_empty())
        .map(|e| (e.note.mark.clone(), tone_color(theme, e.note.tone)))
}

/// A card per plugin for item `id`'s sheet: the plugin's name over its
/// mark and its lines.
#[must_use]
pub fn cards(view: &PluginUiView, theme: &Horizon, id: u32) -> Vec<Section> {
    notes_on(view, id)
        .iter()
        .filter(|e| !e.note.mark.is_empty() || !e.note.lines.is_empty())
        .map(|e| {
            let mut entries = Vec::new();
            if !e.note.mark.is_empty() {
                entries.push(Entry::Verdict {
                    words: e.note.mark.clone(),
                    color: tone_color(theme, e.note.tone),
                });
            }
            entries.extend(e.note.lines.iter().map(|l| Entry::Para {
                text: l.clone(),
                mark: super::sheet::Mark::None,
            }));
            Section {
                title: e.owner.display_name.to_uppercase(),
                count: String::new(),
                entries,
            }
        })
        .collect()
}

/// `sheet` with `cards` at the head of its first tab.
#[must_use]
pub fn with_cards(mut sheet: super::sheet::Sheet, cards: Vec<Section>) -> super::sheet::Sheet {
    if let Some(tab) = sheet.tabs.first_mut()
        && !cards.is_empty()
    {
        tab.sections.splice(0..0, cards);
    }
    sheet
}

/// The first tag the plugins' menu entries take: the menus' own tags stay
/// below it.
pub const MENU_TAG_BASE: u32 = 10_000;

/// The rows the plugins add to item `id`'s menu (a header with each
/// plugin's name over its entries), tagged from [`MENU_TAG_BASE`]; and what
/// each tag stands for: (plugin id, entry id).
#[must_use]
pub fn menu_rows(view: &PluginUiView, id: u32) -> (Vec<Row>, Vec<(String, String)>) {
    let mut rows = Vec::new();
    let mut tags = Vec::new();
    for e in notes_on(view, id)
        .iter()
        .filter(|e| !e.note.menu.is_empty())
    {
        rows.push(Row::Line);
        rows.push(Row::Header(e.owner.display_name.to_uppercase()));
        for entry in &e.note.menu {
            rows.push(Row::entry(
                MENU_TAG_BASE + tags.len() as u32,
                entry.label.clone(),
            ));
            tags.push((e.owner.id.clone(), entry.id.clone()));
        }
    }
    (rows, tags)
}

/// The plugins' parts of `window`'s foot.
#[must_use]
pub fn feet_of(view: &PluginUiView, window: ItemWindow) -> Vec<WindowFootEntry> {
    view.feet
        .iter()
        .filter(|f| f.window == window)
        .cloned()
        .collect()
}

/// One row of the band's height.
pub const BAND_ROW: f32 = 36.0;

/// The band's height for `rows` plugins (none: no band).
#[must_use]
pub fn band_height(rows: usize) -> f32 {
    if rows == 0 {
        0.0
    } else {
        rows as f32 * BAND_ROW + 4.0
    }
}

/// The band of plugins' summaries and buttons over a window's own foot.
#[derive(Debug)]
pub struct FootBand {
    /// The plugin's name, small capitals.
    label: Option<Arc<UiFont>>,
    /// The summary.
    small: Option<Arc<UiFont>>,
    /// The buttons.
    body: Option<Arc<UiFont>>,
    /// What it shows: one row per plugin.
    pub rows: Vec<WindowFootEntry>,
    /// Buttons pressed since the window last looked: (plugin id, button).
    pub pressed: Vec<(String, String)>,
    hover: Option<(f32, f32)>,
}

impl FootBand {
    /// A band in `theme`.
    #[must_use]
    pub fn new(
        label: Option<Arc<UiFont>>,
        small: Option<Arc<UiFont>>,
        body: Option<Arc<UiFont>>,
    ) -> Self {
        Self {
            label,
            small,
            body,
            rows: Vec::new(),
            pressed: Vec::new(),
            hover: None,
        }
    }

    /// Each row's buttons: (row, button, rect).
    fn button_rects(&self, w: f32) -> Vec<(usize, usize, Rect)> {
        let body = self.body.as_deref();
        let mut out = Vec::new();
        for (r, row) in self.rows.iter().enumerate() {
            let mut x = w - 12.0;
            let y = 2.0 + r as f32 * BAND_ROW + 4.0;
            for (b, button) in row.foot.buttons.iter().enumerate().rev() {
                let bw = button_width(body, &button.label);
                x -= bw;
                out.push((r, b, (x, y, bw, BAND_ROW - 8.0)));
                x -= 6.0;
            }
        }
        out
    }

    /// Shows `rows` in band `me`, `w` wide at `y` in its window.
    pub fn show(ui: &mut UiRoot, me: ElementId, rows: Vec<WindowFootEntry>, y: f32, w: f32) {
        let h = band_height(rows.len());
        if let Some(e) = ui.element_mut(me) {
            e.top = y;
            e.width = w;
            e.height = h.max(1.0);
        }
        ui.set_visible(me, !rows.is_empty());
        if let Some(b) = ui.widget_mut::<Self>(me)
            && b.rows != rows
        {
            b.rows = rows;
        }
    }

    /// The buttons pressed in band `me` since the window last looked.
    pub fn take(ui: &mut UiRoot, me: ElementId) -> Vec<(String, String)> {
        ui.widget_mut::<Self>(me)
            .map(|b| std::mem::take(&mut b.pressed))
            .unwrap_or_default()
    }
}

/// A rectangle: left, top, width, height.
type Rect = (f32, f32, f32, f32);

fn inside((x, y, w, h): Rect, px: f32, py: f32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

impl Widget for FootBand {
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
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hover = Some((x, y));
                true
            }
            kind::HOVER_LEAVE => {
                self.hover = None;
                true
            }
            kind::CLICK => {
                let w = ui.element(me).map_or(0.0, |e| e.width);
                if let Some((r, b, _)) = self
                    .button_rects(w)
                    .into_iter()
                    .find(|(_, _, rect)| inside(*rect, x, y))
                    && let Some(row) = self.rows.get(r)
                    && let Some(button) = row.foot.buttons.get(b).filter(|b| b.enabled)
                {
                    self.pressed.push((row.owner.id.clone(), button.id.clone()));
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let Some(w) = ui.element(me).map(|e| e.width) else {
            return;
        };
        let p = Horizon::current().palette;
        rc.fill(12.0, 0.0, w - 24.0, 1.0, tone::bronze(0.18));
        let rects = self.button_rects(w);
        for (r, row) in self.rows.iter().enumerate() {
            let top = 2.0 + r as f32 * BAND_ROW;
            let left_of_buttons = rects
                .iter()
                .filter(|(rr, ..)| *rr == r)
                .map(|(_, _, rect)| rect.0)
                .fold(w - 12.0, f32::min);
            let room = (left_of_buttons - 12.0 - 10.0).max(0.0);
            if let Some(f) = &self.label {
                text::draw(
                    rc,
                    f,
                    &fit(f, &row.owner.display_name.to_uppercase(), room),
                    12.0,
                    top + 4.0,
                    TextStyle::plain(p.muted).tracked(0.8),
                );
            }
            if let Some(f) = &self.small {
                text::draw(
                    rc,
                    f,
                    &fit(f, &row.foot.summary, room),
                    12.0,
                    top + 18.0,
                    TextStyle::plain(p.text_soft),
                );
            }
            for (rr, b, rect) in &rects {
                if *rr != r {
                    continue;
                }
                let Some(fb) = row.foot.buttons.get(*b) else {
                    continue;
                };
                let hot = fb.enabled && self.hover.is_some_and(|(x, y)| inside(*rect, x, y));
                if fb.enabled {
                    button(
                        rc,
                        &Horizon::current(),
                        self.body.as_deref(),
                        *rect,
                        &fb.label,
                        Look::Plain,
                        hot,
                    );
                } else if let Some(f) = &self.body {
                    // Drawn, but it cannot be pressed.
                    let r4 = ac_ui::shapes::CornerRadii::all(4.0);
                    rc.stroke_rounded_rect(
                        rect.0,
                        rect.1,
                        rect.2,
                        rect.3,
                        r4,
                        tone::bronze(0.2),
                        1.0,
                    );
                    text::draw_centred(
                        rc,
                        f,
                        &fb.label,
                        rect.0 + rect.2 * 0.5,
                        rect.1 + ((rect.3 - f.line_height) * 0.5).round(),
                        TextStyle::plain(p.muted),
                    );
                }
            }
        }
    }
}
