//! The chat bottom left: tabs with unread counts over the transcript, and
//! the entry under it (added by the window). Idle, there is no panel: the
//! last few lines fade over the world with a small "Enter to chat". Reading
//! back holds the transcript still; new lines wait behind a "N new
//! messages" button.

use std::collections::HashMap;
use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use ac_runtime::communication::ChatChannelKind;

use super::channels::{channel_color, channel_name, channel_prompt};
use super::commands::Command;
use super::model::{ChatLog, ChatTab, LineKind, ShownLine};

/// The panel's width.
pub const WIDTH: f32 = 440.0;
/// The tabs' row.
pub const TABS: f32 = 24.0;
/// The gap between the rows.
pub const GAP: f32 = 6.0;
/// The transcript's height, active and idle.
pub const BOX: f32 = 232.0;
const IDLE_BOX: f32 = 128.0;
/// The entry's height.
pub const ENTRY: f32 = 32.0;
/// The whole panel's height.
pub const HEIGHT: f32 = TABS + GAP + BOX + GAP + ENTRY;
/// Where the transcript starts.
pub const BOX_TOP: f32 = TABS + GAP;
/// Where the entry starts.
pub const ENTRY_TOP: f32 = BOX_TOP + BOX + GAP;
/// The transcript's padding: top, right, bottom, left.
const PAD: (f32, f32, f32, f32) = (8.0, 14.0, 8.0, 12.0);
/// A row of text.
pub const ROW: f32 = 18.0;
/// The time column and the gap after it.
const TIME: f32 = 30.0 + 8.0;
/// How many lines show while idle, and their opacities, oldest first.
const IDLE_OPACITY: [f32; 6] = [0.3, 0.5, 0.7, 0.9, 1.0, 1.0];
/// How long the panel stays up after the pointer and the keyboard leave,
/// seconds.
const STAY: f64 = 4.0;
/// How long the pointer rests on the idle lines before the panel comes
/// up, seconds (a pointer passing over on its way to the world does not
/// wake it).
const WAKE_DWELL: f64 = 0.2;
/// The "Enter to chat" button.
const ENTER_WORDS: &str = "Enter to chat";
/// The entry's hint at its right end.
const ENTER_HINT: &str = "Enter";
/// The wheel's step, rows.
const WHEEL_ROWS: usize = 3;
/// The tab buttons' height and gap, and the small square buttons' side.
const TAB_GAP: f32 = 2.0;

const TEXT: Color = hex(0xF4_EBDC, 1.0);
const MUTED: Color = hex(0xC8_B89C, 1.0);
const TIME_COLOR: Color = hex(0xA8_9A82, 1.0);
const GOLD: Color = hex(0xE0_B068, 1.0);
const INK: Color = hex(0x1A_140D, 1.0);

/// The fonts the panel uses.
#[derive(Debug, Clone, Default)]
pub struct ChatFonts {
    /// The lines.
    pub body: Option<Arc<UiFont>>,
    /// Speakers' names.
    pub name: Option<Arc<UiFont>>,
    /// Tags and badges.
    pub tag: Option<Arc<UiFont>>,
    /// Times and small words.
    pub small: Option<Arc<UiFont>>,
    /// Tab names.
    pub tab: Option<Arc<UiFont>>,
}

/// A box drawn on the panel: left, top, width, height.
type Spot = (f32, f32, f32, f32);

/// One row of a wrapped line.
#[derive(Debug, Clone, PartialEq)]
struct Row {
    /// The line's sequence number.
    sequence: u64,
    /// The line's first row (time, tag and name go before its text).
    first: bool,
    /// Where the text starts.
    x: f32,
    /// The text.
    text: String,
}

/// What the panel asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatRequest {
    /// The entry should take the keyboard (a click on the panel, "Enter to
    /// chat").
    Focus,
    /// Tab `n` was picked.
    Tab(usize),
    /// The speaker of the line was clicked: name, object id, where.
    Name(String, u32, (f32, f32)),
    /// The channel button was pressed: its top left on the panel.
    Channels((f32, f32)),
    /// A suggested command was picked (a click, Tab).
    Complete(&'static str),
    /// Search opens (true) or closes.
    Search(bool),
    /// A new tab (the + button).
    NewTab,
    /// Tab `n` to edit (a right-click on it, a window's filter button).
    EditTab(usize),
    /// Tab `n` pops out into its own window.
    PopOut(usize),
    /// The window closes and goes back to being a tab.
    PutBack,
    /// The window was let go after a move.
    Moved,
}

/// The chat panel.
#[derive(Debug)]
pub struct ChatPanel {
    theme: Horizon,
    fonts: ChatFonts,
    /// The tabs.
    pub tabs: Vec<ChatTab>,
    /// The tab shown.
    pub current: usize,
    /// The lines and the unread counts.
    pub log: ChatLog,
    /// Show the time before each line (the game's timestamps option).
    pub times: bool,
    /// The panel is up (not idle).
    pub active: bool,
    /// The entry has the keyboard (the window says so every frame).
    pub typing: bool,
    /// When the pointer or the keyboard last had the panel (its clock).
    touched: f64,
    /// How long the pointer has rested on the idle panel, seconds.
    dwell: f64,
    /// Its clock.
    clock: f64,
    /// Rows scrolled up from the newest (0: following).
    scroll: usize,
    /// Lines that came in while reading back.
    waiting: u32,
    /// Each line's rows, by sequence, for the width and fonts in use.
    rows: HashMap<u64, Vec<Row>>,
    /// What the rows were made for: the times shown.
    rows_for: bool,
    /// Where each tab was last drawn (left, width).
    tab_spots: Vec<(usize, f32, f32)>,
    /// The + button and the pop-out button on the tab row; a window's
    /// title buttons: edit, put back, close.
    row_spots: [Option<Spot>; 5],
    /// It is a popped-out tab's window (its only tab is the one shown).
    pub window: bool,
    /// Held by its title: where the pointer took hold.
    grip: Option<(f32, f32)>,
    /// Where each speaker's name was last drawn: the box, its name, its id.
    names: Vec<(Spot, String, u32)>,
    /// The "N new messages" button, when shown.
    new_button: Option<Spot>,
    /// The "Enter to chat" button, when shown.
    enter_button: Option<Spot>,
    hovered: bool,
    /// Where the pointer is on the panel.
    pointer: (f32, f32),
    /// Where a typed line goes.
    pub channel: ChatChannelKind,
    /// Who it goes to on the tell channel.
    pub tell_target: Option<String>,
    /// The entry holds nothing (its prompt shows).
    pub entry_empty: bool,
    /// The channel button, when shown.
    button: Option<Spot>,
    /// The commands suggested for what is being typed.
    pub suggestions: Vec<Command>,
    /// The last teller (a reply suggestion names them).
    pub last_teller: Option<String>,
    /// Where the typed command sends the line, when not the channel.
    pub going_to: Option<String>,
    /// Where each suggestion was drawn.
    suggestion_spots: Vec<Spot>,
    /// Searching: the search row stands in for the tabs.
    pub searching: bool,
    /// What is searched for (the window copies it from its field).
    pub query: String,
    /// The match looked at, counted from the newest (0: the newest).
    pub match_at: usize,
    /// The search row's buttons: previous, next, close; and the search
    /// button on the tab row.
    search_spots: [Option<Spot>; 4],
    /// What it asked for since the window last looked.
    pub requests: Vec<ChatRequest>,
}

impl ChatPanel {
    /// The panel in `theme`, with the starting tabs.
    #[must_use]
    pub fn new(theme: Horizon, fonts: ChatFonts) -> Self {
        Self {
            theme,
            fonts,
            tabs: ChatTab::defaults(),
            current: 0,
            log: ChatLog::default(),
            times: false,
            active: false,
            typing: false,
            touched: f64::NEG_INFINITY,
            dwell: 0.0,
            clock: 0.0,
            scroll: 0,
            waiting: 0,
            rows: HashMap::new(),
            rows_for: false,
            tab_spots: Vec::new(),
            row_spots: [None; 5],
            window: false,
            grip: None,
            names: Vec::new(),
            new_button: None,
            enter_button: None,
            hovered: false,
            pointer: (0.0, 0.0),
            channel: ChatChannelKind::Say,
            tell_target: None,
            entry_empty: true,
            button: None,
            suggestions: Vec::new(),
            last_teller: None,
            going_to: None,
            suggestion_spots: Vec::new(),
            searching: false,
            query: String::new(),
            match_at: 0,
            search_spots: [None; 4],
            requests: Vec::new(),
        }
    }

    /// The channel button's words.
    #[must_use]
    pub fn channel_words(&self) -> String {
        if let Some(to) = &self.going_to {
            return to.clone();
        }
        match (&self.tell_target, self.channel) {
            (Some(who), ChatChannelKind::Tell) => format!("Tell {who}"),
            _ => channel_name(self.channel).to_owned(),
        }
    }

    fn button_width(&self) -> f32 {
        let label = self
            .fonts
            .name
            .as_deref()
            .map_or(60.0, |f| text::measure(f, &self.channel_words(), 0.0));
        (8.0 + 7.0 + 6.0 + label + 6.0 + 10.0 + 8.0).min(WIDTH * 0.45)
    }

    fn hint_width(&self) -> f32 {
        self.fonts
            .small
            .as_deref()
            .map_or(40.0, |f| text::measure(f, ENTER_HINT, 0.0))
    }

    /// Where the entry's field goes on the panel: left, top, width,
    /// height.
    #[must_use]
    pub fn field_rect(&self) -> (f32, f32, f32, f32) {
        let left = 4.0 + self.button_width() + 8.0;
        let right = WIDTH - 10.0 - self.hint_width() - 8.0;
        (left, ENTRY_TOP + 4.0, (right - left).max(40.0), ENTRY - 8.0)
    }

    /// Adds a line. Reading back, the view holds still and the line waits.
    pub fn push(&mut self, line: ShownLine) {
        let looking = (self.active).then_some(self.current);
        let shown = self.tabs.get(self.current).is_some_and(|t| t.shows(&line));
        if shown && self.scroll > 0 {
            let rows = self.rows_of(&line).len();
            self.scroll += rows;
            self.waiting = self.waiting.saturating_add(1);
        }
        self.log.push(line, &self.tabs, looking);
        if self.log.lines.len() > super::model::KEPT - 1 {
            let kept: std::collections::HashSet<u64> =
                self.log.lines.iter().map(|l| l.sequence).collect();
            self.rows.retain(|k, _| kept.contains(k));
        }
    }

    /// Shows tab `tab`: its unread count clears, reading back ends.
    pub fn pick_tab(&mut self, tab: usize) {
        if tab < self.tabs.len() {
            self.current = tab;
            self.scroll = 0;
            self.waiting = 0;
            self.log.seen(tab);
        }
    }

    /// Brings the panel up (Enter, a click).
    pub fn wake(&mut self) {
        self.active = true;
        self.touched = self.clock;
        self.log.seen(self.current);
    }

    /// Back to following the newest line.
    pub fn follow(&mut self) {
        self.scroll = 0;
        self.waiting = 0;
    }

    /// The panel is reading back.
    #[must_use]
    pub fn reading_back(&self) -> bool {
        self.scroll > 0
    }

    fn text_width(&self) -> f32 {
        let time = if self.times { TIME } else { 0.0 };
        WIDTH - PAD.1 - PAD.3 - time
    }

    /// The width of a line's tag and name before its words.
    fn prefix_width(&self, line: &ShownLine) -> f32 {
        let mut w = 0.0;
        if let (Some(tag), Some(f)) = (line.kind.tag(), &self.fonts.tag) {
            w += text::measure(f, tag, TAG_TRACKING) + 6.0;
        }
        if let (Some(who), Some(f)) = (&line.who, &self.fonts.name) {
            w += text::measure(f, who, 0.0) + text::measure(f, ":", 0.0) + 5.0;
        }
        w
    }

    /// `line`'s rows at the width in use (worked out once).
    fn rows_of(&mut self, line: &ShownLine) -> Vec<Row> {
        if self.rows_for != self.times {
            self.rows.clear();
            self.rows_for = self.times;
        }
        if let Some(rows) = self.rows.get(&line.sequence) {
            return rows.clone();
        }
        let width = self.text_width();
        let prefix = self.prefix_width(line);
        let rows = match &self.fonts.body {
            Some(f) => wrap_hanging(&line.words, |t| f.measure_width(t), prefix, width)
                .into_iter()
                .enumerate()
                .map(|(i, text)| Row {
                    sequence: line.sequence,
                    first: i == 0,
                    x: if i == 0 { prefix } else { 0.0 },
                    text,
                })
                .collect(),
            None => vec![Row {
                sequence: line.sequence,
                first: true,
                x: prefix,
                text: line.words.clone(),
            }],
        };
        self.rows.insert(line.sequence, rows.clone());
        rows
    }

    /// The rows that fill `room` rows of the transcript, oldest first,
    /// `skip` rows up from the newest; and whether older rows remain above.
    fn visible_rows(&mut self, room: usize, skip: usize) -> (Vec<Row>, bool) {
        let tab = self.tabs.get(self.current).cloned();
        let mut taken: Vec<Row> = Vec::new();
        let mut skipped = 0;
        let lines: Vec<ShownLine> = self
            .log
            .lines
            .iter()
            .rev()
            .filter(|l| tab.as_ref().is_some_and(|t| t.shows(l)))
            .cloned()
            .collect();
        let mut more = false;
        for line in &lines {
            let rows = self.rows_of(line);
            for row in rows.into_iter().rev() {
                if skipped < skip {
                    skipped += 1;
                    continue;
                }
                if taken.len() == room {
                    more = true;
                    break;
                }
                taken.push(row);
            }
            if more {
                break;
            }
        }
        taken.reverse();
        (taken, more)
    }

    /// How many rows the shown tab has in all.
    fn all_rows(&mut self) -> usize {
        let tab = self.tabs.get(self.current).cloned();
        let lines: Vec<ShownLine> = self
            .log
            .lines
            .iter()
            .filter(|l| tab.as_ref().is_some_and(|t| t.shows(l)))
            .cloned()
            .collect();
        lines.iter().map(|l| self.rows_of(l).len()).sum()
    }

    /// The shown tab's lines that hold the query, newest first.
    #[must_use]
    pub fn matches(&self) -> Vec<u64> {
        let q = self.query.trim().to_lowercase();
        if !self.searching || q.is_empty() {
            return Vec::new();
        }
        let Some(tab) = self.tabs.get(self.current) else {
            return Vec::new();
        };
        self.log
            .lines
            .iter()
            .rev()
            .filter(|l| tab.shows(l) && line_holds(l, &q))
            .map(|l| l.sequence)
            .collect()
    }

    /// Steps to the older (`older`) or newer match and scrolls it into
    /// view.
    pub fn step_match(&mut self, older: bool) {
        let found = self.matches();
        if found.is_empty() {
            return;
        }
        self.match_at = if older {
            (self.match_at + 1) % found.len()
        } else {
            (self.match_at + found.len() - 1) % found.len()
        };
        self.show_line(found[self.match_at]);
    }

    /// Scrolls so line `sequence` sits in the middle of the view.
    fn show_line(&mut self, sequence: u64) {
        let tab = self.tabs.get(self.current).cloned();
        let newer: Vec<ShownLine> = self
            .log
            .lines
            .iter()
            .rev()
            .take_while(|l| l.sequence != sequence)
            .filter(|l| tab.as_ref().is_some_and(|t| t.shows(l)))
            .cloned()
            .collect();
        let below: usize = newer.iter().map(|l| self.rows_of(l).len()).sum();
        let most = self.all_rows().saturating_sub(self.room());
        self.scroll = below.saturating_sub(self.room() / 2).min(most);
    }

    fn room(&self) -> usize {
        ((BOX - PAD.0 - PAD.2) / ROW).floor() as usize
    }

    fn tab_at(&self, x: f32, y: f32) -> Option<usize> {
        if !self.active || !(0.0..TABS).contains(&y) {
            return None;
        }
        self.tab_spots
            .iter()
            .find(|(_, left, w)| (*left..left + w).contains(&x))
            .map(|(i, ..)| *i)
    }

    /// A window: the tab it shows, popped out.
    #[must_use]
    pub fn for_window(theme: Horizon, fonts: ChatFonts, tab: ChatTab) -> Self {
        let mut panel = Self::new(theme, fonts);
        panel.tabs = vec![tab];
        panel.window = true;
        panel.active = true;
        panel
    }

    /// The tab has an entry (the dock always; a window when its tab talks
    /// on a channel).
    #[must_use]
    pub fn has_entry(&self) -> bool {
        !self.window || self.tabs.first().is_some_and(|t| t.talk.is_some())
    }

    /// The panel's height.
    #[must_use]
    pub fn height(&self) -> f32 {
        if self.has_entry() {
            HEIGHT
        } else {
            HEIGHT - GAP - ENTRY
        }
    }

    fn inside(r: Option<Spot>, x: f32, y: f32) -> bool {
        r.is_some_and(|(l, t, w, h)| (l..l + w).contains(&x) && (t..t + h).contains(&y))
    }
}

/// The tags' letter-spacing.
const TAG_TRACKING: f32 = 0.76;

/// `text` wrapped to `width`, its first row starting `indent` in.
#[must_use]
pub fn wrap_hanging(
    text: &str,
    measure: impl Fn(&str) -> f32,
    indent: f32,
    width: f32,
) -> Vec<String> {
    let first_room = (width - indent).max(1.0);
    let mut first = String::new();
    let mut rest = text.trim();
    while let Some(word) = rest.split(' ').next().filter(|w| !w.is_empty()) {
        let candidate = if first.is_empty() {
            word.to_owned()
        } else {
            format!("{first} {word}")
        };
        if measure(&candidate) > first_room {
            break;
        }
        first = candidate;
        rest = rest[word.len()..].trim_start();
    }
    if first.is_empty() && indent <= 0.0 {
        return ac_ui::text::wrap_words(rest, measure, width.max(1.0));
    }
    let mut rows = vec![first];
    if !rest.is_empty() {
        rows.extend(ac_ui::text::wrap_words(rest, measure, width.max(1.0)));
    }
    rows
}

impl Widget for ChatPanel {
    fn hit_test(&self, element: &Element, x: f32, y: f32) -> bool {
        if self.active {
            return element.contains_local(x, y);
        }
        // Idle: the faded lines and the entry's row, where a resting
        // pointer wakes the panel.
        Self::inside(self.enter_button, x, y)
            || ((0.0..WIDTH).contains(&x) && (BOX_TOP + BOX - IDLE_BOX..HEIGHT).contains(&y))
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

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        let (x, y) = self.pointer;
        let words = match (0..5).find(|&i| Self::inside(self.row_spots[i], x, y))? {
            0 => "New tab",
            1 => "Pop this tab out into a window",
            2 => "What this window shows",
            _ => "Close: back into the chat as a tab",
        };
        Some(words.to_owned())
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.kind == kind::KEY_DOWN
            && e.key == ac_ui::Key::Tab
            && let Some(first) = self.suggestions.first()
        {
            self.requests.push(ChatRequest::Complete(first.verb));
            return true;
        }
        if e.target != Some(me) {
            return false;
        }
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = true;
                self.pointer = (x, y);
                if self.active {
                    self.touched = self.clock;
                }
                if let Some((gx, gy)) = self.grip {
                    let (mx, my) = ui.mouse();
                    let screen = ui.size();
                    let h = self.height();
                    if let Some(el) = ui.element_mut(me) {
                        el.left = (mx as f32 - gx).clamp(0.0, (screen.0 - WIDTH).max(0.0));
                        el.top = (my as f32 - gy).clamp(0.0, (screen.1 - h).max(0.0));
                    }
                }
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = false;
                true
            }
            kind::RIGHT_CLICK => {
                if let Some(tab) = self.tab_at(x, y) {
                    self.requests.push(ChatRequest::EditTab(tab));
                }
                true
            }
            kind::MOUSE_DOWN
                if self.window
                    && y < TABS
                    && !(2..5).any(|i| Self::inside(self.row_spots[i], x, y)) =>
            {
                self.grip = Some((x, y));
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                if self.grip.take().is_some() {
                    self.requests.push(ChatRequest::Moved);
                }
                true
            }
            kind::SCROLL if self.active => {
                let total = self.all_rows();
                let most = total.saturating_sub(self.room());
                if e.data0 > 0 {
                    self.scroll = (self.scroll + WHEEL_ROWS).min(most);
                } else {
                    self.scroll = self.scroll.saturating_sub(WHEEL_ROWS);
                    if self.scroll == 0 {
                        self.waiting = 0;
                    }
                }
                true
            }
            kind::CLICK => {
                if let Some(i) = (0..5).find(|&i| Self::inside(self.row_spots[i], x, y)) {
                    self.requests.push(match i {
                        0 => ChatRequest::NewTab,
                        1 => ChatRequest::PopOut(self.current),
                        2 => ChatRequest::EditTab(0),
                        _ => ChatRequest::PutBack,
                    });
                } else if let Some(i) = (0..4).find(|&i| Self::inside(self.search_spots[i], x, y)) {
                    match i {
                        0 => self.step_match(true),
                        1 => self.step_match(false),
                        2 => self.requests.push(ChatRequest::Search(false)),
                        _ => self.requests.push(ChatRequest::Search(true)),
                    }
                } else if let Some(i) = self
                    .suggestion_spots
                    .iter()
                    .position(|r| Self::inside(Some(*r), x, y))
                {
                    if let Some(cmd) = self.suggestions.get(i) {
                        self.requests.push(ChatRequest::Complete(cmd.verb));
                    }
                } else if Self::inside(self.enter_button, x, y) {
                    self.wake();
                    self.requests.push(ChatRequest::Focus);
                } else if let Some(b) = self.button.filter(|_| Self::inside(self.button, x, y)) {
                    self.requests.push(ChatRequest::Channels((b.0, b.1)));
                } else if Self::inside(self.new_button, x, y) {
                    self.follow();
                } else if let Some(tab) = self.tab_at(x, y) {
                    self.pick_tab(tab);
                    self.requests.push(ChatRequest::Tab(tab));
                    self.requests.push(ChatRequest::Focus);
                } else if let Some((_, who, id)) = self
                    .names
                    .iter()
                    .find(|(r, ..)| Self::inside(Some(*r), x, y))
                    .cloned()
                {
                    self.requests.push(ChatRequest::Name(who, id, (x, y)));
                } else {
                    self.wake();
                    self.requests.push(ChatRequest::Focus);
                }
                true
            }
            _ => self.active,
        }
    }

    fn tick(&mut self, _ui: &mut UiRoot, _me: ElementId, dt: f64) {
        self.clock += dt;
        if self.window {
            self.touched = self.clock;
        }
        if self.hovered && !self.active {
            self.dwell += dt;
            if self.dwell >= WAKE_DWELL {
                self.touched = self.clock;
            }
        } else {
            self.dwell = 0.0;
        }
        if self.typing || (self.hovered && self.active) || self.scroll > 0 {
            self.touched = self.clock;
        }
        let up = self.clock - self.touched < STAY;
        if up && !self.active {
            self.active = true;
            self.log.seen(self.current);
        } else if !up && self.active {
            self.active = false;
            self.scroll = 0;
            self.waiting = 0;
        }
        if self.active {
            self.log.seen(self.current);
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let shadow = p.text_shadow;
        self.names.clear();
        self.new_button = None;
        self.enter_button = None;
        self.button = None;
        self.suggestion_spots.clear();
        self.search_spots = [None; 4];
        self.tab_spots.clear();
        self.row_spots = [None; 5];
        let box_h = if self.active { BOX } else { IDLE_BOX };
        let box_top = BOX_TOP + BOX - box_h;
        if self.window {
            self.draw_window_frame(rc);
        } else if self.active {
            if self.searching {
                self.draw_search_row(rc);
            } else {
                self.draw_tabs(rc);
            }
            // The transcript's glass.
            let round = CornerRadii::all(self.theme.metrics.window_radius);
            rc.fill_rounded_rect(0.0, box_top, WIDTH, box_h, round, hex(0, 0.45));
            rc.stroke_rounded_rect(0.0, box_top, WIDTH, box_h, round, hex(0xC9_A062, 0.3), 1.0);
        }
        let room = if self.active {
            self.room()
        } else {
            IDLE_OPACITY.len()
        };
        let (rows, more) = if self.active {
            let skip = self.scroll;
            self.visible_rows(room, skip)
        } else {
            self.idle_rows()
        };
        let bottom = box_top + box_h - PAD.2;
        let top = bottom - rows.len() as f32 * ROW;
        rc.push_clip(0.0, box_top, WIDTH, box_h);
        let lines: HashMap<u64, ShownLine> = self
            .log
            .lines
            .iter()
            .rev()
            .take(400)
            .map(|l| (l.sequence, l.clone()))
            .collect();
        let idle_from = rows.len().saturating_sub(IDLE_OPACITY.len());
        for (i, row) in rows.iter().enumerate() {
            let Some(line) = lines.get(&row.sequence) else {
                continue;
            };
            let y = top + i as f32 * ROW;
            let alpha = if self.active {
                let q = self.query.trim().to_lowercase();
                if self.searching && !q.is_empty() && !line_holds(line, &q) {
                    0.38
                } else {
                    1.0
                }
            } else {
                IDLE_OPACITY
                    .get(i - idle_from.min(i))
                    .copied()
                    .unwrap_or(1.0)
            };
            rc.push_alpha(alpha);
            self.draw_row(rc, line, row, y);
            rc.pop_alpha();
        }
        rc.pop_clip();
        if self.active {
            // The scroll thumb.
            let total = self.all_rows().max(1);
            if total > room {
                let track_top = box_top + PAD.0;
                let track_h = box_h - PAD.0 - PAD.2;
                rc.fill_rounded_rect(
                    WIDTH - 7.0,
                    track_top,
                    3.0,
                    track_h,
                    CornerRadii::all(1.5),
                    hex(0xFF_FFFF, 0.08),
                );
                let share = room as f32 / total as f32;
                let thumb = (track_h * share).max(12.0);
                let from_bottom = self.scroll as f32 / total as f32 * track_h;
                let thumb_top = (track_top + track_h - from_bottom - thumb).max(track_top);
                rc.fill_rounded_rect(
                    WIDTH - 7.0,
                    thumb_top,
                    3.0,
                    thumb,
                    CornerRadii::all(1.5),
                    hex(0xC9_A062, 0.7),
                );
            }
            if self.scroll > 0 {
                self.draw_reading_back(rc, box_top, box_h);
            }
            let _ = more;
            if self.has_entry() {
                self.draw_entry(rc);
            }
            if self.typing && !self.suggestions.is_empty() {
                self.draw_suggestions(rc);
            }
        } else if let Some(f) = self.fonts.small.clone() {
            // "Enter to chat" under the lines.
            let w = text::measure(&f, ENTER_WORDS, 0.0) + 16.0;
            let (bx, by) = (0.0, ENTRY_TOP + 4.0);
            rc.fill_rounded_rect(bx, by, w, 22.0, CornerRadii::all(3.0), hex(0, 0.25));
            let ty = (by + (22.0 - f.line_height) * 0.5).round();
            text::draw(
                rc,
                &f,
                ENTER_WORDS,
                bx + 8.0,
                ty,
                TextStyle::plain(Color { a: 0.7, ..TEXT }).shadowed(shadow),
            );
            self.enter_button = Some((bx, by, w, 22.0));
        }
    }
}

impl ChatPanel {
    /// The last few rows, for the idle view.
    fn idle_rows(&mut self) -> (Vec<Row>, bool) {
        let main = self.current;
        self.current = 0;
        let rows = self.visible_rows(IDLE_OPACITY.len(), 0);
        self.current = main;
        rows
    }

    fn draw_row(&mut self, rc: &mut RenderContext<'_>, line: &ShownLine, row: &Row, y: f32) {
        let shadow = self.theme.palette.text_shadow;
        let color = line.kind.color();
        let mut x = PAD.3;
        if line.mention {
            rc.fill_rounded_rect(
                x - 6.0,
                y,
                WIDTH - PAD.1 - PAD.3 + 12.0,
                ROW,
                CornerRadii::all(3.0),
                hex(0xE0_B068, 0.16),
            );
        }
        if self.times {
            if row.first
                && let (Some(t), Some(f)) = (&line.time, &self.fonts.small)
            {
                let ty = baseline(y, f);
                text::draw(
                    rc,
                    f,
                    t,
                    x,
                    ty,
                    TextStyle::plain(TIME_COLOR).shadowed(shadow),
                );
            }
            x += TIME;
        }
        if row.first {
            let mut px = x;
            if let (Some(tag), Some(f)) = (line.kind.tag(), &self.fonts.tag) {
                let ty = baseline(y, f) + 1.0;
                text::draw(
                    rc,
                    f,
                    tag,
                    px,
                    ty,
                    TextStyle::plain(color)
                        .tracked(TAG_TRACKING)
                        .shadowed(shadow),
                );
                px += text::measure(f, tag, TAG_TRACKING) + 6.0;
            }
            if let (Some(who), Some(f)) = (&line.who, &self.fonts.name) {
                let ty = baseline(y, f);
                let w = text::measure(f, who, 0.0);
                text::draw(rc, f, who, px, ty, TextStyle::plain(color).shadowed(shadow));
                self.names
                    .push(((px, y, w, ROW), who.clone(), line.who_guid));
                px += w;
                text::draw(rc, f, ":", px, ty, TextStyle::plain(color).shadowed(shadow));
            }
        }
        if let Some(f) = &self.fonts.body {
            let words_color = if line.kind == LineKind::Say {
                TEXT
            } else {
                color
            };
            let ty = baseline(y, f);
            let q = self.query.trim().to_lowercase();
            if self.searching && !q.is_empty() {
                let low = row.text.to_lowercase();
                if low.len() == row.text.len() {
                    for (at, _) in low.match_indices(&q) {
                        let left = x + row.x + text::measure(f, &row.text[..at], 0.0);
                        let w = text::measure(f, &row.text[at..at + q.len()], 0.0);
                        rc.fill_rounded_rect(
                            left - 1.0,
                            y + 1.0,
                            w + 2.0,
                            ROW - 2.0,
                            CornerRadii::all(2.0),
                            Color { a: 0.55, ..GOLD },
                        );
                    }
                }
            }
            text::draw(
                rc,
                f,
                &row.text,
                x + row.x,
                ty,
                TextStyle::plain(words_color).shadowed(shadow),
            );
        }
    }

    fn draw_entry(&mut self, rc: &mut RenderContext<'_>) {
        let top = ENTRY_TOP;
        rc.fill_rounded_rect(0.0, top, WIDTH, ENTRY, CornerRadii::all(4.0), hex(0, 0.62));
        rc.stroke_rounded_rect(
            0.0,
            top,
            WIDTH,
            ENTRY,
            CornerRadii::all(4.0),
            hex(0xC9_A062, 0.8),
            1.0,
        );
        let c = channel_color(self.channel);
        let bw = self.button_width();
        let (bx, by) = (4.0, top + 4.0);
        rc.fill_rounded_rect(
            bx,
            by,
            bw,
            24.0,
            CornerRadii::all(3.0),
            Color { a: 0.12, ..c },
        );
        rc.stroke_rounded_rect(
            bx,
            by,
            bw,
            24.0,
            CornerRadii::all(3.0),
            Color { a: 0.4, ..c },
            1.0,
        );
        rc.fill_ellipse(bx + 8.0, by + 12.0 - 3.5, 7.0, 7.0, c);
        if let Some(f) = self.fonts.name.clone() {
            let words = self.channel_words();
            let room = bw - 8.0 - 7.0 - 6.0 - 6.0 - 10.0 - 8.0;
            let words = crate::ui::modern::hud::target::fit(&f, &words, room);
            let ty = (by + (24.0 - f.line_height) * 0.5).round();
            text::draw(rc, &f, &words, bx + 21.0, ty, TextStyle::plain(c));
        }
        // The caret: the menu opens upwards.
        let (cx, cy) = (bx + bw - 8.0 - 5.0, by + 12.0);
        rc.smooth_line(cx - 4.0, cy + 2.0, cx, cy - 2.0, c, 1.8);
        rc.smooth_line(cx, cy - 2.0, cx + 4.0, cy + 2.0, c, 1.8);
        self.button = Some((bx, by, bw, 24.0));
        if let Some(f) = self.fonts.small.clone() {
            let hw = self.hint_width();
            let ty = (top + (ENTRY - f.line_height) * 0.5).round();
            text::draw(
                rc,
                &f,
                ENTER_HINT,
                WIDTH - 10.0 - hw,
                ty,
                TextStyle::plain(TIME_COLOR),
            );
            if self.entry_empty && !self.typing {
                let (fl, _, fw, _) = self.field_rect();
                let prompt = channel_prompt(self.channel, self.tell_target.as_deref());
                let prompt = crate::ui::modern::hud::target::fit(&f, &prompt, fw - 10.0);
                text::draw(
                    rc,
                    &f,
                    &prompt,
                    fl + 5.0,
                    ty,
                    TextStyle::plain(Color { a: 0.55, ..TEXT }),
                );
            }
        }
    }

    fn draw_suggestions(&mut self, rc: &mut RenderContext<'_>) {
        let (Some(bold), Some(small)) = (self.fonts.name.clone(), self.fonts.small.clone()) else {
            return;
        };
        const ROW_H: f32 = 30.0;
        const FOOT: f32 = 24.0;
        let h = 12.0 + self.suggestions.len() as f32 * ROW_H + FOOT;
        let top = ENTRY_TOP - 6.0 - h;
        rc.begin_overlay();
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(
            0.0,
            top,
            WIDTH,
            h,
            6.0,
            14.0,
            34.0,
            self.theme.palette.shadow,
        );
        rc.fill_rounded_rect(0.0, top, WIDTH, h, round, hex(0x18_130E, 0.97));
        rc.stroke_rounded_rect(0.0, top, WIDTH, h, round, hex(0xC9_A062, 0.8), 1.0);
        for (i, cmd) in self.suggestions.clone().iter().enumerate() {
            let y = top + 6.0 + i as f32 * ROW_H;
            if i == 0 {
                rc.fill_rounded_rect(
                    6.0,
                    y,
                    WIDTH - 12.0,
                    ROW_H,
                    CornerRadii::all(3.0),
                    hex(0xE0_B068, 0.16),
                );
            }
            let ty = (y + (ROW_H - bold.line_height) * 0.5).round();
            text::draw(rc, &bold, cmd.verb, 14.0, ty, TextStyle::plain(GOLD));
            let mut x = 14.0 + text::measure(&bold, cmd.verb, 0.0) + 8.0;
            let sty = (y + (ROW_H - small.line_height) * 0.5).round();
            if !cmd.alias.is_empty() {
                let also = format!("also {}", cmd.alias);
                text::draw(rc, &small, &also, x, sty, TextStyle::plain(TIME_COLOR));
                x += text::measure(&small, &also, 0.0) + 12.0;
            }
            let target = cmd
                .reply
                .then(|| self.last_teller.clone())
                .flatten()
                .unwrap_or_default();
            let tw = text::measure(&small, &target, 0.0);
            if !target.is_empty() {
                text::draw(
                    rc,
                    &small,
                    &target,
                    WIDTH - 14.0 - tw,
                    sty,
                    TextStyle::plain(LineKind::Tell.color()),
                );
            }
            let room = WIDTH - 14.0 - tw - 12.0 - x;
            let about = crate::ui::modern::hud::target::fit(&small, cmd.about, room.max(0.0));
            text::draw(rc, &small, &about, x, sty, TextStyle::plain(MUTED));
            self.suggestion_spots.push((6.0, y, WIDTH - 12.0, ROW_H));
        }
        let fy = top + h - FOOT + ((FOOT - small.line_height) * 0.5).round() - 3.0;
        text::draw(
            rc,
            &small,
            "Tab completes \u{b7} click to pick \u{b7} @help lists every command",
            14.0,
            fy,
            TextStyle::plain(TIME_COLOR),
        );
        rc.end_overlay();
    }

    /// Where the search field goes on the panel: left, top, width, height.
    #[must_use]
    pub fn search_field_rect(&self) -> (f32, f32, f32, f32) {
        let count_w = self
            .fonts
            .small
            .as_deref()
            .map_or(110.0, |f| text::measure(f, &self.count_words(), 0.0));
        (
            26.0,
            1.0,
            (WIDTH - 26.0 - count_w - 8.0 - 3.0 * 22.0 - 12.0).max(60.0),
            TABS - 2.0,
        )
    }

    fn count_words(&self) -> String {
        let found = self.matches().len();
        let tab = self.tabs.get(self.current).map_or("", |t| t.name.as_str());
        match found {
            0 if self.query.trim().is_empty() => format!("in {tab}"),
            0 => format!("none in {tab}"),
            n => format!("{} of {n} in {tab}", self.match_at.min(n - 1) + 1),
        }
    }

    fn draw_search_row(&mut self, rc: &mut RenderContext<'_>) {
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(0.0, 0.0, WIDTH, TABS, round, hex(0, 0.6));
        rc.stroke_rounded_rect(0.0, 0.0, WIDTH, TABS, round, hex(0xC9_A062, 0.8), 1.0);
        magnifier(rc, 8.0, (TABS - 12.0) * 0.5, GOLD);
        let mut x = WIDTH - 4.0 - 3.0 * 22.0;
        if let Some(f) = self.fonts.small.clone() {
            let words = self.count_words();
            let w = text::measure(&f, &words, 0.0);
            let ty = ((TABS - f.line_height) * 0.5).round();
            text::draw(rc, &f, &words, x - 8.0 - w, ty, TextStyle::plain(MUTED));
        }
        let mid = TABS * 0.5;
        for i in 0..3 {
            let cx = x + 11.0;
            match i {
                0 => {
                    rc.smooth_line(cx - 4.0, mid + 2.0, cx, mid - 2.0, TEXT, 1.6);
                    rc.smooth_line(cx, mid - 2.0, cx + 4.0, mid + 2.0, TEXT, 1.6);
                }
                1 => {
                    rc.smooth_line(cx - 4.0, mid - 2.0, cx, mid + 2.0, TEXT, 1.6);
                    rc.smooth_line(cx, mid + 2.0, cx + 4.0, mid - 2.0, TEXT, 1.6);
                }
                _ => {
                    rc.smooth_line(cx - 4.0, mid - 4.0, cx + 4.0, mid + 4.0, TEXT, 1.6);
                    rc.smooth_line(cx + 4.0, mid - 4.0, cx - 4.0, mid + 4.0, TEXT, 1.6);
                }
            }
            self.search_spots[i] = Some((x, 1.0, 22.0, TABS - 2.0));
            x += 22.0;
        }
    }

    /// A window's frame and title bar: the tab's name, then its buttons
    /// (what it shows, put back as a tab, close).
    fn draw_window_frame(&mut self, rc: &mut RenderContext<'_>) {
        let h = self.height();
        let opacity = self.tabs.first().map_or(0.62, |t| t.opacity);
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, WIDTH, h, 6.0, 10.0, 30.0, hex(0, 0.45));
        rc.fill_rounded_rect(0.0, 0.0, WIDTH, h, round, hex(0x14_100C, opacity));
        rc.stroke_rounded_rect(0.0, 0.0, WIDTH, h, round, hex(0xC9_A062, 0.8), 1.0);
        rc.fill_rounded_rect(
            0.0,
            TABS + 2.0,
            WIDTH,
            1.0,
            CornerRadii::all(0.0),
            hex(0xC9_A062, 0.3),
        );
        if let (Some(f), Some(tab)) = (self.fonts.tab.clone(), self.tabs.first()) {
            let ty = ((TABS - f.line_height) * 0.5).round() + 2.0;
            text::draw(rc, &f, &tab.name, 10.0, ty, TextStyle::plain(TEXT));
        }
        let c = hex(0xE9_D3AE, 1.0);
        for (i, slot) in [2usize, 4].into_iter().enumerate() {
            let x = WIDTH - 4.0 - (2 - i) as f32 * TABS;
            let (cx, cy) = (x + TABS * 0.5, 2.0 + TABS * 0.5);
            match slot {
                2 => {
                    // What it shows: three lines narrowing.
                    for (k, w) in [10.0, 7.0, 4.0].into_iter().enumerate() {
                        let y = cy - 4.0 + k as f32 * 4.0;
                        rc.smooth_line(cx - w * 0.5, y, cx + w * 0.5, y, c, 1.6);
                    }
                }
                _ => {
                    rc.smooth_line(cx - 4.5, cy - 4.5, cx + 4.5, cy + 4.5, c, 1.6);
                    rc.smooth_line(cx + 4.5, cy - 4.5, cx - 4.5, cy + 4.5, c, 1.6);
                }
            }
            self.row_spots[slot] = Some((x, 2.0, TABS, TABS));
        }
    }

    fn draw_tabs(&mut self, rc: &mut RenderContext<'_>) {
        let (Some(f), Some(badge)) = (self.fonts.tab.clone(), self.fonts.tag.clone()) else {
            return;
        };
        // The tabs share what the + , pop-out and search buttons leave,
        // their padding giving way first, then the longest names (cut
        // short with an ellipsis).
        let shown: Vec<(f32, f32)> = self
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.window)
            .map(|(i, t)| {
                let unread = self.log.unread(i);
                let badge_w = if unread > 0 && i != self.current {
                    (text::measure(&badge, &unread.to_string(), 0.0) + 8.0).max(15.0) + 5.0
                } else {
                    0.0
                };
                (text::measure(&f, &t.name, 0.0), badge_w)
            })
            .collect();
        let room = WIDTH - 3.0 * (TABS + TAB_GAP) - 4.0;
        let natural: f32 = shown.iter().map(|(l, b)| l + b + 18.0 + TAB_GAP).sum();
        let excess = (natural - room).max(0.0);
        let pad = (9.0 - excess / (2.0 * shown.len().max(1) as f32)).max(4.0);
        let fixed: f32 = shown.iter().map(|(_, b)| b + 2.0 * pad + TAB_GAP).sum();
        let labels: Vec<f32> = shown.iter().map(|(l, _)| *l).collect();
        let cap = label_cap(&labels, room - fixed);
        let mut x = 0.0;
        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.window {
                continue;
            }
            let on = i == self.current;
            let unread = self.log.unread(i);
            let mut label_w = text::measure(&f, &tab.name, 0.0);
            let name = if label_w > cap {
                label_w = cap;
                shorten(&f, &tab.name, cap)
            } else {
                tab.name.clone()
            };
            let badge_text = unread.to_string();
            let badge_w = if unread > 0 && !on {
                (text::measure(&badge, &badge_text, 0.0) + 8.0).max(15.0) + 5.0
            } else {
                0.0
            };
            let w = pad + label_w + badge_w + pad;
            let round = CornerRadii::all(3.0);
            rc.fill_rounded_rect(x, 0.0, w, TABS, round, hex(0, if on { 0.62 } else { 0.3 }));
            if on {
                rc.stroke_rounded_rect(x, 0.0, w, TABS, round, hex(0xC9_A062, 0.8), 1.0);
            }
            let ty = ((TABS - f.line_height) * 0.5).round();
            text::draw(
                rc,
                &f,
                &name,
                x + pad,
                ty,
                TextStyle::plain(if on { GOLD } else { TEXT }),
            );
            if badge_w > 0.0 {
                let bw = badge_w - 5.0;
                let bx = x + pad + label_w + 5.0;
                let fill = match tab.talk {
                    Some(ac_runtime::communication::ChatChannelKind::Fellowship) => {
                        LineKind::Fellow.color()
                    }
                    Some(ac_runtime::communication::ChatChannelKind::Tell) => {
                        LineKind::Tell.color()
                    }
                    _ => GOLD,
                };
                rc.fill_rounded_rect(
                    bx,
                    (TABS - 15.0) * 0.5,
                    bw,
                    15.0,
                    CornerRadii::all(7.5),
                    fill,
                );
                let by = ((TABS - badge.line_height) * 0.5).round();
                text::draw_centred(
                    rc,
                    &badge,
                    &badge_text,
                    bx + bw * 0.5,
                    by,
                    TextStyle::plain(INK),
                );
            }
            self.tab_spots.push((i, x, w));
            x += w + TAB_GAP;
        }
        // + for a new tab after the tabs.
        if x + TABS <= WIDTH - 2.0 * (TABS + TAB_GAP) {
            square_button(rc, x, PlusOrPop::Plus);
            self.row_spots[0] = Some((x, 0.0, TABS, TABS));
        }
        // Pop the shown tab out (not Main), left of search.
        if self.current != 0 {
            let px = WIDTH - 2.0 * TABS - TAB_GAP;
            square_button(rc, px, PlusOrPop::Pop);
            self.row_spots[1] = Some((px, 0.0, TABS, TABS));
        }
        // Search, at the row's right end.
        let sx = WIDTH - TABS;
        rc.fill_rounded_rect(sx, 0.0, TABS, TABS, CornerRadii::all(3.0), hex(0, 0.3));
        magnifier(
            rc,
            sx + (TABS - 12.0) * 0.5,
            (TABS - 12.0) * 0.5,
            hex(0xE9_D3AE, 1.0),
        );
        self.search_spots[3] = Some((sx, 0.0, TABS, TABS));
    }

    fn draw_reading_back(&mut self, rc: &mut RenderContext<'_>, box_top: f32, box_h: f32) {
        let Some(f) = self.fonts.small.clone() else {
            return;
        };
        let words = "Reading back";
        let w = text::measure(&f, words, 0.0) + 16.0;
        let x = (WIDTH - w) * 0.5;
        rc.fill_rounded_rect(
            x,
            box_top + 6.0,
            w,
            18.0,
            CornerRadii::all(9.0),
            hex(0, 0.55),
        );
        text::draw(
            rc,
            &f,
            words,
            x + 8.0,
            (box_top + 6.0 + (18.0 - f.line_height) * 0.5).round(),
            TextStyle::plain(MUTED),
        );
        let label = match self.waiting {
            0 => "Back to the newest".to_owned(),
            1 => "1 new message".to_owned(),
            n => format!("{n} new messages"),
        };
        let Some(bold) = self.fonts.name.clone() else {
            return;
        };
        let bw = text::measure(&bold, &label, 0.0) + 24.0 + 12.0;
        let bx = (WIDTH - bw) * 0.5;
        let by = box_top + box_h - 8.0 - 26.0;
        rc.soft_shadow(bx, by, bw, 26.0, 13.0, 4.0, 12.0, hex(0, 0.45));
        rc.fill_rounded_rect(
            bx,
            by,
            bw,
            26.0,
            CornerRadii::all(13.0),
            hex(0xE0_B068, 0.92),
        );
        text::draw(
            rc,
            &bold,
            &label,
            bx + 12.0,
            (by + (26.0 - bold.line_height) * 0.5).round(),
            TextStyle::plain(INK),
        );
        // A small arrow down.
        let ax = bx + bw - 12.0 - 6.0;
        let ay = by + 13.0;
        rc.smooth_line(ax, ay - 5.0, ax, ay + 5.0, INK, 2.0);
        rc.smooth_line(ax - 4.0, ay + 1.0, ax, ay + 5.0, INK, 2.0);
        rc.smooth_line(ax + 4.0, ay + 1.0, ax, ay + 5.0, INK, 2.0);
        self.new_button = Some((bx, by, bw, 26.0));
    }
}

/// `line` holds `query` (lower case) in its words or its speaker's name.
/// The widest a tab's name may be drawn so that names `widths` fit in
/// `room` together: the longest are cut down to the same width first, and
/// none below a few letters (the row then runs out of room).
fn label_cap(widths: &[f32], room: f32) -> f32 {
    const LEAST: f32 = 28.0;
    let total: f32 = widths.iter().sum();
    if total <= room {
        return f32::INFINITY;
    }
    let mut sorted = widths.to_vec();
    sorted.sort_by(f32::total_cmp);
    let mut left = room;
    for (k, w) in sorted.iter().enumerate() {
        let rest = (sorted.len() - k) as f32;
        let share = left / rest;
        if *w > share {
            return share.max(LEAST);
        }
        left -= w;
    }
    f32::INFINITY
}

/// `name` cut on a whole character with an ellipsis to fit `width`.
fn shorten(font: &UiFont, name: &str, width: f32) -> String {
    let dots = if font.glyph('\u{2026}').is_some() {
        "\u{2026}"
    } else {
        "..."
    };
    for (n, _) in name.char_indices().rev().filter(|&(n, _)| n > 0) {
        let cut = format!("{}{dots}", name[..n].trim_end());
        if text::measure(font, &cut, 0.0) <= width {
            return cut;
        }
    }
    dots.to_owned()
}

fn line_holds(line: &ShownLine, query: &str) -> bool {
    line.words.to_lowercase().contains(query)
        || line
            .who
            .as_ref()
            .is_some_and(|w| w.to_lowercase().contains(query))
}

/// What a small square button on the tab row shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlusOrPop {
    Plus,
    Pop,
}

/// A small square button on the tab row at `x`.
fn square_button(rc: &mut RenderContext<'_>, x: f32, what: PlusOrPop) {
    rc.fill_rounded_rect(x, 0.0, TABS, TABS, CornerRadii::all(3.0), hex(0, 0.3));
    let c = hex(0xE9_D3AE, 1.0);
    let (cx, cy) = (x + TABS * 0.5, TABS * 0.5);
    match what {
        PlusOrPop::Plus => {
            rc.smooth_line(cx - 5.0, cy, cx + 5.0, cy, c, 2.0);
            rc.smooth_line(cx, cy - 5.0, cx, cy + 5.0, c, 2.0);
        }
        PlusOrPop::Pop => {
            // A box with an arrow leaving its corner.
            rc.smooth_line(cx - 5.0, cy - 2.0, cx - 5.0, cy + 5.0, c, 1.6);
            rc.smooth_line(cx - 5.0, cy + 5.0, cx + 2.0, cy + 5.0, c, 1.6);
            rc.smooth_line(cx - 1.0, cy + 1.0, cx + 5.0, cy - 5.0, c, 1.6);
            rc.smooth_line(cx + 1.0, cy - 5.0, cx + 5.0, cy - 5.0, c, 1.6);
            rc.smooth_line(cx + 5.0, cy - 5.0, cx + 5.0, cy - 1.0, c, 1.6);
        }
    }
}

/// A magnifying glass in a 12-unit box.
fn magnifier(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
    let r = 4.2;
    let (cx, cy) = (x + 5.0, y + 5.0);
    rc.stroke_arc(cx, cy, r, 0.0, std::f32::consts::TAU, color, 1.5);
    rc.smooth_line(cx + 3.2, cy + 3.2, x + 11.5, y + 11.5, color, 1.6);
}

/// Where text in `font` sits in a row starting at `y`.
fn baseline(y: f32, font: &UiFont) -> f32 {
    (y + (ROW - font.line_height) * 0.5).round()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_core::chat::{ChatKind, LogTextType};
    use ac_runtime::communication::ChatLine;

    fn line(n: u64, class: LogTextType, sender: &str, words: &str) -> ShownLine {
        ShownLine::of(
            &ChatLine {
                sequence: n,
                kind: ChatKind::Channel,
                log_text_type: class as u32,
                combat_kind: None,
                received_unix_ms: 0,
                text: words.to_owned(),
                channel_name: String::new(),
                sender: sender.to_owned(),
                sender_guid: 9,
                words: words.to_owned(),
                display_text: String::new(),
                channel_id: 0,
                spans: None,
            },
            "Kestrel",
        )
    }

    #[test]
    fn the_first_row_starts_after_the_name() {
        let measure = |t: &str| t.len() as f32 * 10.0;
        let rows = wrap_hanging("aa bb cc dd", measure, 60.0, 100.0);
        assert_eq!(rows, vec!["aa", "bb cc dd"]);
        let rows = wrap_hanging("aaaaaa bb", measure, 60.0, 100.0);
        assert_eq!(rows, vec!["", "aaaaaa bb"]);
        assert_eq!(wrap_hanging("aa", measure, 0.0, 100.0), vec!["aa"]);
    }

    #[test]
    fn reading_back_holds_still_and_counts_what_waits() {
        let mut panel = ChatPanel::new(Horizon::BRONZE, ChatFonts::default());
        panel.wake();
        for n in 0..40 {
            panel.push(line(n, LogTextType::Speech, "Osk", "hello"));
        }
        panel.scroll = 5;
        panel.push(line(40, LogTextType::Speech, "Osk", "more"));
        assert_eq!(panel.scroll, 6);
        assert_eq!(panel.waiting, 1);
        // A line the tab does not show changes nothing.
        panel.pick_tab(2);
        panel.scroll = 5;
        panel.push(line(41, LogTextType::Speech, "Osk", "elsewhere"));
        assert_eq!(panel.scroll, 5);
        panel.follow();
        assert!(!panel.reading_back());
    }

    #[test]
    fn the_panel_rests_after_a_while_alone() {
        let mut panel = ChatPanel::new(Horizon::BRONZE, ChatFonts::default());
        let mut ui = UiRoot::new();
        let root = ui.root();
        panel.wake();
        panel.tick(&mut ui, root, 1.0);
        assert!(panel.active);
        panel.tick(&mut ui, root, STAY);
        assert!(!panel.active);
        panel.typing = true;
        panel.tick(&mut ui, root, 0.1);
        assert!(panel.active);
    }

    #[test]
    fn the_longest_tab_names_give_way_first() {
        assert_eq!(label_cap(&[40.0, 50.0], 100.0), f32::INFINITY);
        // 30 + 40 + 80 in 120: the short two stay, the long one takes 50.
        assert_eq!(label_cap(&[30.0, 80.0, 40.0], 120.0), 50.0);
        // Two long ones share what the short one leaves.
        assert_eq!(label_cap(&[30.0, 90.0, 90.0], 130.0), 50.0);
        // Never below a few letters.
        assert_eq!(label_cap(&[90.0, 90.0], 20.0), 28.0);
    }

    #[test]
    fn a_pointer_resting_on_the_idle_lines_wakes_the_panel() {
        let mut panel = ChatPanel::new(Horizon::BRONZE, ChatFonts::default());
        let mut ui = UiRoot::new();
        let root = ui.root();
        panel.tick(&mut ui, root, 1.0);
        assert!(!panel.active);
        let element = Element::at(0.0, 0.0, WIDTH, HEIGHT);
        // The faded lines answer the pointer; the empty space above them
        // stays the world's.
        assert!(panel.hit_test(&element, 20.0, ENTRY_TOP - 10.0));
        assert!(!panel.hit_test(&element, 20.0, 10.0));
        panel.hovered = true;
        panel.tick(&mut ui, root, 0.1);
        assert!(!panel.active, "a pointer passing over does not wake it");
        panel.tick(&mut ui, root, 0.15);
        assert!(panel.active);
        panel.hovered = false;
        panel.tick(&mut ui, root, STAY + 0.1);
        assert!(!panel.active);
    }

    #[test]
    fn unseen_lines_count_on_other_tabs_only() {
        let mut panel = ChatPanel::new(Horizon::BRONZE, ChatFonts::default());
        panel.wake();
        panel.push(line(1, LogTextType::Fellowship, "Mira", "pull"));
        assert_eq!(panel.log.unread(0), 0);
        assert_eq!(panel.log.unread(2), 1);
        panel.pick_tab(2);
        assert_eq!(panel.log.unread(2), 0);
    }
}
