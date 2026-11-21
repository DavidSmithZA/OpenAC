//! The Options window's frame: the header with its search, the section
//! list with Character select and Exit game under it, the section's groups
//! of settings (or every setting the search matches, each with where it
//! lives), and the foot with what changed, Defaults, Revert and Apply.
//!
//! The frame holds the values (see [`OptionsModel`]) and changes them
//! itself; what it asks of the game window is in [`OptionsRequest`].

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::options::OptionValue;
use ac_ui::panels::options::keys::{OptionKey, Section as Stored};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::{Control, OptionsModel, Row, Shown, number, slider_words};
use super::spec::{self, Action, Section};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{self, ButtonLook, Rect, Scroll, hot, inside};

/// The window's size.
pub const WIDTH: f32 = 1000.0;
pub const HEIGHT: f32 = 660.0;
const HEADER: f32 = 52.0;
const NAV_W: f32 = 210.0;
const FOOT: f32 = 60.0;
const PAD_X: f32 = 28.0;
const CONTROL_W: f32 = 300.0;
const ROOM_TOP: f32 = HEADER;
const ROOM: f32 = HEIGHT - HEADER - FOOT;
const ROW_MIN: f32 = 52.0;
const GRID_ROW: f32 = 30.0;
const GRID_LABEL_W: f32 = 190.0;
const MENU_ROW: f32 = 28.0;
const NAV_ROW: f32 = 36.0;
const NAV_GAP: f32 = 2.0;

/// The window's glyph: a cog.
const COG: &[&[(f32, f32)]] = &[
    &[(12.0, 2.0), (12.0, 5.0)],
    &[(12.0, 19.0), (12.0, 22.0)],
    &[(2.0, 12.0), (5.0, 12.0)],
    &[(19.0, 12.0), (22.0, 12.0)],
    &[(4.9, 4.9), (7.0, 7.0)],
    &[(17.0, 17.0), (19.1, 19.1)],
    &[(4.9, 19.1), (7.0, 17.0)],
    &[(17.0, 7.0), (19.1, 4.9)],
    &[
        (12.0, 9.0),
        (14.1, 9.9),
        (15.0, 12.0),
        (14.1, 14.1),
        (12.0, 15.0),
        (9.9, 14.1),
        (9.0, 12.0),
        (9.9, 9.9),
        (12.0, 9.0),
    ],
];
/// A tick.
const TICK: &[&[(f32, f32)]] = &[&[(5.0, 12.5), (10.0, 17.0), (19.0, 7.0)]];
/// A chevron down.
const CHEVRON: &[&[(f32, f32)]] = &[&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]];

/// The fonts the frame draws with.
#[derive(Debug, Clone, Default)]
pub struct OptionsFonts {
    /// The window's title.
    pub title: Option<Arc<UiFont>>,
    /// The section list.
    pub nav: Option<Arc<UiFont>>,
    /// The section's title.
    pub heading: Option<Arc<UiFont>>,
    /// The section's line, the search hint.
    pub intro: Option<Arc<UiFont>>,
    /// Group headings (spaced capitals).
    pub label: Option<Arc<UiFont>>,
    /// A setting's name.
    pub name: Option<Arc<UiFont>>,
    /// What it does.
    pub desc: Option<Arc<UiFont>>,
    /// Values and buttons.
    pub control: Option<Arc<UiFont>>,
    /// The OPENAC tag, notes, where a match lives.
    pub small: Option<Arc<UiFont>>,
}

/// What the frame asks of the game window.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionsRequest {
    /// Close it.
    Close,
    /// A stored section of the values changed: make it live; save it
    /// unless the second is off (a slider still held).
    Changed(Stored, bool),
    /// Apply: the character's options are saved.
    Applied,
    /// Character select.
    CharacterSelect,
    /// Exit the game.
    ExitGame,
    /// A button.
    Act(Action),
    /// Chat tab `tab` shows the message types of group `group` (or not).
    TabShows {
        /// The tab.
        tab: usize,
        /// The group of message types.
        group: usize,
        /// Shown.
        on: bool,
    },
    /// Empty the search.
    ClearSearch,
    /// Catch the next key for slot `slot` of keyboard row `row`.
    CaptureKey {
        /// The keyboard row.
        row: usize,
        /// The slot.
        slot: usize,
    },
    /// Empty slot `slot` of keyboard row `row`.
    EraseKey {
        /// The keyboard row.
        row: usize,
        /// The slot.
        slot: usize,
    },
    /// Take the clashing key from what holds it.
    TakeKey,
    /// The keys back to the saved ones.
    KeysRevert,
    /// The keys to the game's defaults.
    KeysDefaults,
}

/// A chat tab as the grid shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridTab {
    /// Its name.
    pub name: String,
    /// The message types it shows.
    pub filter: u64,
}

/// A key binding as the Controls section lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyLine {
    /// Its row in the keyboard's list.
    pub row: usize,
    /// The action's name.
    pub label: String,
    /// The heading it is listed under.
    pub group: &'static str,
    /// The names of its keys, in slot order.
    pub keys: Vec<String>,
    /// It differs from the saved keys.
    pub changed: bool,
    /// The client has an action for it (else its keys are only kept).
    pub mapped: bool,
}

/// A key the player pressed that something else holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clash {
    /// The keyboard row it was pressed for.
    pub row: usize,
    /// The slot.
    pub slot: usize,
    /// What holds it, in words.
    pub words: String,
    /// It can be taken from there (else the game keeps it).
    pub can_take: bool,
}

/// The keys each row has room for.
pub const KEY_SLOTS: usize = 3;
const KEY_ROW: f32 = 40.0;
const CLASH_H: f32 = 44.0;

/// A clickable part of the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Part {
    Close,
    Nav(Section),
    CharacterSelect,
    ExitGame,
    Defaults,
    Revert,
    Apply,
    /// A row's switch (a volume's included).
    Switch(usize),
    /// A row's slider (a volume's included).
    Track(usize),
    /// A row's choice by index, shown as a segment.
    Segment(usize, usize),
    /// A row's menu button.
    Drop(usize),
    /// A choice in the open menu.
    MenuChoice(usize),
    /// A row's button.
    Button(usize),
    /// The grid's box of tab and group.
    Cell(usize, usize),
    /// A key line's slot.
    KeySlot(usize, usize),
    /// Take the clashing key.
    ClashTake,
    /// Leave it.
    ClashKeep,
}

/// One line of the scrolled content.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Line {
    /// The section's (or the search's) title and line.
    Title,
    /// A group's heading: its words index (a group, or a section when
    /// searching) and whether its settings live on the character.
    Heading(usize, bool),
    /// A row.
    Row(usize),
    /// Nothing matched.
    Nothing,
    /// A heading over key lines.
    KeyHeading(&'static str),
    /// A key line (an index of the frame's keys).
    Key(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Placed {
    line: Line,
    y: f32,
    h: f32,
}

/// The Options window's frame.
#[derive(Debug)]
pub struct OptionsFrame {
    theme: Horizon,
    fonts: OptionsFonts,
    /// The rows and their values.
    pub model: OptionsModel,
    /// The section shown.
    pub section: Section,
    /// What is typed in the search.
    pub search: String,
    /// The search field has the keyboard.
    pub searching: bool,
    /// The chat tabs, for the grid.
    pub tabs: Vec<GridTab>,
    /// What the frame asks of the window.
    pub requests: Vec<OptionsRequest>,
    /// The key bindings (Controls).
    pub keys: Vec<KeyLine>,
    /// How many of them differ from the saved keys.
    pub keys_changed: usize,
    /// The keyboard row and slot waiting for a key.
    pub capture: Option<(usize, usize)>,
    /// A key pressed that something else holds.
    pub clash: Option<Clash>,
    scroll: Scroll,
    hover: Option<(f32, f32)>,
    /// The row whose menu is open.
    menu: Option<usize>,
    /// The row whose slider is held.
    held: Option<usize>,
    /// What the held slider changed so far (saved when it is let go).
    slid: Option<Stored>,
}

/// The words of row group `g` and whether its settings are saved on the
/// character (every one of them is a character option).
fn group_on_character(rows: &[Row], g: usize) -> bool {
    let keyed: Vec<&Row> = rows
        .iter()
        .filter(|r| r.group == g && r.key.is_some())
        .collect();
    !keyed.is_empty()
        && keyed
            .iter()
            .all(|r| matches!(r.key, Some(OptionKey::Character(_))))
}

impl OptionsFrame {
    /// A frame over `model`, on Interface.
    #[must_use]
    pub fn new(theme: Horizon, fonts: OptionsFonts, model: OptionsModel) -> Self {
        Self {
            theme,
            fonts,
            model,
            section: Section::Interface,
            search: String::new(),
            searching: false,
            tabs: Vec::new(),
            requests: Vec::new(),
            scroll: Scroll::default(),
            hover: None,
            menu: None,
            held: None,
            slid: None,
            keys: Vec::new(),
            keys_changed: 0,
            capture: None,
            clash: None,
        }
    }

    /// The search box's place.
    #[must_use]
    pub fn search_rect() -> Rect {
        (WIDTH - 12.0 - 30.0 - 12.0 - 300.0, 11.0, 300.0, 30.0)
    }

    /// Show `section` from its top.
    pub fn show(&mut self, section: Section) {
        self.section = section;
        self.scroll = Scroll::default();
        self.menu = None;
    }

    fn searching_rows(&self) -> Option<Vec<usize>> {
        let q = self.search.trim();
        (!q.is_empty()).then(|| self.model.search(q))
    }

    fn content_x() -> f32 {
        NAV_W + PAD_X
    }

    fn content_right() -> f32 {
        WIDTH - PAD_X
    }

    fn desc_width() -> f32 {
        Self::content_right() - Self::content_x() - CONTROL_W - 24.0
    }

    fn desc_lines(&self, row: &Row) -> Vec<String> {
        match (&self.fonts.desc, row.desc.is_empty()) {
            (Some(f), false) => kit::wrap(f, &row.desc, Self::desc_width()),
            _ => Vec::new(),
        }
    }

    fn row_height(&self, i: usize) -> f32 {
        let row = &self.model.rows[i];
        if row.control == Control::ChatGrid {
            return GRID_ROW * (crate::ui::modern::chat::model::GROUPS.len() as f32 + 1.0) + 12.0;
        }
        let lines = self.desc_lines(row).len() as f32;
        (16.0 + 18.0 + lines * 16.0 + if lines > 0.0 { 3.0 } else { 0.0 }).max(ROW_MIN)
    }

    /// The content's lines from its top, and its height.
    fn layout(&self) -> (Vec<Placed>, f32) {
        let mut out = Vec::new();
        let mut y = 18.0;
        out.push(Placed {
            line: Line::Title,
            y,
            h: 46.0,
        });
        y += 46.0 + 22.0;
        let rows = &self.model.rows;
        let heading =
            |out: &mut Vec<Placed>, y: &mut f32, words: usize, server: bool, first: bool| {
                if !first {
                    *y += 18.0;
                }
                out.push(Placed {
                    line: Line::Heading(words, server),
                    y: *y,
                    h: 28.0,
                });
                *y += 28.0;
            };
        if let Some(found) = self.searching_rows() {
            let key_found = self.key_matches();
            if found.is_empty() && key_found.is_empty() {
                out.push(Placed {
                    line: Line::Nothing,
                    y,
                    h: 40.0,
                });
                return (out, y + 40.0 + 24.0);
            }
            let mut first = true;
            for (s, section) in Section::ALL.iter().enumerate() {
                let in_it: Vec<usize> = found
                    .iter()
                    .copied()
                    .filter(|i| rows[*i].section == *section)
                    .collect();
                if in_it.is_empty() {
                    continue;
                }
                let server = in_it
                    .iter()
                    .all(|i| matches!(rows[*i].key, Some(OptionKey::Character(_))));
                heading(&mut out, &mut y, s, server, first);
                first = false;
                for i in in_it {
                    let h = self.row_height(i);
                    out.push(Placed {
                        line: Line::Row(i),
                        y,
                        h,
                    });
                    y += h;
                }
            }
            if !key_found.is_empty() {
                self.push_keys(&mut out, &mut y, &key_found, first);
            }
            return (out, y + 24.0);
        }
        let mut last_group = None;
        let mut first = true;
        for (i, row) in rows.iter().enumerate() {
            if row.section != self.section || !self.model.shows(row) {
                continue;
            }
            if last_group != Some(row.group) {
                last_group = Some(row.group);
                let server = group_on_character(rows, row.group);
                heading(&mut out, &mut y, row.group, server, first);
                first = false;
            }
            let h = self.row_height(i);
            out.push(Placed {
                line: Line::Row(i),
                y,
                h,
            });
            y += h;
        }
        if self.section == Section::Controls {
            let all: Vec<usize> = (0..self.keys.len()).collect();
            self.push_keys(&mut out, &mut y, &all, first);
        }
        (out, y + 24.0)
    }

    /// The key lines (by index) whose action's name holds every word of
    /// the search.
    fn key_matches(&self) -> Vec<usize> {
        let words: Vec<String> = self
            .search
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        if words.is_empty() {
            return Vec::new();
        }
        self.keys
            .iter()
            .enumerate()
            .filter(|(_, k)| {
                let hay = format!("{} {} keys", k.label, k.group).to_lowercase();
                words.iter().all(|w| hay.contains(w.as_str()))
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn key_height(&self, k: usize) -> f32 {
        let clash = self
            .clash
            .as_ref()
            .is_some_and(|c| self.keys.get(k).is_some_and(|l| l.row == c.row));
        KEY_ROW + if clash { CLASH_H } else { 0.0 }
    }

    /// Key lines `which` under their headings.
    fn push_keys(&self, out: &mut Vec<Placed>, y: &mut f32, which: &[usize], first: bool) {
        let mut last = None;
        let mut first = first;
        for &k in which {
            let Some(line) = self.keys.get(k) else {
                continue;
            };
            if last != Some(line.group) {
                last = Some(line.group);
                if !first {
                    *y += 18.0;
                }
                first = false;
                out.push(Placed {
                    line: Line::KeyHeading(line.group),
                    y: *y,
                    h: 28.0,
                });
                *y += 28.0;
            }
            let h = self.key_height(k);
            out.push(Placed {
                line: Line::Key(k),
                y: *y,
                h,
            });
            *y += h;
        }
    }

    /// Key line `k`'s slots (and its clash's buttons), drawn from `top`.
    fn key_parts(&self, k: usize, top: f32) -> Vec<(Part, Rect)> {
        let right = Self::content_right();
        let (w, gap) = (108.0, 6.0);
        let y = top + (KEY_ROW - 28.0) * 0.5;
        let mut out: Vec<(Part, Rect)> = (0..KEY_SLOTS)
            .map(|s| {
                let x = right - (KEY_SLOTS - s) as f32 * (w + gap) + gap;
                (Part::KeySlot(k, s), (x, y, w, 28.0))
            })
            .collect();
        if let (Some(c), Some(line)) = (&self.clash, self.keys.get(k))
            && c.row == line.row
        {
            let f = self.fonts.control.as_ref();
            let by = top + KEY_ROW + (CLASH_H - 4.0 - 26.0) * 0.5;
            let keep_w = kit::button_width("Keep", f);
            let keep = (right - 8.0 - keep_w, by, keep_w, 26.0);
            out.push((Part::ClashKeep, keep));
            if c.can_take {
                let take_w = kit::button_width("Use it here", f);
                out.push((Part::ClashTake, (keep.0 - 6.0 - take_w, by, take_w, 26.0)));
            }
        }
        out
    }

    fn choice_index(row: &Row, value: Option<&OptionValue>) -> Option<usize> {
        let Control::Choice { values, .. } = &row.control else {
            return None;
        };
        values.iter().position(|v| Some(v) == value)
    }

    fn segments_fit(&self, labels: &[String]) -> bool {
        let Some(f) = &self.fonts.control else {
            return false;
        };
        labels.len() <= 4
            && labels
                .iter()
                .map(|l| text::measure(f, l, 0.0) + 28.0)
                .sum::<f32>()
                <= CONTROL_W
    }

    fn drop_width(&self, labels: &[String]) -> f32 {
        let widest = self.fonts.control.as_ref().map_or(140.0, |f| {
            labels
                .iter()
                .map(|l| text::measure(f, l, 0.0))
                .fold(0.0, f32::max)
        });
        (widest + 44.0).clamp(180.0, CONTROL_W)
    }

    /// Row `i`'s parts, drawn `top` (in the window) and `h` high.
    fn row_parts(&self, i: usize, top: f32, h: f32) -> Vec<(Part, Rect)> {
        let row = &self.model.rows[i];
        let right = Self::content_right();
        let cy = top + (h * 0.5).round();
        match &row.control {
            Control::Switch { .. } => {
                vec![(Part::Switch(i), (right - 38.0, cy - 10.0, 38.0, 20.0))]
            }
            Control::Slider { .. } => {
                vec![(
                    Part::Track(i),
                    (right - 60.0 - 8.0 - 200.0, cy - 10.0, 200.0, 20.0),
                )]
            }
            Control::Volume { .. } => {
                let track = (right - 50.0 - 8.0 - 180.0, cy - 10.0, 180.0, 20.0);
                vec![
                    (
                        Part::Switch(i),
                        (track.0 - 16.0 - 38.0, cy - 10.0, 38.0, 20.0),
                    ),
                    (Part::Track(i), track),
                ]
            }
            Control::Choice { labels, .. } => {
                if self.segments_fit(labels) {
                    let Some(f) = &self.fonts.control else {
                        return Vec::new();
                    };
                    let widths: Vec<f32> = labels
                        .iter()
                        .map(|l| text::measure(f, l, 0.0) + 28.0)
                        .collect();
                    let mut x = right - widths.iter().sum::<f32>();
                    widths
                        .iter()
                        .enumerate()
                        .map(|(k, w)| {
                            let r = (x, cy - 15.0, *w, 30.0);
                            x += w;
                            (Part::Segment(i, k), r)
                        })
                        .collect()
                } else {
                    let w = self.drop_width(labels);
                    vec![(Part::Drop(i), (right - w, cy - 15.0, w, 30.0))]
                }
            }
            Control::Button(_, verb) => {
                let w = kit::button_width(verb, self.fonts.control.as_ref());
                vec![(Part::Button(i), (right - w, cy - 15.0, w, 30.0))]
            }
            Control::ChatGrid => {
                let x0 = Self::content_x();
                let n = self.tabs.len().max(1) as f32;
                let col = (right - x0 - GRID_LABEL_W) / n;
                let mut out = Vec::new();
                for g in 0..crate::ui::modern::chat::model::GROUPS.len() {
                    let y = top + GRID_ROW * (g as f32 + 1.0);
                    for t in 0..self.tabs.len() {
                        let cx = x0 + GRID_LABEL_W + col * (t as f32 + 0.5);
                        out.push((Part::Cell(t, g), (cx - 12.0, y + 3.0, 24.0, 24.0)));
                    }
                }
                out
            }
        }
    }

    /// The open menu's choices' places.
    fn menu_parts(&self, layout: &[Placed]) -> Vec<(Part, Rect)> {
        let Some(i) = self.menu else {
            return Vec::new();
        };
        let Some(p) = layout.iter().find(|p| p.line == Line::Row(i)) else {
            return Vec::new();
        };
        let top = ROOM_TOP + p.y - self.scroll.at;
        let Some((_, drop)) = self
            .row_parts(i, top, p.h)
            .into_iter()
            .find(|(part, _)| matches!(part, Part::Drop(_)))
        else {
            return Vec::new();
        };
        let Control::Choice { labels, .. } = &self.model.rows[i].control else {
            return Vec::new();
        };
        let n = labels.len() as f32;
        let h = n * MENU_ROW + 8.0;
        let below = drop.1 + drop.3 + 4.0;
        let y = if below + h > HEIGHT - 8.0 {
            (drop.1 - 4.0 - h).max(8.0)
        } else {
            below
        };
        (0..labels.len())
            .map(|k| {
                (
                    Part::MenuChoice(k),
                    (drop.0, y + 4.0 + k as f32 * MENU_ROW, drop.2, MENU_ROW),
                )
            })
            .collect()
    }

    fn nav_rect(k: usize) -> Rect {
        (
            12.0,
            HEADER + 14.0 + k as f32 * (NAV_ROW + NAV_GAP),
            NAV_W - 24.0,
            NAV_ROW,
        )
    }

    fn foot_buttons(&self) -> [(Part, Rect); 3] {
        let f = self.fonts.control.as_ref();
        let y = HEIGHT - FOOT + (FOOT - 32.0) * 0.5;
        let apply_w = kit::button_width("Apply", f) + 8.0;
        let revert_w = kit::button_width("Revert", f);
        let defaults_w = kit::button_width("Defaults", f);
        let apply = (WIDTH - 20.0 - apply_w, y, apply_w, 32.0);
        let revert = (apply.0 - 8.0 - revert_w, y, revert_w, 32.0);
        let defaults = (revert.0 - 8.0 - defaults_w, y, defaults_w, 32.0);
        [
            (Part::Defaults, defaults),
            (Part::Revert, revert),
            (Part::Apply, apply),
        ]
    }

    /// Every part under (`x`, `y`), the open menu first.
    fn part_at(&self, x: f32, y: f32) -> Option<Part> {
        let (layout, _) = self.layout();
        if self.menu.is_some() {
            return self
                .menu_parts(&layout)
                .into_iter()
                .find(|(_, r)| inside(*r, x, y))
                .map(|(p, _)| p);
        }
        if inside(kit::close_rect(WIDTH, HEADER), x, y) {
            return Some(Part::Close);
        }
        for (k, s) in Section::ALL.iter().enumerate() {
            if inside(Self::nav_rect(k), x, y) {
                return Some(Part::Nav(*s));
            }
        }
        let (select, exit) = Self::nav_foot();
        if inside(select, x, y) {
            return Some(Part::CharacterSelect);
        }
        if inside(exit, x, y) {
            return Some(Part::ExitGame);
        }
        for (p, r) in self.foot_buttons() {
            if inside(r, x, y) {
                return Some(p);
            }
        }
        if !inside((NAV_W, ROOM_TOP, WIDTH - NAV_W, ROOM), x, y) {
            return None;
        }
        for p in &layout {
            let top = ROOM_TOP + p.y - self.scroll.at;
            if y < top || y >= top + p.h {
                continue;
            }
            let parts = match p.line {
                Line::Row(i) => self.row_parts(i, top, p.h),
                Line::Key(k) => self.key_parts(k, top),
                _ => continue,
            };
            return parts
                .into_iter()
                .find(|(_, r)| inside(*r, x, y))
                .map(|(p, _)| p);
        }
        None
    }

    fn nav_foot() -> (Rect, Rect) {
        let exit = (12.0, HEIGHT - 14.0 - 32.0, NAV_W - 24.0, 32.0);
        let select = (12.0, exit.1 - 6.0 - 32.0, NAV_W - 24.0, 32.0);
        (select, exit)
    }

    fn changed(&mut self, section: Option<Stored>, persist: bool) {
        if let Some(s) = section {
            self.requests.push(OptionsRequest::Changed(s, persist));
        }
    }

    fn set(&mut self, key: OptionKey, value: &OptionValue, persist: bool) {
        let s = self.model.set(key, value);
        self.changed(s, persist);
    }

    fn row_available(&self, i: usize) -> bool {
        !self.model.unavailable(&self.model.rows[i])
    }

    fn slide(&mut self, i: usize, x: f32, persist: bool) {
        let row = self.model.rows[i].clone();
        let (min, max, step) = match &row.control {
            Control::Slider { min, max, step, .. } => (*min, *max, *step),
            Control::Volume { .. } => (0.0, 1.0, 0.0),
            _ => return,
        };
        let (layout, _) = self.layout();
        let Some(p) = layout.iter().find(|p| p.line == Line::Row(i)) else {
            return;
        };
        let top = ROOM_TOP + p.y - self.scroll.at;
        let Some((_, track)) = self
            .row_parts(i, top, p.h)
            .into_iter()
            .find(|(part, _)| matches!(part, Part::Track(_)))
        else {
            return;
        };
        let share = f64::from(((x - track.0) / track.2).clamp(0.0, 1.0));
        let mut v = min + share * (max - min);
        if step > 0.0 {
            v = min + ((v - min) / step).round() * step;
        }
        if let Some(key) = row.key {
            let changed = self.model.set(key, &OptionValue::Float(v as f32));
            if changed.is_some() {
                self.slid = changed;
            }
            self.changed(changed, persist);
        }
    }

    fn click(&mut self, x: f32, y: f32) {
        let part = self.part_at(x, y);
        if self.menu.is_some() {
            if let (Some(Part::MenuChoice(k)), Some(i)) = (part, self.menu) {
                let row = self.model.rows[i].clone();
                if let (Control::Choice { values, .. }, Some(key)) = (&row.control, row.key)
                    && let Some(v) = values.get(k)
                {
                    self.set(key, v, true);
                }
            }
            self.menu = None;
            return;
        }
        let Some(part) = part else {
            return;
        };
        match part {
            Part::Close => self.requests.push(OptionsRequest::Close),
            Part::Nav(s) => {
                if !self.search.is_empty() {
                    self.requests.push(OptionsRequest::ClearSearch);
                    self.search.clear();
                }
                self.show(s);
            }
            Part::CharacterSelect => self.requests.push(OptionsRequest::CharacterSelect),
            Part::ExitGame => self.requests.push(OptionsRequest::ExitGame),
            Part::Defaults => {
                // While searching, only the rows found go back.
                let touched = match self.searching_rows() {
                    Some(rows) => self.model.defaults_of(&rows),
                    None => self.model.defaults(Some(self.section)),
                };
                for s in touched {
                    self.requests.push(OptionsRequest::Changed(s, true));
                }
                if self.search.trim().is_empty() && self.section == Section::Controls {
                    self.requests.push(OptionsRequest::KeysDefaults);
                }
            }
            Part::Revert => {
                for s in self.model.revert() {
                    self.requests.push(OptionsRequest::Changed(s, true));
                }
                if self.keys_changed > 0 {
                    self.requests.push(OptionsRequest::KeysRevert);
                }
                self.clash = None;
            }
            Part::Apply => {
                self.model.applied();
                self.requests.push(OptionsRequest::Applied);
            }
            Part::Switch(i) if self.row_available(i) => {
                let row = self.model.rows[i].clone();
                let key = match row.control {
                    Control::Volume { switch } => Some(switch),
                    _ => row.key,
                };
                if let Some(key) = key {
                    let on = self.model.value(key) == Some(OptionValue::Bool(true));
                    self.set(key, &OptionValue::Bool(!on), true);
                }
            }
            Part::Segment(i, k) if self.row_available(i) => {
                let row = self.model.rows[i].clone();
                if let (Control::Choice { values, .. }, Some(key)) = (&row.control, row.key)
                    && let Some(v) = values.get(k)
                {
                    self.set(key, v, true);
                }
            }
            Part::Drop(i) if self.row_available(i) => self.menu = Some(i),
            Part::Button(i) => {
                if let Control::Button(action, _) = self.model.rows[i].control {
                    self.requests.push(OptionsRequest::Act(action));
                }
            }
            Part::KeySlot(k, slot) => {
                if let Some(line) = self.keys.get(k) {
                    let row = line.row;
                    self.clash = None;
                    self.capture = Some((row, slot));
                    self.requests.push(OptionsRequest::CaptureKey { row, slot });
                }
            }
            Part::ClashTake => {
                self.clash = None;
                self.requests.push(OptionsRequest::TakeKey);
            }
            Part::ClashKeep => self.clash = None,
            Part::Cell(t, g) => {
                if let Some(tab) = self.tabs.get_mut(t) {
                    let mask = crate::ui::modern::chat::model::group_mask(g);
                    let on = tab.filter & mask != mask;
                    if on {
                        tab.filter |= mask;
                    } else {
                        tab.filter &= !mask;
                    }
                    self.requests.push(OptionsRequest::TabShows {
                        tab: t,
                        group: g,
                        on,
                    });
                }
            }
            _ => {}
        }
    }
}

impl Widget for OptionsFrame {
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
                if let Some(i) = self.held {
                    self.slide(i, x, false);
                    return true;
                }
                false
            }
            kind::HOVER_LEAVE => {
                self.hover = None;
                false
            }
            kind::MOUSE_DOWN => {
                if self.menu.is_none()
                    && let Some(Part::Track(i)) = self.part_at(x, y)
                    && self.row_available(i)
                {
                    self.held = Some(i);
                    self.slide(i, x, false);
                }
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                if let Some(i) = self.held.take() {
                    // A release moves it a last time; a lost capture (no
                    // place) keeps it where it is. Either way what the
                    // hold changed is saved now.
                    if e.kind == kind::MOUSE_UP {
                        self.slide(i, x, false);
                    }
                    let slid = self.slid.take();
                    self.changed(slid, true);
                }
                true
            }
            kind::SCROLL => {
                self.menu = None;
                let (_, total) = self.layout();
                self.scroll.wheel(e.data0, 3.0 * ROW_MIN, total, ROOM);
                true
            }
            kind::RIGHT_CLICK => {
                if let Some(Part::KeySlot(k, slot)) = self.part_at(x, y)
                    && let Some(line) = self.keys.get(k)
                {
                    self.requests.push(OptionsRequest::EraseKey {
                        row: line.row,
                        slot,
                    });
                }
                true
            }
            kind::CLICK => {
                if !matches!(self.part_at(x, y), Some(Part::Track(_))) || self.menu.is_some() {
                    self.click(x, y);
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let (layout, total) = self.layout();
        self.scroll.hold(total, ROOM);
        self.draw_frame(rc);
        self.draw_nav(rc);
        self.draw_content(rc, &layout, total);
        self.draw_foot(rc);
        self.draw_menu(rc, &layout);
    }
}

impl OptionsFrame {
    fn draw_frame(&self, rc: &mut RenderContext<'_>) {
        let theme = &self.theme;
        kit::window_back(rc, theme, WIDTH, HEIGHT, HEADER, true);
        kit::title(rc, theme, self.fonts.title.as_ref(), COG, "Options", HEADER);
        let c = kit::close_rect(WIDTH, HEADER);
        kit::close(rc, theme, c, hot(self.hover, c));
        kit::search_box(
            rc,
            theme,
            Self::search_rect(),
            "Search all settings",
            self.search.is_empty() && !self.searching,
            self.searching || !self.search.is_empty(),
            self.fonts.intro.as_ref(),
        );
    }

    fn draw_nav(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        rc.fill(0.0, HEADER, NAV_W, HEIGHT - HEADER, hex(0, 0.22));
        rc.fill(NAV_W - 1.0, HEADER, 1.0, HEIGHT - HEADER, kit::bronze(0.25));
        let searching = !self.search.trim().is_empty();
        for (k, s) in Section::ALL.iter().enumerate() {
            let r = Self::nav_rect(k);
            let current = !searching && *s == self.section;
            let lit = hot(self.hover, r);
            let radius = CornerRadii::all(4.0);
            if current {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, kit::gold(0.16));
                rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, kit::gold(1.0), 1.0);
            } else if lit {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, hex(0xFF_FFFF, 0.05));
            }
            let ic = if current { kit::gold(1.0) } else { kit::SOFT };
            stroke(rc, r.0 + 10.0, r.1 + 10.0, 16.0, s.glyph(), ic, 1.8);
            if let Some(f) = &self.fonts.nav {
                let y = r.1 + ((r.3 - f.line_height) * 0.5).round();
                let fg = if current { p.text } else { kit::WARM };
                text::draw(rc, f, s.words(), r.0 + 36.0, y, TextStyle::plain(fg));
            }
            if self.model.changed_in(*s) > 0 || (*s == Section::Controls && self.keys_changed > 0) {
                rc.fill_rounded_rect(
                    r.0 + r.2 - 16.0,
                    r.1 + r.3 * 0.5 - 3.0,
                    6.0,
                    6.0,
                    CornerRadii::all(3.0),
                    kit::gold(1.0),
                );
            }
        }
        let (select, exit) = Self::nav_foot();
        rc.fill(12.0, select.1 - 12.0, NAV_W - 24.0, 1.0, kit::bronze(0.25));
        kit::button(
            rc,
            &self.theme,
            select,
            "Character select",
            self.fonts.control.as_ref(),
            ButtonLook::Plain,
            hot(self.hover, select),
            true,
        );
        kit::button(
            rc,
            &self.theme,
            exit,
            "Exit game",
            self.fonts.control.as_ref(),
            ButtonLook::Danger,
            hot(self.hover, exit),
            true,
        );
    }

    fn draw_content(&self, rc: &mut RenderContext<'_>, layout: &[Placed], total: f32) {
        let p = self.theme.palette;
        let x0 = Self::content_x();
        let right = Self::content_right();
        rc.push_clip(NAV_W, ROOM_TOP, WIDTH - NAV_W, ROOM);
        let found = self.searching_rows();
        for placed in layout {
            let top = ROOM_TOP + placed.y - self.scroll.at;
            if top > ROOM_TOP + ROOM || top + placed.h < ROOM_TOP {
                continue;
            }
            match placed.line {
                Line::Title => {
                    let (title, intro) = match &found {
                        Some(f) => (
                            match f.len() + self.key_matches().len() {
                                1 => format!("1 setting matches \"{}\"", self.search.trim()),
                                n => format!("{n} settings match \"{}\"", self.search.trim()),
                            },
                            "From every section. Change them here.".to_owned(),
                        ),
                        None => (
                            self.section.words().to_owned(),
                            self.section.intro().to_owned(),
                        ),
                    };
                    if let Some(f) = &self.fonts.heading {
                        text::draw(rc, f, &title, x0, top, TextStyle::plain(p.text));
                    }
                    if let Some(f) = &self.fonts.intro {
                        text::draw(rc, f, &intro, x0, top + 28.0, TextStyle::plain(kit::SOFT));
                    }
                }
                Line::Heading(words, server) => {
                    let title = if found.is_some() {
                        Section::ALL[words].words().to_uppercase()
                    } else {
                        spec::GROUPS[words].title.to_owned()
                    };
                    self.draw_heading(rc, &title, server, x0, top, right);
                }
                Line::Row(i) => self.draw_row(rc, i, top, placed.h, found.is_some()),
                Line::KeyHeading(title) => self.draw_heading(rc, title, false, x0, top, right),
                Line::Key(k) => self.draw_key(rc, k, top),
                Line::Nothing => {
                    if let Some(f) = &self.fonts.desc {
                        text::draw(
                            rc,
                            f,
                            "Nothing matches. Try another word.",
                            x0,
                            top + 8.0,
                            TextStyle::plain(kit::SOFT),
                        );
                    }
                }
            }
        }
        rc.pop_clip();
        kit::scrollbar(
            rc,
            WIDTH - 12.0,
            ROOM_TOP + 6.0,
            ROOM - 12.0,
            total,
            self.scroll.at,
        );
    }

    fn draw_heading(
        &self,
        rc: &mut RenderContext<'_>,
        title: &str,
        server: bool,
        x: f32,
        top: f32,
        right: f32,
    ) {
        let mut tx = x;
        if let Some(f) = &self.fonts.label {
            text::draw(
                rc,
                f,
                title,
                x,
                top + 4.0,
                TextStyle::plain(kit::WARM).tracked(kit::LABEL_TRACKING),
            );
            tx += text::measure(f, title, kit::LABEL_TRACKING) + 12.0;
        }
        if server && let Some(f) = &self.fonts.small {
            text::draw(
                rc,
                f,
                "Saved on your character",
                tx,
                top + 4.0,
                TextStyle::plain(kit::SOFT),
            );
        }
        rc.fill(x, top + 24.0, right - x, 1.0, kit::bronze(0.25));
    }

    #[allow(clippy::too_many_lines)]
    fn draw_row(&self, rc: &mut RenderContext<'_>, i: usize, top: f32, h: f32, crumbs: bool) {
        let p = self.theme.palette;
        let row = &self.model.rows[i];
        let x0 = Self::content_x();
        let right = Self::content_right();
        let off = self.model.unavailable(row);
        let fade = if off { 0.45 } else { 1.0 };
        if row.control == Control::ChatGrid {
            self.draw_grid(rc, i, top);
            return;
        }
        rc.fill(x0, top + h - 1.0, right - x0, 1.0, kit::bronze(0.1));
        if self.model.row_changed(row) {
            rc.fill_rounded_rect(
                x0 - 14.0,
                top + h * 0.5 - 3.0,
                6.0,
                6.0,
                CornerRadii::all(3.0),
                kit::gold(1.0),
            );
        }
        let desc = self.desc_lines(row);
        let block = 18.0
            + if desc.is_empty() {
                0.0
            } else {
                3.0 + desc.len() as f32 * 16.0
            };
        let mut y = top + ((h - block) * 0.5).round();
        if let Some(f) = &self.fonts.name {
            let fg = if off { p.muted } else { p.text };
            let words = kit::fit(f, &row.label, Self::desc_width());
            text::draw(rc, f, &words, x0, y + 1.0, TextStyle::plain(fg));
            let mut tx = x0 + text::measure(f, &words, 0.0) + 8.0;
            if row.openac
                && let Some(s) = &self.fonts.small
            {
                let w = text::measure(s, "OPENAC", 0.6) + 12.0;
                let r = (tx, y + 2.0, w, 15.0);
                rc.stroke_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(7.5),
                    kit::bronze(0.5),
                    1.0,
                );
                text::draw(
                    rc,
                    s,
                    "OPENAC",
                    r.0 + 6.0,
                    r.1 + ((r.3 - s.line_height) * 0.5).round(),
                    TextStyle::plain(kit::gold(1.0)).tracked(0.6),
                );
                tx += w + 8.0;
            }
            if crumbs && let Some(s) = &self.fonts.small {
                let crumb = format!(
                    "{} \u{203a} {}",
                    row.section.words(),
                    title_case(spec::GROUPS[row.group].title)
                );
                text::draw(rc, s, &crumb, tx, y + 3.0, TextStyle::plain(kit::SOFT));
            }
        }
        y += 21.0;
        if let Some(f) = &self.fonts.desc {
            kit::lines(rc, f, &desc, x0, y, 16.0, kit::SOFT);
        }
        let value = row.key.and_then(|k| self.model.value(k));
        for (part, r) in self.row_parts(i, top, h) {
            let lit = !off && hot(self.hover, r);
            match part {
                Part::Switch(_) => {
                    let key = match row.control {
                        Control::Volume { switch } => Some(switch),
                        _ => row.key,
                    };
                    let mut on =
                        key.and_then(|k| self.model.value(k)) == Some(OptionValue::Bool(true));
                    if let Control::Switch { invert: true } = row.control {
                        on = !on;
                    }
                    draw_switch(rc, r, on, fade, lit);
                }
                Part::Track(_) => {
                    let (min, max, shown) = match &row.control {
                        Control::Slider {
                            min, max, shown, ..
                        } => (*min, *max, shown.clone()),
                        _ => (0.0, 1.0, Shown::Percent),
                    };
                    let v = number(value.as_ref());
                    let share = if max > min {
                        ((v - min) / (max - min)) as f32
                    } else {
                        0.0
                    };
                    let quiet = matches!(row.control, Control::Volume { switch } if self.model.value(switch) != Some(OptionValue::Bool(true)));
                    draw_slider(
                        rc,
                        r,
                        share.clamp(0.0, 1.0),
                        if quiet { fade * 0.5 } else { fade },
                        lit || self.held == Some(i),
                    );
                    if let Some(f) = &self.fonts.control {
                        let words = slider_words(min, max, &shown, v);
                        let ww = text::measure(f, &words, 0.0);
                        text::draw(
                            rc,
                            f,
                            &words,
                            right - ww,
                            r.1 + ((r.3 - f.line_height) * 0.5).round(),
                            TextStyle::plain(Color { a: fade, ..p.text }),
                        );
                    }
                }
                Part::Segment(_, k) => {
                    let Control::Choice { labels, .. } = &row.control else {
                        continue;
                    };
                    let on = Self::choice_index(row, value.as_ref()) == Some(k);
                    let first = k == 0;
                    let last = k + 1 == labels.len();
                    let radius = CornerRadii::new(
                        if first { 4.0 } else { 0.0 },
                        if last { 4.0 } else { 0.0 },
                        if last { 4.0 } else { 0.0 },
                        if first { 4.0 } else { 0.0 },
                    );
                    let fill = if on {
                        kit::gold(0.85 * fade)
                    } else if lit {
                        hex(0xFF_FFFF, 0.06)
                    } else {
                        hex(0, 0.25)
                    };
                    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, fill);
                    rc.stroke_rounded_rect(
                        r.0,
                        r.1,
                        r.2,
                        r.3,
                        radius,
                        kit::bronze(0.5 * fade),
                        1.0,
                    );
                    if let Some(f) = &self.fonts.control {
                        let ink = if on {
                            hex(0x1A_140D, 1.0)
                        } else {
                            Color {
                                a: fade,
                                ..kit::WARM
                            }
                        };
                        let w = text::measure(f, &labels[k], 0.0);
                        text::draw(
                            rc,
                            f,
                            &labels[k],
                            (r.0 + (r.2 - w) * 0.5).round(),
                            r.1 + ((r.3 - f.line_height) * 0.5).round(),
                            TextStyle::plain(ink),
                        );
                    }
                }
                Part::Drop(_) => {
                    let Control::Choice { labels, .. } = &row.control else {
                        continue;
                    };
                    let words = Self::choice_index(row, value.as_ref())
                        .and_then(|k| labels.get(k).cloned())
                        .or_else(|| match &value {
                            Some(OptionValue::Text(t)) => Some(t.clone()),
                            Some(OptionValue::Int(n)) => Some(n.to_string()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let radius = CornerRadii::all(4.0);
                    let open = self.menu == Some(i);
                    rc.fill_rounded_rect(
                        r.0,
                        r.1,
                        r.2,
                        r.3,
                        radius,
                        hex(0, if lit { 0.5 } else { 0.35 }),
                    );
                    let edge = if open {
                        kit::gold(1.0)
                    } else {
                        kit::bronze(0.5 * fade)
                    };
                    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, edge, 1.0);
                    if let Some(f) = &self.fonts.control {
                        let words = kit::fit(f, &words, r.2 - 40.0);
                        text::draw(
                            rc,
                            f,
                            &words,
                            r.0 + 10.0,
                            r.1 + ((r.3 - f.line_height) * 0.5).round(),
                            TextStyle::plain(Color { a: fade, ..p.text }),
                        );
                    }
                    stroke(
                        rc,
                        r.0 + r.2 - 24.0,
                        r.1 + 8.0,
                        14.0,
                        CHEVRON,
                        kit::SOFT,
                        1.6,
                    );
                }
                Part::Button(_) => {
                    if let Control::Button(_, verb) = row.control {
                        kit::button(
                            rc,
                            &self.theme,
                            r,
                            verb,
                            self.fonts.control.as_ref(),
                            ButtonLook::Plain,
                            lit,
                            true,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn draw_key(&self, rc: &mut RenderContext<'_>, k: usize, top: f32) {
        let p = self.theme.palette;
        let Some(line) = self.keys.get(k) else {
            return;
        };
        let x0 = Self::content_x();
        let right = Self::content_right();
        let h = self.key_height(k);
        rc.fill(x0, top + h - 1.0, right - x0, 1.0, kit::bronze(0.1));
        if line.changed {
            rc.fill_rounded_rect(
                x0 - 14.0,
                top + KEY_ROW * 0.5 - 3.0,
                6.0,
                6.0,
                CornerRadii::all(3.0),
                kit::gold(1.0),
            );
        }
        if let Some(f) = &self.fonts.name {
            let fg = if line.mapped { p.text } else { p.muted };
            let words = kit::fit(f, &line.label, Self::desc_width());
            text::draw(
                rc,
                f,
                &words,
                x0,
                top + ((KEY_ROW - f.line_height) * 0.5).round(),
                TextStyle::plain(fg),
            );
        }
        let clash = self.clash.as_ref().filter(|c| c.row == line.row);
        for (part, r) in self.key_parts(k, top) {
            let lit = hot(self.hover, r);
            match part {
                Part::KeySlot(_, slot) => {
                    let catching = self.capture == Some((line.row, slot));
                    let clashing = clash.is_some_and(|c| c.slot == slot);
                    let name = line.keys.get(slot).filter(|n| !n.is_empty());
                    let radius = CornerRadii::all(4.0);
                    let (fill, edge, width, ink, words) = if catching {
                        (
                            kit::gold(0.12),
                            kit::gold(1.0),
                            2.0,
                            kit::gold(1.0),
                            "Press a key".to_owned(),
                        )
                    } else if clashing {
                        (
                            hex(0xFF_8A5B, 0.12),
                            hex(0xFF_8A5B, 1.0),
                            2.0,
                            hex(0xFF_C9A8, 1.0),
                            "In use".to_owned(),
                        )
                    } else if let Some(n) = name {
                        (
                            hex(0, if lit { 0.5 } else { 0.35 }),
                            kit::bronze(if lit { 0.9 } else { 0.5 }),
                            1.0,
                            p.text,
                            n.clone(),
                        )
                    } else {
                        (
                            hex(0, if lit { 0.35 } else { 0.18 }),
                            kit::bronze(if lit { 0.7 } else { 0.3 }),
                            1.0,
                            kit::SOFT,
                            "Add key".to_owned(),
                        )
                    };
                    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, fill);
                    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, edge, width);
                    if let Some(f) = &self.fonts.control {
                        let words = kit::fit(f, &words, r.2 - 12.0);
                        let w = text::measure(f, &words, 0.0);
                        text::draw(
                            rc,
                            f,
                            &words,
                            (r.0 + (r.2 - w) * 0.5).round(),
                            r.1 + ((r.3 - f.line_height) * 0.5).round(),
                            TextStyle::plain(ink),
                        );
                    }
                }
                Part::ClashTake | Part::ClashKeep => {
                    let (words, look) = if part == Part::ClashTake {
                        ("Use it here", ButtonLook::Primary)
                    } else {
                        ("Keep", ButtonLook::Plain)
                    };
                    kit::button(
                        rc,
                        &self.theme,
                        r,
                        words,
                        self.fonts.control.as_ref(),
                        look,
                        lit,
                        true,
                    );
                }
                _ => {}
            }
        }
        if let Some(c) = clash {
            let b = (x0, top + KEY_ROW, right - x0, CLASH_H - 4.0);
            let radius = CornerRadii::all(4.0);
            rc.fill_rounded_rect(b.0, b.1, b.2, b.3, radius, hex(0xFF_8A5B, 0.1));
            rc.stroke_rounded_rect(b.0, b.1, b.2, b.3, radius, hex(0xFF_8A5B, 0.6), 1.0);
            if let Some(f) = &self.fonts.desc {
                let room = b.2 - 24.0 - if c.can_take { 210.0 } else { 80.0 };
                let words = kit::fit(f, &c.words, room);
                text::draw(
                    rc,
                    f,
                    &words,
                    b.0 + 10.0,
                    b.1 + ((b.3 - f.line_height) * 0.5).round(),
                    TextStyle::plain(hex(0xFF_C9A8, 1.0)),
                );
            }
        }
    }

    fn draw_grid(&self, rc: &mut RenderContext<'_>, i: usize, top: f32) {
        use crate::ui::modern::chat::model::{GROUPS, group_mask};
        let p = self.theme.palette;
        let x0 = Self::content_x();
        let right = Self::content_right();
        let n = self.tabs.len().max(1) as f32;
        let col = (right - x0 - GRID_LABEL_W) / n;
        if let Some(f) = &self.fonts.small {
            text::draw(
                rc,
                f,
                "MESSAGE TYPE",
                x0,
                top + 10.0,
                TextStyle::plain(kit::SOFT).tracked(0.6),
            );
            for (t, tab) in self.tabs.iter().enumerate() {
                let words = kit::fit(f, &tab.name.to_uppercase(), col - 8.0);
                let w = text::measure(f, &words, 0.6);
                let cx = x0 + GRID_LABEL_W + col * (t as f32 + 0.5);
                text::draw(
                    rc,
                    f,
                    &words,
                    (cx - w * 0.5).round(),
                    top + 10.0,
                    TextStyle::plain(kit::SOFT).tracked(0.6),
                );
            }
        }
        for (g, (name, _)) in GROUPS.iter().enumerate() {
            let y = top + GRID_ROW * (g as f32 + 1.0);
            rc.fill(x0, y, right - x0, 1.0, kit::bronze(0.1));
            if let Some(f) = &self.fonts.control {
                text::draw(
                    rc,
                    f,
                    name,
                    x0,
                    y + ((GRID_ROW - f.line_height) * 0.5).round(),
                    TextStyle::plain(p.text),
                );
            }
            let mask = group_mask(g);
            for (t, tab) in self.tabs.iter().enumerate() {
                let cx = x0 + GRID_LABEL_W + col * (t as f32 + 0.5);
                let on = tab.filter & mask == mask;
                let some = !on && tab.filter & mask != 0;
                let b = (cx - 8.0, y + (GRID_ROW - 16.0) * 0.5, 16.0, 16.0);
                let lit = hot(self.hover, (cx - 12.0, y + 3.0, 24.0, 24.0));
                let radius = CornerRadii::all(3.0);
                if on {
                    rc.fill_rounded_rect(b.0, b.1, b.2, b.3, radius, kit::gold(1.0));
                    stroke(rc, b.0, b.1, 16.0, TICK, hex(0x1A_140D, 1.0), 2.0);
                } else {
                    rc.stroke_rounded_rect(
                        b.0,
                        b.1,
                        b.2,
                        b.3,
                        radius,
                        kit::bronze(if lit || some { 0.9 } else { 0.5 }),
                        1.5,
                    );
                }
            }
        }
        let _ = i;
        if self.tabs.is_empty()
            && let Some(f) = &self.fonts.desc
        {
            text::draw(
                rc,
                f,
                "The chat's tabs show here.",
                x0 + GRID_LABEL_W,
                top + 10.0,
                TextStyle::plain(kit::SOFT),
            );
        }
    }

    fn draw_foot(&self, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        rc.fill(NAV_W, HEIGHT - FOOT, WIDTH - NAV_W, 1.0, kit::bronze(0.35));
        let changes = self.model.changed().len() + self.keys_changed;
        let words = match changes {
            0 => "Changes show live. Apply keeps them.".to_owned(),
            1 => "1 change, shown live. Apply keeps it, Revert undoes it.".to_owned(),
            n => format!("{n} changes, shown live. Apply keeps them, Revert undoes them."),
        };
        let cy = HEIGHT - FOOT * 0.5;
        if changes > 0 {
            rc.fill_rounded_rect(
                NAV_W + PAD_X,
                cy - 3.0,
                6.0,
                6.0,
                CornerRadii::all(3.0),
                kit::gold(1.0),
            );
        }
        if let Some(f) = &self.fonts.intro {
            text::draw(
                rc,
                f,
                &words,
                NAV_W + PAD_X + 14.0,
                cy - (f.line_height * 0.5).round(),
                TextStyle::plain(kit::SOFT),
            );
        }
        for (part, r) in self.foot_buttons() {
            let lit = hot(self.hover, r);
            match part {
                Part::Defaults => {
                    if lit {
                        rc.fill_rounded_rect(
                            r.0,
                            r.1,
                            r.2,
                            r.3,
                            CornerRadii::all(4.0),
                            hex(0xFF_FFFF, 0.05),
                        );
                    }
                    if let Some(f) = &self.fonts.control {
                        let w = text::measure(f, "Defaults", 0.0);
                        text::draw(
                            rc,
                            f,
                            "Defaults",
                            (r.0 + (r.2 - w) * 0.5).round(),
                            r.1 + ((r.3 - f.line_height) * 0.5).round(),
                            TextStyle::plain(if lit { p.text } else { kit::SOFT }),
                        );
                    }
                }
                Part::Revert => kit::button(
                    rc,
                    &self.theme,
                    r,
                    "Revert",
                    self.fonts.control.as_ref(),
                    ButtonLook::Plain,
                    lit,
                    changes > 0,
                ),
                Part::Apply => kit::button(
                    rc,
                    &self.theme,
                    r,
                    "Apply",
                    self.fonts.control.as_ref(),
                    ButtonLook::Primary,
                    lit,
                    changes > 0,
                ),
                _ => {}
            }
        }
    }

    fn draw_menu(&self, rc: &mut RenderContext<'_>, layout: &[Placed]) {
        let Some(i) = self.menu else {
            return;
        };
        let parts = self.menu_parts(layout);
        let (Some(first), Some(last)) = (parts.first(), parts.last()) else {
            return;
        };
        let row = &self.model.rows[i];
        let Control::Choice { labels, .. } = &row.control else {
            return;
        };
        let p = self.theme.palette;
        let value = row.key.and_then(|k| self.model.value(k));
        let chosen = Self::choice_index(row, value.as_ref());
        let back = (
            first.1.0,
            first.1.1 - 4.0,
            first.1.2,
            last.1.1 + last.1.3 + 4.0 - (first.1.1 - 4.0),
        );
        let radius = CornerRadii::all(4.0);
        rc.soft_shadow(back.0, back.1, back.2, back.3, 4.0, 10.0, 24.0, hex(0, 0.5));
        rc.fill_rounded_rect(back.0, back.1, back.2, back.3, radius, hex(0x18_130E, 0.98));
        rc.stroke_rounded_rect(
            back.0,
            back.1,
            back.2,
            back.3,
            radius,
            kit::bronze(0.7),
            1.0,
        );
        for (part, r) in parts {
            let Part::MenuChoice(k) = part else {
                continue;
            };
            if hot(self.hover, r) {
                rc.fill(r.0 + 2.0, r.1, r.2 - 4.0, r.3, kit::gold(0.16));
            }
            if let Some(f) = &self.fonts.control {
                let on = chosen == Some(k);
                let words = kit::fit(f, &labels[k], r.2 - 36.0);
                text::draw(
                    rc,
                    f,
                    &words,
                    r.0 + 26.0,
                    r.1 + ((r.3 - f.line_height) * 0.5).round(),
                    TextStyle::plain(if on { kit::gold(1.0) } else { p.text }),
                );
                if on {
                    stroke(rc, r.0 + 7.0, r.1 + 7.0, 14.0, TICK, kit::gold(1.0), 1.8);
                }
            }
        }
    }
}

fn draw_switch(rc: &mut RenderContext<'_>, r: Rect, on: bool, fade: f32, lit: bool) {
    let track = if on {
        kit::gold(0.9 * fade)
    } else {
        hex(0xFF_FFFF, if lit { 0.2 } else { 0.14 } * fade)
    };
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(r.3 * 0.5), track);
    let knob = if on {
        hex(0x1A_140D, fade)
    } else {
        Color {
            a: fade,
            ..kit::SOFT
        }
    };
    let kx = if on { r.0 + r.2 - 17.0 } else { r.0 + 3.0 };
    rc.fill_rounded_rect(kx, r.1 + 3.0, 14.0, 14.0, CornerRadii::all(7.0), knob);
}

fn draw_slider(rc: &mut RenderContext<'_>, r: Rect, share: f32, fade: f32, lit: bool) {
    let cy = r.1 + r.3 * 0.5;
    rc.fill_rounded_rect(
        r.0,
        cy - 2.0,
        r.2,
        4.0,
        CornerRadii::all(2.0),
        hex(0xFF_FFFF, 0.14 * fade),
    );
    rc.fill_rounded_rect(
        r.0,
        cy - 2.0,
        r.2 * share,
        4.0,
        CornerRadii::all(2.0),
        kit::gold(fade),
    );
    let kx = r.0 + r.2 * share - 7.0;
    let size = if lit { 16.0 } else { 14.0 };
    rc.fill_rounded_rect(
        kx + (14.0 - size) * 0.5,
        cy - size * 0.5,
        size,
        size,
        CornerRadii::all(size * 0.5),
        kit::gold(fade),
    );
}

/// "SHORTCUT BARS" as "Shortcut bars".
fn title_case(words: &str) -> String {
    let lower = words.to_lowercase();
    let mut c = lower.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::modern::options::model::{Sources, rows};
    use ac_ui::panels::options::keys::LiveValues;

    fn frame() -> OptionsFrame {
        let strings = |_: &str| None;
        let res = vec!["1920x1080".to_owned(), "2560x1440".to_owned()];
        let src = Sources {
            strings: &strings,
            resolutions: &res,
        };
        let live = LiveValues {
            settings: ac_host::settings::PlayerSettings::default(),
            options1: ac_runtime::character::DEFAULT_OPTIONS1,
            options2: ac_runtime::character::DEFAULT_OPTIONS2,
        };
        OptionsFrame::new(
            Horizon::BRONZE,
            OptionsFonts::default(),
            OptionsModel::new(rows(&src), live),
        )
    }

    fn row_top(f: &mut OptionsFrame, label: &str) -> (usize, f32, f32) {
        let (layout, _) = f.layout();
        let (i, y, h) = layout
            .iter()
            .find_map(|p| match p.line {
                Line::Row(i) if f.model.rows[i].label == label => Some((i, p.y, p.h)),
                _ => None,
            })
            .expect("the row shows");
        f.scroll.reveal(y, h, ROOM);
        (i, ROOM_TOP + y - f.scroll.at, h)
    }

    #[test]
    fn a_switch_changes_its_setting_live() {
        let mut f = frame();
        f.show(Section::Gameplay);
        let (i, top, h) = row_top(&mut f, "Run by default");
        let (_, r) = f.row_parts(i, top, h)[0];
        f.click(r.0 + 5.0, r.1 + 5.0);
        assert!(matches!(
            f.requests.as_slice(),
            [OptionsRequest::Changed(Stored::Character(_), true)]
        ));
        assert_eq!(f.model.changed().len(), 1);
        f.click(f.foot_buttons()[1].1.0 + 4.0, f.foot_buttons()[1].1.1 + 4.0);
        assert!(f.model.changed().is_empty(), "Revert undoes it");
    }

    #[test]
    fn a_section_lists_its_groups_and_the_search_every_match() {
        let mut f = frame();
        f.show(Section::Sound);
        let (layout, _) = f.layout();
        assert!(layout.iter().any(|p| matches!(p.line, Line::Heading(..))));
        f.search = "run".to_owned();
        let (layout, _) = f.layout();
        let rows = layout
            .iter()
            .filter(|p| matches!(p.line, Line::Row(_)))
            .count();
        assert!(rows >= 2, "{rows}");
    }

    #[test]
    fn an_inverted_switch_shows_the_opposite() {
        let mut f = frame();
        let (i, top, h) = row_top(&mut f, "Distance fog");
        let (_, r) = f.row_parts(i, top, h)[0];
        f.click(r.0 + 5.0, r.1 + 5.0);
        let fog = OptionKey::Character(ac_runtime::character::option_id::DISABLE_DISTANCE_FOG);
        assert_eq!(
            f.model.value(fog),
            Some(OptionValue::Bool(true)),
            "switched off disables the fog"
        );
    }

    #[test]
    fn a_menu_opens_and_picks() {
        let mut f = frame();
        f.show(Section::Graphics);
        let (i, top, h) = row_top(&mut f, "Resolution");
        let (part, r) = f.row_parts(i, top, h)[0];
        assert!(matches!(part, Part::Drop(_)));
        f.click(r.0 + 5.0, r.1 + 5.0);
        assert_eq!(f.menu, Some(i));
        let (layout, _) = f.layout();
        let (_, c) = f.menu_parts(&layout)[1];
        f.click(c.0 + 5.0, c.1 + 5.0);
        assert_eq!(f.menu, None);
        assert_eq!(
            f.model.value(OptionKey::Display(
                ac_ui::panels::options::keys::DisplayField::Resolution
            )),
            Some(OptionValue::Text("2560x1440".to_owned()))
        );
    }
}
