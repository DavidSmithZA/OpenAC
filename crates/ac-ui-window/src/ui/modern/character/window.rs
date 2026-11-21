//! The Character window's frame and its two scrolled lists. The frame
//! draws the header with its three tabs, the identity band (the level
//! plate that grows with the number, name, title, heritage and PK status,
//! the experience line and bar, chips for unassigned experience, skill
//! credits, luminance and vitae), the attributes and vitals with their
//! raise buttons, the chosen row's details (how it adds up, what it is,
//! +1, +10 and Max, or Train), the Titles page's side card and the
//! Information page's cards. The skill list and the title grid are
//! children that scroll by themselves.

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::character::{CharacterSheet, CharacterSkill, format_xp};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::{
    self, Amount, Line, Row, SkillGroup, Tone, affordable, breakdown, cost, short_cost, signed,
    skill_groups,
};
use crate::ui::modern::hud::lines::{circle, stroke};
use crate::ui::modern::kit::{self, ButtonLook, Rect, Scroll, Tab, hot, inside};

/// The window's size.
pub const WIDTH: f32 = 1100.0;
pub const HEIGHT: f32 = 720.0;
const HEADER: f32 = 52.0;
const BAND_H: f32 = 104.0;
const BODY_TOP: f32 = HEADER + BAND_H + 16.0;
const BOTTOM: f32 = HEIGHT - 16.0;
const ATTR_X: f32 = 24.0;
const ATTR_W: f32 = 300.0;
const GROUP_TITLE: f32 = 26.0;
const ATTR_ROW: f32 = 42.0;
/// The skill list.
pub const SKILLS_X: f32 = 344.0;
pub const SKILLS_W: f32 = 410.0;
pub const SKILLS_TOP: f32 = BODY_TOP + 34.0;
pub const SKILLS_H: f32 = BOTTOM - SKILLS_TOP;
const ASIDE_X: f32 = 774.0;
const ASIDE_W: f32 = 302.0;
/// The title grid.
pub const TITLES_X: f32 = 24.0;
pub const TITLES_W: f32 = WIDTH - 24.0 - 300.0 - 20.0 - 24.0;
pub const TITLES_TOP: f32 = BODY_TOP + 38.0;
pub const TITLES_H: f32 = BOTTOM - TITLES_TOP;
const ASIDE_HEAD: f32 = 96.0;
const ASIDE_FOOT: f32 = 92.0;

/// The window's glyph: a figure.
const FIGURE: &[&[(f32, f32)]] = &[&[
    (4.0, 21.0),
    (4.5, 18.0),
    (7.0, 15.5),
    (12.0, 14.5),
    (17.0, 15.5),
    (19.5, 18.0),
    (20.0, 21.0),
]];

/// The pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterPage {
    /// Attributes, vitals and skills.
    #[default]
    Stats,
    /// Titles.
    Titles,
    /// Information.
    Info,
}

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct CharacterFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// Tabs, buttons, strong words.
    pub strong: Option<Arc<UiFont>>,
    /// Quiet words.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals.
    pub label: Option<Arc<UiFont>>,
    /// Body words.
    pub body: Option<Arc<UiFont>>,
    /// A row's name.
    pub name: Option<Arc<UiFont>>,
    /// A row's value.
    pub value: Option<Arc<UiFont>>,
    /// The level's figure.
    pub level: Option<Arc<UiFont>>,
    /// The character's name.
    pub big: Option<Arc<UiFont>>,
    /// The chosen row's value.
    pub huge: Option<Arc<UiFont>>,
    /// The chosen row's name.
    pub heading: Option<Arc<UiFont>>,
}

/// One card on the Information page: its title and lines of words.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InfoCard {
    /// The title, in capitals.
    pub title: String,
    /// Rows of a label and a value.
    pub rows: Vec<(String, String)>,
    /// Paragraphs of the game's own words.
    pub prose: Vec<String>,
}

/// What the window shows besides the sheet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharacterExtras {
    /// What vitae leaves (none without vitae).
    pub vitae: Option<f32>,
    /// The experience still to earn for 1% of vitae back.
    pub vitae_experience: Option<i64>,
    /// The spells on the chosen row: name and what it adds.
    pub spell_lines: Vec<(String, i32)>,
    /// Every earned title (id, words), in order.
    pub titles: Vec<(u32, String)>,
    /// The title shown (its id).
    pub display_title: Option<u32>,
    /// The Information page's cards.
    pub info: Vec<InfoCard>,
}

/// What the frame asks of the window.
#[derive(Debug, Clone)]
pub enum CharacterRequest {
    /// Close it.
    Close,
    /// Show a page.
    Page(CharacterPage),
    /// A row was chosen.
    Choose(Row),
    /// Send a raise (or a training, which is asked first).
    Send(PanelOutput),
    /// Display this title.
    SetTitle(u32),
}

/// The Character window's frame.
#[derive(Debug)]
pub struct CharacterFrame {
    theme: Horizon,
    fonts: CharacterFonts,
    /// The page shown.
    pub page: CharacterPage,
    /// The sheet.
    pub sheet: CharacterSheet,
    /// What else it shows.
    pub extras: CharacterExtras,
    /// The row chosen.
    pub chosen: Option<Row>,
    /// The title picked on the Titles page.
    pub title_pick: Option<u32>,
    /// A raise was sent and the sheet has not changed since.
    pub waiting: bool,
    /// The search field of the page shown has the keyboard, and what is
    /// typed in it.
    pub searching: bool,
    pub search: String,
    /// What it asked for since the window last looked.
    pub requests: Vec<CharacterRequest>,
    aside_scroll: Scroll,
    info_scroll: Scroll,
    hover: Option<(f32, f32)>,
}

impl CharacterFrame {
    /// An empty window in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: CharacterFonts) -> Self {
        Self {
            theme,
            fonts,
            page: CharacterPage::Stats,
            sheet: CharacterSheet::default(),
            extras: CharacterExtras::default(),
            chosen: None,
            title_pick: None,
            waiting: false,
            searching: false,
            search: String::new(),
            requests: Vec::new(),
            aside_scroll: Scroll::default(),
            info_scroll: Scroll::default(),
            hover: None,
        }
    }

    /// The skills' search box (its field goes over it).
    #[must_use]
    pub fn skill_search_rect() -> Rect {
        (SKILLS_X + SKILLS_W - 6.0 - 170.0, BODY_TOP, 170.0, 26.0)
    }

    /// The titles' search box.
    #[must_use]
    pub fn title_search_rect() -> Rect {
        (TITLES_X + TITLES_W - 220.0, BODY_TOP, 220.0, 28.0)
    }

    /// Chooses `row`; its details start at the top.
    pub fn choose(&mut self, row: Row) {
        if self.chosen != Some(row) {
            self.aside_scroll = Scroll::default();
        }
        self.chosen = Some(row);
    }

    fn tabs(&self) -> [(Tab<'static>, Rect); 3] {
        let f = self.fonts.strong.as_ref();
        let x0 = self
            .fonts
            .title
            .as_ref()
            .map_or(140.0, |t| 46.0 + text::measure(t, "Character", 0.0) + 16.0);
        let words = ["Attributes and skills", "Titles", "Information"];
        let pages = [
            CharacterPage::Stats,
            CharacterPage::Titles,
            CharacterPage::Info,
        ];
        let y = (HEADER - 34.0) * 0.5;
        let mut x = x0;
        let mut out: Vec<(Tab<'static>, Rect)> = Vec::new();
        for (w, p) in words.into_iter().zip(pages) {
            let tab = Tab {
                words: w,
                count: None,
                on: self.page == p,
                red: false,
            };
            let tw = kit::tab_width(&tab, f, None);
            out.push((tab, (x, y, tw, 34.0)));
            x += tw + 2.0;
        }
        let mut it = out.into_iter();
        let (a, b, c) = (it.next(), it.next(), it.next());
        match (a, b, c) {
            (Some(a), Some(b), Some(c)) => [a, b, c],
            _ => unreachable!("three tabs"),
        }
    }

    fn plate_width(&self) -> f32 {
        let words = self.level_words();
        self.fonts
            .level
            .as_ref()
            .map_or(72.0, |f| (text::measure(f, &words, 0.0) + 28.0).max(72.0))
    }

    fn level_words(&self) -> String {
        self.sheet.level.map_or_else(
            || "???".to_owned(),
            |l| format_xp(u64::try_from(l).unwrap_or(0)),
        )
    }

    fn chips(&self) -> Vec<(&'static str, String, Color, Option<String>, Rect)> {
        let s = &self.sheet;
        let gold = kit::gold(1.0);
        let text_c = self.theme.palette.text;
        let mut list: Vec<(&'static str, String, Color, Option<String>)> = vec![
            (
                "Unassigned",
                format!("{} XP", format_xp(s.unassigned_xp)),
                gold,
                Some("Experience you can spend on attributes, vitals and skills".to_owned()),
            ),
            (
                "Skill credits",
                s.skill_credits.to_string(),
                text_c,
                Some("Spend credits to train new skills".to_owned()),
            ),
        ];
        if s.luminance_visible() {
            list.push((
                "Luminance",
                format!(
                    "{} of {}",
                    format_xp(s.available_luminance),
                    format_xp(s.maximum_luminance)
                ),
                text_c,
                Some("Spent with luminance trainers".to_owned()),
            ));
        }
        if let Some(v) = self.extras.vitae.filter(|v| *v < 1.0) {
            let tip = self.extras.vitae_experience.map(|x| {
                format!(
                    "{} XP more brings back the next 1%",
                    format_xp(u64::try_from(x.max(0)).unwrap_or(0))
                )
            });
            list.push((
                "Vitae",
                format!(
                    "{}%",
                    crate::ui::modern::hud::effects::vitae_penalty_percent(v)
                ),
                kit::HARM,
                tip,
            ));
        }
        let mut x = self.band_right_x();
        let y = HEADER + 66.0;
        list.into_iter()
            .map(|(k, v, c, tip)| {
                let w = self
                    .fonts
                    .small
                    .as_ref()
                    .map_or(100.0, |f| text::measure(f, k, 0.0))
                    + 6.0
                    + self
                        .fonts
                        .strong
                        .as_ref()
                        .map_or(60.0, |f| text::measure(f, &v, 0.0))
                    + 20.0;
                let r = (x, y, w, 24.0);
                x += w + 8.0;
                (k, v, c, tip, r)
            })
            .collect()
    }

    fn band_right_x(&self) -> f32 {
        24.0 + self.plate_width() + 20.0 + 250.0 + 20.0
    }

    fn attr_rows() -> Vec<(Row, Rect)> {
        let mut out = Vec::new();
        let mut y = BODY_TOP + GROUP_TITLE;
        for i in 0..6 {
            out.push((Row::Attribute(i), (ATTR_X, y, ATTR_W, ATTR_ROW)));
            y += ATTR_ROW + 2.0;
        }
        y += 14.0 + GROUP_TITLE;
        for i in 0..3 {
            out.push((Row::Vital(i), (ATTR_X, y, ATTR_W, ATTR_ROW)));
            y += ATTR_ROW + 2.0;
        }
        out
    }

    fn plus_rect(r: Rect) -> Rect {
        (r.0 + r.2 - 6.0 - 26.0, r.1 + (r.3 - 26.0) * 0.5, 26.0, 26.0)
    }

    fn aside() -> Rect {
        (ASIDE_X, BODY_TOP, ASIDE_W, BOTTOM - BODY_TOP)
    }

    fn aside_buttons(&self) -> Vec<(Amount, Rect)> {
        let Some(row) = self.chosen else {
            return Vec::new();
        };
        let (x, _, w, _) = Self::aside();
        let y = BOTTOM - 14.0 - 34.0;
        if !model::raised_with_experience(&self.sheet, row) {
            return vec![(Amount::One, (x + 14.0, y, w - 28.0, 34.0))];
        }
        let each = (w - 28.0 - 12.0) / 3.0;
        [Amount::One, Amount::Ten, Amount::Max]
            .into_iter()
            .enumerate()
            .map(|(i, a)| (a, (x + 14.0 + i as f32 * (each + 6.0), y, each, 34.0)))
            .collect()
    }

    fn title_aside() -> Rect {
        (WIDTH - 24.0 - 300.0, BODY_TOP, 300.0, 210.0)
    }

    fn title_button() -> Rect {
        let (x, y, w, h) = Self::title_aside();
        (x + 16.0, y + h - 16.0 - 34.0, w - 32.0, 34.0)
    }

    fn info_cards_layout(&self) -> (Vec<(usize, Rect)>, f32) {
        let cols = 3.0;
        let gap = 12.0;
        let w = (WIDTH - 48.0 - gap * (cols - 1.0)) / cols;
        let mut heights = [0.0_f32; 3];
        let mut out = Vec::new();
        for (i, card) in self.extras.info.iter().enumerate() {
            let col = heights
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.total_cmp(b.1))
                .map_or(0, |(c, _)| c);
            let h = self.card_height(card, w);
            let x = 24.0 + col as f32 * (w + gap);
            out.push((i, (x, heights[col], w, h)));
            heights[col] += h + gap;
        }
        let total = heights.iter().copied().fold(0.0, f32::max);
        (out, total)
    }

    fn card_height(&self, card: &InfoCard, w: f32) -> f32 {
        let prose_lines: usize = self.fonts.body.as_ref().map_or(0, |f| {
            card.prose
                .iter()
                .map(|p| kit::wrap(f, p, w - 28.0).len() + 1)
                .sum()
        });
        14.0 + 20.0 + card.rows.len() as f32 * 20.0 + prose_lines as f32 * 18.0 + 10.0
    }

    fn row_words(&self, row: Row) -> (String, String, i32, Color) {
        let s = &self.sheet;
        let base = model::base(s, row);
        let spelled = model::spells(s, row);
        let value = model::value(s, row);
        let color = match spelled.signum() {
            1 => kit::GOOD,
            -1 => kit::HARM,
            _ => self.theme.palette.text,
        };
        let sub = if spelled == 0 {
            format!("base {base}")
        } else {
            format!("base {base} \u{b7} spells {}", signed(spelled))
        };
        (model::name(s, row), sub, value, color)
    }
}

impl Widget for CharacterFrame {
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
                match self.page {
                    CharacterPage::Info => {
                        let (_, total) = self.info_cards_layout();
                        self.info_scroll
                            .wheel(e.data0, 60.0, total, BOTTOM - BODY_TOP);
                    }
                    CharacterPage::Stats => {
                        let total = self.aside_total();
                        self.aside_scroll.wheel(
                            e.data0,
                            48.0,
                            total,
                            BOTTOM - BODY_TOP - ASIDE_HEAD - ASIDE_FOOT,
                        );
                    }
                    CharacterPage::Titles => {}
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
        if at.1 < HEADER + BAND_H {
            return self
                .chips()
                .into_iter()
                .find(|c| inside(c.4, at.0, at.1))
                .and_then(|c| c.3);
        }
        if self.page != CharacterPage::Stats {
            return None;
        }
        for (row, r) in Self::attr_rows() {
            if inside(Self::plus_rect(r), at.0, at.1) {
                let (_, xp) = cost(&self.sheet, row, Amount::One);
                return Some(format!(
                    "Raise {} by one for {} XP",
                    model::name(&self.sheet, row),
                    format_xp(xp)
                ));
            }
        }
        for (a, r) in self.aside_buttons() {
            if inside(r, at.0, at.1) {
                return Some(match a {
                    Amount::One => "Raise by one".to_owned(),
                    Amount::Ten => "Raise by ten".to_owned(),
                    Amount::Max => "As many points as your unassigned XP buys".to_owned(),
                });
            }
        }
        None
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        self.draw_frame(rc);
        self.draw_band(rc);
        match self.page {
            CharacterPage::Stats => {
                self.draw_attributes(rc);
                self.draw_skills_heading(rc);
                self.draw_aside(rc);
            }
            CharacterPage::Titles => self.draw_titles(rc),
            CharacterPage::Info => self.draw_info(rc),
        }
    }
}

impl CharacterFrame {
    fn click(&mut self, x: f32, y: f32) {
        if inside(kit::close_rect(WIDTH, HEADER), x, y) {
            self.requests.push(CharacterRequest::Close);
            return;
        }
        let tabs = self.tabs();
        let pages = [
            CharacterPage::Stats,
            CharacterPage::Titles,
            CharacterPage::Info,
        ];
        if let Some(i) = tabs.iter().position(|(_, r)| inside(*r, x, y)) {
            self.page = pages[i];
            self.requests.push(CharacterRequest::Page(pages[i]));
            return;
        }
        match self.page {
            CharacterPage::Stats => {
                for (row, r) in Self::attr_rows() {
                    if inside(Self::plus_rect(r), x, y) {
                        self.choose(row);
                        self.requests.push(CharacterRequest::Choose(row));
                        self.send_raise(row, Amount::One);
                        return;
                    }
                    if inside(r, x, y) {
                        self.choose(row);
                        self.requests.push(CharacterRequest::Choose(row));
                        return;
                    }
                }
                if let Some((a, _)) = self
                    .aside_buttons()
                    .into_iter()
                    .find(|(_, r)| inside(*r, x, y))
                    && let Some(row) = self.chosen
                {
                    self.send_raise(row, a);
                }
            }
            CharacterPage::Titles => {
                if inside(Self::title_button(), x, y)
                    && let Some(id) = self.title_pick
                    && Some(id) != self.extras.display_title
                {
                    self.requests.push(CharacterRequest::SetTitle(id));
                }
            }
            CharacterPage::Info => {}
        }
    }

    /// Asks for `row` to be raised by `amount` (an untrained skill is
    /// trained), unless a raise is still on its way.
    pub fn send_raise(&mut self, row: Row, amount: Amount) {
        if self.waiting {
            return;
        }
        let out = match row {
            Row::Skill(id) if !model::raised_with_experience(&self.sheet, row) => {
                model::train(&self.sheet, id)
            }
            _ => model::raise(&self.sheet, row, amount),
        };
        if let Some(out) = out {
            let training = matches!(out, PanelOutput::TrainSkill { .. });
            // A training is asked first; the sheet answers either way.
            self.waiting = !training;
            self.requests.push(CharacterRequest::Send(out));
        }
    }

    fn draw_frame(&self, rc: &mut RenderContext<'_>) {
        let theme = &self.theme;
        kit::window_back(rc, theme, WIDTH, HEIGHT, HEADER, true);
        let x = kit::title(
            rc,
            theme,
            self.fonts.title.as_ref(),
            &[],
            "Character",
            HEADER,
        );
        let _ = x;
        circle(
            rc,
            20.0,
            (HEADER - 18.0) * 0.5,
            18.0,
            (12.0, 8.0),
            4.0,
            kit::gold(1.0),
            2.0,
        );
        stroke(
            rc,
            20.0,
            (HEADER - 18.0) * 0.5,
            18.0,
            FIGURE,
            kit::gold(1.0),
            2.0,
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
        let c = kit::close_rect(WIDTH, HEADER);
        kit::close(rc, theme, c, hot(self.hover, c));
    }

    #[allow(clippy::too_many_lines)]
    fn draw_band(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let s = &self.sheet;
        rc.fill(0.0, HEADER + BAND_H - 1.0, WIDTH, 1.0, kit::bronze(0.25));
        // The level plate.
        let pw = self.plate_width();
        let (px, py) = (24.0, HEADER + (BAND_H - 64.0) * 0.5);
        let r = CornerRadii::all(6.0);
        rc.fill_vertical_gradient(px, py, pw, 64.0, r, kit::gold(0.18), hex(0, 0.3));
        rc.stroke_rounded_rect(px, py, pw, 64.0, r, kit::bronze(0.8), 1.0);
        if let Some(f) = &self.fonts.label {
            let w = text::measure(f, "LEVEL", kit::LABEL_TRACKING);
            text::draw(
                rc,
                f,
                "LEVEL",
                px + (pw - w) * 0.5,
                py + 10.0,
                TextStyle::plain(kit::SOFT).tracked(kit::LABEL_TRACKING),
            );
        }
        if let Some(f) = &self.fonts.level {
            let words = self.level_words();
            let w = text::measure(f, &words, 0.0);
            text::draw(
                rc,
                f,
                &words,
                px + (pw - w) * 0.5,
                py + 24.0,
                TextStyle::plain(p.text),
            );
        }
        // Name, title, heritage.
        let nx = px + pw + 20.0;
        let titled = s.title.as_deref().is_some_and(|t| !t.trim().is_empty());
        let mut ny = HEADER + if titled { 22.0 } else { 31.0 };
        if let Some(f) = &self.fonts.big {
            text::draw(
                rc,
                f,
                &kit::fit(f, &s.name, 250.0),
                nx,
                ny,
                TextStyle::plain(p.text),
            );
            ny += 26.0;
        }
        if let Some(f) = &self.fonts.body
            && let Some(title) = s.title.as_deref().filter(|t| !t.trim().is_empty())
        {
            text::draw(
                rc,
                f,
                &kit::fit(f, title, 250.0),
                nx,
                ny,
                TextStyle::plain(kit::WARM),
            );
            ny += 18.0;
        }
        if let Some(f) = &self.fonts.small {
            let mut parts: Vec<String> = Vec::new();
            let who: Vec<&str> = [s.gender.as_deref(), s.heritage.as_deref()]
                .into_iter()
                .flatten()
                .collect();
            if !who.is_empty() {
                parts.push(who.join(" "));
            }
            if let Some(pk) = s.pk_status.as_deref().filter(|t| !t.trim().is_empty()) {
                parts.push(pk.trim().to_owned());
            }
            let words = parts.join(" \u{b7} ");
            text::draw(
                rc,
                f,
                &kit::fit(f, &words, 250.0),
                nx,
                ny,
                TextStyle::plain(kit::SOFT),
            );
        }
        // Experience.
        let bx = self.band_right_x();
        let bw = WIDTH - 24.0 - bx;
        let ty = HEADER + 22.0;
        if let (Some(sm), Some(st)) = (&self.fonts.small, &self.fonts.strong) {
            let total = format!("{} XP", format_xp(s.total_xp));
            text::draw(rc, sm, "Total", bx, ty, TextStyle::plain(kit::SOFT));
            let tx = bx + text::measure(sm, "Total", 0.0) + 5.0;
            text::draw(rc, st, &total, tx, ty - 1.0, TextStyle::plain(p.text));
            let (next, after) = match (&s.xp_to_next_level_text, s.level) {
                (Some(t), _) => (t.clone(), String::new()),
                (None, Some(l)) => (
                    format!("{} XP", format_xp(s.xp_to_next_level)),
                    format!("to level {}", format_xp(u64::try_from(l + 1).unwrap_or(0))),
                ),
                (None, None) => (String::new(), String::new()),
            };
            let aw = text::measure(sm, &after, 0.0);
            text::draw(
                rc,
                sm,
                &after,
                bx + bw - aw,
                ty,
                TextStyle::plain(kit::SOFT),
            );
            let nw = text::measure(st, &next, 0.0);
            text::draw(
                rc,
                st,
                &next,
                bx + bw - aw - 5.0 - nw,
                ty - 1.0,
                TextStyle::plain(p.text),
            );
        }
        kit::meter(rc, (bx, ty + 22.0, bw, 6.0), s.xp_fraction, kit::gold(1.0));
        for (k, v, c, _, r) in self.chips() {
            if r.0 + r.2 > WIDTH - 24.0 {
                break;
            }
            let round = CornerRadii::all(12.0);
            let (edge, fill) = if c == kit::HARM {
                (hex(0xFF_8A8E, 0.6), hex(0xFF_6B70, 0.1))
            } else if c == kit::gold(1.0) {
                (kit::gold(1.0), kit::gold(0.14))
            } else {
                (kit::bronze(0.4), Color::TRANSPARENT)
            };
            rc.fill_rounded_rect(r.0, r.1, r.2, r.3, round, fill);
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, edge, 1.0);
            if let (Some(sm), Some(st)) = (&self.fonts.small, &self.fonts.strong) {
                let y = r.1 + ((r.3 - sm.line_height) * 0.5).round();
                text::draw(rc, sm, k, r.0 + 10.0, y, TextStyle::plain(kit::SOFT));
                let vx = r.0 + 10.0 + text::measure(sm, k, 0.0) + 6.0;
                let vy = r.1 + ((r.3 - st.line_height) * 0.5).round();
                text::draw(rc, st, &v, vx, vy, TextStyle::plain(c));
            }
        }
    }

    fn group_title(&self, rc: &mut RenderContext<'_>, words: &str, x: f32, y: f32, w: f32) {
        if let Some(f) = &self.fonts.label {
            text::draw(
                rc,
                f,
                words,
                x,
                y,
                TextStyle::plain(kit::WARM).tracked(kit::LABEL_TRACKING),
            );
        }
        rc.fill(x, y + 18.0, w, 1.0, kit::bronze(0.25));
    }

    fn draw_attributes(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        self.group_title(rc, "ATTRIBUTES", ATTR_X, BODY_TOP, ATTR_W);
        let vitals_top = BODY_TOP + GROUP_TITLE + 6.0 * (ATTR_ROW + 2.0) + 14.0;
        self.group_title(rc, "VITALS", ATTR_X, vitals_top, ATTR_W);
        for (row, r) in Self::attr_rows() {
            let on = self.chosen == Some(row);
            let radius = CornerRadii::all(4.0);
            if on {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, kit::gold(0.14));
                rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, kit::gold(1.0), 1.0);
            } else if hot(self.hover, r) {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, hex(0xFF_FFFF, 0.04));
            }
            let (name, sub, value, color) = self.row_words(row);
            let plus = Self::plus_rect(r);
            let cost_right = plus.0 - 8.0;
            let (_, xp) = cost(&self.sheet, row, Amount::One);
            if let Some(f) = &self.fonts.small {
                let c = short_cost(xp);
                let w = text::measure(f, &c, 0.0);
                let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
                text::draw(rc, f, &c, cost_right - w, y, TextStyle::plain(kit::SOFT));
            }
            let value_right = cost_right - 50.0 - 8.0;
            if let Some(f) = &self.fonts.value {
                let words = value.to_string();
                let w = text::measure(f, &words, 0.0);
                let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
                text::draw(rc, f, &words, value_right - w, y, TextStyle::plain(color));
            }
            if let Some(f) = &self.fonts.name {
                text::draw(
                    rc,
                    f,
                    &name,
                    r.0 + 10.0,
                    r.1 + 6.0,
                    TextStyle::plain(p.text),
                );
            }
            if let Some(f) = &self.fonts.small {
                text::draw(
                    rc,
                    f,
                    &sub,
                    r.0 + 10.0,
                    r.1 + 23.0,
                    TextStyle::plain(kit::SOFT),
                );
            }
            let can = !self.waiting && affordable(&self.sheet, row, Amount::One);
            plus_button(
                rc,
                plus,
                can,
                hot(self.hover, plus),
                self.fonts.strong.as_ref(),
            );
        }
    }

    fn draw_skills_heading(&self, rc: &mut RenderContext<'_>) {
        if let Some(f) = &self.fonts.label {
            let y = BODY_TOP + ((26.0 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                "SKILLS",
                SKILLS_X,
                y,
                TextStyle::plain(kit::WARM).tracked(kit::LABEL_TRACKING),
            );
        }
        kit::search_box(
            rc,
            &self.theme,
            Self::skill_search_rect(),
            "Find a skill",
            self.search.is_empty() && !self.searching,
            self.searching,
            self.fonts.small.as_ref(),
        );
    }

    fn aside_total(&self) -> f32 {
        let Some(row) = self.chosen else {
            return 0.0;
        };
        let lines = breakdown(
            &self.sheet,
            row,
            &self.extras.spell_lines,
            self.extras.vitae,
        );
        let about = self.fonts.body.as_ref().map_or(0, |f| {
            kit::wrap(f, &model::about(&self.sheet, row), ASIDE_W - 36.0).len()
        });
        12.0 + 18.0 + lines.len() as f32 * 19.0 + 14.0 + about as f32 * 18.0 + 12.0
    }

    #[allow(clippy::too_many_lines)]
    fn draw_aside(&mut self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let (x, y, w, h) = Self::aside();
        let radius = CornerRadii::all(4.0);
        rc.fill_rounded_rect(x, y, w, h, radius, hex(0, 0.28));
        rc.stroke_rounded_rect(x, y, w, h, radius, kit::bronze(0.35), 1.0);
        let Some(row) = self.chosen else {
            if let Some(f) = &self.fonts.body {
                let lines = kit::wrap(
                    f,
                    "Choose an attribute, vital or skill to see how it adds up and raise it.",
                    w - 40.0,
                );
                kit::lines(rc, f, &lines, x + 20.0, y + 24.0, 18.0, p.muted);
            }
            return;
        };
        let s = &self.sheet;
        // The head.
        let name = model::name(s, row);
        let (chip, chip_gold) = match row {
            Row::Attribute(_) => ("ATTRIBUTE", false),
            Row::Vital(_) => ("VITAL", false),
            Row::Skill(id) => s.skill(id).map_or(("SKILL", false), |k| {
                let g = SkillGroup::of(k);
                (g.chip(), g == SkillGroup::Specialized)
            }),
        };
        let mut hx = x + 14.0;
        if let Some(f) = &self.fonts.heading {
            let shown = kit::fit(f, &name, w - 28.0 - 100.0);
            text::draw(rc, f, &shown, hx, y + 14.0, TextStyle::plain(p.text));
            hx += text::measure(f, &shown, 0.0) + 8.0;
        }
        if let Some(f) = &self.fonts.label {
            let cw = text::measure(f, chip, 0.6) + 16.0;
            let edge = if chip_gold {
                kit::gold(1.0)
            } else {
                kit::bronze(0.4)
            };
            let ink = if chip_gold { kit::gold(1.0) } else { kit::SOFT };
            rc.stroke_rounded_rect(hx, y + 15.0, cw, 18.0, CornerRadii::all(9.0), edge, 1.0);
            text::draw(
                rc,
                f,
                chip,
                hx + 8.0,
                y + 15.0 + ((18.0 - f.line_height) * 0.5).round(),
                TextStyle::plain(ink).tracked(0.6),
            );
        }
        let value = model::value(s, row);
        let spelled = model::spells(s, row);
        let lost = model::vitae(s, row);
        let vcolor = match spelled.signum() {
            1 => kit::GOOD,
            -1 => kit::HARM,
            _ => p.text,
        };
        let trained = model::raised_with_experience(s, row);
        let mut vx = x + 14.0;
        if let Some(f) = &self.fonts.huge {
            let words = value.to_string();
            text::draw(rc, f, &words, vx, y + 40.0, TextStyle::plain(vcolor));
            vx += text::measure(f, &words, 0.0) + 8.0;
        }
        if let Some(f) = &self.fonts.small {
            let note = if !trained {
                "untrained".to_owned()
            } else if lost < 0 {
                format!("{} before vitae", value - lost)
            } else if spelled != 0 {
                format!("{} without spells", model::base(s, row))
            } else if let Row::Vital(i) = row {
                format!("{} now", s.vital_current.get(i).copied().unwrap_or(0))
            } else {
                String::new()
            };
            text::draw(rc, f, &note, vx, y + 58.0, TextStyle::plain(kit::SOFT));
        }
        rc.fill(x, y + ASIDE_HEAD - 1.0, w, 1.0, kit::bronze(0.2));
        // The body.
        let body_top = y + ASIDE_HEAD;
        let room = h - ASIDE_HEAD - ASIDE_FOOT;
        let total = self.aside_total();
        self.aside_scroll.hold(total, room);
        rc.push_clip(x, body_top, w, room);
        let left = x + 14.0;
        let right = x + w - 14.0 - 8.0;
        let mut by = body_top + 12.0 - self.aside_scroll.at;
        if let Some(f) = &self.fonts.label {
            let words = if trained {
                "HOW IT ADDS UP"
            } else {
                "IF YOU TRAIN IT"
            };
            text::draw(
                rc,
                f,
                words,
                left,
                by,
                TextStyle::plain(kit::SOFT).tracked(kit::LABEL_TRACKING),
            );
        }
        by += 18.0;
        let lines: Vec<Line> = breakdown(s, row, &self.extras.spell_lines, self.extras.vitae);
        for (k, v, tone) in &lines {
            let (kc, vc) = match tone {
                Tone::Plain => (p.text, p.text),
                Tone::Good => (p.text, kit::GOOD),
                Tone::Bad => (p.text, kit::HARM),
                Tone::Quiet => (kit::SOFT, kit::SOFT),
            };
            if let (Some(b), Some(st)) = (&self.fonts.body, &self.fonts.strong) {
                let vw = text::measure(st, v, 0.0);
                text::draw(rc, st, v, right - vw, by, TextStyle::plain(vc));
                let kw = right - vw - 10.0 - left;
                text::draw(rc, b, &kit::fit(b, k, kw), left, by, TextStyle::plain(kc));
            }
            by += 19.0;
        }
        by += 14.0;
        if let Some(f) = &self.fonts.body {
            let about = kit::wrap(f, &model::about(s, row), w - 36.0);
            kit::lines(rc, f, &about, left, by, 18.0, kit::WARM);
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            x + w - 8.0,
            body_top + 4.0,
            room - 8.0,
            total,
            self.aside_scroll.at,
        );
        // The foot.
        let foot = y + h - ASIDE_FOOT;
        rc.fill(x, foot, w, 1.0, kit::bronze(0.2));
        let (cost_label, cost_value) = if trained {
            let (_, xp) = cost(s, row, Amount::One);
            (
                "Next point",
                if xp == 0 {
                    "at the top".to_owned()
                } else {
                    format!("{} XP", format_xp(xp))
                },
            )
        } else {
            let need = match row {
                Row::Skill(id) => s.skill(id).map_or(0, |k| k.trained_cost),
                _ => 0,
            };
            (
                "Training costs",
                format!("{need} of your {} credits", s.skill_credits),
            )
        };
        if let (Some(sm), Some(st)) = (&self.fonts.small, &self.fonts.strong) {
            let ly = foot + 14.0;
            text::draw(
                rc,
                sm,
                cost_label,
                x + 14.0,
                ly,
                TextStyle::plain(kit::SOFT),
            );
            let vw = text::measure(st, &cost_value, 0.0);
            text::draw(
                rc,
                st,
                &cost_value,
                x + w - 14.0 - vw,
                ly - 1.0,
                TextStyle::plain(p.text),
            );
        }
        for (amount, r) in self.aside_buttons() {
            let (label, sub, can) = self.button_words(row, amount);
            let look = if amount == Amount::One {
                ButtonLook::Primary
            } else {
                ButtonLook::Plain
            };
            let lit = hot(self.hover, r);
            kit::button(rc, &self.theme, r, "", None, look, lit, can);
            let ink = if amount == Amount::One {
                hex(0x1A_140D, if can { 1.0 } else { 0.45 })
            } else {
                Color {
                    a: if can { 1.0 } else { 0.45 },
                    ..p.text
                }
            };
            if let Some(f) = &self.fonts.strong {
                let lw = text::measure(f, &label, 0.0);
                text::draw(
                    rc,
                    f,
                    &label,
                    r.0 + (r.2 - lw) * 0.5,
                    r.1 + 3.0,
                    TextStyle::plain(ink),
                );
            }
            if let Some(f) = &self.fonts.label {
                let sw = text::measure(f, &sub, 0.0).min(r.2 - 6.0);
                let sub = kit::fit(f, &sub, r.2 - 6.0);
                text::draw(
                    rc,
                    f,
                    &sub,
                    r.0 + (r.2 - sw) * 0.5,
                    r.1 + 19.0,
                    TextStyle::plain(Color {
                        a: ink.a * 0.8,
                        ..ink
                    }),
                );
            }
        }
    }

    fn button_words(&self, row: Row, amount: Amount) -> (String, String, bool) {
        let s = &self.sheet;
        if !model::raised_with_experience(s, row) {
            let (name, need) = match row {
                Row::Skill(id) => s
                    .skill(id)
                    .map_or((String::new(), 0), |k| (k.name.clone(), k.trained_cost)),
                _ => (String::new(), 0),
            };
            let left = s.skill_credits - need;
            let can = !self.waiting && need > 0 && left >= 0;
            let sub = if left >= 0 {
                format!("leaves {left} credit{}", if left == 1 { "" } else { "s" })
            } else {
                format!("needs {} more", -left)
            };
            return (format!("Train {name}"), sub, can);
        }
        let (points, xp) = cost(s, row, amount);
        let can = !self.waiting && affordable(s, row, amount);
        let label = match amount {
            Amount::One => "+1".to_owned(),
            Amount::Ten => "+10".to_owned(),
            Amount::Max => "Max".to_owned(),
        };
        let sub = if xp == 0 && amount != Amount::Max {
            "at the top".to_owned()
        } else if amount == Amount::Max {
            if points == 0 {
                "not enough XP".to_owned()
            } else {
                format!("+{points} \u{b7} {} XP", short_cost(xp))
            }
        } else if s.unassigned_xp < xp {
            "not enough XP".to_owned()
        } else {
            format!("{} XP", short_cost(xp))
        };
        (label, sub, can)
    }

    fn draw_titles(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let mut x = TITLES_X;
        if let Some(f) = &self.fonts.label {
            let y = BODY_TOP + ((28.0 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                "EARNED TITLES",
                x,
                y,
                TextStyle::plain(kit::WARM).tracked(kit::LABEL_TRACKING),
            );
            x += text::measure(f, "EARNED TITLES", kit::LABEL_TRACKING) + 10.0;
        }
        if let Some(f) = &self.fonts.small {
            let y = BODY_TOP + ((28.0 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                &self.extras.titles.len().to_string(),
                x,
                y,
                TextStyle::plain(kit::SOFT),
            );
        }
        kit::search_box(
            rc,
            &self.theme,
            Self::title_search_rect(),
            "Find a title",
            self.search.is_empty() && !self.searching,
            self.searching,
            self.fonts.small.as_ref(),
        );
        // The side card.
        let (ax, ay, aw, ah) = Self::title_aside();
        let radius = CornerRadii::all(4.0);
        rc.fill_rounded_rect(ax, ay, aw, ah, radius, hex(0, 0.28));
        rc.stroke_rounded_rect(ax, ay, aw, ah, radius, kit::bronze(0.35), 1.0);
        let title_of = |id: Option<u32>| {
            id.and_then(|id| self.extras.titles.iter().find(|(t, _)| *t == id))
                .map(|(_, w)| w.clone())
        };
        let shown = title_of(self.extras.display_title).unwrap_or_else(|| "no title".to_owned());
        let pick = self
            .title_pick
            .filter(|p| Some(*p) != self.extras.display_title);
        let mut y = ay + 16.0;
        if let Some(f) = &self.fonts.label {
            text::draw(
                rc,
                f,
                "OTHERS SEE YOU AS",
                ax + 16.0,
                y,
                TextStyle::plain(kit::SOFT).tracked(kit::LABEL_TRACKING),
            );
            y += 20.0;
        }
        if let Some(f) = &self.fonts.heading {
            text::draw(
                rc,
                f,
                &kit::fit(f, &self.sheet.name, aw - 32.0),
                ax + 16.0,
                y,
                TextStyle::plain(p.text),
            );
            y += 22.0;
        }
        if let Some(f) = &self.fonts.body {
            let words = title_of(pick).unwrap_or_else(|| shown.clone());
            text::draw(
                rc,
                f,
                &kit::fit(f, &words, aw - 32.0),
                ax + 16.0,
                y,
                TextStyle::plain(kit::WARM),
            );
            y += 22.0;
        }
        if let Some(f) = &self.fonts.small {
            let note = if pick.is_some() {
                format!(
                    "Selected. Your current title is \u{201c}{shown}\u{201d}. The change shows once the server confirms it."
                )
            } else {
                "Your current title. Pick another from the list to show it instead.".to_owned()
            };
            let lines = kit::wrap(f, &note, aw - 32.0);
            kit::lines(rc, f, &lines, ax + 16.0, y, 16.0, kit::SOFT);
        }
        let b = Self::title_button();
        kit::button(
            rc,
            &self.theme,
            b,
            "Display this title",
            self.fonts.strong.as_ref(),
            ButtonLook::Primary,
            hot(self.hover, b),
            pick.is_some(),
        );
    }

    fn draw_info(&mut self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let (cards, total) = self.info_cards_layout();
        let room = BOTTOM - BODY_TOP;
        self.info_scroll.hold(total, room);
        rc.push_clip(0.0, BODY_TOP, WIDTH, room);
        let top = BODY_TOP - self.info_scroll.at;
        for (i, (x, y, w, h)) in cards {
            let Some(card) = self.extras.info.get(i) else {
                continue;
            };
            let y = top + y;
            if y > BOTTOM || y + h < BODY_TOP {
                continue;
            }
            let radius = CornerRadii::all(4.0);
            rc.fill_rounded_rect(x, y, w, h, radius, hex(0, 0.25));
            rc.stroke_rounded_rect(x, y, w, h, radius, kit::bronze(0.3), 1.0);
            let mut cy = y + 14.0;
            if let Some(f) = &self.fonts.label {
                text::draw(
                    rc,
                    f,
                    &card.title,
                    x + 14.0,
                    cy,
                    TextStyle::plain(kit::WARM).tracked(kit::LABEL_TRACKING),
                );
                cy += 20.0;
            }
            for (k, v) in &card.rows {
                if let (Some(b), Some(st)) = (&self.fonts.body, &self.fonts.strong) {
                    let vw = text::measure(st, v, 0.0).min(w * 0.6);
                    let v = kit::fit(st, v, w * 0.6);
                    text::draw(rc, st, &v, x + w - 14.0 - vw, cy, TextStyle::plain(p.text));
                    text::draw(
                        rc,
                        b,
                        &kit::fit(b, k, w - 28.0 - vw - 10.0),
                        x + 14.0,
                        cy,
                        TextStyle::plain(kit::SOFT),
                    );
                }
                cy += 20.0;
            }
            if let Some(f) = &self.fonts.body {
                for para in &card.prose {
                    let lines = kit::wrap(f, para, w - 28.0);
                    cy = kit::lines(rc, f, &lines, x + 14.0, cy, 18.0, kit::WARM) + 18.0;
                }
            }
        }
        rc.pop_clip();
        kit::scrollbar(rc, WIDTH - 14.0, BODY_TOP, room, total, self.info_scroll.at);
    }
}

/// A small "+" button.
fn plus_button(
    rc: &mut RenderContext<'_>,
    r: Rect,
    can: bool,
    lit: bool,
    font: Option<&Arc<UiFont>>,
) {
    let a = if can { 1.0 } else { 0.4 };
    let radius = CornerRadii::all(4.0);
    rc.fill_rounded_rect(
        r.0,
        r.1,
        r.2,
        r.3,
        radius,
        kit::gold(if lit && can { 0.28 } else { 0.14 } * a),
    );
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, kit::bronze(0.55 * a), 1.0);
    let _ = font;
    let c = kit::gold(a);
    rc.fill(r.0 + r.2 * 0.5 - 5.0, r.1 + r.3 * 0.5 - 0.75, 10.0, 1.5, c);
    rc.fill(r.0 + r.2 * 0.5 - 0.75, r.1 + r.3 * 0.5 - 5.0, 1.5, 10.0, c);
}

// ── The skill list ──────────────────────────────────────────────────────

const GROUP_H: f32 = 30.0;
const ROW_H: f32 = 34.0;
const ROW_GAP: f32 = 2.0;

/// What the skill list asks of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRequest {
    /// Choose the skill.
    Choose(u32),
    /// Raise it by one (or train it).
    Raise(u32),
}

/// The skills under their group headings, scrolled by themselves.
#[derive(Debug)]
pub struct SkillList {
    theme: Horizon,
    fonts: CharacterFonts,
    /// The groups shown.
    pub groups: Vec<(SkillGroup, Vec<CharacterSkill>)>,
    /// The sheet's unassigned experience and skill credits (which buttons
    /// can be pressed).
    pub unassigned: u64,
    pub credits: i32,
    /// A raise is on its way.
    pub waiting: bool,
    /// The skill chosen.
    pub chosen: Option<u32>,
    /// What it asked for since the window last looked.
    pub requests: Vec<SkillRequest>,
    scroll: Scroll,
    hover: Option<(f32, f32)>,
}

impl SkillList {
    /// An empty list.
    #[must_use]
    pub fn new(theme: Horizon, fonts: CharacterFonts) -> Self {
        Self {
            theme,
            fonts,
            groups: Vec::new(),
            unassigned: 0,
            credits: 0,
            waiting: false,
            chosen: None,
            requests: Vec::new(),
            scroll: Scroll::default(),
            hover: None,
        }
    }

    /// The list follows `sheet` through the search.
    pub fn show(&mut self, sheet: &CharacterSheet, search: &str) {
        self.groups = skill_groups(sheet, search)
            .into_iter()
            .map(|(g, v)| (g, v.into_iter().cloned().collect()))
            .collect();
        self.unassigned = sheet.unassigned_xp;
        self.credits = sheet.skill_credits;
    }

    fn row_tops(&self) -> (Vec<(u32, f32)>, f32) {
        let mut y = 0.0;
        let mut out = Vec::new();
        for (_, skills) in &self.groups {
            y += GROUP_H;
            for s in skills {
                out.push((s.id, y));
                y += ROW_H + ROW_GAP;
            }
        }
        (out, y)
    }

    fn button_rect(w: f32, top: f32, trained: bool) -> Rect {
        let bw = if trained { 26.0 } else { 46.0 };
        (w - 10.0 - 6.0 - bw, top + (ROW_H - 24.0) * 0.5, bw, 24.0)
    }

    fn skill(&self, id: u32) -> Option<&CharacterSkill> {
        self.groups.iter().flat_map(|(_, v)| v).find(|s| s.id == id)
    }
}

impl Widget for SkillList {
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
        let w = ui.element(me).map_or(SKILLS_W, |el| el.width);
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
                let (_, total) = self.row_tops();
                self.scroll
                    .wheel(e.data0, 2.0 * (ROW_H + ROW_GAP), total, SKILLS_H);
                true
            }
            kind::CLICK => {
                let at = y + self.scroll.at;
                let hit = self
                    .row_tops()
                    .0
                    .into_iter()
                    .find(|(_, t)| at >= *t && at < *t + ROW_H);
                if let Some((id, top)) = hit {
                    let trained = self.skill(id).is_some_and(|s| s.advancement.is_trained());
                    let b = Self::button_rect(w, top - self.scroll.at, trained);
                    self.chosen = Some(id);
                    self.requests.push(SkillRequest::Choose(id));
                    if inside(b, x, y) {
                        self.requests.push(SkillRequest::Raise(id));
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let (w, h) = ui
            .element(me)
            .map_or((SKILLS_W, SKILLS_H), |e| (e.width, e.height));
        let (_, total) = self.row_tops();
        self.scroll.hold(total, h);
        let right = w - 10.0;
        rc.push_clip(0.0, 0.0, w, h);
        let mut y = -self.scroll.at;
        for (group, skills) in &self.groups {
            if y + GROUP_H > 0.0 && y < h {
                let color = match group {
                    SkillGroup::Specialized => kit::gold(1.0),
                    SkillGroup::Trained => kit::WARM,
                    SkillGroup::Untrained => kit::SOFT,
                    SkillGroup::Unusable => hex(0x7A_7166, 1.0),
                };
                kit::group_heading(
                    rc,
                    &self.theme,
                    self.fonts.label.as_ref(),
                    0.0,
                    y + 6.0,
                    GROUP_H - 8.0,
                    right,
                    group.title(),
                    Some(&skills.len().to_string()),
                    color,
                );
            }
            y += GROUP_H;
            for s in skills {
                if y + ROW_H > 0.0 && y < h {
                    self.draw_row(rc, s, *group, y, w);
                }
                y += ROW_H + ROW_GAP;
            }
        }
        rc.pop_clip();
        kit::scrollbar(rc, w - 5.0, 0.0, h, total, self.scroll.at);
        let _ = p;
    }
}

impl SkillList {
    fn draw_row(
        &self,
        rc: &mut RenderContext<'_>,
        s: &CharacterSkill,
        group: SkillGroup,
        y: f32,
        w: f32,
    ) {
        let p = self.theme.palette;
        let right = w - 10.0;
        let on = self.chosen == Some(s.id);
        let radius = CornerRadii::all(4.0);
        let fade = if group == SkillGroup::Unusable {
            0.6
        } else {
            1.0
        };
        if on {
            rc.fill_rounded_rect(0.0, y, right, ROW_H, radius, kit::gold(0.14));
            rc.stroke_rounded_rect(0.0, y, right, ROW_H, radius, kit::gold(1.0), 1.0);
        } else if hot(self.hover, (0.0, y, right, ROW_H)) {
            rc.fill_rounded_rect(0.0, y, right, ROW_H, radius, hex(0xFF_FFFF, 0.04));
        }
        let dot = match group {
            SkillGroup::Specialized => kit::gold(1.0),
            SkillGroup::Trained => kit::WARM,
            SkillGroup::Untrained => kit::SOFT,
            SkillGroup::Unusable => hex(0x7A_7166, 1.0),
        };
        rc.fill_rounded_rect(
            10.0,
            y + ROW_H * 0.5 - 3.0,
            6.0,
            6.0,
            CornerRadii::all(3.0),
            dot,
        );
        let trained = s.advancement.is_trained();
        let b = Self::button_rect(w, y, trained);
        let cost_right = b.0 - 8.0;
        let cost_words = if trained {
            if s.raise_cost == 0 {
                "top".to_owned()
            } else {
                short_cost(s.raise_cost)
            }
        } else {
            format!("{} credits", s.trained_cost)
        };
        if let Some(f) = &self.fonts.small {
            let cw = text::measure(f, &cost_words, 0.0);
            let ty = y + ((ROW_H - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                &cost_words,
                cost_right - cw,
                ty,
                TextStyle::plain(Color {
                    a: fade,
                    ..kit::SOFT
                }),
            );
        }
        let value_right = cost_right - 62.0;
        let delta = s.current_level - s.vitae_modifier - s.base_level;
        let vcolor = match delta.signum() {
            1 => kit::GOOD,
            -1 => kit::HARM,
            _ => p.text,
        };
        let value = if group == SkillGroup::Unusable {
            "\u{2014}".to_owned()
        } else {
            s.current_level.to_string()
        };
        if let Some(f) = &self.fonts.value {
            let vw = text::measure(f, &value, 0.0);
            let ty = y + ((ROW_H - f.line_height) * 0.5).round();
            text::draw(
                rc,
                f,
                &value,
                value_right - vw,
                ty,
                TextStyle::plain(Color { a: fade, ..vcolor }),
            );
        }
        if let Some(f) = &self.fonts.name {
            let ty = y + ((ROW_H - f.line_height) * 0.5).round();
            let name = kit::fit(f, &s.name, value_right - 60.0 - 24.0);
            text::draw(
                rc,
                f,
                &name,
                24.0,
                ty,
                TextStyle::plain(Color { a: fade, ..p.text }),
            );
        }
        let lit = hot(self.hover, b);
        if trained {
            let can = !self.waiting && s.raise_cost > 0 && self.unassigned >= s.raise_cost;
            plus_button(rc, b, can, lit, self.fonts.strong.as_ref());
        } else {
            let can = !self.waiting && s.trained_cost > 0 && self.credits >= s.trained_cost;
            let a = if can { 1.0 } else { 0.45 };
            rc.stroke_rounded_rect(
                b.0,
                b.1,
                b.2,
                b.3,
                CornerRadii::all(4.0),
                kit::bronze(0.55 * a),
                1.0,
            );
            if lit && can {
                rc.fill_rounded_rect(b.0, b.1, b.2, b.3, CornerRadii::all(4.0), kit::gold(0.12));
            }
            if let Some(f) = &self.fonts.label {
                let tw = text::measure(f, "Train", 0.0);
                let ty = b.1 + ((b.3 - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    "Train",
                    b.0 + (b.2 - tw) * 0.5,
                    ty,
                    TextStyle::plain(Color { a, ..kit::WARM }),
                );
            }
        }
    }
}

// ── The title grid ──────────────────────────────────────────────────────

const TITLE_H: f32 = 40.0;
const TITLE_GAP: f32 = 6.0;

/// The earned titles, three to a row, scrolled by themselves.
#[derive(Debug)]
pub struct TitleGrid {
    theme: Horizon,
    fonts: CharacterFonts,
    /// The titles shown (id, words).
    pub titles: Vec<(u32, String)>,
    /// The title shown to others.
    pub shown: Option<u32>,
    /// The title picked.
    pub pick: Option<u32>,
    /// A title was picked since the window last looked.
    pub picked: Option<u32>,
    scroll: Scroll,
    hover: Option<(f32, f32)>,
}

impl TitleGrid {
    /// An empty grid.
    #[must_use]
    pub fn new(theme: Horizon, fonts: CharacterFonts) -> Self {
        Self {
            theme,
            fonts,
            titles: Vec::new(),
            shown: None,
            pick: None,
            picked: None,
            scroll: Scroll::default(),
            hover: None,
        }
    }

    fn cell(w: f32, i: usize) -> Rect {
        let cw = (w - 10.0 - 2.0 * TITLE_GAP) / 3.0;
        let (col, row) = ((i % 3) as f32, (i / 3) as f32);
        (
            col * (cw + TITLE_GAP),
            row * (TITLE_H + TITLE_GAP),
            cw,
            TITLE_H,
        )
    }

    fn total(&self) -> f32 {
        self.titles.len().div_ceil(3) as f32 * (TITLE_H + TITLE_GAP)
    }
}

impl Widget for TitleGrid {
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
        let w = ui.element(me).map_or(TITLES_W, |el| el.width);
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
                let total = self.total();
                self.scroll
                    .wheel(e.data0, 2.0 * (TITLE_H + TITLE_GAP), total, TITLES_H);
                true
            }
            kind::CLICK => {
                let at = y + self.scroll.at;
                if let Some((id, _)) = self
                    .titles
                    .iter()
                    .enumerate()
                    .find(|(i, _)| inside(Self::cell(w, *i), x, at))
                    .map(|(_, t)| t.clone())
                {
                    self.pick = Some(id);
                    self.picked = Some(id);
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let (w, h) = ui
            .element(me)
            .map_or((TITLES_W, TITLES_H), |e| (e.width, e.height));
        let total = self.total();
        self.scroll.hold(total, h);
        rc.push_clip(0.0, 0.0, w, h);
        if self.titles.is_empty()
            && let Some(f) = &self.fonts.body
        {
            text::draw(
                rc,
                f,
                "No titles match.",
                4.0,
                12.0,
                TextStyle::plain(p.muted),
            );
        }
        for (i, (id, words)) in self.titles.iter().enumerate() {
            let (cx, cy, cw, ch) = Self::cell(w, i);
            let cy = cy - self.scroll.at;
            if cy + ch < 0.0 || cy > h {
                continue;
            }
            let picked = self.pick == Some(*id);
            let current = self.shown == Some(*id);
            let radius = CornerRadii::all(4.0);
            let fill = if picked {
                kit::gold(0.16)
            } else {
                hex(0, 0.22)
            };
            let edge = if picked {
                kit::gold(1.0)
            } else if current {
                kit::bronze(0.6)
            } else if hot(self.hover, (cx, cy, cw, ch)) {
                kit::bronze(0.45)
            } else {
                kit::bronze(0.2)
            };
            rc.fill_rounded_rect(cx, cy, cw, ch, radius, fill);
            rc.stroke_rounded_rect(cx, cy, cw, ch, radius, edge, 1.0);
            let mut right = cx + cw - 12.0;
            if current && let Some(f) = &self.fonts.label {
                let tw = text::measure(f, "SHOWN", 0.4);
                right -= tw;
                let ty = cy + ((ch - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    "SHOWN",
                    right,
                    ty,
                    TextStyle::plain(kit::gold(1.0)).tracked(0.4),
                );
                right -= 8.0;
            }
            if let Some(f) = &self.fonts.name {
                let ty = cy + ((ch - f.line_height) * 0.5).round();
                let shown = kit::fit(f, words, right - cx - 12.0);
                text::draw(rc, f, &shown, cx + 12.0, ty, TextStyle::plain(p.text));
            }
        }
        rc.pop_clip();
        kit::scrollbar(rc, w - 5.0, 0.0, h, total, self.scroll.at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_ui::panels::character::SkillAdvancement;

    fn sheet() -> CharacterSheet {
        CharacterSheet {
            name: "Aerin".to_owned(),
            level: Some(4782),
            attributes: [100; 6],
            attribute_base: [100; 6],
            attribute_innate: [60; 6],
            attribute_raise_costs: [1_000; 9],
            attribute_raise10_costs: [12_000; 9],
            attribute_max_raises: [(3, 3_100); 9],
            unassigned_xp: 5_000,
            skills: vec![CharacterSkill {
                id: 23,
                name: "Lockpick".to_owned(),
                advancement: SkillAdvancement::Untrained,
                usable_untrained: true,
                trained_cost: 6,
                ..CharacterSkill::default()
            }],
            ..CharacterSheet::default()
        }
    }

    #[test]
    fn the_level_plate_grows_with_the_number() {
        let mut f = CharacterFrame::new(Horizon::BRONZE, CharacterFonts::default());
        f.sheet = sheet();
        assert_eq!(f.level_words(), "4,782");
        f.sheet.level = None;
        assert_eq!(f.level_words(), "???");
    }

    #[test]
    fn a_plus_raises_by_one_and_waits_for_the_sheet() {
        let mut f = CharacterFrame::new(Horizon::BRONZE, CharacterFonts::default());
        f.sheet = sheet();
        let (row, r) = CharacterFrame::attr_rows()[0];
        let plus = CharacterFrame::plus_rect(r);
        f.click(plus.0 + 4.0, plus.1 + 4.0);
        assert_eq!(f.chosen, Some(row));
        assert!(f.requests.iter().any(|r| matches!(
            r,
            CharacterRequest::Send(PanelOutput::RaiseAttribute {
                attribute: 1,
                xp: 1_000
            })
        )));
        assert!(f.waiting);
        f.requests.clear();
        f.click(plus.0 + 4.0, plus.1 + 4.0);
        assert!(
            !f.requests
                .iter()
                .any(|r| matches!(r, CharacterRequest::Send(_)))
        );
    }

    #[test]
    fn an_untrained_skill_trains_with_credits() {
        let mut f = CharacterFrame::new(Horizon::BRONZE, CharacterFonts::default());
        f.sheet = sheet();
        f.sheet.skill_credits = 7;
        f.choose(Row::Skill(23));
        let (label, sub, can) = f.button_words(Row::Skill(23), Amount::One);
        assert_eq!(label, "Train Lockpick");
        assert_eq!(sub, "leaves 1 credit");
        assert!(can);
        f.send_raise(Row::Skill(23), Amount::One);
        assert!(matches!(
            f.requests.as_slice(),
            [CharacterRequest::Send(PanelOutput::TrainSkill {
                skill: 23,
                credits: 6
            })]
        ));
    }
}
