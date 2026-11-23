//! The Spellbook's frame and its two lists. The frame draws the header
//! (Spells and Components tabs, the search), the filters down the left
//! (schools, levels, highest level only; or component types), the chosen
//! spell's details (mana, difficulty against the player's skill, the
//! components with how many are carried, the bar slots it is on; Cast, Add
//! to bar, Forget with its question), and the Components page's heading.
//! The spell list and the components table are children of their own, so
//! they scroll by themselves and a spell can be dragged out onto a bar.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_core::spells::MagicSchool;
use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{DragPayload, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::magic::SpellbookShortcutPayload;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{DragGhost, Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::spellbook::{
    CATEGORIES, ComponentRow, SCHOOLS, SpellFilter, SpellRow, SpellSort, category_one,
    chosen_school, level_bit, school_color, school_counts, school_name, shown,
};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, ButtonLook, Rect, Scroll, Tab, hot, inside};

/// The window's size.
pub const WIDTH: f32 = 940.0;
pub const HEIGHT: f32 = 640.0;
const HEADER: f32 = 52.0;
const TOP: f32 = 68.0;
const BOTTOM: f32 = TOP + 556.0;
const NAV_X: f32 = 24.0;
const NAV_W: f32 = 176.0;
/// The middle column (the spell list).
pub const LIST_X: f32 = 220.0;
pub const LIST_W: f32 = 376.0;
/// The list starts under its heading row.
pub const LIST_TOP: f32 = TOP + 34.0;
pub const LIST_H: f32 = BOTTOM - LIST_TOP;
const DETAIL_X: f32 = 612.0;
const DETAIL_W: f32 = 304.0;
/// The components table, under its heading and column names.
pub const TABLE_X: f32 = 220.0;
pub const TABLE_W: f32 = 696.0;
pub const TABLE_TOP: f32 = TOP + 34.0 + 24.0;
pub const TABLE_H: f32 = BOTTOM - TABLE_TOP;
const SEARCH_W: f32 = 220.0;
const DETAIL_HEAD: f32 = 128.0;
const DETAIL_FOOT: f32 = 56.0;
const NAV_ROW: f32 = 30.0;

/// The book's glyph.
const BOOK: &[&[(f32, f32)]] = &[&[
    (4.0, 21.0),
    (4.0, 5.0),
    (5.0, 3.5),
    (6.0, 3.0),
    (19.0, 3.0),
    (19.0, 19.0),
    (6.0, 19.0),
    (4.5, 20.0),
    (4.0, 21.0),
]];
/// A bin, for Forget.
const BIN: &[&[(f32, f32)]] = &[
    &[(4.0, 7.0), (20.0, 7.0)],
    &[(9.0, 7.0), (9.0, 4.0), (15.0, 4.0), (15.0, 7.0)],
    &[(6.0, 7.0), (7.0, 20.0), (17.0, 20.0), (18.0, 7.0)],
    &[(10.0, 11.0), (10.0, 16.0)],
    &[(14.0, 11.0), (14.0, 16.0)],
];
/// A plus, for Add to bar.
const PLUS: &[&[(f32, f32)]] = &[&[(12.0, 5.0), (12.0, 19.0)], &[(5.0, 12.0), (19.0, 12.0)]];

/// The pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BookPage {
    /// The spells.
    #[default]
    Spells,
    /// The components.
    Components,
}

/// The fonts the book draws with.
#[derive(Debug, Clone, Default)]
pub struct BookFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// Tabs, nav rows, buttons.
    pub strong: Option<Arc<UiFont>>,
    /// Quiet words, counts.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals.
    pub label: Option<Arc<UiFont>>,
    /// The search hint, words.
    pub body: Option<Arc<UiFont>>,
    /// A row's name.
    pub name: Option<Arc<UiFont>>,
    /// The details' name.
    pub heading: Option<Arc<UiFont>>,
    /// The list's heading.
    pub section: Option<Arc<UiFont>>,
    /// The filters down the left.
    pub nav: Option<Arc<UiFont>>,
}

/// One component of the chosen spell.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellComponent {
    /// Its name.
    pub name: String,
    /// How many one cast uses.
    pub uses: u32,
    /// How many are carried.
    pub carried: i32,
}

/// The chosen spell, as the details show it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellDetails {
    /// The spell.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its school (for the tile's colour).
    pub school: MagicSchool,
    /// "War Magic · Level VIII".
    pub line: String,
    /// Its mana.
    pub mana: i32,
    /// Its difficulty.
    pub difficulty: i32,
    /// The player's skill in its school.
    pub skill: u32,
    /// The game's description.
    pub description: String,
    /// Its components.
    pub components: Vec<SpellComponent>,
    /// The bar slots it is on: where, and its key.
    pub spots: Vec<(String, String)>,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum BookRequest {
    /// Close it.
    Close,
    /// Show a page.
    Page(BookPage),
    /// Show one school (none: all).
    School(Option<MagicSchool>),
    /// Turn a level on or off.
    Level(i32),
    /// Show only the highest level of each spell, or all.
    HighestOnly(bool),
    /// The next order.
    Sort,
    /// Cast the spell.
    Cast(u32),
    /// Put it on the first free slot of a bar.
    AddToBar(u32),
    /// Forget it (asked and answered).
    Forget(u32),
    /// Show one component type (none: all).
    Category(Option<u32>),
}

/// The Spellbook's frame.
#[derive(Debug)]
pub struct BookFrame {
    fonts: BookFonts,
    /// The page shown.
    pub page: BookPage,
    /// Every learned spell.
    pub spells: Vec<SpellRow>,
    /// What picks the spells shown.
    pub filter: SpellFilter,
    /// The chosen spell's details.
    pub details: Option<SpellDetails>,
    /// A spell can be cast now (Magic, or a caster in hand in Peace).
    pub can_cast: bool,
    /// Every component row.
    pub components: Vec<ComponentRow>,
    /// The component type shown (none: all).
    pub category: Option<u32>,
    /// The search field has the keyboard.
    pub searching: bool,
    /// The question before forgetting shows.
    pub asking_forget: bool,
    /// What it asked for since the window last looked.
    pub requests: Vec<BookRequest>,
    detail_scroll: Scroll,
    hover: Option<(f32, f32)>,
}

impl BookFrame {
    /// An empty book in `theme`.
    #[must_use]
    pub fn new(fonts: BookFonts) -> Self {
        Self {
            fonts,
            page: BookPage::Spells,
            spells: Vec::new(),
            filter: SpellFilter {
                highest_only: true,
                ..SpellFilter::default()
            },
            details: None,
            can_cast: false,
            components: Vec::new(),
            category: None,
            searching: false,
            asking_forget: false,
            requests: Vec::new(),
            detail_scroll: Scroll::default(),
            hover: None,
        }
    }

    /// The details show `d` (the scroll goes back to the top for another
    /// spell).
    pub fn show_details(&mut self, d: Option<SpellDetails>) {
        if d.as_ref().map(|d| d.id) != self.details.as_ref().map(|d| d.id) {
            self.detail_scroll = Scroll::default();
            self.asking_forget = false;
        }
        self.details = d;
    }

    /// The search box's place (the field goes over it).
    #[must_use]
    pub fn search_rect() -> Rect {
        (
            WIDTH - 12.0 - 30.0 - 10.0 - SEARCH_W,
            (HEADER - 30.0) * 0.5,
            SEARCH_W,
            30.0,
        )
    }

    fn tabs(&self) -> [(Tab<'static>, Rect); 2] {
        let f = self.fonts.strong.as_ref();
        let x = self
            .fonts
            .title
            .as_ref()
            .map_or(140.0, |t| 46.0 + text::measure(t, "Spellbook", 0.0) + 16.0);
        let spells = Tab {
            words: "Spells",
            count: None,
            on: self.page == BookPage::Spells,
            red: false,
        };
        let comps = Tab {
            words: "Components",
            count: None,
            on: self.page == BookPage::Components,
            red: false,
        };
        let w1 = kit::tab_width(&spells, f, None);
        let w2 = kit::tab_width(&comps, f, None);
        let y = (HEADER - 34.0) * 0.5;
        [
            (spells, (x, y, w1, 34.0)),
            (comps, (x + w1 + 2.0, y, w2, 34.0)),
        ]
    }

    fn nav_rows(&self) -> Vec<(Option<u32>, String, usize, Color, Rect)> {
        let mut out = Vec::new();
        let mut y = TOP + 18.0;
        let gold = kit::gold(1.0);
        match self.page {
            BookPage::Spells => {
                let counts = school_counts(&self.spells);
                out.push((
                    None,
                    "All schools".to_owned(),
                    self.spells.len(),
                    gold,
                    (NAV_X, y, NAV_W, NAV_ROW),
                ));
                for (i, (school, name, color)) in SCHOOLS.iter().enumerate() {
                    y += NAV_ROW + 2.0;
                    out.push((
                        Some(school.0),
                        (*name).to_owned(),
                        counts[i],
                        *color,
                        (NAV_X, y, NAV_W, NAV_ROW),
                    ));
                }
            }
            BookPage::Components => {
                out.push((
                    None,
                    "All".to_owned(),
                    self.components.len(),
                    gold,
                    (NAV_X, y, NAV_W, NAV_ROW),
                ));
                for (i, name) in CATEGORIES.iter().enumerate() {
                    y += NAV_ROW + 2.0;
                    let n = self
                        .components
                        .iter()
                        .filter(|c| c.category == i as u32)
                        .count();
                    out.push((
                        Some(i as u32),
                        (*name).to_owned(),
                        n,
                        category_color(i as u32),
                        (NAV_X, y, NAV_W, NAV_ROW),
                    ));
                }
            }
        }
        out
    }

    fn levels_top() -> f32 {
        TOP + 18.0 + 6.0 * (NAV_ROW + 2.0) + 14.0
    }

    fn level_rects() -> Vec<(i32, Rect)> {
        let top = Self::levels_top() + 18.0;
        let w = (NAV_W - 3.0 * 4.0) / 4.0;
        (1..=8)
            .map(|l| {
                let i = (l - 1) as f32;
                let (col, row) = (i % 4.0, (i / 4.0).floor());
                (l, (NAV_X + col * (w + 4.0), top + row * 30.0, w, 26.0))
            })
            .collect()
    }

    fn highest_rect() -> Rect {
        (NAV_X, Self::levels_top() + 18.0 + 60.0 + 6.0, NAV_W, 20.0)
    }

    fn sort_rect(&self) -> Rect {
        let w = self.fonts.small.as_ref().map_or(90.0, |f| {
            text::measure(f, "Sort", 0.0) + 6.0 + text::measure(f, self.filter.sort.words(), 0.0)
        }) + 32.0;
        (LIST_X + LIST_W - 6.0 - w, TOP, w, 26.0)
    }

    fn detail_body_room() -> f32 {
        BOTTOM - TOP - DETAIL_HEAD - DETAIL_FOOT
    }

    fn foot_rects() -> (Rect, Rect, Rect) {
        let y = BOTTOM - DETAIL_FOOT + 12.0;
        let forget = (DETAIL_X + DETAIL_W - 14.0 - 32.0, y, 32.0, 32.0);
        let add = (forget.0 - 6.0 - 118.0, y, 118.0, 32.0);
        let cast = (DETAIL_X + 14.0, y, add.0 - 6.0 - DETAIL_X - 14.0, 32.0);
        (cast, add, forget)
    }

    fn forget_box() -> Rect {
        (DETAIL_X, BOTTOM - DETAIL_FOOT - 140.0, DETAIL_W, 136.0)
    }

    fn forget_buttons() -> (Rect, Rect) {
        let (x, y, w, h) = Self::forget_box();
        let forget = (x + w - 14.0 - 104.0, y + h - 14.0 - 30.0, 104.0, 30.0);
        let keep = (forget.0 - 6.0 - 72.0, forget.1, 72.0, 30.0);
        (keep, forget)
    }

    fn detail_lines(&self, d: &SpellDetails) -> (Vec<String>, f32) {
        let width = DETAIL_W - 28.0 - 8.0;
        let desc = self
            .fonts
            .body
            .as_ref()
            .map(|f| kit::wrap(f, &d.description, width))
            .unwrap_or_default();
        let mut h = 10.0 + desc.len() as f32 * 18.0 + 12.0;
        h += 18.0 + d.components.len().max(1) as f32 * 18.0 + 12.0;
        h += 18.0 + d.spots.len().max(1) as f32 * 18.0 + 10.0;
        (desc, h)
    }
}

/// A component type's colour.
fn category_color(category: u32) -> Color {
    match category {
        0 => hex(0xB3_AA9C, 1.0),
        1 => kit::GOOD,
        2 => hex(0xC9_A2FF, 1.0),
        3 => hex(0x6E_A8FF, 1.0),
        4 => kit::gold(1.0),
        5 => kit::HARM,
        _ => tone::soft(1.0),
    }
}

impl Widget for BookFrame {
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
                if let Some(d) = self.details.clone() {
                    let total = self.detail_lines(&d).1;
                    self.detail_scroll
                        .wheel(e.data0, 48.0, total, Self::detail_body_room());
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
        if self.page == BookPage::Spells && self.details.is_some() {
            let (cast, add, forget) = Self::foot_rects();
            if inside(cast, at.0, at.1) && !self.can_cast {
                return Some("Go into Magic mode, or hold a caster, to cast".to_owned());
            }
            if inside(add, at.0, at.1) {
                return Some("Add to a bar, or drag the spell onto one".to_owned());
            }
            if inside(forget, at.0, at.1) {
                return Some("Forget this spell".to_owned());
            }
        }
        if self.page == BookPage::Spells && inside(self.sort_rect(), at.0, at.1) {
            return Some("Order by the game's order, name, or level".to_owned());
        }
        None
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        self.draw_frame(rc);
        self.draw_nav(rc);
        match self.page {
            BookPage::Spells => {
                self.draw_list_heading(rc);
                self.draw_details(rc);
            }
            BookPage::Components => self.draw_table_heading(rc),
        }
    }
}

impl BookFrame {
    fn click(&mut self, x: f32, y: f32) {
        if inside(kit::close_rect(WIDTH, HEADER), x, y) {
            self.requests.push(BookRequest::Close);
            return;
        }
        if self.asking_forget {
            let (keep, forget) = Self::forget_buttons();
            if inside(forget, x, y)
                && let Some(d) = &self.details
            {
                self.requests.push(BookRequest::Forget(d.id));
            }
            if inside(forget, x, y) || inside(keep, x, y) || !inside(Self::forget_box(), x, y) {
                self.asking_forget = false;
            }
            return;
        }
        let tabs = self.tabs();
        if let Some(i) = tabs.iter().position(|(_, r)| inside(*r, x, y)) {
            let page = if i == 0 {
                BookPage::Spells
            } else {
                BookPage::Components
            };
            self.page = page;
            self.requests.push(BookRequest::Page(page));
            return;
        }
        if let Some((key, ..)) = self.nav_rows().into_iter().find(|(.., r)| inside(*r, x, y)) {
            match self.page {
                BookPage::Spells => self
                    .requests
                    .push(BookRequest::School(key.map(MagicSchool))),
                BookPage::Components => {
                    self.category = key;
                    self.requests.push(BookRequest::Category(key));
                }
            }
            return;
        }
        if self.page == BookPage::Components {
            return;
        }
        if let Some((level, _)) = Self::level_rects()
            .into_iter()
            .find(|(_, r)| inside(*r, x, y))
        {
            self.requests.push(BookRequest::Level(level));
            return;
        }
        if inside(Self::highest_rect(), x, y) {
            self.filter.highest_only = !self.filter.highest_only;
            self.requests
                .push(BookRequest::HighestOnly(self.filter.highest_only));
            return;
        }
        if inside(self.sort_rect(), x, y) {
            self.filter.sort = self.filter.sort.next();
            self.requests.push(BookRequest::Sort);
            return;
        }
        if let Some(d) = &self.details {
            let (cast, add, forget) = Self::foot_rects();
            if inside(cast, x, y) && self.can_cast {
                self.requests.push(BookRequest::Cast(d.id));
            } else if inside(add, x, y) {
                self.requests.push(BookRequest::AddToBar(d.id));
            } else if inside(forget, x, y) {
                self.asking_forget = true;
            }
        }
    }

    fn draw_frame(&self, rc: &mut RenderContext<'_>) {
        let theme = &Horizon::current();
        kit::window_back(rc, theme, WIDTH, HEIGHT, HEADER, true);
        kit::title(
            rc,
            theme,
            self.fonts.title.as_ref(),
            BOOK,
            "Spellbook",
            HEADER,
        );
        for (tab, r) in self.tabs() {
            kit::draw_tab(
                rc,
                theme,
                &tab,
                r,
                self.fonts.strong.as_ref(),
                None,
                hot(self.hover, r),
            );
        }
        let hint = match self.page {
            BookPage::Spells => "Search spells",
            BookPage::Components => "Search components",
        };
        kit::search_box(
            rc,
            theme,
            Self::search_rect(),
            hint,
            self.filter.search.is_empty() && !self.searching,
            self.searching,
            self.fonts.body.as_ref(),
        );
        let c = kit::close_rect(WIDTH, HEADER);
        kit::close(rc, theme, c, hot(self.hover, c));
    }

    #[allow(clippy::too_many_lines)]
    fn draw_nav(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let title = match self.page {
            BookPage::Spells => "SCHOOL",
            BookPage::Components => "TYPE",
        };
        if let Some(f) = &self.fonts.label {
            text::draw(
                rc,
                f,
                title,
                NAV_X,
                TOP,
                TextStyle::plain(tone::soft(1.0)).tracked(kit::LABEL_TRACKING),
            );
        }
        let chosen_school = chosen_school(self.filter.bits).map(|s| s.0);
        for (key, words, n, color, r) in self.nav_rows() {
            let on = match self.page {
                BookPage::Spells => key == chosen_school,
                BookPage::Components => key == self.category,
            };
            let lit = hot(self.hover, r);
            let radius = CornerRadii::all(4.0);
            if on {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, kit::gold(0.16));
                rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, kit::gold(1.0), 1.0);
            } else if lit {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, hex(0xFF_FFFF, 0.04));
            }
            rc.fill_rounded_rect(
                r.0 + 7.0,
                r.1 + (r.3 - 8.0) * 0.5,
                8.0,
                8.0,
                CornerRadii::all(2.0),
                color,
            );
            let count = n.to_string();
            let mut right = r.0 + r.2 - 7.0;
            if let Some(f) = &self.fonts.small {
                let w = text::measure(f, &count, 0.0);
                right -= w;
                let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
                text::draw(rc, f, &count, right, y, TextStyle::plain(tone::soft(1.0)));
            }
            if let Some(f) = &self.fonts.nav {
                let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
                let x = r.0 + 22.0;
                let shown = kit::fit(f, &words, right - 6.0 - x);
                text::draw(rc, f, &shown, x, y, TextStyle::plain(p.text));
            }
        }
        if self.page == BookPage::Components {
            let (x, y, w) = (NAV_X, TOP + 18.0 + 8.0 * (NAV_ROW + 2.0) + 8.0, NAV_W);
            if let (Some(s), Some(b)) = (&self.fonts.small, &self.fonts.strong) {
                let lines = kit::wrap(
                    s,
                    "Set how many of each component you want to carry. Component vendors offer to top you up to these amounts.",
                    w - 20.0,
                );
                let h = 10.0 + 18.0 + lines.len() as f32 * 15.0 + 10.0;
                rc.fill_rounded_rect(x, y, w, h, CornerRadii::all(4.0), hex(0xFF_FFFF, 0.04));
                text::draw(
                    rc,
                    b,
                    "Restock amounts",
                    x + 10.0,
                    y + 10.0,
                    TextStyle::plain(tone::warm(1.0)),
                );
                kit::lines(rc, s, &lines, x + 10.0, y + 28.0, 15.0, tone::soft(1.0));
            }
            return;
        }
        // The levels.
        if let Some(f) = &self.fonts.label {
            text::draw(
                rc,
                f,
                "LEVEL",
                NAV_X,
                Self::levels_top(),
                TextStyle::plain(tone::soft(1.0)).tracked(kit::LABEL_TRACKING),
            );
        }
        for (level, r) in Self::level_rects() {
            let on = self.filter.bits & level_bit(level) != 0;
            let radius = CornerRadii::all(4.0);
            let lit = hot(self.hover, r);
            if on {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, kit::gold(0.18));
            }
            let edge = if on {
                kit::gold(1.0)
            } else if lit {
                kit::bronze(0.6)
            } else {
                kit::bronze(0.3)
            };
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, edge, 1.0);
            if let (Some(f), Some(numeral)) = (&self.fonts.small, super::effects::roman(level)) {
                let color = if on { p.text } else { p.muted };
                let w = text::measure(f, numeral, 0.0);
                let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    numeral,
                    r.0 + ((r.2 - w) * 0.5).round(),
                    y,
                    TextStyle::plain(color),
                );
            }
        }
        // Highest level only.
        let r = Self::highest_rect();
        let on = self.filter.highest_only;
        let (sx, sy) = (r.0, r.1 + 2.0);
        let track = if on {
            kit::gold(0.85)
        } else {
            hex(0xFF_FFFF, 0.12)
        };
        rc.fill_rounded_rect(sx, sy, 30.0, 16.0, CornerRadii::all(8.0), track);
        let knob_x = if on { sx + 16.0 } else { sx + 2.0 };
        let knob = if on { tone::ink(1.0) } else { p.text };
        rc.fill_rounded_rect(knob_x, sy + 2.0, 12.0, 12.0, CornerRadii::all(6.0), knob);
        if let Some(f) = &self.fonts.body {
            let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                "Highest level only",
                sx + 38.0,
                y,
                TextStyle::plain(p.text),
            );
        }
        if let Some(f) = &self.fonts.small {
            let lines = kit::wrap(
                f,
                "Hides lower levels of spells you know at a higher level.",
                NAV_W,
            );
            kit::lines(rc, f, &lines, NAV_X, r.1 + r.3 + 6.0, 15.0, tone::soft(1.0));
        }
    }

    fn draw_list_heading(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let title = chosen_school(self.filter.bits).map_or("All schools", school_name);
        let shown_count: usize = shown(&self.spells, &self.filter)
            .iter()
            .map(|(_, r)| r.len())
            .sum();
        let mut x = LIST_X;
        if let Some(f) = &self.fonts.section {
            let y = TOP + ((26.0 - f.line_height) * 0.5).round();
            text::draw(rc, f, title, x, y, TextStyle::plain(p.text));
            x += text::measure(f, title, 0.0) + 8.0;
        }
        if let Some(f) = &self.fonts.small {
            let words = format!("{shown_count} shown of {} learned", self.spells.len());
            let y = TOP + ((26.0 - f.line_height) * 0.5).round();
            text::draw(rc, f, &words, x, y, TextStyle::plain(tone::soft(1.0)));
        }
        let r = self.sort_rect();
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
        if let Some(f) = &self.fonts.small {
            let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                "Sort",
                r.0 + 8.0,
                y,
                TextStyle::plain(tone::soft(1.0)),
            );
            let sx = r.0 + 8.0 + text::measure(f, "Sort", 0.0) + 6.0;
            text::draw(
                rc,
                f,
                self.filter.sort.words(),
                sx,
                y,
                TextStyle::plain(p.text),
            );
        }
        stroke(
            rc,
            r.0 + r.2 - 18.0,
            r.1 + 7.0,
            12.0,
            &[&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]],
            tone::soft(1.0),
            1.5,
        );
    }

    fn draw_table_heading(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let title = self
            .category
            .and_then(|c| CATEGORIES.get(c as usize).copied())
            .unwrap_or("All components");
        let mut x = TABLE_X;
        if let Some(f) = &self.fonts.section {
            let y = TOP + ((26.0 - f.line_height) * 0.5).round();
            text::draw(rc, f, title, x, y, TextStyle::plain(p.text));
            x += text::measure(f, title, 0.0) + 8.0;
        }
        if let Some(f) = &self.fonts.small {
            let words = super::spellbook::components_summary(&self.components);
            let y = TOP + ((26.0 - f.line_height) * 0.5).round();
            text::draw(rc, f, &words, x, y, TextStyle::plain(tone::soft(1.0)));
        }
        if let Some(f) = &self.fonts.label {
            let y = TOP + 34.0 + 4.0;
            let cols = table_columns();
            let style = TextStyle::plain(tone::soft(1.0)).tracked(kit::LABEL_TRACKING);
            text::draw(rc, f, "COMPONENT", TABLE_X + cols[0], y, style);
            text::draw(rc, f, "TYPE", TABLE_X + cols[1], y, style);
            let w = text::measure(f, "CARRIED", kit::LABEL_TRACKING);
            text::draw(rc, f, "CARRIED", TABLE_X + cols[3] - 12.0 - w, y, style);
            let w = text::measure(f, "RESTOCK TO", kit::LABEL_TRACKING);
            text::draw(rc, f, "RESTOCK TO", TABLE_X + cols[4] - 12.0 - w, y, style);
            text::draw(rc, f, "STOCK", TABLE_X + cols[4], y, style);
        }
        rc.fill(TABLE_X, TABLE_TOP - 4.0, TABLE_W, 1.0, kit::bronze(0.25));
    }

    #[allow(clippy::too_many_lines)]
    fn draw_details(&mut self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let (x, y, w, h) = (DETAIL_X, TOP, DETAIL_W, BOTTOM - TOP);
        let radius = CornerRadii::all(4.0);
        rc.fill_rounded_rect(x, y, w, h, radius, hex(0, 0.28));
        rc.stroke_rounded_rect(x, y, w, h, radius, kit::bronze(0.35), 1.0);
        let Some(d) = self.details.clone() else {
            if let Some(f) = &self.fonts.body {
                let lines = kit::wrap(
                    f,
                    "Choose a spell to see what it costs, what it needs, and where it sits on your bars.",
                    w - 40.0,
                );
                kit::lines(rc, f, &lines, x + 20.0, y + 24.0, 18.0, p.muted);
            }
            return;
        };
        // The head.
        let tile = (x + 14.0, y + 14.0);
        let color = school_color(d.school);
        rc.fill_vertical_gradient(
            tile.0,
            tile.1,
            52.0,
            52.0,
            radius,
            Color { a: 0.2, ..color },
            hex(0, 0.55),
        );
        if let Some(icon) = d.icon {
            rc.sprite(
                icon,
                tile.0 + 2.0,
                tile.1 + 2.0,
                48.0,
                48.0,
                (0.0, 0.0, 1.0, 1.0),
                Color::WHITE,
            );
        }
        rc.stroke_rounded_rect(tile.0, tile.1, 52.0, 52.0, radius, kit::bronze(0.6), 1.0);
        let tx = tile.0 + 52.0 + 12.0;
        let room = x + w - 14.0 - tx;
        let mut ty = tile.1 + 4.0;
        if let Some(f) = &self.fonts.heading {
            for line in kit::wrap(f, &d.name, room).iter().take(2) {
                text::draw(rc, f, line, tx, ty, TextStyle::plain(p.text));
                ty += 18.0;
            }
        }
        if let Some(f) = &self.fonts.small {
            text::draw(
                rc,
                f,
                &kit::fit(f, &d.line, room),
                tx,
                ty + 2.0,
                TextStyle::plain(tone::soft(1.0)),
            );
        }
        // The three facts.
        let fy = tile.1 + 52.0 + 10.0;
        let fw = (w - 28.0 - 12.0) / 3.0;
        let skill_color = if i64::from(d.skill) >= i64::from(d.difficulty) {
            kit::GOOD
        } else {
            kit::HARM
        };
        let facts = [
            ("Mana", d.mana.to_string(), p.text),
            ("Difficulty", d.difficulty.to_string(), p.text),
            ("Your skill", d.skill.to_string(), skill_color),
        ];
        for (i, (k, v, c)) in facts.iter().enumerate() {
            let fx = x + 14.0 + i as f32 * (fw + 6.0);
            rc.fill_rounded_rect(fx, fy, fw, 40.0, radius, hex(0xFF_FFFF, 0.04));
            if let Some(f) = &self.fonts.small {
                text::draw(
                    rc,
                    f,
                    k,
                    fx + 8.0,
                    fy + 5.0,
                    TextStyle::plain(tone::soft(1.0)),
                );
            }
            if let Some(f) = &self.fonts.strong {
                text::draw(rc, f, v, fx + 8.0, fy + 20.0, TextStyle::plain(*c));
            }
        }
        rc.fill(x, y + DETAIL_HEAD - 1.0, w, 1.0, kit::bronze(0.2));
        // The body.
        let (desc, total) = self.detail_lines(&d);
        let body_top = y + DETAIL_HEAD;
        let room = Self::detail_body_room();
        self.detail_scroll.hold(total, room);
        rc.push_clip(x, body_top, w, room);
        let left = x + 14.0;
        let right = x + w - 14.0 - 8.0;
        let mut by = body_top + 10.0 - self.detail_scroll.at;
        if let Some(f) = &self.fonts.body {
            by = kit::lines(rc, f, &desc, left, by, 18.0, tone::warm(1.0));
        }
        by += 12.0;
        let label = |rc: &mut RenderContext<'_>, words: &str, at: f32| {
            if let Some(f) = &self.fonts.label {
                text::draw(
                    rc,
                    f,
                    words,
                    left,
                    at,
                    TextStyle::plain(tone::soft(1.0)).tracked(kit::LABEL_TRACKING),
                );
            }
        };
        label(rc, "COMPONENTS", by);
        by += 18.0;
        if d.components.is_empty()
            && let Some(f) = &self.fonts.body
        {
            text::draw(rc, f, "None needed", left, by, TextStyle::plain(p.muted));
            by += 18.0;
        }
        for c in &d.components {
            let (dot, have, color) = if c.carried <= 0 {
                (kit::HARM, "none".to_owned(), kit::HARM)
            } else if c.carried < 10 {
                (
                    kit::gold(1.0),
                    format!("{} left", c.carried),
                    kit::gold(1.0),
                )
            } else {
                (kit::GOOD, format!("{} left", c.carried), p.text)
            };
            rc.fill_rounded_rect(left, by + 6.0, 6.0, 6.0, CornerRadii::all(3.0), dot);
            if let (Some(f), Some(s)) = (&self.fonts.body, &self.fonts.strong) {
                let hw = text::measure(s, &have, 0.0);
                text::draw(rc, s, &have, right - hw, by, TextStyle::plain(color));
                let uses = format!("\u{d7}{}", c.uses);
                let uw = text::measure(f, &uses, 0.0);
                let ux = right - 62.0 - uw;
                text::draw(rc, f, &uses, ux, by, TextStyle::plain(tone::soft(1.0)));
                let name = kit::fit(f, &c.name, ux - 8.0 - (left + 14.0));
                text::draw(rc, f, &name, left + 14.0, by, TextStyle::plain(p.text));
            }
            by += 18.0;
        }
        by += 12.0;
        label(rc, "ON YOUR BARS", by);
        by += 18.0;
        if let Some(f) = &self.fonts.body {
            if d.spots.is_empty() {
                text::draw(rc, f, "Not on a bar", left, by, TextStyle::plain(p.muted));
            }
            for (spot, key) in &d.spots {
                let kw = self
                    .fonts
                    .strong
                    .as_ref()
                    .map_or(0.0, |s| text::measure(s, key, 0.0));
                if let Some(s) = &self.fonts.strong {
                    text::draw(rc, s, key, right - kw, by, TextStyle::plain(p.text));
                }
                let shown = kit::fit(f, spot, right - kw - 8.0 - left);
                text::draw(rc, f, &shown, left, by, TextStyle::plain(p.text));
                by += 18.0;
            }
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            x + w - 8.0,
            body_top + 4.0,
            room - 8.0,
            total,
            self.detail_scroll.at,
        );
        // The foot.
        let foot = BOTTOM - DETAIL_FOOT;
        rc.fill(x, foot, w, 1.0, kit::bronze(0.2));
        let (cast, add, forget) = Self::foot_rects();
        kit::button(
            rc,
            &Horizon::current(),
            cast,
            "Cast",
            self.fonts.strong.as_ref(),
            ButtonLook::Primary,
            hot(self.hover, cast),
            self.can_cast,
        );
        kit::button(
            rc,
            &Horizon::current(),
            add,
            "",
            None,
            ButtonLook::Plain,
            hot(self.hover, add),
            true,
        );
        stroke(rc, add.0 + 10.0, add.1 + 9.0, 14.0, PLUS, p.icon, 1.6);
        if let Some(f) = &self.fonts.strong {
            let ty = add.1 + ((add.3 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                "Add to bar",
                add.0 + 30.0,
                ty,
                TextStyle::plain(p.text),
            );
        }
        let lit = hot(self.hover, forget) || self.asking_forget;
        let r = CornerRadii::all(4.0);
        rc.fill_rounded_rect(forget.0, forget.1, forget.2, forget.3, r, hex(0, 0.35));
        rc.stroke_rounded_rect(
            forget.0,
            forget.1,
            forget.2,
            forget.3,
            r,
            if lit { kit::HARM } else { kit::bronze(0.45) },
            1.0,
        );
        stroke(
            rc,
            forget.0 + 8.0,
            forget.1 + 8.0,
            16.0,
            BIN,
            if lit { kit::HARM } else { p.icon },
            1.5,
        );
        if self.asking_forget {
            self.draw_forget(rc, &d);
        }
    }

    fn draw_forget(&self, rc: &mut RenderContext<'_>, d: &SpellDetails) {
        let p = Horizon::current().palette;
        let (x, y, w, h) = Self::forget_box();
        rc.soft_shadow(x, y, w, h, 6.0, 12.0, 30.0, hex(0, 0.6));
        let r = CornerRadii::all(6.0);
        rc.fill_rounded_rect(x, y, w, h, r, tone::raised(0.98));
        rc.stroke_rounded_rect(x, y, w, h, r, kit::HARM, 1.0);
        let mut ty = y + 14.0;
        if let Some(f) = &self.fonts.strong {
            let t = kit::fit(f, &format!("Forget {}?", d.name), w - 28.0);
            text::draw(rc, f, &t, x + 14.0, ty, TextStyle::plain(p.text));
            ty += 22.0;
        }
        if let Some(f) = &self.fonts.small {
            let slots = match d.spots.len() {
                0 => "your spellbook".to_owned(),
                1 => "your spellbook and the 1 bar slot it is on".to_owned(),
                n => format!("your spellbook and the {n} bar slots it is on"),
            };
            let words =
                format!("It leaves {slots}. To cast it again you need to learn it from a scroll.");
            let lines = kit::wrap(f, &words, w - 28.0);
            kit::lines(rc, f, &lines, x + 14.0, ty, 16.0, tone::warm(1.0));
        }
        let (keep, forget) = Self::forget_buttons();
        if let Some(f) = &self.fonts.strong {
            let kw = text::measure(f, "Keep it", 0.0);
            let ky = keep.1 + ((keep.3 - f.line_height) * 0.5).round();
            let color = if hot(self.hover, keep) {
                p.text
            } else {
                tone::soft(1.0)
            };
            text::draw(
                rc,
                f,
                "Keep it",
                keep.0 + (keep.2 - kw) * 0.5,
                ky,
                TextStyle::plain(color),
            );
        }
        kit::button(
            rc,
            &Horizon::current(),
            forget,
            "Forget spell",
            self.fonts.strong.as_ref(),
            ButtonLook::Danger,
            hot(self.hover, forget),
            true,
        );
    }
}

/// The table's column starts within it: name, type, (carried ends at the
/// restock column less a gap), restock (ends at stock less a gap), stock.
fn table_columns() -> [f32; 5] {
    let inner = TABLE_W - 20.0;
    let unit = inner / (2.4 + 1.3 + 1.0 + 1.0 + 1.6);
    let name = 10.0;
    let kind = name + 2.4 * unit;
    let carried = kind + 1.3 * unit;
    let restock = carried + 1.0 * unit;
    let stock = restock + 1.0 * unit;
    [name, kind, carried, restock, stock]
}

// ── The spell list ──────────────────────────────────────────────────────

const GROUP_H: f32 = 30.0;
const ROW_H: f32 = 38.0;
const ROW_GAP: f32 = 2.0;

/// What the list asks of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListRequest {
    /// Choose the spell.
    Choose(u32),
    /// Put it on a bar (a double click).
    Favourite(u32),
    /// Examine it (a right click).
    Examine(u32),
}

/// The spell list: rows under school headings, scrolled by itself; a row
/// dragged out carries the spell to the bars.
#[derive(Debug)]
pub struct SpellList {
    fonts: BookFonts,
    /// The spells shown, by school.
    pub groups: Vec<(MagicSchool, Vec<SpellRow>)>,
    /// The spell chosen.
    pub chosen: Option<u32>,
    /// What it asked for since the window last looked.
    pub requests: Vec<ListRequest>,
    /// A spell is being dragged out (the others fade).
    pub lifted: Option<u32>,
    scroll: Scroll,
    pressed: Option<u32>,
    hover: Option<(f32, f32)>,
}

impl SpellList {
    /// An empty list.
    #[must_use]
    pub fn new(fonts: BookFonts) -> Self {
        Self {
            fonts,
            groups: Vec::new(),
            chosen: None,
            requests: Vec::new(),
            lifted: None,
            scroll: Scroll::default(),
            pressed: None,
            hover: None,
        }
    }

    /// Back to the top.
    pub fn to_top(&mut self) {
        self.scroll = Scroll::default();
    }

    /// Each row's spell and top, and the content's height.
    #[must_use]
    pub fn row_tops(&self) -> (Vec<(u32, f32)>, f32) {
        let mut y = 0.0;
        let mut out = Vec::new();
        for (_, rows) in &self.groups {
            y += GROUP_H;
            for r in rows {
                out.push((r.id, y));
                y += ROW_H + ROW_GAP;
            }
        }
        (out, y)
    }

    fn spell_at(&self, y: f32) -> Option<u32> {
        let at = y + self.scroll.at;
        self.row_tops()
            .0
            .into_iter()
            .find(|(_, t)| at >= *t && at < *t + ROW_H)
            .map(|(id, _)| id)
    }

    fn row(&self, id: u32) -> Option<&SpellRow> {
        self.groups.iter().flat_map(|(_, r)| r).find(|r| r.id == id)
    }

    /// Scrolls so the chosen spell shows.
    pub fn reveal_chosen(&mut self) {
        let Some(id) = self.chosen else {
            return;
        };
        if let Some((_, top)) = self.row_tops().0.into_iter().find(|(s, _)| *s == id) {
            self.scroll
                .reveal(top - GROUP_H.min(top), ROW_H + GROUP_H.min(top), LIST_H);
        }
    }
}

impl Widget for SpellList {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn is_drag_source(&self, _element: &Element) -> bool {
        self.pressed.is_some()
    }

    fn drag_payload(&mut self, _ui: &mut UiRoot, _me: ElementId) -> Option<DragPayload> {
        let spell_id = self.pressed?;
        Some(DragPayload::new(SpellbookShortcutPayload { spell_id }))
    }

    fn drag_ghost(&mut self, _ui: &mut UiRoot, _me: ElementId) -> Option<DragGhost> {
        let texture = self
            .pressed
            .and_then(|id| self.row(id))
            .and_then(|r| r.icon)?;
        Some(DragGhost {
            texture,
            width: 32,
            height: 32,
        })
    }

    fn set_drag_source_active(
        &mut self,
        _ui: &mut UiRoot,
        _me: ElementId,
        active: bool,
        _payload: Option<&DragPayload>,
    ) {
        self.lifted = if active { self.pressed } else { None };
        if let Some(id) = self.lifted {
            self.chosen = Some(id);
            self.requests.push(ListRequest::Choose(id));
        }
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
            kind::MOUSE_DOWN => {
                self.pressed = self.spell_at(y);
                true
            }
            kind::SCROLL => {
                let (_, total) = self.row_tops();
                self.scroll
                    .wheel(e.data0, 2.0 * (ROW_H + ROW_GAP), total, LIST_H);
                true
            }
            kind::CLICK => {
                if let Some(id) = self.spell_at(y) {
                    self.chosen = Some(id);
                    self.requests.push(ListRequest::Choose(id));
                }
                true
            }
            kind::DOUBLE_CLICK => {
                if let Some(id) = self.spell_at(y) {
                    self.requests.push(ListRequest::Favourite(id));
                }
                true
            }
            kind::RIGHT_CLICK => {
                if let Some(id) = self.spell_at(y) {
                    self.chosen = Some(id);
                    self.requests.push(ListRequest::Choose(id));
                    self.requests.push(ListRequest::Examine(id));
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let (w, h) = ui
            .element(me)
            .map_or((LIST_W, LIST_H), |e| (e.width, e.height));
        let (_, total) = self.row_tops();
        self.scroll.hold(total, h);
        let right = w - 10.0;
        rc.push_clip(0.0, 0.0, w, h);
        let mut y = -self.scroll.at;
        if self.groups.is_empty()
            && let Some(f) = &self.fonts.body
        {
            text::draw(
                rc,
                f,
                "No spells match.",
                4.0,
                12.0,
                TextStyle::plain(p.muted),
            );
        }
        for (school, rows) in &self.groups {
            if y + GROUP_H > 0.0 && y < h {
                let color = school_color(*school);
                let title = school_name(*school).to_uppercase();
                if let Some(f) = &self.fonts.label {
                    let ty = y + 10.0 + ((GROUP_H - 12.0 - f.line_height) * 0.5).round();
                    text::draw(
                        rc,
                        f,
                        &title,
                        0.0,
                        ty,
                        TextStyle::plain(color).tracked(kit::LABEL_TRACKING),
                    );
                    let tw = text::measure(f, &title, kit::LABEL_TRACKING);
                    let n = rows.len().to_string();
                    let nw = self
                        .fonts
                        .small
                        .as_ref()
                        .map_or(0.0, |s| text::measure(s, &n, 0.0));
                    rc.fill(
                        tw + 8.0,
                        y + 10.0 + ((GROUP_H - 12.0) * 0.5).round(),
                        (right - nw - 8.0 - tw - 8.0).max(0.0),
                        1.0,
                        kit::bronze(0.25),
                    );
                    if let Some(s) = &self.fonts.small {
                        text::draw(rc, s, &n, right - nw, ty, TextStyle::plain(tone::soft(1.0)));
                    }
                }
            }
            y += GROUP_H;
            for r in rows {
                if y + ROW_H > 0.0 && y < h {
                    self.draw_row(rc, r, y, right);
                }
                y += ROW_H + ROW_GAP;
            }
        }
        rc.pop_clip();
        kit::scrollbar(rc, w - 5.0, 0.0, h, total, self.scroll.at);
    }
}

impl SpellList {
    fn draw_row(&self, rc: &mut RenderContext<'_>, r: &SpellRow, y: f32, right: f32) {
        let p = Horizon::current().palette;
        let on = self.chosen == Some(r.id);
        let lit = hot(self.hover, (0.0, y, right, ROW_H));
        let radius = CornerRadii::all(4.0);
        let fade = if self.lifted.is_some_and(|l| l != r.id) {
            0.5
        } else {
            1.0
        };
        if on {
            rc.fill_rounded_rect(0.0, y, right, ROW_H, radius, kit::gold(0.14));
            rc.stroke_rounded_rect(0.0, y, right, ROW_H, radius, kit::gold(1.0), 1.0);
        } else if lit {
            rc.fill_rounded_rect(0.0, y, right, ROW_H, radius, hex(0xFF_FFFF, 0.04));
        }
        let color = school_color(r.school);
        rc.fill_vertical_gradient(
            4.0,
            y + 4.0,
            30.0,
            30.0,
            radius,
            Color {
                a: 0.2 * fade,
                ..color
            },
            hex(0, 0.5 * fade),
        );
        if let Some(icon) = r.icon {
            rc.sprite(
                icon,
                5.0,
                y + 5.0,
                28.0,
                28.0,
                (0.0, 0.0, 1.0, 1.0),
                Color {
                    a: fade,
                    ..Color::WHITE
                },
            );
        }
        let mut tag_x = right - 8.0;
        if let (Some(bar), Some(f)) = (&r.bar, &self.fonts.label) {
            let tag = kit::fit(f, &bar.to_uppercase(), 90.0);
            let tw = text::measure(f, &tag, 0.4);
            tag_x -= tw;
            let ty = y + ((ROW_H - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                &tag,
                tag_x,
                ty,
                TextStyle::plain(Color {
                    a: fade,
                    ..kit::gold(1.0)
                })
                .tracked(0.4),
            );
            tag_x -= 8.0;
        }
        let nx = 4.0 + 30.0 + 10.0;
        let room = tag_x - nx;
        if let Some(f) = &self.fonts.name {
            let name = kit::fit(f, &r.name, room);
            text::draw(
                rc,
                f,
                &name,
                nx,
                y + 4.0,
                TextStyle::plain(Color { a: fade, ..p.text }),
            );
        }
        if let Some(f) = &self.fonts.small {
            text::draw(
                rc,
                f,
                &r.sub(),
                nx,
                y + 21.0,
                TextStyle::plain(Color {
                    a: fade,
                    ..tone::soft(1.0)
                }),
            );
        }
    }
}

// ── The components table ────────────────────────────────────────────────

const TABLE_ROW: f32 = 36.0;

/// What the table asks of the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableRequest {
    /// Select the component's pack (its object).
    Select(u32),
    /// Edit the restock amount of the component on the row whose top (in
    /// the table, scrolled) is given.
    EditRestock(u32, f32),
}

/// The components table, scrolled by itself.
#[derive(Debug)]
pub struct ComponentTable {
    fonts: BookFonts,
    /// The rows shown.
    pub rows: Vec<ComponentRow>,
    /// The component chosen.
    pub chosen: Option<u32>,
    /// The component whose restock amount is being edited.
    pub editing: Option<u32>,
    /// What it asked for since the window last looked.
    pub requests: Vec<TableRequest>,
    scroll: Scroll,
    hover: Option<(f32, f32)>,
}

impl ComponentTable {
    /// An empty table.
    #[must_use]
    pub fn new(fonts: BookFonts) -> Self {
        Self {
            fonts,
            rows: Vec::new(),
            chosen: None,
            editing: None,
            requests: Vec::new(),
            scroll: Scroll::default(),
            hover: None,
        }
    }

    /// The restock box of a row whose top is `y`, in the table.
    #[must_use]
    pub fn restock_rect(y: f32) -> Rect {
        let cols = table_columns();
        (
            cols[4] - 12.0 - 64.0,
            y + (TABLE_ROW - 24.0) * 0.5,
            64.0,
            24.0,
        )
    }

    /// Where the table is scrolled to.
    #[must_use]
    pub fn scrolled(&self) -> f32 {
        self.scroll.at
    }
}

impl Widget for ComponentTable {
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
                let total = self.rows.len() as f32 * TABLE_ROW;
                self.scroll.wheel(e.data0, 2.0 * TABLE_ROW, total, TABLE_H);
                self.editing = None;
                true
            }
            kind::CLICK => {
                let at = y + self.scroll.at;
                let i = (at / TABLE_ROW).floor();
                if i >= 0.0
                    && let Some(r) = self.rows.get(i as usize)
                {
                    let top = i * TABLE_ROW - self.scroll.at;
                    if inside(Self::restock_rect(top), x, y) {
                        self.editing = Some(r.component);
                        self.requests
                            .push(TableRequest::EditRestock(r.component, top));
                    } else {
                        self.chosen = Some(r.component);
                        if r.object != 0 {
                            self.requests.push(TableRequest::Select(r.object));
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let (w, h) = ui
            .element(me)
            .map_or((TABLE_W, TABLE_H), |e| (e.width, e.height));
        let total = self.rows.len() as f32 * TABLE_ROW;
        self.scroll.hold(total, h);
        let cols = table_columns();
        rc.push_clip(0.0, 0.0, w, h);
        if self.rows.is_empty()
            && let Some(f) = &self.fonts.body
        {
            text::draw(
                rc,
                f,
                "You carry no spell components.",
                10.0,
                12.0,
                TextStyle::plain(p.muted),
            );
        }
        for (i, r) in self.rows.iter().enumerate() {
            let y = i as f32 * TABLE_ROW - self.scroll.at;
            if y + TABLE_ROW < 0.0 || y > h {
                continue;
            }
            let on = self.chosen == Some(r.component);
            if on {
                rc.fill_rounded_rect(
                    0.0,
                    y,
                    w - 8.0,
                    TABLE_ROW,
                    CornerRadii::all(4.0),
                    kit::gold(0.12),
                );
            } else if hot(self.hover, (0.0, y, w, TABLE_ROW)) {
                rc.fill(0.0, y, w - 8.0, TABLE_ROW, hex(0xFF_FFFF, 0.03));
            }
            rc.fill(0.0, y + TABLE_ROW - 1.0, w - 8.0, 1.0, kit::bronze(0.1));
            let color = category_color(r.category);
            let tile = (cols[0], y + (TABLE_ROW - 24.0) * 0.5);
            rc.fill_vertical_gradient(
                tile.0,
                tile.1,
                24.0,
                24.0,
                CornerRadii::all(4.0),
                Color { a: 0.2, ..color },
                hex(0, 0.5),
            );
            if let Some(icon) = r.icon {
                rc.sprite(
                    icon,
                    tile.0,
                    tile.1,
                    24.0,
                    24.0,
                    (0.0, 0.0, 1.0, 1.0),
                    Color::WHITE,
                );
            }
            let have_color = if r.carried <= 0 {
                kit::HARM
            } else if r.short() {
                kit::gold(1.0)
            } else {
                p.text
            };
            if let Some(f) = &self.fonts.name {
                let ty = y + ((TABLE_ROW - f.line_height) * 0.5).round();
                let name = kit::fit(f, &r.name, cols[1] - 12.0 - (cols[0] + 32.0));
                text::draw(rc, f, &name, cols[0] + 32.0, ty, TextStyle::plain(p.text));
                let have = r.carried.max(0).to_string();
                let hw = text::measure(f, &have, 0.0);
                text::draw(
                    rc,
                    f,
                    &have,
                    cols[3] - 12.0 - hw,
                    ty,
                    TextStyle::plain(have_color),
                );
            }
            if let Some(f) = &self.fonts.body {
                let ty = y + ((TABLE_ROW - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    category_one(r.category),
                    cols[1],
                    ty,
                    TextStyle::plain(tone::soft(1.0)),
                );
            }
            // The restock box (the window's field covers it while editing).
            let b = Self::restock_rect(y);
            let radius = CornerRadii::all(4.0);
            rc.fill_rounded_rect(b.0, b.1, b.2, b.3, radius, hex(0, 0.35));
            let editing = self.editing == Some(r.component);
            let edge = if editing {
                kit::gold(0.9)
            } else if hot(self.hover, b) {
                kit::bronze(0.7)
            } else {
                kit::bronze(0.45)
            };
            rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, radius, edge, 1.0);
            if !editing && let Some(f) = &self.fonts.body {
                let words = if r.restock == 0 {
                    "\u{2014}".to_owned()
                } else {
                    r.restock.to_string()
                };
                let tw = text::measure(f, &words, 0.0);
                let ty = b.1 + ((b.3 - f.line_height) * 0.5).round();
                let color = if r.restock == 0 { p.muted } else { p.text };
                text::draw(
                    rc,
                    f,
                    &words,
                    b.0 + b.2 - 6.0 - tw,
                    ty,
                    TextStyle::plain(color),
                );
            }
            // The stock bar.
            let bar_color = if r.carried <= 0 {
                kit::HARM
            } else if r.short() {
                kit::gold(1.0)
            } else {
                kit::GOOD
            };
            let bw = (w - 8.0 - 10.0 - cols[4]).max(20.0);
            if r.restock > 0 {
                kit::meter(
                    rc,
                    (cols[4], y + TABLE_ROW * 0.5 - 2.0, bw, 4.0),
                    r.stock(),
                    bar_color,
                );
            } else if let Some(f) = &self.fonts.small {
                let ty = y + ((TABLE_ROW - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    "No restock set",
                    cols[4],
                    ty,
                    TextStyle::plain(p.muted),
                );
            }
        }
        rc.pop_clip();
        kit::scrollbar(rc, w - 5.0, 0.0, h, total, self.scroll.at);
    }
}

/// The spell list's groups for the frame's spells and filter.
#[must_use]
pub fn groups_for(frame: &BookFrame) -> Vec<(MagicSchool, Vec<SpellRow>)> {
    shown(&frame.spells, &frame.filter)
        .into_iter()
        .map(|(s, rows)| (s, rows.into_iter().cloned().collect()))
        .collect()
}

/// Whether the sort was [`SpellSort::School`] (the game's order).
#[must_use]
pub fn in_game_order(sort: SpellSort) -> bool {
    sort == SpellSort::School
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spell(id: u32, school: MagicSchool) -> SpellRow {
        SpellRow {
            id,
            name: format!("Spell {id}"),
            school,
            level: 1,
            mana: 10,
            family: 0,
            sort_key: id as i32,
            icon: None,
            bar: None,
        }
    }

    #[test]
    fn a_click_in_the_list_chooses_the_spell_under_it() {
        let mut list = SpellList::new(BookFonts::default());
        list.groups = vec![(
            MagicSchool::WAR_MAGIC,
            vec![
                spell(1, MagicSchool::WAR_MAGIC),
                spell(2, MagicSchool::WAR_MAGIC),
            ],
        )];
        assert_eq!(list.spell_at(GROUP_H + 5.0), Some(1));
        assert_eq!(list.spell_at(GROUP_H + ROW_H + ROW_GAP + 5.0), Some(2));
        assert_eq!(list.spell_at(5.0), None);
    }

    #[test]
    fn forgetting_asks_first() {
        let mut f = BookFrame::new(BookFonts::default());
        f.details = Some(SpellDetails {
            id: 9,
            ..SpellDetails::default()
        });
        let (_, _, forget) = BookFrame::foot_rects();
        f.click(forget.0 + 4.0, forget.1 + 4.0);
        assert!(f.asking_forget);
        assert!(f.requests.is_empty());
        let (_, yes) = BookFrame::forget_buttons();
        f.click(yes.0 + 4.0, yes.1 + 4.0);
        assert_eq!(f.requests, vec![BookRequest::Forget(9)]);
        assert!(!f.asking_forget);
    }

    #[test]
    fn cast_waits_for_magic() {
        let mut f = BookFrame::new(BookFonts::default());
        f.details = Some(SpellDetails {
            id: 9,
            ..SpellDetails::default()
        });
        let (cast, _, _) = BookFrame::foot_rects();
        f.click(cast.0 + 4.0, cast.1 + 4.0);
        assert!(f.requests.is_empty());
        f.can_cast = true;
        f.click(cast.0 + 4.0, cast.1 + 4.0);
        assert_eq!(f.requests, vec![BookRequest::Cast(9)]);
    }

    #[test]
    fn the_table_columns_run_left_to_right() {
        let c = table_columns();
        assert!(c.windows(2).all(|w| w[0] < w[1]));
        assert!(c[4] < TABLE_W);
    }
}
