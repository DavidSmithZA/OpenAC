//! The Journal's frame: the header, the Quests and Notes tabs; on Quests
//! the list (with its search, OPENAC) and the picked quest's detail; on
//! Notes the pages (searchable, New) and the page read with its place and
//! countdown. What can be clicked is noted as it is drawn.

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text;
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::{JournalTab, NotesScreen, QuestDetail, QuestState, QuestsScreen};
use crate::ui::modern::hud::lines::{circle, stroke};
use crate::ui::modern::kit::{
    self, ButtonLook, Hits, MUTED, Rect, Scroll, bronze, gold, inside, put, put_mid, put_right,
    spaced,
};

/// The window's size.
pub const WIDTH: f32 = 900.0;
pub const HEIGHT: f32 = 600.0;
const HEADER: f32 = 52.0;
const TABS: f32 = 40.0;
const BODY: f32 = HEADER + TABS;
const PAD: f32 = 20.0;
const LIST_W: f32 = 320.0;
const PAGES_W: f32 = 280.0;
const QUEST_ROW: f32 = 44.0;
const PAGE_ROW: f32 = 46.0;
const TOP: f32 = BODY + 12.0;
const BOTTOM: f32 = HEIGHT - PAD;
const DETAIL_X: f32 = PAD + LIST_W + 16.0;
const PAGE_X: f32 = PAD + PAGES_W + 16.0;
const BOX_H: f32 = 100.0;

/// An open book, the window's glyph.
const BOOK: &[&[(f32, f32)]] = &[
    &[(12.0, 6.0), (12.0, 20.0)],
    &[
        (12.0, 6.0),
        (8.0, 4.5),
        (3.0, 4.5),
        (3.0, 18.5),
        (8.0, 18.5),
        (12.0, 20.0),
    ],
    &[
        (12.0, 6.0),
        (16.0, 4.5),
        (21.0, 4.5),
        (21.0, 18.5),
        (16.0, 18.5),
        (12.0, 20.0),
    ],
];
/// An eye, for a quest shown on screen.
const EYE: &[&[(f32, f32)]] = &[
    &[
        (2.0, 12.0),
        (6.0, 7.0),
        (12.0, 5.0),
        (18.0, 7.0),
        (22.0, 12.0),
    ],
    &[
        (2.0, 12.0),
        (6.0, 17.0),
        (12.0, 19.0),
        (18.0, 17.0),
        (22.0, 12.0),
    ],
];
/// A pin, for the Map button.
const PIN: &[&[(f32, f32)]] = &[&[
    (12.0, 21.0),
    (6.0, 13.0),
    (5.0, 9.0),
    (7.0, 5.0),
    (12.0, 3.0),
    (17.0, 5.0),
    (19.0, 9.0),
    (18.0, 13.0),
    (12.0, 21.0),
]];
/// Arrows for the page buttons.
const FIRST: &[&[(f32, f32)]] = &[
    &[(16.0, 6.0), (10.0, 12.0), (16.0, 18.0)],
    &[(7.0, 6.0), (7.0, 18.0)],
];
const PREV: &[&[(f32, f32)]] = &[&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]];
const NEXT: &[&[(f32, f32)]] = &[&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]];
const LAST: &[&[(f32, f32)]] = &[
    &[(8.0, 6.0), (14.0, 12.0), (8.0, 18.0)],
    &[(17.0, 6.0), (17.0, 18.0)],
];
/// A plus, for New.
const PLUS: &[&[(f32, f32)]] = &[&[(12.0, 5.0), (12.0, 19.0)], &[(5.0, 12.0), (19.0, 12.0)]];

/// The fonts the window draws with.
#[derive(Debug, Clone, Default)]
pub struct JournalFonts {
    /// The title.
    pub title: Option<Arc<UiFont>>,
    /// A quest's name over its detail.
    pub heading: Option<Arc<UiFont>>,
    /// Names, buttons.
    pub strong: Option<Arc<UiFont>>,
    /// Words.
    pub body: Option<Arc<UiFont>>,
    /// Quiet lines.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals, tags.
    pub label: Option<Arc<UiFont>>,
    /// A running countdown.
    pub big: Option<Arc<UiFont>>,
}

/// The fields over the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Find a quest.
    QuestSearch,
    /// Search the pages.
    NoteSearch,
    /// The page's label, title and notes.
    Label,
    Title,
    Notes,
    /// The countdown's days, hours and minutes.
    Days,
    Hours,
    Minutes,
}

impl Slot {
    /// Every field.
    pub const ALL: [Self; 8] = [
        Self::QuestSearch,
        Self::NoteSearch,
        Self::Label,
        Self::Title,
        Self::Notes,
        Self::Days,
        Self::Hours,
        Self::Minutes,
    ];

    /// The field holds more than one line.
    #[must_use]
    pub const fn lines(self) -> bool {
        matches!(self, Self::Notes)
    }

    /// The most characters it takes (0: no limit).
    #[must_use]
    pub const fn most(self) -> usize {
        match self {
            Self::Label => ac_core::journal::MAX_LABEL_LENGTH,
            Self::Title => ac_core::journal::MAX_TITLE_LENGTH,
            Self::Notes => ac_core::journal::MAX_NOTES_LENGTH,
            Self::Days | Self::Hours | Self::Minutes => 3,
            Self::QuestSearch | Self::NoteSearch => 0,
        }
    }

    /// One of the page's own fields (what it holds is kept on the page).
    #[must_use]
    pub const fn on_page(self) -> bool {
        !matches!(self, Self::QuestSearch | Self::NoteSearch)
    }
}

/// What the frame asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum JournalAsk {
    /// Close the window.
    Close,
    /// The page shown changed.
    Tab(JournalTab),
    /// Show (or stop showing) a quest on screen.
    Show(u32, bool),
    /// Abandon a quest (or take a done one off the list): asks first.
    Abandon(u32),
    /// Open the map on a cell.
    Map(u32),
    /// Open the map on map coordinates (east, north).
    MapAt(f32, f32),
    /// Add a page; read page `n`; delete page `n` (asks first).
    NewPage,
    Goto(usize),
    Delete(usize),
    /// Record where the player is on the page.
    Record,
    /// Start or stop the page's countdown.
    Start,
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
enum Hit {
    Ask(JournalAsk),
    Tab(JournalTab),
    Quest(u32),
    Swallow,
}

/// The Journal's frame.
#[derive(Debug)]
pub struct JournalFrame {
    theme: Horizon,
    fonts: JournalFonts,
    /// The page shown.
    pub tab: JournalTab,
    /// The quests.
    pub quests: QuestsScreen,
    /// The notes.
    pub notes: NotesScreen,
    /// The quest picked (0: the first).
    pub picked: u32,
    /// Which fields hold nothing, and how much each holds.
    pub empty: Vec<Slot>,
    pub counts: Vec<(Slot, usize)>,
    /// The field with the keyboard.
    pub focused: Option<Slot>,
    /// What it asked for since the window last looked.
    pub asks: Vec<JournalAsk>,
    hits: Hits<Hit>,
    scroll: [Scroll; 3],
    rooms: [(f32, Rect); 3],
}

impl JournalFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: JournalFonts) -> Self {
        Self {
            theme,
            fonts,
            tab: JournalTab::Quests,
            quests: QuestsScreen::default(),
            notes: NotesScreen::default(),
            picked: 0,
            empty: Slot::ALL.to_vec(),
            counts: Vec::new(),
            focused: None,
            asks: Vec::new(),
            hits: Hits::default(),
            scroll: [Scroll::default(); 3],
            rooms: [(0.0, (0.0, 0.0, 0.0, 0.0)); 3],
        }
    }

    /// Shows page `tab`.
    pub fn show(&mut self, tab: JournalTab) {
        if self.tab != tab {
            self.asks.push(JournalAsk::Tab(tab));
        }
        self.tab = tab;
    }

    /// Shows quest `id` on the Quests page.
    pub fn pick(&mut self, id: u32) {
        self.show(JournalTab::Quests);
        self.picked = id;
    }

    fn quest_search_box() -> Rect {
        (PAD, TOP, LIST_W, 32.0)
    }

    fn note_search_box() -> Rect {
        (PAD, TOP, PAGES_W - 6.0 - 74.0, 32.0)
    }

    fn page_box() -> Rect {
        (PAGE_X, TOP, WIDTH - PAD - PAGE_X, BOTTOM - TOP)
    }

    fn input_box(slot: Slot) -> Rect {
        let p = Self::page_box();
        let (x, w) = (p.0 + 16.0, p.2 - 32.0);
        let boxes_top = p.1 + p.3 - 14.0 - BOX_H;
        let timer_x = x + (w + 10.0) * 0.5;
        match slot {
            Slot::QuestSearch => Self::quest_search_box(),
            Slot::NoteSearch => Self::note_search_box(),
            Slot::Label => (x, p.1 + 66.0, 150.0, 32.0),
            Slot::Title => (x + 158.0, p.1 + 66.0, w - 158.0, 32.0),
            Slot::Notes => (x, p.1 + 124.0, w, boxes_top - 10.0 - (p.1 + 124.0)),
            Slot::Days => (timer_x + 12.0, boxes_top + 32.0, 40.0, 28.0),
            Slot::Hours => (timer_x + 12.0 + 62.0, boxes_top + 32.0, 40.0, 28.0),
            Slot::Minutes => (timer_x + 12.0 + 124.0, boxes_top + 32.0, 40.0, 28.0),
        }
    }

    /// Where field `slot` goes on the page shown (none: it hides).
    #[must_use]
    pub fn field_rect(&self, slot: Slot) -> Option<Rect> {
        match (self.tab, slot) {
            (JournalTab::Quests, Slot::QuestSearch) => {
                let b = Self::quest_search_box();
                Some((b.0 + 22.0, b.1, b.2 - 22.0 - 66.0, b.3))
            }
            (JournalTab::Notes, Slot::NoteSearch) => {
                let b = Self::note_search_box();
                Some((b.0 + 22.0, b.1, b.2 - 22.0, b.3))
            }
            (JournalTab::Notes, s) if s.on_page() && self.notes.current > 0 => {
                let running = self.notes.left.is_some();
                if running && matches!(s, Slot::Days | Slot::Hours | Slot::Minutes) {
                    return None;
                }
                Some(Self::input_box(s))
            }
            _ => None,
        }
    }

    fn is_empty(&self, slot: Slot) -> bool {
        self.empty.contains(&slot)
    }

    fn count(&self, slot: Slot) -> usize {
        self.counts
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or(0, |(_, n)| *n)
    }

    fn click(&mut self, x: f32, y: f32) {
        match self.hits.at(x, y) {
            Some(Hit::Ask(ask)) => self.asks.push(ask),
            Some(Hit::Tab(tab)) => self.show(tab),
            Some(Hit::Quest(id)) => self.picked = id,
            Some(Hit::Swallow) | None => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn button(
        &self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Hit>,
        r: Rect,
        words: &str,
        look: ButtonLook,
        on: bool,
        hit: Hit,
    ) {
        let lit = h.add(r, if on { hit } else { Hit::Swallow });
        kit::button(
            rc,
            &self.theme,
            r,
            words,
            self.fonts.strong.as_ref(),
            look,
            lit && on,
            on,
        );
    }

    fn draw_frame(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let theme = &self.theme;
        let f = &self.fonts;
        kit::window_back(rc, theme, WIDTH, HEIGHT, HEADER, false);
        let right = kit::title(rc, theme, f.title.as_ref(), BOOK, "Journal", HEADER);
        put_mid(
            rc,
            f.small.as_ref(),
            "J",
            right + 10.0,
            0.0,
            HEADER,
            theme.palette.muted,
        );
        let close = kit::close_rect(WIDTH, HEADER);
        let lit = h.add(close, Hit::Ask(JournalAsk::Close));
        kit::close(rc, theme, close, lit);
        rc.fill(0.0, BODY - 1.0, WIDTH, 1.0, bronze(0.35));
        let mut x = 16.0;
        for (tab, words, count) in [
            (JournalTab::Quests, "Quests", self.quests.count),
            (JournalTab::Notes, "Notes", self.notes.count),
        ] {
            let t = kit::Tab {
                words,
                count: (count > 0).then(|| count.to_string()),
                on: self.tab == tab,
                red: false,
            };
            let w = kit::tab_width(&t, f.strong.as_ref(), f.label.as_ref());
            let r = (x, HEADER, w, TABS);
            let lit = h.add(r, Hit::Tab(tab));
            kit::draw_tab(rc, theme, &t, r, f.strong.as_ref(), f.label.as_ref(), lit);
            x += w + 4.0;
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_quests(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let p = self.theme.palette;
        let f = self.fonts.clone();
        // The search.
        let b = Self::quest_search_box();
        kit::search_box(
            rc,
            &self.theme,
            (b.0, b.1, b.2, b.3),
            "Find a quest",
            self.is_empty(Slot::QuestSearch),
            self.focused == Some(Slot::QuestSearch),
            f.body.as_ref(),
        );
        kit::openac_tag(rc, f.label.as_ref(), b.0 + b.2 - 62.0, b.1 + b.3 * 0.5);
        // The list.
        let room = (PAD, TOP + 40.0, LIST_W, BOTTOM - (TOP + 40.0));
        let total: f32 = self
            .quests
            .groups
            .iter()
            .map(|(_, rows)| 26.0 + rows.len() as f32 * (QUEST_ROW + 2.0))
            .sum();
        self.rooms[0] = (total, room);
        self.scroll[0].hold(total, room.3);
        let picked = self.quests.detail.as_ref().map_or(0, |d| d.id);
        if self.quests.groups.is_empty() {
            let words = if self.quests.count == 0 {
                "No quests yet. Quests you take up show here."
            } else {
                "No quest has that name."
            };
            if let Some(b) = &f.body {
                kit::lines(
                    rc,
                    b,
                    &kit::wrap(b, words, room.2 - 8.0),
                    room.0,
                    room.1 + 8.0,
                    18.0,
                    MUTED,
                );
            }
        }
        rc.push_clip(room.0, room.1, room.2, room.3);
        let mut y = room.1 - self.scroll[0].at;
        for (state, rows) in &self.quests.groups {
            kit::group_heading(
                rc,
                &self.theme,
                f.label.as_ref(),
                room.0,
                y,
                26.0,
                room.0 + room.2 - 8.0,
                state.heading(),
                None,
                kit::SOFT,
            );
            y += 26.0;
            for row in rows {
                let r = (room.0, y, room.2 - 8.0, QUEST_ROW);
                let lit = h.add(r, Hit::Quest(row.id)) && kit::hot(h.hover, room);
                kit::pick_row(rc, r, row.id == picked, lit);
                let right = r.0 + r.2 - if row.shown { 30.0 } else { 10.0 };
                if let Some(st) = &f.strong {
                    let ink = if *state == QuestState::Done {
                        kit::SOFT
                    } else {
                        p.text
                    };
                    put(
                        rc,
                        Some(st),
                        &kit::fit(st, &row.name, right - r.0 - 10.0),
                        r.0 + 10.0,
                        y + 6.0,
                        ink,
                    );
                }
                if let Some(sm) = &f.small {
                    let ink = if row.timed { gold(1.0) } else { MUTED };
                    put(
                        rc,
                        Some(sm),
                        &kit::fit(sm, &row.sub, right - r.0 - 10.0),
                        r.0 + 10.0,
                        y + 25.0,
                        ink,
                    );
                }
                if row.shown {
                    eye(rc, r.0 + r.2 - 26.0, y + 14.0, gold(1.0));
                }
                y += QUEST_ROW + 2.0;
            }
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            room.0 + room.2 - 5.0,
            room.1,
            room.3,
            total,
            self.scroll[0].at,
        );
        // The detail.
        let d_box = (DETAIL_X, TOP, WIDTH - PAD - DETAIL_X, BOTTOM - TOP);
        kit::panel(rc, d_box, 0.3);
        if let Some(d) = self.quests.detail.clone() {
            self.draw_detail(rc, h, &d, d_box);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_detail(
        &mut self,
        rc: &mut RenderContext<'_>,
        h: &mut Hits<Hit>,
        d: &QuestDetail,
        b: Rect,
    ) {
        let p = self.theme.palette;
        let f = self.fonts.clone();
        let x = b.0 + 16.0;
        let w = b.2 - 32.0;
        // The head: the name, where it stands, show on screen.
        let (ink, edge) = state_colors(d.state);
        let pill_x = kit::pill(
            rc,
            f.strong.as_ref(),
            b.0 + b.2 - 16.0,
            b.1 + 24.0,
            d.state.words(),
            ink,
            edge,
        );
        if let Some(hf) = &f.heading {
            let name = kit::fit(hf, &d.name, pill_x - 10.0 - x);
            put(rc, Some(hf), &name, x, b.1 + 14.0, p.text);
        }
        let cy = b.1 + 56.0;
        let sw = (x, cy - 9.0, 34.0, 18.0);
        let lit = h.add(
            (x, cy - 12.0, 200.0, 24.0),
            Hit::Ask(JournalAsk::Show(d.id, !d.shown)),
        );
        kit::switch(rc, sw, d.shown, lit);
        let end = f
            .body
            .as_ref()
            .map_or(100.0, |b| text::measure(b, "Show on screen", 0.0));
        put_mid(
            rc,
            f.body.as_ref(),
            "Show on screen",
            x + 44.0,
            cy - 10.0,
            20.0,
            kit::WARM,
        );
        kit::openac_tag(rc, f.label.as_ref(), x + 54.0 + end, cy);
        rc.fill(b.0 + 1.0, b.1 + 76.0, b.2 - 2.0, 1.0, bronze(0.2));
        // The body, scrolled.
        let foot_h = 56.0;
        let room = (b.0, b.1 + 77.0, b.2, b.3 - 77.0 - foot_h);
        rc.push_clip(room.0, room.1, room.2, room.3);
        let mut y = room.1 + 12.0 - self.scroll[1].at;
        let top = y;
        if let Some(bf) = &f.body {
            let lines = kit::wrap(bf, &d.description, w);
            y = kit::lines(rc, bf, &lines, x, y, bf.line_height + 6.0, kit::WARM);
        }
        if let Some(progress) = &d.progress {
            y += 10.0;
            kit::group_heading(
                rc,
                &self.theme,
                f.label.as_ref(),
                x,
                y,
                20.0,
                x + w,
                "PROGRESS",
                None,
                kit::SOFT,
            );
            y += 24.0;
            put(rc, f.strong.as_ref(), progress, x, y, p.text);
            y += 20.0;
        }
        if let Some((label, time, soon)) = &d.time {
            y += 10.0;
            kit::group_heading(
                rc,
                &self.theme,
                f.label.as_ref(),
                x,
                y,
                20.0,
                x + w,
                "TIME",
                None,
                kit::SOFT,
            );
            y += 22.0;
            put_mid(rc, f.body.as_ref(), label, x, y, 26.0, kit::WARM);
            let ink = if *soon { gold(1.0) } else { kit::SOFT };
            put_right(rc, f.strong.as_ref(), time, x + w, y, 26.0, ink);
            y += 30.0;
        }
        y += 10.0;
        kit::group_heading(
            rc,
            &self.theme,
            f.label.as_ref(),
            x,
            y,
            20.0,
            x + w,
            "WHO AND WHERE",
            None,
            kit::SOFT,
        );
        y += 26.0;
        for place in &d.places {
            put(rc, f.small.as_ref(), place.role, x, y + 3.0, kit::SOFT);
            put(rc, f.strong.as_ref(), &place.who, x + 96.0, y, p.text);
            put(
                rc,
                f.small.as_ref(),
                &place.place,
                x + 96.0,
                y + 18.0,
                MUTED,
            );
            if let Some(cell) = place.cell {
                let r = (x + w - 70.0, y + 3.0, 70.0, 26.0);
                let lit = h.add(r, Hit::Ask(JournalAsk::Map(cell))) && kit::hot(h.hover, room);
                kit::button(rc, &self.theme, r, "", None, ButtonLook::Plain, lit, true);
                stroke(rc, r.0 + 10.0, r.1 + 6.0, 14.0, PIN, kit::WARM, 1.5);
                put_mid(rc, f.strong.as_ref(), "Map", r.0 + 30.0, r.1, r.3, p.text);
            }
            y += 42.0;
        }
        rc.pop_clip();
        let total = y + self.scroll[1].at - top + 12.0;
        self.rooms[1] = (total, room);
        self.scroll[1].hold(total, room.3);
        kit::scrollbar(
            rc,
            b.0 + b.2 - 8.0,
            room.1,
            room.3,
            total,
            self.scroll[1].at,
        );
        // The foot.
        let fy = b.1 + b.3 - foot_h;
        rc.fill(b.0 + 1.0, fy, b.2 - 2.0, 1.0, bronze(0.2));
        let bw = kit::button_width(d.abandon, f.strong.as_ref());
        let r = (b.0 + b.2 - 16.0 - bw, fy + 12.0, bw, 32.0);
        self.button(
            rc,
            h,
            r,
            d.abandon,
            ButtonLook::Danger,
            true,
            Hit::Ask(JournalAsk::Abandon(d.id)),
        );
        if let Some(sm) = &f.small {
            let note = kit::fit(sm, d.foot, r.0 - x - 10.0);
            put_mid(rc, Some(sm), &note, x, fy, foot_h, MUTED);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_notes(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let p = self.theme.palette;
        let f = self.fonts.clone();
        let n = self.notes.clone();
        // The search and New.
        let b = Self::note_search_box();
        kit::search_box(
            rc,
            &self.theme,
            b,
            "Search your pages",
            self.is_empty(Slot::NoteSearch),
            self.focused == Some(Slot::NoteSearch),
            f.body.as_ref(),
        );
        let r = (b.0 + b.2 + 6.0, b.1, 74.0, 32.0);
        let lit = h.add(r, Hit::Ask(JournalAsk::NewPage));
        kit::button(rc, &self.theme, r, "", None, ButtonLook::Plain, lit, true);
        stroke(rc, r.0 + 10.0, r.1 + 9.0, 14.0, PLUS, p.text, 1.6);
        put_mid(rc, f.strong.as_ref(), "New", r.0 + 30.0, r.1, r.3, p.text);
        // The pages.
        let room = (PAD, TOP + 40.0, PAGES_W, BOTTOM - 22.0 - (TOP + 40.0));
        let total = n.rows.len() as f32 * (PAGE_ROW + 2.0);
        self.rooms[2] = (total, room);
        self.scroll[2].hold(total, room.3);
        if n.count == 0 {
            put(
                rc,
                f.body.as_ref(),
                "No pages yet. New adds one.",
                room.0,
                room.1 + 8.0,
                MUTED,
            );
        }
        rc.push_clip(room.0, room.1, room.2, room.3);
        for (i, row) in n.rows.iter().enumerate() {
            let y = room.1 + i as f32 * (PAGE_ROW + 2.0) - self.scroll[2].at;
            let r = (room.0, y, room.2 - 8.0, PAGE_ROW);
            let lit = h.add(r, Hit::Ask(JournalAsk::Goto(row.number))) && kit::hot(h.hover, room);
            kit::pick_row(rc, r, row.number == n.current, lit);
            put_mid(
                rc,
                f.small.as_ref(),
                &row.number.to_string(),
                r.0 + 10.0,
                y,
                PAGE_ROW,
                MUTED,
            );
            if let Some(st) = &f.strong {
                put(
                    rc,
                    Some(st),
                    &kit::fit(st, &row.title, r.2 - 44.0),
                    r.0 + 32.0,
                    y + 6.0,
                    p.text,
                );
            }
            let mut tx = r.0 + 32.0;
            if !row.label.is_empty()
                && let Some(t) = &f.small
            {
                let w = text::measure(t, &row.label, 0.0) + 12.0;
                let c = (tx, y + 25.0, w, 16.0);
                rc.stroke_rounded_rect(
                    c.0,
                    c.1,
                    c.2,
                    c.3,
                    CornerRadii::all(3.0),
                    bronze(0.35),
                    1.0,
                );
                put_mid(rc, Some(t), &row.label, c.0 + 6.0, c.1, c.3, kit::SOFT);
                tx += w + 6.0;
            }
            if let Some(left) = &row.left {
                put(
                    rc,
                    f.small.as_ref(),
                    &format!("{left} left"),
                    tx,
                    y + 26.0,
                    gold(1.0),
                );
            }
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            room.0 + room.2 - 5.0,
            room.1,
            room.3,
            total,
            self.scroll[2].at,
        );
        put(
            rc,
            f.small.as_ref(),
            "Kept on this computer, one journal per character.",
            PAD,
            BOTTOM - 14.0,
            MUTED,
        );
        // The page.
        let pb = Self::page_box();
        kit::panel(rc, pb, 0.3);
        if n.current == 0 {
            put_mid(
                rc,
                f.body.as_ref(),
                "Press New to start a page.",
                pb.0 + 16.0,
                pb.1,
                60.0,
                MUTED,
            );
            return;
        }
        self.draw_page(rc, h, &n, pb);
    }

    #[allow(clippy::too_many_lines)]
    fn draw_page(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>, n: &NotesScreen, pb: Rect) {
        let f = &self.fonts;
        let x = pb.0 + 16.0;
        let w = pb.2 - 32.0;
        // The page buttons.
        let y = pb.1 + 14.0;
        let mut bx = x - 4.0;
        let last = n.count;
        let nav = |rc: &mut RenderContext<'_>,
                   h: &mut Hits<Hit>,
                   bx: f32,
                   glyph: &[&[(f32, f32)]],
                   to: usize,
                   on: bool| {
            let r = (bx, y, 26.0, 26.0);
            let lit = on && h.add(r, Hit::Ask(JournalAsk::Goto(to)));
            if lit {
                rc.fill_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(4.0),
                    hex(0xFF_FFFF, 0.06),
                );
            }
            stroke(
                rc,
                r.0 + 5.0,
                r.1 + 5.0,
                16.0,
                glyph,
                if on { kit::WARM } else { hex(0xA8_9A82, 0.4) },
                1.8,
            );
        };
        nav(rc, h, bx, FIRST, 1, n.current > 1);
        bx += 28.0;
        nav(
            rc,
            h,
            bx,
            PREV,
            n.current.saturating_sub(1).max(1),
            n.current > 1,
        );
        bx += 32.0;
        let words = format!("Page {} of {last}", n.current);
        put_mid(rc, f.body.as_ref(), &words, bx, y, 26.0, kit::SOFT);
        bx += f
            .body
            .as_ref()
            .map_or(80.0, |b| text::measure(b, &words, 0.0))
            + 6.0;
        nav(rc, h, bx, NEXT, (n.current + 1).min(last), n.current < last);
        bx += 28.0;
        nav(rc, h, bx, LAST, last, n.current < last);
        let words = "Delete page…";
        let dw = kit::button_width(words, f.strong.as_ref());
        self.button(
            rc,
            h,
            (x + w - dw, y - 1.0, dw, 28.0),
            words,
            ButtonLook::Danger,
            true,
            Hit::Ask(JournalAsk::Delete(n.current)),
        );
        // Label, title, notes.
        let counter = |rc: &mut RenderContext<'_>, slot: Slot, words: &str, r: Rect| {
            put(rc, f.small.as_ref(), words, r.0, r.1 - 16.0, kit::SOFT);
            let c = format!(
                "{} / {}",
                self.count(slot),
                ac_ui::modern::numbers::group_thousands(slot.most() as i64)
            );
            put_right(rc, f.small.as_ref(), &c, r.0 + r.2, r.1 - 18.0, 16.0, MUTED);
        };
        for (slot, words, hint) in [
            (Slot::Label, "Label", "A short label"),
            (Slot::Title, "Title", "A title"),
            (Slot::Notes, "Notes", "Write anything here"),
        ] {
            let r = Self::input_box(slot);
            counter(rc, slot, words, r);
            kit::field_box(
                rc,
                &self.theme,
                r,
                hint,
                self.is_empty(slot),
                self.focused == Some(slot),
                f.body.as_ref(),
            );
        }
        // The place and the countdown.
        let top = pb.1 + pb.3 - 14.0 - BOX_H;
        let bw = (w - 10.0) * 0.5;
        let place = (x, top, bw, BOX_H);
        let round = CornerRadii::all(4.0);
        rc.stroke_rounded_rect(place.0, place.1, place.2, place.3, round, bronze(0.25), 1.0);
        spaced(
            rc,
            f.label.as_ref(),
            "PLACE",
            place.0 + 12.0,
            place.1 + 10.0,
        );
        let (words, ink) = match &n.place {
            Some(p) => (p.as_str(), self.theme.palette.text),
            None => ("Nowhere recorded", MUTED),
        };
        put(
            rc,
            f.strong.as_ref(),
            words,
            place.0 + 12.0,
            place.1 + 30.0,
            ink,
        );
        let rw = kit::button_width("Record where I am", f.strong.as_ref());
        self.button(
            rc,
            h,
            (place.0 + 12.0, place.1 + 60.0, rw, 28.0),
            "Record where I am",
            ButtonLook::Plain,
            true,
            Hit::Ask(JournalAsk::Record),
        );
        let at = (n.page.location_x, n.page.location_y);
        self.button(
            rc,
            h,
            (place.0 + 18.0 + rw, place.1 + 60.0, 60.0, 28.0),
            "Map",
            ButtonLook::Plain,
            n.place.is_some(),
            Hit::Ask(JournalAsk::MapAt(at.0, at.1)),
        );
        let timer = (x + bw + 10.0, top, bw, BOX_H);
        let running = n.left.is_some();
        if running {
            rc.fill_rounded_rect(timer.0, timer.1, timer.2, timer.3, round, gold(0.08));
        }
        rc.stroke_rounded_rect(
            timer.0,
            timer.1,
            timer.2,
            timer.3,
            round,
            if running { gold(0.6) } else { bronze(0.25) },
            1.0,
        );
        spaced(
            rc,
            f.label.as_ref(),
            "COUNTDOWN",
            timer.0 + 12.0,
            timer.1 + 10.0,
        );
        if let Some(left) = &n.left {
            put(
                rc,
                f.big.as_ref(),
                left,
                timer.0 + 12.0,
                timer.1 + 28.0,
                gold(1.0),
            );
            self.button(
                rc,
                h,
                (timer.0 + 12.0, timer.1 + 60.0, 70.0, 28.0),
                "Stop",
                ButtonLook::Plain,
                true,
                Hit::Ask(JournalAsk::Stop),
            );
            let started = format!(
                "started from {}d {}h {}m",
                n.page.timer_days, n.page.timer_hours, n.page.timer_minutes
            );
            put_mid(
                rc,
                f.small.as_ref(),
                &started,
                timer.0 + 92.0,
                timer.1 + 60.0,
                28.0,
                MUTED,
            );
        } else {
            for (slot, unit) in [(Slot::Days, "d"), (Slot::Hours, "h"), (Slot::Minutes, "m")] {
                let r = Self::input_box(slot);
                kit::field_box(
                    rc,
                    &self.theme,
                    r,
                    "",
                    true,
                    self.focused == Some(slot),
                    f.body.as_ref(),
                );
                put_mid(
                    rc,
                    f.small.as_ref(),
                    unit,
                    r.0 + r.2 + 5.0,
                    r.1,
                    r.3,
                    kit::SOFT,
                );
            }
            let set = !(self.is_empty(Slot::Days)
                && self.is_empty(Slot::Hours)
                && self.is_empty(Slot::Minutes));
            let ok = set && !n.other_running;
            self.button(
                rc,
                h,
                (timer.0 + timer.2 - 12.0 - 64.0, timer.1 + 32.0, 64.0, 28.0),
                "Start",
                ButtonLook::Plain,
                ok,
                Hit::Ask(JournalAsk::Start),
            );
            let note = if n.other_running {
                "Another page's countdown runs."
            } else {
                "Counts down even while you are away."
            };
            if let Some(sm) = &f.small {
                put(
                    rc,
                    Some(sm),
                    &kit::fit(sm, note, timer.2 - 24.0),
                    timer.0 + 12.0,
                    timer.1 + 70.0,
                    MUTED,
                );
            }
        }
    }
}

/// A quest state's pill colours.
fn state_colors(state: QuestState) -> (Color, Color) {
    match state {
        QuestState::Active => (gold(1.0), gold(0.6)),
        QuestState::Waiting => (hex(0x8F_B8FF, 1.0), hex(0x8F_B8FF, 0.5)),
        QuestState::Available => (kit::GOOD, hex(0x7F_D08A, 0.5)),
        QuestState::Done => (kit::SOFT, bronze(0.35)),
    }
}

/// An eye at (`x`, `y`), 16 across.
fn eye(rc: &mut RenderContext<'_>, x: f32, y: f32, c: Color) {
    stroke(rc, x, y, 16.0, EYE, c, 1.5);
    circle(rc, x, y, 16.0, (12.0, 12.0), 3.0, c, 1.5);
}

impl Widget for JournalFrame {
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
                self.hits.hover = Some((x, y));
                false
            }
            kind::HOVER_LEAVE => {
                self.hits.hover = None;
                false
            }
            kind::CLICK => {
                self.click(x, y);
                true
            }
            kind::RIGHT_CLICK => true,
            kind::SCROLL => {
                let Some((hx, hy)) = self.hits.hover else {
                    return false;
                };
                for (i, (total, room)) in self.rooms.iter().enumerate() {
                    if inside(*room, hx, hy) {
                        self.scroll[i].wheel(e.data0, 46.0, *total, room.3);
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let mut h = Hits::new(self.hits.hover);
        self.draw_frame(rc, &mut h);
        match self.tab {
            JournalTab::Quests => self.draw_quests(rc, &mut h),
            JournalTab::Notes => self.draw_notes(rc, &mut h),
        }
        self.hits = h;
    }
}
