//! Draws a [`Sheet`]: the head (icon or level and health), the facts, the
//! tab row when there is more than one tab, the chosen tab's sections in a
//! scrolling body, the spell legend, and a foot of buttons when the sheet
//! has any (the inventory's item details: Equip or Use, then small ones).

use std::sync::Arc;

use ac_ui::draw::{Color, ColorVertex, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::fit;
use super::sheet::{Entry, Mark, Sheet};
use crate::ui::modern::bars::edit::wrap;
use crate::ui::modern::hud::lines::stroke;

/// The side padding.
const PAD: f32 = 14.0;
/// The tab row's height.
const TABS: f32 = 32.0;
/// A row's height.
const ROW: f32 = 19.0;
/// The foot's height.
const FOOT: f32 = 56.0;
/// The scrollbar's width.
const BAR: f32 = 5.0;
/// One wheel notch.
const WHEEL: f32 = 48.0;

/// The fonts a sheet draws with.
#[derive(Debug, Clone, Default)]
pub struct SheetFonts {
    /// The name.
    pub name: Option<Arc<UiFont>>,
    /// The line under it, facts' names, notes.
    pub small: Option<Arc<UiFont>>,
    /// Section titles and the level's word.
    pub label: Option<Arc<UiFont>>,
    /// Rows and paragraphs.
    pub body: Option<Arc<UiFont>>,
    /// Values, spell names, tabs, buttons.
    pub strong: Option<Arc<UiFont>>,
    /// The level.
    pub big: Option<Arc<UiFont>>,
}

/// A small button in the foot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FootIcon {
    /// Give to the target.
    Give,
    /// Drop on the ground.
    Drop,
    /// Salvage.
    Salvage,
    /// More (a menu).
    More,
}

impl FootIcon {
    fn lines(self) -> &'static [&'static [(f32, f32)]] {
        match self {
            Self::Give => &[
                &[(3.0, 12.0), (16.0, 12.0)],
                &[(12.0, 7.0), (17.0, 12.0), (12.0, 17.0)],
                &[(20.0, 4.0), (20.0, 20.0)],
            ],
            Self::Drop => &[
                &[(12.0, 3.0), (12.0, 16.0)],
                &[(7.0, 11.0), (12.0, 16.0), (17.0, 11.0)],
                &[(5.0, 21.0), (19.0, 21.0)],
            ],
            Self::Salvage => &[
                &[(4.0, 20.0), (13.0, 11.0)],
                &[
                    (11.0, 9.0),
                    (15.0, 5.0),
                    (19.0, 9.0),
                    (15.0, 13.0),
                    (11.0, 9.0),
                ],
            ],
            Self::More => &[
                &[(5.0, 12.0), (5.5, 12.0)],
                &[(12.0, 12.0), (12.5, 12.0)],
                &[(19.0, 12.0), (19.5, 12.0)],
            ],
        }
    }
}

/// The sheet's foot: the main button and the small ones.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Foot {
    /// The main button's words (Equip, Use, Loot).
    pub primary: String,
    /// The main button can be pressed.
    pub primary_on: bool,
    /// The small buttons: icon, hover words, can be pressed.
    pub small: Vec<(FootIcon, String, bool)>,
}

/// What a sheet asks of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetRequest {
    /// The close button.
    Close,
    /// The main button.
    Primary,
    /// Small button `n`, at this screen point (its foot).
    Small(usize, (i32, i32)),
}

/// A laid-out line of the body.
#[derive(Debug, Clone, PartialEq)]
enum Line {
    /// A section title at `y`.
    Title {
        y: f32,
        title: String,
        count: String,
    },
    /// A row.
    Row {
        y: f32,
        key: String,
        value: String,
        mark: Mark,
    },
    /// A row with a bar.
    Bar {
        y: f32,
        key: String,
        value: String,
        share: f32,
        color: Color,
    },
    /// Words at `x`, `y` in `color`; `strong` for a spell's name.
    Words {
        y: f32,
        text: String,
        color: Color,
        strong: bool,
        small: bool,
    },
    /// An inscription's box.
    Box { y: f32, h: f32, editable: bool },
    /// A plugin's mark: a chip.
    Chip { y: f32, words: String, color: Color },
    /// The legend of the spell marks.
    Legend { y: f32 },
}

/// The sheet widget.
#[derive(Debug)]
pub struct SheetView {
    theme: Horizon,
    fonts: SheetFonts,
    sheet: Sheet,
    /// The tab chosen, by name (kept while the sheet's subject stays).
    tab: String,
    /// It has a close button.
    pub closable: bool,
    /// It is a window of its own: it draws its own ground, edge and shadow.
    pub framed: bool,
    /// Its foot (none: no foot).
    pub foot: Option<Foot>,
    scroll: f32,
    lines: Vec<Line>,
    content: f32,
    /// The width and sheet the lines were laid for.
    laid_width: f32,
    dirty: bool,
    hover: Option<(f32, f32)>,
    /// What it asked for since the window last looked.
    pub requests: Vec<SheetRequest>,
    /// A field lies over the writable inscription: its words are the
    /// field's, not drawn by the sheet.
    pub covered: bool,
    /// The writable inscription's box (top and height in the body) and
    /// the lines of its words.
    writing: Option<(f32, f32, std::ops::Range<usize>)>,
}

impl SheetView {
    /// An empty sheet in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: SheetFonts) -> Self {
        Self {
            theme,
            fonts,
            sheet: Sheet::default(),
            tab: String::new(),
            closable: false,
            framed: false,
            foot: None,
            scroll: 0.0,
            lines: Vec::new(),
            content: 0.0,
            laid_width: 0.0,
            dirty: true,
            hover: None,
            requests: Vec::new(),
            covered: false,
            writing: None,
        }
    }

    /// Shows `sheet` (a new subject starts at its first tab, at the top).
    pub fn set(&mut self, sheet: Sheet) {
        if sheet == self.sheet {
            return;
        }
        if sheet.subject != self.sheet.subject {
            self.scroll = 0.0;
            self.tab.clear();
        }
        self.sheet = sheet;
        self.dirty = true;
    }

    /// The sheet shown.
    #[must_use]
    pub fn sheet(&self) -> &Sheet {
        &self.sheet
    }

    /// Opens tab `label` if the sheet has it.
    pub fn choose_tab(&mut self, label: &str) {
        if self.sheet.tabs.iter().any(|t| t.label == label) {
            self.tab = label.to_owned();
            self.scroll = 0.0;
            self.dirty = true;
        }
    }

    /// Where a field writing the inscription goes, in the sheet's own
    /// coordinates: the writable box while it shows whole in the body.
    #[must_use]
    pub fn writing_box(&mut self, w: f32, h: f32) -> Option<(f32, f32, f32, f32)> {
        if self.dirty || (self.laid_width - w).abs() > 0.5 {
            self.lay_out(w);
        }
        let (y, bh, _) = self.writing.clone()?;
        let top = self.body_top();
        let y = top - self.scroll.clamp(0.0, self.most(h)) + y;
        (y >= top && y + bh <= self.body_bottom(h)).then_some((PAD, y, w - PAD * 2.0 - BAR, bh))
    }

    /// The inscription the sheet shows as writable, if any.
    #[must_use]
    pub fn writable_inscription(&self) -> Option<&str> {
        self.sheet.tabs.get(self.tab_index()).and_then(|t| {
            t.sections
                .iter()
                .flat_map(|s| &s.entries)
                .find_map(|e| match e {
                    Entry::Inscription {
                        text,
                        editable: true,
                        ..
                    } => Some(text.as_str()),
                    _ => None,
                })
        })
    }

    /// Tab `label`'s place in the tab row of a sheet `w` wide (none when
    /// the sheet has no row or no such tab).
    #[must_use]
    pub fn tab_rect(&self, label: &str, w: f32) -> Option<(f32, f32, f32, f32)> {
        let n = self.sheet.tabs.len();
        let i = self.sheet.tabs.iter().position(|t| t.label == label)?;
        (n > 1).then(|| {
            let tab_w = (w - 12.0) / n as f32;
            (6.0 + i as f32 * tab_w, self.head_height(), tab_w, TABS)
        })
    }

    fn tab_index(&self) -> usize {
        self.sheet
            .tabs
            .iter()
            .position(|t| t.label == self.tab)
            .unwrap_or(0)
    }

    /// The head's height.
    fn head_height(&self) -> f32 {
        let h = &self.sheet.head;
        let mut y = PAD + if h.icon.is_some() { 52.0 } else { 40.0 };
        if h.health.is_some() {
            y += 14.0;
        }
        if h.failed {
            y += 20.0;
        }
        if !self.sheet.facts.is_empty() {
            y += 10.0 + 38.0;
        }
        y + 10.0
    }

    fn body_top(&self) -> f32 {
        self.head_height() + if self.sheet.tabs.len() > 1 { TABS } else { 0.0 }
    }

    fn body_bottom(&self, h: f32) -> f32 {
        h - if self.foot.is_some() { FOOT } else { 0.0 }
    }

    fn lay_out(&mut self, width: f32) {
        self.laid_width = width;
        self.dirty = false;
        self.lines.clear();
        self.writing = None;
        let (Some(body), Some(small), Some(strong)) =
            (&self.fonts.body, &self.fonts.small, &self.fonts.strong)
        else {
            return;
        };
        let p = self.theme.palette;
        let inner = width - PAD * 2.0 - BAR;
        let mut y = 4.0;
        let Some(tab) = self.sheet.tabs.get(self.tab_index()) else {
            return;
        };
        for section in &tab.sections {
            y += 10.0;
            if !section.title.is_empty() {
                self.lines.push(Line::Title {
                    y,
                    title: section.title.clone(),
                    count: section.count.clone(),
                });
                y += 18.0;
            }
            for e in &section.entries {
                match e {
                    Entry::Row { key, value, mark } => {
                        self.lines.push(Line::Row {
                            y,
                            key: key.clone(),
                            value: value.clone(),
                            mark: *mark,
                        });
                        y += ROW;
                    }
                    Entry::Bar {
                        key,
                        value,
                        share,
                        color,
                    } => {
                        self.lines.push(Line::Bar {
                            y,
                            key: key.clone(),
                            value: value.clone(),
                            share: *share,
                            color: *color,
                        });
                        y += ROW + 3.0;
                    }
                    Entry::Para { text, mark } => {
                        let color = match mark {
                            Mark::Up | Mark::Met => p.good,
                            Mark::Down | Mark::Unmet => p.bad_text,
                            Mark::Gold => p.accent,
                            Mark::None => p.icon,
                        };
                        for l in wrap(body, text, inner) {
                            self.lines.push(Line::Words {
                                y,
                                text: l,
                                color,
                                strong: false,
                                small: false,
                            });
                            y += body.line_height + 3.0;
                        }
                        y += 3.0;
                    }
                    Entry::Verdict { words, color } => {
                        self.lines.push(Line::Chip {
                            y,
                            words: words.clone(),
                            color: *color,
                        });
                        y += strong.line_height + 12.0;
                    }
                    Entry::Spell { name, text, gold } => {
                        y += 2.0;
                        self.lines.push(Line::Words {
                            y,
                            text: fit(strong, name, inner),
                            color: if *gold { p.accent } else { hex(0x7F_B2FF, 1.0) },
                            strong: true,
                            small: false,
                        });
                        y += strong.line_height + 2.0;
                        for l in wrap(small, text, inner) {
                            self.lines.push(Line::Words {
                                y,
                                text: l,
                                color: p.text_soft,
                                strong: false,
                                small: true,
                            });
                            y += small.line_height + 2.0;
                        }
                        y += 4.0;
                    }
                    Entry::Inscription {
                        text,
                        editable,
                        note,
                    } => {
                        let words = wrap(body, text, inner - 20.0);
                        let h = (words.len().max(1) as f32 * (body.line_height + 3.0) + 16.0)
                            .max(if *editable { 64.0 } else { 0.0 });
                        self.lines.push(Line::Box {
                            y: y + 2.0,
                            h,
                            editable: *editable,
                        });
                        let first = self.lines.len();
                        let mut ty = y + 10.0;
                        for l in words {
                            self.lines.push(Line::Words {
                                y: ty,
                                text: l,
                                color: p.text,
                                strong: false,
                                small: false,
                            });
                            ty += body.line_height + 3.0;
                        }
                        if *editable {
                            self.writing = Some((y + 2.0, h, first..self.lines.len()));
                        }
                        y += h + 8.0;
                        for l in wrap(small, note, inner) {
                            self.lines.push(Line::Words {
                                y,
                                text: l,
                                color: p.muted,
                                strong: false,
                                small: true,
                            });
                            y += small.line_height + 2.0;
                        }
                        y += 4.0;
                    }
                }
            }
        }
        if self.sheet.has_spell_marks() {
            y += 12.0;
            self.lines.push(Line::Legend { y });
            y += small.line_height;
        }
        self.content = y + 12.0;
    }

    fn most(&self, h: f32) -> f32 {
        (self.content - (self.body_bottom(h) - self.body_top())).max(0.0)
    }

    /// The foot's buttons: the main one, then each small one.
    fn foot_rects(&self, w: f32, h: f32) -> Vec<(f32, f32, f32, f32)> {
        let Some(foot) = &self.foot else {
            return Vec::new();
        };
        let y = h - FOOT + 10.0;
        let n = foot.small.len() as f32;
        let small_w = n * 32.0 + n * 6.0;
        let main_w = (w - PAD * 2.0 - small_w).max(40.0);
        let mut out = vec![(PAD, y, main_w, 32.0)];
        for i in 0..foot.small.len() {
            out.push((PAD + main_w + 6.0 + i as f32 * 38.0, y, 32.0, 32.0));
        }
        out
    }

    fn close_rect(&self, w: f32) -> (f32, f32, f32, f32) {
        (w - 10.0 - 26.0, 12.0, 26.0, 26.0)
    }
}

fn inside((x, y, w, h): (f32, f32, f32, f32), px: f32, py: f32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

/// A small mark after a value: an arrow, a tick or a cross.
fn draw_mark(rc: &mut RenderContext<'_>, mark: Mark, x: f32, cy: f32, color: Color) {
    let v = |x: f32, y: f32| ColorVertex { x, y, color };
    match mark {
        Mark::Up => {
            let _ = rc.fill_convex_polygon(&[
                v(x, cy + 3.0),
                v(x + 7.0, cy + 3.0),
                v(x + 3.5, cy - 3.0),
            ]);
        }
        Mark::Down => {
            let _ = rc.fill_convex_polygon(&[
                v(x, cy - 3.0),
                v(x + 7.0, cy - 3.0),
                v(x + 3.5, cy + 3.0),
            ]);
        }
        Mark::Met => stroke(
            rc,
            x - 1.0,
            cy - 5.0,
            10.0,
            &[&[(4.0, 12.0), (9.0, 17.0), (20.0, 6.0)]],
            color,
            1.6,
        ),
        Mark::Unmet => stroke(
            rc,
            x - 1.0,
            cy - 5.0,
            10.0,
            &[&[(6.0, 6.0), (18.0, 18.0)], &[(18.0, 6.0), (6.0, 18.0)]],
            color,
            1.6,
        ),
        Mark::Gold | Mark::None => {}
    }
}

impl Widget for SheetView {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn clips_children(&self) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return false;
        };
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hover = Some((x, y));
                false
            }
            kind::HOVER_LEAVE => {
                self.hover = None;
                false
            }
            kind::SCROLL => {
                let by = if e.data0 > 0 { -WHEEL } else { WHEEL };
                self.scroll = (self.scroll + by).clamp(0.0, self.most(h));
                true
            }
            kind::CLICK => {
                if self.closable && inside(self.close_rect(w), x, y) {
                    self.requests.push(SheetRequest::Close);
                    return true;
                }
                let hit = self
                    .sheet
                    .tabs
                    .iter()
                    .map(|t| t.label.clone())
                    .find(|l| self.tab_rect(l, w).is_some_and(|r| inside(r, x, y)));
                if let Some(label) = hit {
                    self.choose_tab(&label);
                    return true;
                }
                let rects = self.foot_rects(w, h);
                if let Some(i) = rects.iter().position(|r| inside(*r, x, y)) {
                    let on = self.foot.as_ref().is_some_and(|f| {
                        if i == 0 {
                            f.primary_on
                        } else {
                            f.small.get(i - 1).is_some_and(|s| s.2)
                        }
                    });
                    if on {
                        let (sx, sy) = ui.screen_position(me);
                        let r = rects[i];
                        self.requests.push(if i == 0 {
                            SheetRequest::Primary
                        } else {
                            SheetRequest::Small(i - 1, ((sx + r.0) as i32, (sy + r.1 + r.3) as i32))
                        });
                    }
                }
                true
            }
            _ => true,
        }
    }

    fn tooltip_text(&self, element: &Element) -> Option<String> {
        let (x, y) = self.hover?;
        let foot = self.foot.as_ref()?;
        let rects = self.foot_rects(element.width, element.height);
        rects
            .iter()
            .skip(1)
            .position(|r| inside(*r, x, y))
            .and_then(|i| foot.small.get(i))
            .map(|s| s.1.clone())
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        if self.dirty || (self.laid_width - w).abs() > 0.5 {
            self.lay_out(w);
        }
        self.scroll = self.scroll.clamp(0.0, self.most(h));
        let p = self.theme.palette;
        if self.framed {
            let round = CornerRadii::all(self.theme.metrics.window_radius);
            rc.soft_shadow(0.0, 0.0, w, h, 6.0, 18.0, 50.0, hex(0, 0.5));
            rc.fill_rounded_rect(0.0, 0.0, w, h, round, hex(0x14_100C, 0.97));
            rc.stroke_rounded_rect(0.0, 0.0, w, h, round, hex(0xC9_A062, 0.8), 1.0);
        }
        let f = &self.fonts;
        let head = &self.sheet.head;
        // The head.
        let mut x = PAD;
        let top = PAD;
        if let Some(icon) = head.icon {
            let r = CornerRadii::all(4.0);
            rc.fill_rounded_rect(x, top, 52.0, 52.0, r, hex(0, 0.45));
            rc.stroke_rounded_rect(x, top, 52.0, 52.0, r, hex(0xC9_A062, 0.6), 1.0);
            rc.sprite(
                icon,
                x + 10.0,
                top + 10.0,
                32.0,
                32.0,
                (0.0, 0.0, 1.0, 1.0),
                Color::WHITE,
            );
            x += 52.0 + 12.0;
        }
        let right = w - PAD - if self.closable { 30.0 } else { 0.0 };
        let level_w = match (&head.level, &f.big) {
            (Some(l), Some(big)) => text::measure(big, l, 0.0).max(40.0) + 8.0,
            _ => 0.0,
        };
        let room = (right - x - level_w).max(20.0);
        let head_h = if head.icon.is_some() { 52.0 } else { 40.0 };
        if let (Some(name), Some(small)) = (&f.name, &f.small) {
            let lines = wrap(name, &head.name, room);
            let lines: Vec<String> = lines.into_iter().take(2).collect();
            let block = lines.len() as f32 * name.line_height + 3.0 + small.line_height;
            let mut y = (top + (head_h - block) * 0.5).max(top).round();
            for l in &lines {
                text::draw(
                    rc,
                    name,
                    &fit(name, l, room),
                    x,
                    y,
                    TextStyle::plain(head.name_color.unwrap_or(p.text)),
                );
                y += name.line_height;
            }
            text::draw(
                rc,
                small,
                &fit(small, &head.sub, room),
                x,
                y + 3.0,
                TextStyle::plain(p.text_soft),
            );
        }
        if let (Some(level), Some(big), Some(label)) = (&head.level, &f.big, &f.label) {
            let lw = text::measure(label, "LEVEL", 1.0);
            text::draw(
                rc,
                label,
                "LEVEL",
                right - lw,
                top + 2.0,
                TextStyle::plain(p.text_soft).tracked(1.0),
            );
            let bw = text::measure(big, level, 0.0);
            text::draw(
                rc,
                big,
                level,
                right - bw,
                top + 2.0 + label.line_height + 1.0,
                TextStyle::plain(p.accent),
            );
        }
        if self.closable {
            let (cx, cy, cw, _) = self.close_rect(w);
            let hot = self
                .hover
                .is_some_and(|(hx, hy)| inside(self.close_rect(w), hx, hy));
            stroke(
                rc,
                cx + (cw - 14.0) * 0.5,
                cy + 6.0,
                14.0,
                &[&[(6.0, 6.0), (18.0, 18.0)], &[(18.0, 6.0), (6.0, 18.0)]],
                if hot { p.accent } else { p.icon },
                1.6,
            );
        }
        let mut y = top + head_h;
        if let Some(share) = head.health {
            y += 8.0;
            let bar_w = w - PAD * 2.0 - 44.0;
            let r = CornerRadii::all(2.0);
            rc.fill_rounded_rect(PAD, y, bar_w, 4.0, r, hex(0, 0.5));
            rc.fill_rounded_rect(PAD, y, bar_w * share.clamp(0.0, 1.0), 4.0, r, p.enemy);
            if let Some(strong) = &f.strong {
                let words = format!("{}%", (share * 100.0).round() as i32);
                let ww = text::measure(strong, &words, 0.0);
                text::draw(
                    rc,
                    strong,
                    &words,
                    w - PAD - ww,
                    y + 2.0 - strong.line_height * 0.5,
                    TextStyle::plain(p.text),
                );
            }
            y += 6.0;
        }
        if head.failed
            && let Some(small) = &f.small
        {
            y += 6.0;
            text::draw(
                rc,
                small,
                "Your assessment failed. Only its health is known.",
                PAD,
                y,
                TextStyle::plain(p.accent),
            );
            y += 14.0;
        }
        if !self.sheet.facts.is_empty()
            && let (Some(small), Some(strong)) = (&f.small, &f.strong)
        {
            y += 10.0;
            let n = self.sheet.facts.len().min(3) as f32;
            let cw = (w - PAD * 2.0 - 6.0 * (n - 1.0)) / n;
            for (i, (k, v)) in self.sheet.facts.iter().take(3).enumerate() {
                let fx = PAD + i as f32 * (cw + 6.0);
                rc.fill_rounded_rect(fx, y, cw, 38.0, CornerRadii::all(4.0), hex(0xFF_FFFF, 0.04));
                text::draw(
                    rc,
                    small,
                    &fit(small, k, cw - 14.0),
                    fx + 8.0,
                    y + 5.0,
                    TextStyle::plain(p.text_soft),
                );
                text::draw(
                    rc,
                    strong,
                    &fit(strong, v, cw - 14.0),
                    fx + 8.0,
                    y + 7.0 + small.line_height,
                    TextStyle::plain(p.text),
                );
            }
        }
        let head_bottom = self.head_height();
        rc.fill(0.0, head_bottom - 1.0, w, 1.0, hex(0xC9_A062, 0.2));
        // The tabs.
        let n = self.sheet.tabs.len();
        if n > 1
            && let Some(strong) = &f.strong
        {
            let tab_w = (w - 12.0) / n as f32;
            let chosen = self.tab_index();
            for (i, t) in self.sheet.tabs.iter().enumerate() {
                let tx = 6.0 + i as f32 * tab_w;
                let on = i == chosen;
                text::draw_centred(
                    rc,
                    strong,
                    &t.label,
                    tx + tab_w * 0.5,
                    head_bottom + (TABS - strong.line_height) * 0.5,
                    TextStyle::plain(if on { p.text } else { p.text_soft }),
                );
                if on {
                    rc.fill(tx, head_bottom + TABS - 2.0, tab_w, 2.0, p.accent);
                }
            }
            rc.fill(0.0, head_bottom + TABS - 1.0, w, 1.0, hex(0xC9_A062, 0.25));
        }
        // The body.
        let body_top = self.body_top();
        let body_bottom = self.body_bottom(h);
        rc.push_clip(0.0, body_top, w, body_bottom - body_top);
        let off = body_top - self.scroll;
        let inner_right = w - PAD - BAR;
        let hidden = match (&self.writing, self.covered) {
            (Some((_, _, r)), true) => r.clone(),
            _ => 0..0,
        };
        for (i, line) in self.lines.iter().enumerate() {
            if hidden.contains(&i) {
                continue;
            }
            match line {
                Line::Title { y, title, count } => {
                    let y = off + y;
                    if let Some(label) = &f.label {
                        let tw = text::measure(label, title, 1.0);
                        text::draw(
                            rc,
                            label,
                            title,
                            PAD,
                            y,
                            TextStyle::plain(p.text_soft).tracked(1.0),
                        );
                        let cw = f
                            .small
                            .as_ref()
                            .map_or(0.0, |s| text::measure(s, count, 0.0));
                        if let Some(s) = &f.small {
                            text::draw(
                                rc,
                                s,
                                count,
                                inner_right - cw,
                                y - 1.0,
                                TextStyle::plain(p.text_soft),
                            );
                        }
                        let x0 = PAD + tw + 8.0;
                        let x1 = inner_right - cw - if cw > 0.0 { 8.0 } else { 0.0 };
                        if x1 > x0 {
                            rc.fill(
                                x0,
                                (y + label.line_height * 0.5).round(),
                                x1 - x0,
                                1.0,
                                hex(0xC9_A062, 0.22),
                            );
                        }
                    }
                }
                Line::Row {
                    y,
                    key,
                    value,
                    mark,
                } => {
                    let (Some(body), Some(strong)) = (&f.body, &f.strong) else {
                        continue;
                    };
                    let y = off + y;
                    let color = match mark {
                        Mark::Up | Mark::Met => p.good,
                        Mark::Down | Mark::Unmet => p.bad_text,
                        Mark::Gold => p.accent,
                        Mark::None => p.text,
                    };
                    let mark_w = if *mark == Mark::None || *mark == Mark::Gold {
                        0.0
                    } else {
                        12.0
                    };
                    let vw = text::measure(strong, value, 0.0);
                    let vx = inner_right - mark_w - vw;
                    let key_room = (vx - PAD - 10.0).max(30.0);
                    text::draw(
                        rc,
                        body,
                        &fit(body, key, key_room),
                        PAD,
                        y,
                        TextStyle::plain(p.icon),
                    );
                    text::draw(
                        rc,
                        strong,
                        value,
                        vx.max(PAD + 40.0),
                        y,
                        TextStyle::plain(color),
                    );
                    if mark_w > 0.0 {
                        draw_mark(
                            rc,
                            *mark,
                            inner_right - 8.0,
                            y + strong.line_height * 0.5,
                            color,
                        );
                    }
                }
                Line::Bar {
                    y,
                    key,
                    value,
                    share,
                    color,
                } => {
                    let (Some(body), Some(strong)) = (&f.body, &f.strong) else {
                        continue;
                    };
                    let y = off + y;
                    let vw = text::measure(strong, value, 0.0);
                    text::draw(rc, body, key, PAD, y, TextStyle::plain(p.icon));
                    text::draw(
                        rc,
                        strong,
                        value,
                        inner_right - vw,
                        y,
                        TextStyle::plain(p.text),
                    );
                    let bx = inner_right - vw - 8.0 - 56.0;
                    let by = y + body.line_height * 0.5 - 1.5;
                    let r = CornerRadii::all(1.5);
                    rc.fill_rounded_rect(bx, by, 56.0, 3.0, r, hex(0, 0.5));
                    rc.fill_rounded_rect(bx, by, 56.0 * share.clamp(0.0, 1.0), 3.0, r, *color);
                }
                Line::Words {
                    y,
                    text: words,
                    color,
                    strong,
                    small,
                } => {
                    let font = if *strong {
                        &f.strong
                    } else if *small {
                        &f.small
                    } else {
                        &f.body
                    };
                    if let Some(font) = font {
                        text::draw(rc, font, words, PAD, off + y, TextStyle::plain(*color));
                    }
                }
                Line::Legend { y } => {
                    if let Some(s) = &f.small {
                        let y = off + y;
                        let cy = y + s.line_height * 0.5;
                        draw_mark(rc, Mark::Up, PAD, cy, p.good);
                        text::draw(
                            rc,
                            s,
                            "raised by a spell",
                            PAD + 11.0,
                            y,
                            TextStyle::plain(p.muted),
                        );
                        let x = PAD + 11.0 + text::measure(s, "raised by a spell", 0.0) + 14.0;
                        draw_mark(rc, Mark::Down, x, cy, p.bad_text);
                        text::draw(rc, s, "lowered", x + 11.0, y, TextStyle::plain(p.muted));
                    }
                }
                Line::Chip { y, words, color } => {
                    if let Some(f) = &f.strong {
                        let cw = text::measure(f, words, 0.0).ceil() + 18.0;
                        let ch = f.line_height + 8.0;
                        let r = CornerRadii::all(ch * 0.5);
                        let c = *color;
                        rc.fill_rounded_rect(
                            PAD,
                            off + y,
                            cw,
                            ch,
                            r,
                            Color::rgba(c.r, c.g, c.b, 0.16),
                        );
                        rc.stroke_rounded_rect(
                            PAD,
                            off + y,
                            cw,
                            ch,
                            r,
                            Color::rgba(c.r, c.g, c.b, 0.65),
                            1.0,
                        );
                        text::draw(rc, f, words, PAD + 9.0, off + y + 4.0, TextStyle::plain(c));
                    }
                }
                Line::Box { y, h, editable } => {
                    let edge = if *editable {
                        p.accent
                    } else {
                        Color::TRANSPARENT
                    };
                    let r = CornerRadii::all(4.0);
                    let fill = if *editable {
                        hex(0, 0.35)
                    } else {
                        hex(0xFF_FFFF, 0.04)
                    };
                    rc.fill_rounded_rect(PAD, off + y, inner_right - PAD, *h, r, fill);
                    if *editable {
                        rc.stroke_rounded_rect(PAD, off + y, inner_right - PAD, *h, r, edge, 1.0);
                    }
                }
            }
        }
        rc.pop_clip();
        // The scrollbar.
        let room = body_bottom - body_top;
        if self.content > room && room > 0.0 {
            let most = self.most(h);
            let th = (room * room / self.content).max(18.0);
            let ty = body_top + (room - th) * (self.scroll / most.max(1.0));
            rc.fill_rounded_rect(
                w - BAR - 4.0,
                ty,
                BAR,
                th,
                CornerRadii::all(2.5),
                hex(0xC9_A062, 0.7),
            );
        }
        // The foot.
        if let (Some(foot), Some(strong)) = (&self.foot, &f.strong) {
            rc.fill(0.0, h - FOOT, w, 1.0, hex(0xC9_A062, 0.2));
            let rects = self.foot_rects(w, h);
            let r = CornerRadii::all(4.0);
            let hot = |rect: (f32, f32, f32, f32)| {
                self.hover.is_some_and(|(hx, hy)| inside(rect, hx, hy))
            };
            let (mx, my, mw, mh) = rects[0];
            let a = if foot.primary_on { 1.0 } else { 0.4 };
            let fill = if foot.primary_on && hot(rects[0]) {
                hex(0xE0_B068, 1.0)
            } else {
                hex(0xE0_B068, 0.85 * a)
            };
            rc.fill_rounded_rect(mx, my, mw, mh, r, fill);
            text::draw_centred(
                rc,
                strong,
                &foot.primary,
                mx + mw * 0.5,
                my + (mh - strong.line_height) * 0.5,
                TextStyle::plain(p.accent_ink),
            );
            for (i, (icon, _, on)) in foot.small.iter().enumerate() {
                let rect = rects[i + 1];
                let (bx, by, bw, bh) = rect;
                rc.fill_rounded_rect(bx, by, bw, bh, r, hex(0, 0.35));
                let edge = if *on && hot(rect) {
                    p.accent
                } else {
                    hex(0xC9_A062, 0.45)
                };
                rc.stroke_rounded_rect(bx, by, bw, bh, r, edge, 1.0);
                let ink = if *on { p.icon } else { hex(0xE9_D3AE, 0.35) };
                stroke(rc, bx + 8.5, by + 8.5, 15.0, icon.lines(), ink, 1.6);
            }
        }
    }
}
