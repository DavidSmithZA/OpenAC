//! The Effects window's frame: Helpful and Harmful tabs; the list with its
//! search, sort and filter chips, grouped (ending soon, timed, from your
//! gear; harmful ending first) under a vitae card after a death; and the
//! chosen effect's details with Cast again, Show item, or a word on how a
//! harmful one ends.

use ac_ui::modern::tone;
use std::collections::HashMap;
use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::indicators::format_seconds_left;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::effects::{Effect, EffectKind, EffectsList, Group, Sort, Source, grouped};
use crate::ui::modern::hud::effects::vitae_penalty_percent;
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, ButtonLook, Rect, Scroll, Tab, hot, inside};

/// The window's size.
pub const WIDTH: f32 = 760.0;
pub const HEIGHT: f32 = 560.0;
const HEADER: f32 = 52.0;
const TABS_H: f32 = 40.0;
const BODY_TOP: f32 = HEADER + TABS_H + 12.0;
const FOOT: f32 = 52.0;
const BODY_BOTTOM: f32 = HEIGHT - FOOT - 12.0;
const LIST_X: f32 = 20.0;
const LIST_W: f32 = 420.0;
const DETAIL_X: f32 = 456.0;
const DETAIL_W: f32 = WIDTH - 20.0 - DETAIL_X;
const SORT_W: f32 = 170.0;
const GROUP_H: f32 = 26.0;
const ROW_H: f32 = 44.0;
const ROW_GAP: f32 = 2.0;
const VITAE_H: f32 = 64.0;
const DETAIL_HEAD: f32 = 68.0;
const DETAIL_FOOT: f32 = 52.0;
const WHEEL: f32 = 2.0 * (ROW_H + ROW_GAP);

/// The window's glyph: a star of eight points.
const SPARK: &[&[(f32, f32)]] = &[
    &[(12.0, 3.0), (12.0, 21.0)],
    &[(3.0, 12.0), (21.0, 12.0)],
    &[(5.6, 5.6), (18.4, 18.4)],
    &[(18.4, 5.6), (5.6, 18.4)],
];
/// The settings glyph: three sliders.
const SLIDERS: &[&[(f32, f32)]] = &[
    &[(4.0, 6.0), (20.0, 6.0)],
    &[(4.0, 12.0), (20.0, 12.0)],
    &[(4.0, 18.0), (20.0, 18.0)],
    &[(9.0, 4.0), (9.0, 8.0)],
    &[(15.0, 10.0), (15.0, 14.0)],
    &[(7.0, 16.0), (7.0, 20.0)],
];
/// A chevron down.
const CHEVRON: &[&[(f32, f32)]] = &[&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]];

/// The fonts the frame draws with.
#[derive(Debug, Clone, Default)]
pub struct EffectsFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// Tabs.
    pub tab: Option<Arc<UiFont>>,
    /// The tabs' and chips' counts.
    pub badge: Option<Arc<UiFont>>,
    /// The search hint, chips, buttons.
    pub body: Option<Arc<UiFont>>,
    /// A row's name.
    pub name: Option<Arc<UiFont>>,
    /// A row's second line, captions.
    pub small: Option<Arc<UiFont>>,
    /// Times.
    pub time: Option<Arc<UiFont>>,
    /// Spaced capitals.
    pub label: Option<Arc<UiFont>>,
    /// The details' name.
    pub heading: Option<Arc<UiFont>>,
    /// The details' words.
    pub detail: Option<Arc<UiFont>>,
    /// The details' figures.
    pub strong: Option<Arc<UiFont>>,
    /// Vitae's percentage.
    pub big: Option<Arc<UiFont>>,
}

/// What the frame shows besides the effects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectsExtras {
    /// Each spell's icon.
    pub icons: HashMap<u32, Option<TextureRef>>,
    /// The spells the player knows, each with where it sits on the bars
    /// ("On Buffs bar, Shift+4", "Not on a bar").
    pub known: HashMap<u32, String>,
    /// The experience still to earn for 1% of vitae back.
    pub vitae_experience: Option<i64>,
    /// Times are shown (the "show spell durations" option).
    pub show_times: bool,
    /// The warning time before a helpful one ends, seconds (0: never).
    pub soon: f64,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectsRequest {
    /// Close it.
    Close,
    /// Cast this spell again.
    CastAgain(u32),
    /// Show the item an effect comes from.
    ShowItem(u32),
    /// Open the settings.
    Settings,
}

/// The Effects window's frame.
#[derive(Debug)]
pub struct EffectsWindow {
    fonts: EffectsFonts,
    /// The effects.
    pub list: EffectsList,
    /// What else it shows.
    pub extras: EffectsExtras,
    /// The Harmful tab shows.
    pub harmful: bool,
    /// The chip chosen (none: All).
    pub filter: Option<EffectKind>,
    /// The list's order.
    pub sort: Sort,
    /// What is typed in the search.
    pub search: String,
    /// The search field has the keyboard.
    pub searching: bool,
    /// The effect chosen.
    pub chosen: Option<u32>,
    /// What it asked for since the window last looked.
    pub requests: Vec<EffectsRequest>,
    list_scroll: Scroll,
    detail_scroll: Scroll,
    hover: Option<(f32, f32)>,
}

impl EffectsWindow {
    /// An empty window in `theme`.
    #[must_use]
    pub fn new(fonts: EffectsFonts) -> Self {
        Self {
            fonts,
            list: EffectsList::default(),
            extras: EffectsExtras::default(),
            harmful: false,
            filter: None,
            sort: Sort::TimeLeft,
            search: String::new(),
            searching: false,
            chosen: None,
            requests: Vec::new(),
            list_scroll: Scroll::default(),
            detail_scroll: Scroll::default(),
            hover: None,
        }
    }

    /// Shows the Helpful or Harmful tab, from the top.
    pub fn show_tab(&mut self, harmful: bool) {
        if self.harmful != harmful {
            self.harmful = harmful;
            self.chosen = None;
            self.list_scroll = Scroll::default();
            self.detail_scroll = Scroll::default();
        }
    }

    /// The search box's place (the field goes over it).
    #[must_use]
    pub fn search_rect() -> Rect {
        (LIST_X, BODY_TOP, LIST_W - 8.0 - SORT_W, 32.0)
    }

    fn sort_rect() -> Rect {
        (LIST_X + LIST_W - SORT_W, BODY_TOP, SORT_W, 32.0)
    }

    fn tabs(&self) -> [(Tab<'static>, Rect); 2] {
        let helpful = Tab {
            words: "Helpful",
            count: Some(self.list.helpful.len().to_string()),
            on: !self.harmful,
            red: false,
        };
        let harm_count = match self.list.vitae {
            Some(_) => format!("{} \u{b7} vitae", self.list.harmful.len()),
            None => self.list.harmful.len().to_string(),
        };
        let harmful = Tab {
            words: "Harmful",
            count: Some(harm_count),
            on: self.harmful,
            red: true,
        };
        let f = self.fonts.tab.as_ref();
        let b = self.fonts.badge.as_ref();
        let w1 = kit::tab_width(&helpful, f, b);
        let w2 = kit::tab_width(&harmful, f, b);
        [
            (helpful, (16.0, HEADER, w1, TABS_H)),
            (harmful, (16.0 + w1 + 4.0, HEADER, w2, TABS_H)),
        ]
    }

    fn chip_rects(&self) -> Vec<(Option<EffectKind>, String, Rect)> {
        if self.harmful {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut x = LIST_X;
        let y = BODY_TOP + 32.0 + 8.0;
        let all = std::iter::once((None, "All")).chain(
            EffectKind::FILTERS
                .into_iter()
                .map(|k| (Some(k), k.words())),
        );
        for (k, words) in all {
            let count = self.list.helpful_of(k).to_string();
            let w = kit::chip_width(
                words,
                Some(&count),
                self.fonts.body.as_ref(),
                self.fonts.badge.as_ref(),
            );
            out.push((k, count, (x, y, w, 26.0)));
            x += w + 4.0;
        }
        out
    }

    fn list_top(&self) -> f32 {
        if self.harmful {
            BODY_TOP + 32.0 + 8.0
        } else {
            BODY_TOP + 32.0 + 8.0 + 26.0 + 8.0
        }
    }

    fn list_room(&self) -> f32 {
        BODY_BOTTOM - self.list_top()
    }

    fn groups(&self) -> Vec<(Group, Vec<&Effect>)> {
        grouped(
            &self.list,
            self.harmful,
            self.filter,
            &self.search,
            self.sort,
            self.extras.soon,
        )
    }

    fn vitae_shown(&self) -> bool {
        self.harmful && self.list.vitae.is_some()
    }

    /// Each row's identity and its top within the list's content; and the
    /// content's height.
    fn row_tops(&self) -> (Vec<(u32, f32)>, f32) {
        let mut y = if self.vitae_shown() {
            VITAE_H + 8.0
        } else {
            0.0
        };
        let mut out = Vec::new();
        for (_, rows) in self.groups() {
            y += GROUP_H;
            for r in rows {
                out.push((r.identity, y));
                y += ROW_H + ROW_GAP;
            }
        }
        (out, y)
    }

    /// The effect the details show: the one chosen, else the first listed.
    #[must_use]
    pub fn shown(&self) -> Option<&Effect> {
        let groups = self.groups();
        let listed = || groups.iter().flat_map(|(_, r)| r.iter()).copied();
        self.chosen
            .and_then(|id| listed().find(|e| e.identity == id))
            .or_else(|| listed().next())
            .map(|e| self.list.find(e.identity).unwrap_or(e))
    }

    fn detail_rect() -> Rect {
        (DETAIL_X, BODY_TOP, DETAIL_W, BODY_BOTTOM - BODY_TOP)
    }

    fn detail_body_room() -> f32 {
        BODY_BOTTOM - BODY_TOP - DETAIL_HEAD - DETAIL_FOOT
    }

    fn action_rect(&self, e: &Effect) -> Option<(Rect, String)> {
        let (x, y) = (DETAIL_X + 14.0, BODY_BOTTOM - DETAIL_FOOT + 10.0);
        let words = match &e.source {
            Source::You if self.extras.known.contains_key(&e.spell_id) => "Cast again".to_owned(),
            Source::Gear { .. } => "Show item".to_owned(),
            _ => return None,
        };
        let w = kit::button_width(&words, self.fonts.body.as_ref());
        Some(((x, y, w, 32.0), words))
    }

    fn settings_rect() -> Rect {
        (WIDTH - 20.0 - 100.0, HEIGHT - FOOT + 11.0, 100.0, 30.0)
    }

    /// The details' body lines, wrapped, and its height.
    fn detail_lines(&self, e: &Effect) -> (Vec<String>, Vec<String>, f32) {
        let width = DETAIL_W - 28.0 - 8.0;
        let desc = self
            .fonts
            .detail
            .as_ref()
            .map(|f| kit::wrap(f, &e.description, width))
            .unwrap_or_default();
        let under = match (&e.under, &self.fonts.small) {
            (Some(u), Some(f)) => kit::wrap(f, u, width - 20.0),
            _ => Vec::new(),
        };
        let grid = 3.0 * 22.0
            + if e.left.is_some() && self.extras.show_times {
                8.0
            } else {
                0.0
            };
        let mut h = 12.0 + grid + 12.0 + desc.len() as f32 * 18.0;
        if !under.is_empty() {
            h += 12.0 + 16.0 + under.len() as f32 * 16.0;
        }
        (desc, under, h + 12.0)
    }

    fn time_color(&self, e: &Effect) -> Color {
        if e.harmful {
            if e.share_left() < 0.15 {
                kit::RED
            } else {
                kit::HARM
            }
        } else if e.left.is_none() {
            Horizon::current().palette.muted
        } else if Group::of_helpful(e, self.extras.soon) == Group::EndingSoon {
            kit::SOON
        } else {
            Horizon::current().palette.text
        }
    }

    fn time_words(&self, e: &Effect, whole: bool) -> String {
        match (e.left, e.total) {
            (None, _) if whole => "While you wear it".to_owned(),
            (None, _) => "worn".to_owned(),
            (Some(_), _) if !self.extras.show_times => String::new(),
            (Some(t), Some(total)) if whole => format!(
                "{} of {}",
                format_seconds_left(t),
                format_seconds_left(total)
            ),
            (Some(t), _) => format_seconds_left(t),
        }
    }
}

impl Widget for EffectsWindow {
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
                let at = self.hover.unwrap_or((x, y));
                if inside(Self::detail_rect(), at.0, at.1) {
                    let total = self.shown().map_or(0.0, |e| self.detail_lines(e).2);
                    self.detail_scroll
                        .wheel(e.data0, 48.0, total, Self::detail_body_room());
                } else {
                    let (_, total) = self.row_tops();
                    let room = self.list_room();
                    self.list_scroll.wheel(e.data0, WHEEL, total, room);
                }
                true
            }
            kind::CLICK => {
                self.click(x, y);
                true
            }
            _ => false,
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        let at = self.hover?;
        if inside(Self::sort_rect(), at.0, at.1) {
            return Some("Sort by time left, name, or what it changes".to_owned());
        }
        if inside(Self::settings_rect(), at.0, at.1) {
            return Some("Options \u{203a} Interface \u{203a} Vitals and effects".to_owned());
        }
        None
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        self.draw_frame(rc);
        self.draw_list(rc);
        if let Some(e) = self.shown().cloned() {
            self.draw_details(rc, &e);
        } else {
            self.draw_empty_details(rc);
        }
        self.draw_foot(rc);
    }
}

impl EffectsWindow {
    fn click(&mut self, x: f32, y: f32) {
        if inside(kit::close_rect(WIDTH, HEADER), x, y) {
            self.requests.push(EffectsRequest::Close);
            return;
        }
        let tabs = self.tabs();
        if let Some(i) = tabs.iter().position(|(_, r)| inside(*r, x, y)) {
            self.show_tab(i == 1);
            return;
        }
        if inside(Self::sort_rect(), x, y) {
            self.sort = self.sort.next();
            return;
        }
        if let Some((k, _, _)) = self
            .chip_rects()
            .into_iter()
            .find(|(_, _, r)| inside(*r, x, y))
        {
            self.filter = k;
            self.list_scroll = Scroll::default();
            return;
        }
        if inside(Self::settings_rect(), x, y) {
            self.requests.push(EffectsRequest::Settings);
            return;
        }
        if let Some(e) = self.shown().cloned()
            && let Some((r, _)) = self.action_rect(&e)
            && inside(r, x, y)
        {
            match e.source {
                Source::Gear { item, .. } => self.requests.push(EffectsRequest::ShowItem(item)),
                _ => self.requests.push(EffectsRequest::CastAgain(e.spell_id)),
            }
            return;
        }
        let top = self.list_top();
        if inside((LIST_X, top, LIST_W, self.list_room()), x, y) {
            let at = y - top + self.list_scroll.at;
            let (rows, _) = self.row_tops();
            if let Some((id, _)) = rows.into_iter().find(|(_, t)| at >= *t && at < *t + ROW_H) {
                if self.chosen != Some(id) {
                    self.detail_scroll = Scroll::default();
                }
                self.chosen = Some(id);
            }
        }
    }

    fn draw_frame(&self, rc: &mut RenderContext<'_>) {
        let theme = &Horizon::current();
        kit::window_back(rc, theme, WIDTH, HEIGHT, HEADER, false);
        kit::title(
            rc,
            theme,
            self.fonts.title.as_ref(),
            SPARK,
            "Effects on you",
            HEADER,
        );
        let c = kit::close_rect(WIDTH, HEADER);
        kit::close(rc, theme, c, hot(self.hover, c));
        rc.fill(0.0, HEADER + TABS_H - 1.0, WIDTH, 1.0, kit::bronze(0.35));
        for (tab, r) in self.tabs() {
            kit::draw_tab(
                rc,
                theme,
                &tab,
                r,
                self.fonts.tab.as_ref(),
                self.fonts.badge.as_ref(),
                hot(self.hover, r),
            );
        }
        // The search and the sort.
        let s = Self::search_rect();
        kit::search_box(
            rc,
            theme,
            s,
            "Find a spell, or what it changes",
            self.search.is_empty() && !self.searching,
            self.searching,
            self.fonts.body.as_ref(),
        );
        let r = Self::sort_rect();
        let lit = hot(self.hover, r);
        let radius = CornerRadii::all(4.0);
        rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, hex(0, 0.35));
        rc.stroke_rounded_rect(
            r.0,
            r.1,
            r.2,
            r.3,
            radius,
            kit::bronze(if lit { 0.8 } else { 0.45 }),
            1.0,
        );
        if let Some(f) = &self.fonts.body {
            let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                "Sort",
                r.0 + 10.0,
                y,
                TextStyle::plain(Horizon::current().palette.muted),
            );
            let x = r.0 + 10.0 + text::measure(f, "Sort", 0.0) + 6.0;
            let words = kit::fit(f, self.sort.words(), r.0 + r.2 - 26.0 - x);
            text::draw(
                rc,
                f,
                &words,
                x,
                y,
                TextStyle::plain(Horizon::current().palette.text),
            );
        }
        stroke(
            rc,
            r.0 + r.2 - 22.0,
            r.1 + 9.0,
            14.0,
            CHEVRON,
            Horizon::current().palette.icon,
            1.6,
        );
        for (k, count, r) in self.chip_rects() {
            let words = k.map_or("All", EffectKind::words);
            kit::chip(
                rc,
                theme,
                r,
                words,
                Some(&count),
                self.fonts.body.as_ref(),
                self.fonts.badge.as_ref(),
                k == self.filter,
                hot(self.hover, r),
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_list(&mut self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let top = self.list_top();
        let room = self.list_room();
        let (rows, total) = self.row_tops();
        self.list_scroll.hold(total, room);
        let scroll = self.list_scroll.at;
        let right = LIST_X + LIST_W - 10.0;
        let shown = self.shown().map(|e| e.identity);
        rc.push_clip(LIST_X - 2.0, top, LIST_W + 4.0, room);
        let mut y = top - scroll;
        if self.vitae_shown() {
            self.draw_vitae(rc, y, right);
            y += VITAE_H + 8.0;
        }
        let groups = self.groups();
        if groups.is_empty()
            && !self.vitae_shown()
            && let Some(f) = &self.fonts.body
        {
            let words = if self.search.trim().is_empty() {
                if self.harmful {
                    "Nothing harmful on you."
                } else {
                    "No helpful effects on you."
                }
            } else {
                "Nothing matches the search."
            };
            text::draw(
                rc,
                f,
                words,
                LIST_X + 4.0,
                top + 12.0,
                TextStyle::plain(p.muted),
            );
        }
        for (group, list) in &groups {
            if y + GROUP_H > top && y < top + room {
                let color = match group {
                    Group::EndingSoon => kit::SOON,
                    Group::EndingFirst => kit::HARM,
                    _ => tone::soft(1.0),
                };
                kit::group_heading(
                    rc,
                    &Horizon::current(),
                    self.fonts.label.as_ref(),
                    LIST_X,
                    y,
                    GROUP_H,
                    right,
                    group.title(),
                    Some(&list.len().to_string()),
                    color,
                );
            }
            y += GROUP_H;
            for e in list {
                if y + ROW_H > top && y < top + room {
                    self.draw_row(rc, e, y, right, shown == Some(e.identity));
                }
                y += ROW_H + ROW_GAP;
            }
        }
        rc.pop_clip();
        let _ = rows;
        kit::scrollbar(rc, LIST_X + LIST_W - 5.0, top, room, total, scroll);
    }

    fn draw_vitae(&self, rc: &mut RenderContext<'_>, y: f32, right: f32) {
        let Some(kept) = self.list.vitae else {
            return;
        };
        let p = Horizon::current().palette;
        let (x, w) = (LIST_X, right - LIST_X);
        let radius = CornerRadii::all(4.0);
        rc.fill_rounded_rect(x, y, w, VITAE_H, radius, hex(0x3C_0E10, 0.45));
        rc.stroke_rounded_rect(x, y, w, VITAE_H, radius, hex(0xFF_6B70, 0.6), 1.0);
        let percent = vitae_penalty_percent(kept);
        if let Some(f) = &self.fonts.label {
            let words = "VITAE";
            let tw = text::measure(f, words, kit::LABEL_TRACKING);
            text::draw(
                rc,
                f,
                words,
                x + 12.0 + (52.0 - tw) * 0.5,
                y + 14.0,
                TextStyle::plain(kit::HARM).tracked(kit::LABEL_TRACKING),
            );
        }
        if let Some(f) = &self.fonts.big {
            let words = format!("{percent}%");
            let tw = text::measure(f, &words, 0.0);
            text::draw(
                rc,
                f,
                &words,
                x + 12.0 + (52.0 - tw) * 0.5,
                y + 27.0,
                TextStyle::plain(p.text),
            );
        }
        if let Some(f) = &self.fonts.small {
            let tx = x + 12.0 + 52.0 + 12.0;
            let width = x + w - 12.0 - tx;
            let mut lines = kit::wrap(
                f,
                &format!(
                    "From your recent death: health, stamina, mana and skills are {percent}% lower."
                ),
                width,
            );
            let first = lines.len();
            if let Some(more) = self.extras.vitae_experience {
                lines.extend(kit::wrap(
                    f,
                    &format!(
                        "1% comes back after {} more experience.",
                        ac_ui::panels::indicators::format_experience(
                            i32::try_from(more.max(0)).unwrap_or(i32::MAX)
                        )
                    ),
                    width,
                ));
            }
            let step = 16.0;
            let mut ty = y + ((VITAE_H - lines.len() as f32 * step) * 0.5).round();
            for (i, line) in lines.iter().enumerate() {
                let color = if i < first {
                    tone::warm(1.0)
                } else {
                    tone::soft(1.0)
                };
                text::draw(rc, f, line, tx, ty, TextStyle::plain(color));
                ty += step;
            }
        }
    }

    fn draw_tile(&self, rc: &mut RenderContext<'_>, e: &Effect, x: f32, y: f32, side: f32) {
        let p = Horizon::current().palette;
        let radius = CornerRadii::all(4.0);
        rc.fill_rounded_rect(x, y, side, side, radius, p.well);
        if let Some(Some(icon)) = self.extras.icons.get(&e.spell_id) {
            rc.sprite(
                *icon,
                x + 1.0,
                y + 1.0,
                side - 2.0,
                side - 2.0,
                (0.0, 0.0, 1.0, 1.0),
                Color::WHITE,
            );
        }
        let edge = if e.harmful {
            hex(0xFF_6B70, 0.6)
        } else {
            kit::bronze(0.4)
        };
        rc.stroke_rounded_rect(x, y, side, side, radius, edge, 1.0);
    }

    fn draw_row(&self, rc: &mut RenderContext<'_>, e: &Effect, y: f32, right: f32, on: bool) {
        let p = Horizon::current().palette;
        let (x, w) = (LIST_X, right - LIST_X);
        let radius = CornerRadii::all(4.0);
        let lit = hot(self.hover, (x, y, w, ROW_H));
        let fill = if on {
            kit::gold(0.12)
        } else if lit {
            hex(0xFF_FFFF, 0.04)
        } else {
            hex(0, 0.2)
        };
        rc.fill_rounded_rect(x, y, w, ROW_H, radius, fill);
        if on {
            rc.stroke_rounded_rect(x, y, w, ROW_H, radius, kit::gold(1.0), 1.0);
        }
        self.draw_tile(rc, e, x + 6.0, y + 7.0, 30.0);
        // The time, right.
        let time_x = right - 10.0 - 62.0;
        let tc = self.time_color(e);
        if let Some(f) = &self.fonts.time {
            let words = self.time_words(e, false);
            let tw = text::measure(f, &words, 0.0);
            text::draw(
                rc,
                f,
                &words,
                right - 10.0 - tw,
                y + 9.0,
                TextStyle::plain(tc),
            );
        }
        if e.left.is_some() && self.extras.show_times {
            kit::meter(
                rc,
                (right - 10.0 - 56.0, y + 28.0, 56.0, 3.0),
                e.share_left(),
                tc,
            );
        }
        // The name and its line.
        let nx = x + 6.0 + 30.0 + 10.0;
        let room = time_x - 8.0 - nx;
        if let Some(f) = &self.fonts.name {
            let name = kit::fit(f, &e.name, room);
            text::draw(rc, f, &name, nx, y + 7.0, TextStyle::plain(p.text));
        }
        if let Some(f) = &self.fonts.small {
            let change = e.change_words();
            let by = e.source.row_words();
            let mut cx = nx;
            let ty = y + 25.0;
            if !change.is_empty() {
                let shown = kit::fit(f, &change, room);
                let color = if e.harmful { kit::HARM } else { kit::GOOD };
                text::draw(rc, f, &shown, cx, ty, TextStyle::plain(color));
                cx += text::measure(f, &shown, 0.0);
                let sep = " \u{b7} ";
                text::draw(rc, f, sep, cx, ty, TextStyle::plain(p.muted));
                cx += text::measure(f, sep, 0.0);
            }
            let rest = (nx + room - cx).max(0.0);
            if rest > 12.0 {
                text::draw(
                    rc,
                    f,
                    &kit::fit(f, &by, rest),
                    cx,
                    ty,
                    TextStyle::plain(p.muted),
                );
            }
        }
    }

    fn draw_empty_details(&self, rc: &mut RenderContext<'_>) {
        let (x, y, w, h) = Self::detail_rect();
        let radius = CornerRadii::all(4.0);
        rc.fill_rounded_rect(x, y, w, h, radius, hex(0, 0.28));
        rc.stroke_rounded_rect(x, y, w, h, radius, kit::bronze(0.3), 1.0);
        if let Some(f) = &self.fonts.detail {
            let words = "Choose an effect to see what it does.";
            for (i, line) in kit::wrap(f, words, w - 40.0).iter().enumerate() {
                text::draw(
                    rc,
                    f,
                    line,
                    x + 20.0,
                    y + 24.0 + i as f32 * 18.0,
                    TextStyle::plain(Horizon::current().palette.muted),
                );
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_details(&mut self, rc: &mut RenderContext<'_>, e: &Effect) {
        let p = Horizon::current().palette;
        let (x, y, w, h) = Self::detail_rect();
        let radius = CornerRadii::all(4.0);
        rc.fill_rounded_rect(x, y, w, h, radius, hex(0, 0.28));
        rc.stroke_rounded_rect(x, y, w, h, radius, kit::bronze(0.3), 1.0);
        // The head.
        self.draw_tile(rc, e, x + 14.0, y + 14.0, 40.0);
        let tx = x + 14.0 + 40.0 + 10.0;
        let room = x + w - 14.0 - tx;
        let mut ty = y + 15.0;
        if let Some(f) = &self.fonts.heading {
            let lines = kit::wrap(f, &e.name, room);
            for line in lines.iter().take(2) {
                text::draw(rc, f, line, tx, ty, TextStyle::plain(p.text));
                ty += 18.0;
            }
        }
        if let Some(f) = &self.fonts.small {
            text::draw(
                rc,
                f,
                &kit::fit(f, &e.school, room),
                tx,
                ty + 2.0,
                TextStyle::plain(tone::soft(1.0)),
            );
        }
        rc.fill(x, y + DETAIL_HEAD - 1.0, w, 1.0, kit::bronze(0.2));
        // The body, scrolled.
        let (desc, under, total) = self.detail_lines(e);
        let body_top = y + DETAIL_HEAD;
        let body_room = Self::detail_body_room();
        self.detail_scroll.hold(total, body_room);
        rc.push_clip(x, body_top, w, body_room);
        let left = x + 14.0;
        let value_x = left + 70.0 + 10.0;
        let value_w = x + w - 14.0 - 8.0 - value_x;
        let mut by = body_top + 12.0 - self.detail_scroll.at;
        let label = |rc: &mut RenderContext<'_>, words: &str, at: f32| {
            if let Some(f) = &self.fonts.small {
                text::draw(rc, f, words, left, at + 1.0, TextStyle::plain(p.muted));
            }
        };
        // Changes.
        label(rc, "Changes", by);
        if let Some(f) = &self.fonts.strong {
            let stat = e.stat_words();
            let stat = if stat.is_empty() {
                "\u{2014}".to_owned()
            } else {
                stat
            };
            let shown = kit::fit(f, &stat, value_w - 50.0);
            text::draw(rc, f, &shown, value_x, by, TextStyle::plain(p.text));
            if !e.delta.is_empty() {
                let dx = value_x + text::measure(f, &shown, 0.0) + 8.0;
                let color = if e.harmful { kit::HARM } else { kit::GOOD };
                text::draw(rc, f, &e.delta, dx, by, TextStyle::plain(color));
            }
        }
        by += 22.0;
        label(
            rc,
            if matches!(e.source, Source::Gear { .. }) {
                "From"
            } else {
                "Cast by"
            },
            by,
        );
        if let Some(f) = &self.fonts.strong {
            let who = kit::fit(f, &e.source.detail_words(), value_w);
            text::draw(rc, f, &who, value_x, by, TextStyle::plain(p.text));
        }
        by += 22.0;
        label(rc, "Time left", by);
        if let Some(f) = &self.fonts.strong {
            let words = if e.left.is_some() && !self.extras.show_times {
                "Hidden (Options)".to_owned()
            } else {
                self.time_words(e, true)
            };
            text::draw(
                rc,
                f,
                &words,
                value_x,
                by,
                TextStyle::plain(self.time_color(e)),
            );
        }
        by += 22.0;
        if e.left.is_some() && self.extras.show_times {
            kit::meter(
                rc,
                (value_x, by - 4.0, value_w, 4.0),
                e.share_left(),
                self.time_color(e),
            );
            by += 8.0;
        }
        by += 12.0;
        if let Some(f) = &self.fonts.detail {
            by = kit::lines(rc, f, &desc, left, by, 18.0, tone::warm(1.0));
        }
        if !under.is_empty()
            && let Some(f) = &self.fonts.small
        {
            by += 12.0;
            let bh = 16.0 + under.len() as f32 * 16.0;
            let bw = x + w - 14.0 - 8.0 - left;
            dashed(rc, (left, by, bw, bh), kit::bronze(0.45));
            kit::lines(rc, f, &under, left + 10.0, by + 8.0, 16.0, tone::soft(1.0));
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            x + w - 8.0,
            body_top + 4.0,
            body_room - 8.0,
            total,
            self.detail_scroll.at,
        );
        // The foot.
        let foot = y + h - DETAIL_FOOT;
        rc.fill(x, foot, w, 1.0, kit::bronze(0.2));
        let note_x;
        if let Some((r, words)) = self.action_rect(e) {
            let look = if matches!(e.source, Source::Gear { .. }) {
                ButtonLook::Plain
            } else {
                ButtonLook::Primary
            };
            kit::button(
                rc,
                &Horizon::current(),
                r,
                &words,
                self.fonts.body.as_ref(),
                look,
                hot(self.hover, r),
                true,
            );
            note_x = r.0 + r.2 + 10.0;
            let note = match &e.source {
                Source::Gear { .. } => "Lasts while you wear it".to_owned(),
                _ => self
                    .extras
                    .known
                    .get(&e.spell_id)
                    .cloned()
                    .unwrap_or_default(),
            };
            if let Some(f) = &self.fonts.small {
                let shown = kit::fit(f, &note, x + w - 14.0 - note_x);
                let ny = foot + ((DETAIL_FOOT - f.line_height) * 0.5).round();
                text::draw(rc, f, &shown, note_x, ny, TextStyle::plain(p.muted));
            }
        } else if let Some(f) = &self.fonts.small {
            let words = if e.harmful {
                "Ends on its own when the time runs out, or sooner if you dispel it."
            } else {
                "Cast on you by someone else; it ends when the time runs out."
            };
            let lines = kit::wrap(f, words, w - 28.0);
            let step = 15.0;
            let ny = foot + ((DETAIL_FOOT - lines.len() as f32 * step) * 0.5).round();
            kit::lines(rc, f, &lines, x + 14.0, ny, step, tone::soft(1.0));
        }
    }

    fn draw_foot(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let top = HEIGHT - FOOT;
        rc.fill(0.0, top, WIDTH, 1.0, kit::bronze(0.3));
        let mut x = 20.0;
        if let Some(f) = &self.fonts.small {
            let y = top + ((FOOT - f.line_height) * 0.5).round();
            let soon = self.extras.soon;
            let warn = if soon > 0.0 {
                format!(
                    "Warns {} min before a helpful one ends",
                    (soon / 60.0).round() as u32
                )
            } else {
                "No warning before a helpful one ends".to_owned()
            };
            rc.fill_rounded_rect(
                x,
                top + FOOT * 0.5 - 3.0,
                6.0,
                6.0,
                CornerRadii::all(3.0),
                kit::SOON,
            );
            x += 12.0;
            text::draw(rc, f, &warn, x, y, TextStyle::plain(p.muted));
            x += text::measure(f, &warn, 0.0) + 14.0;
            text::draw(
                rc,
                f,
                "Hover a count on screen for a quick list",
                x,
                y,
                TextStyle::plain(p.muted),
            );
        }
        let r = Self::settings_rect();
        let lit = hot(self.hover, r);
        kit::button(
            rc,
            &Horizon::current(),
            r,
            "",
            None,
            ButtonLook::Plain,
            lit,
            true,
        );
        stroke(rc, r.0 + 12.0, r.1 + 8.0, 14.0, SLIDERS, p.icon, 1.5);
        if let Some(f) = &self.fonts.body {
            let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
            text::draw(rc, f, "Settings", r.0 + 32.0, y, TextStyle::plain(p.text));
        }
    }
}

/// A dashed edge round `r`.
fn dashed(rc: &mut RenderContext<'_>, r: Rect, color: Color) {
    let (x, y, w, h) = r;
    let dash: f32 = 5.0;
    let gap: f32 = 4.0;
    let mut t = x + 3.0;
    while t < x + w - 3.0 {
        let run = dash.min(x + w - 3.0 - t);
        rc.fill(t, y, run, 1.0, color);
        rc.fill(t, y + h - 1.0, run, 1.0, color);
        t += dash + gap;
    }
    let mut t = y + 3.0;
    while t < y + h - 3.0 {
        let run = dash.min(y + h - 3.0 - t);
        rc.fill(x, t, 1.0, run, color);
        rc.fill(x + w - 1.0, t, 1.0, run, color);
        t += dash + gap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(id: u32, left: Option<f64>, harmful: bool) -> Effect {
        Effect {
            identity: id,
            spell_id: id,
            name: format!("Spell {id}"),
            school: String::new(),
            description: "Does a thing.".to_owned(),
            kind: EffectKind::Attributes,
            stat: "Focus".to_owned(),
            delta: "+45".to_owned(),
            from_to: None,
            source: Source::You,
            left,
            total: left.map(|_| 5400.0),
            under: None,
            harmful,
        }
    }

    fn window() -> EffectsWindow {
        let mut w = EffectsWindow::new(EffectsFonts::default());
        w.list = EffectsList {
            helpful: vec![
                effect(1, Some(100.0), false),
                effect(2, Some(4000.0), false),
            ],
            harmful: vec![effect(3, Some(20.0), true)],
            vitae: Some(0.94),
        };
        w.extras.soon = 300.0;
        w.extras.show_times = true;
        w
    }

    #[test]
    fn the_details_show_the_first_listed_until_one_is_chosen() {
        let mut w = window();
        assert_eq!(w.shown().map(|e| e.identity), Some(1));
        w.chosen = Some(2);
        assert_eq!(w.shown().map(|e| e.identity), Some(2));
        w.show_tab(true);
        assert_eq!(w.chosen, None);
        assert_eq!(w.shown().map(|e| e.identity), Some(3));
    }

    #[test]
    fn a_click_on_a_row_chooses_it() {
        let mut w = window();
        let (rows, _) = w.row_tops();
        let (id, top) = rows[1];
        w.click(LIST_X + 40.0, w.list_top() + top + 10.0);
        assert_eq!(w.chosen, Some(id));
    }

    #[test]
    fn the_harmful_list_starts_under_the_vitae_card() {
        let mut w = window();
        w.show_tab(true);
        let (rows, _) = w.row_tops();
        assert!((rows[0].1 - (VITAE_H + 8.0 + GROUP_H)).abs() < 1e-6);
    }

    #[test]
    fn cast_again_is_only_for_the_players_own_known_spells() {
        let mut w = window();
        let e = w.list.helpful[0].clone();
        assert!(w.action_rect(&e).is_none());
        w.extras.known.insert(e.spell_id, "Not on a bar".to_owned());
        assert_eq!(
            w.action_rect(&e).map(|(_, s)| s),
            Some("Cast again".to_owned())
        );
    }
}
