//! The Recall list the button on the radar's rim opens. Places first, each
//! sending what typing its @ command sends (the command is shown beside
//! it, so players learn them), greyed with the reason where the player
//! cannot go: no house, no allegiance, not a player killer. Then the
//! recall spells the player knows, greyed outside Magic mode as spells on
//! a bar are. A click goes and closes the list; a press outside closes it.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_host::settings::CustomRecall;
use ac_runtime::gameplay::GameplayCommand;
use ac_runtime::session::SessionCommand;
use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{DragPayload, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::panels::magic::SpellbookShortcutPayload;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{DragGhost, Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::target::fit;

/// The list's width.
pub const WIDTH: f32 = 300.0;
/// Its tallest; a longer list scrolls.
pub const MAX_HEIGHT: f32 = 520.0;
const HEAD: f32 = 36.0;
const SECTION: f32 = 24.0;
const ROW: f32 = 40.0;
const PAD: f32 = 6.0;
const TILE: f32 = 26.0;
const WHEEL_STEP: f32 = 2.0 * ROW;
const ADD_ROW: f32 = 32.0;

/// A place a recall goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// The last lifestone tied to.
    Lifestone,
    /// The player's house.
    House,
    /// The monarch's mansion.
    Mansion,
    /// The allegiance's hometown.
    Hometown,
    /// The Marketplace.
    Marketplace,
    /// The player killer arena.
    PkArena,
    /// The PK lite arena.
    PkLiteArena,
}

impl Place {
    /// Every place, in the list's order.
    pub const ALL: [Self; 7] = [
        Self::Lifestone,
        Self::House,
        Self::Mansion,
        Self::Hometown,
        Self::Marketplace,
        Self::PkArena,
        Self::PkLiteArena,
    ];

    /// Its name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Lifestone => "Lifestone",
            Self::House => "House",
            Self::Mansion => "Allegiance mansion",
            Self::Hometown => "Allegiance hometown",
            Self::Marketplace => "Marketplace",
            Self::PkArena => "Player killer arena",
            Self::PkLiteArena => "PK lite arena",
        }
    }

    /// What it is, under the name.
    #[must_use]
    pub fn about(self) -> &'static str {
        match self {
            Self::Lifestone => "The last lifestone you tied to",
            Self::House => "Your house",
            Self::Mansion => "Your monarch's mansion",
            Self::Hometown => "Set by your monarch",
            Self::Marketplace => "The Marketplace",
            Self::PkArena => "For player killers",
            Self::PkLiteArena => "For PK lite players",
        }
    }

    /// The name of the icon it shows (as a recall of the player's own
    /// names its icon), so it can go on a bar.
    #[must_use]
    pub fn icon_name(self) -> &'static str {
        match self {
            Self::Lifestone => "lifestone",
            Self::House => "house",
            Self::Mansion => "mansion",
            Self::Hometown => "flag",
            Self::Marketplace => "coin",
            Self::PkArena | Self::PkLiteArena => "swords",
        }
    }

    /// The @ command it sends, as typed.
    #[must_use]
    pub fn command(self) -> &'static str {
        match self {
            Self::Lifestone => "@ls",
            Self::House => "@hr",
            Self::Mansion => "@hom",
            Self::Hometown => "@ah",
            Self::Marketplace => "@mp",
            Self::PkArena => "@pkarena",
            Self::PkLiteArena => "@pklarena",
        }
    }
}

/// What decides which recalls the player can use. Unknown facts leave a
/// place open: the server answers for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecallFacts {
    /// The player owns a house (none: not told yet).
    pub owns_house: Option<bool>,
    /// The player is in an allegiance (none: not told yet).
    pub in_allegiance: Option<bool>,
    /// The player is a player killer.
    pub player_killer: bool,
    /// The player is PK lite.
    pub pk_lite: bool,
    /// The player is in Magic mode.
    pub magic: bool,
    /// The recall spells the player knows: id and name.
    pub spells: Vec<(u32, String)>,
    /// The server (world) played on, when known.
    pub server: Option<String>,
    /// The player's own recalls shown on it, with their place in the
    /// saved list.
    pub own: Vec<(usize, CustomRecall)>,
}

impl RecallFacts {
    /// Why the player cannot use `place`, if they cannot.
    #[must_use]
    pub fn refusal(&self, place: Place) -> Option<&'static str> {
        match place {
            Place::House if self.owns_house == Some(false) => Some("You have no house"),
            Place::Mansion | Place::Hometown if self.in_allegiance == Some(false) => {
                Some("You are not in an allegiance")
            }
            Place::PkArena if !self.player_killer => Some("Player killers only"),
            Place::PkLiteArena if !self.pk_lite => Some("PK lite only"),
            _ => None,
        }
    }
}

/// The recall spells shown first, in order, with what each does.
const KNOWN_SPELLS: [(&str, &str); 4] = [
    ("Portal Recall", "The last portal you went through"),
    ("Lifestone Recall", "As @ls, cast as a spell"),
    ("Primary Portal Recall", "Your primary portal tie"),
    ("Secondary Portal Recall", "Your secondary portal tie"),
];

/// The recall spells among `known` (id, name): the four usual ones first
/// in their order, then any other spell named as a recall, by name.
#[must_use]
pub fn recall_spells(known: impl IntoIterator<Item = (u32, String)>) -> Vec<(u32, String)> {
    let mut spells: Vec<(u32, String)> = known
        .into_iter()
        .filter(|(_, name)| name.contains("Recall"))
        .collect();
    let rank = |name: &str| {
        KNOWN_SPELLS
            .iter()
            .position(|(n, _)| *n == name)
            .unwrap_or(KNOWN_SPELLS.len())
    };
    spells.sort_by(|a, b| {
        rank(&a.1)
            .cmp(&rank(&b.1))
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });
    spells.dedup_by(|a, b| a.1 == b.1);
    spells
}

fn spell_about(name: &str) -> &'static str {
    KNOWN_SPELLS
        .iter()
        .find(|(n, _)| *n == name)
        .map_or("Cast as a spell", |(_, about)| about)
}

/// A row of the list.
#[derive(Debug, Clone, PartialEq)]
enum Row {
    Section(String),
    Place(Place),
    Spell {
        id: u32,
        name: String,
        icon: Option<TextureRef>,
    },
    Own {
        index: usize,
        recall: CustomRecall,
    },
    AddOwn,
}

/// What the player asked of their own recalls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnRequest {
    /// Add one.
    Add,
    /// Change the one at this place in the saved list.
    Edit(usize),
    /// Move it up (-1) or down (+1) among those shown.
    Move(usize, i32),
    /// Remove it.
    Remove(usize),
}

/// The icons a recall of the player's own can have, by saved name.
pub const OWN_ICONS: [&str; 7] = [
    "recall",
    "lifestone",
    "house",
    "flag",
    "coin",
    "swords",
    "mansion",
];

/// Draws own-recall icon `name` (the recall mark for an unknown one) in a
/// 14-unit box.
pub fn own_icon(rc: &mut RenderContext<'_>, name: &str, x: f32, y: f32, color: Color) {
    own_icon_sized(rc, name, x, y, 14.0, color);
}

/// The same in a `size`-unit box.
pub fn own_icon_sized(
    rc: &mut RenderContext<'_>,
    name: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) {
    let k = size / 14.0;
    let place = match name {
        "lifestone" => Place::Lifestone,
        "house" => Place::House,
        "mansion" => Place::Mansion,
        "flag" => Place::Hometown,
        "coin" => Place::Marketplace,
        "swords" => Place::PkArena,
        _ => return recall_mark_sized(rc, x, y, k, color),
    };
    place_icon(rc, place, x, y, k, color);
}

/// A recall on the move: what a row dragged out of the list carries (a
/// place or one of the player's own; a recall spell travels as a spell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallPayload {
    /// Its name.
    pub name: String,
    /// What it sends, as if typed in chat.
    pub command: String,
    /// Its icon's name.
    pub icon: String,
}

/// The action buttons a right-clicked own recall shows: what each does
/// and its word.
const ARMED: [(&str, i32); 4] = [("Edit", 0), ("Up", -1), ("Down", 1), ("Remove", 2)];

/// The fonts the list uses.
#[derive(Debug, Clone, Default)]
pub struct RecallFonts {
    /// The title and the names.
    pub name: Option<Arc<UiFont>>,
    /// What each is, the commands.
    pub small: Option<Arc<UiFont>>,
    /// Section headings.
    pub label: Option<Arc<UiFont>>,
}

/// The Recall list.
#[derive(Debug)]
pub struct RecallList {
    theme: Horizon,
    fonts: RecallFonts,
    facts: RecallFacts,
    rows: Vec<Row>,
    hovered: Option<usize>,
    scroll: f32,
    /// The own recall showing its action buttons (a row index).
    armed: Option<usize>,
    /// The row the left button went down on (what a drag carries).
    held: Option<usize>,
    /// What the player asked of their own recalls, for the window to do.
    pub requests: Vec<OwnRequest>,
}

impl RecallList {
    /// An empty list.
    #[must_use]
    pub fn new(theme: Horizon, fonts: RecallFonts) -> Self {
        let mut list = Self {
            theme,
            fonts,
            facts: RecallFacts::default(),
            rows: Vec::new(),
            hovered: None,
            scroll: 0.0,
            armed: None,
            held: None,
            requests: Vec::new(),
        };
        list.build(&mut |_| None);
        list
    }

    /// Shows what `facts` allow; `icon` finds a spell's icon.
    pub fn set_facts(
        &mut self,
        facts: RecallFacts,
        icon: &mut dyn FnMut(u32) -> Option<TextureRef>,
    ) {
        if facts != self.facts {
            self.facts = facts;
            self.build(icon);
        }
    }

    fn build(&mut self, icon: &mut dyn FnMut(u32) -> Option<TextureRef>) {
        let mut rows = vec![Row::Section("PLACES".to_owned())];
        rows.extend(Place::ALL.map(Row::Place));
        if !self.facts.spells.is_empty() {
            rows.push(Row::Section("YOUR RECALL SPELLS".to_owned()));
            for (id, name) in &self.facts.spells {
                rows.push(Row::Spell {
                    id: *id,
                    name: name.clone(),
                    icon: icon(*id),
                });
            }
        }
        let title = match &self.facts.server {
            Some(server) => format!("YOUR OWN \u{b7} {}", server.to_uppercase()),
            None => "YOUR OWN".to_owned(),
        };
        rows.push(Row::Section(title));
        for (index, recall) in &self.facts.own {
            rows.push(Row::Own {
                index: *index,
                recall: recall.clone(),
            });
        }
        rows.push(Row::AddOwn);
        self.rows = rows;
        self.armed = None;
        self.scroll = self.scroll.clamp(0.0, self.scroll_max());
    }

    fn row_height(row: &Row) -> f32 {
        match row {
            Row::Section(_) => SECTION,
            Row::AddOwn => ADD_ROW,
            _ => ROW,
        }
    }

    fn content_height(&self) -> f32 {
        self.rows.iter().map(Self::row_height).sum()
    }

    /// The list's height for what it holds.
    #[must_use]
    pub fn height(&self) -> f32 {
        (HEAD + self.content_height() + PAD).min(MAX_HEIGHT)
    }

    fn room(&self) -> f32 {
        self.height() - HEAD - PAD
    }

    fn scroll_max(&self) -> f32 {
        // Under half a unit over is rounding, not something to scroll to.
        let over = self.content_height() - self.room();
        if over < 0.5 { 0.0 } else { over }
    }

    /// Back to the top (the list opened).
    pub fn reset(&mut self) {
        self.scroll = 0.0;
        self.hovered = None;
        self.armed = None;
    }

    /// The armed row's buttons: what each asks and its left and width.
    fn armed_buttons(&self) -> Vec<(i32, &'static str, f32, f32)> {
        let Some(f) = &self.fonts.small else {
            return Vec::new();
        };
        let mut right = WIDTH - 12.0;
        let mut out = Vec::new();
        for (word, action) in ARMED.iter().rev() {
            let w = text::measure(f, word, 0.0) + 12.0;
            right -= w;
            out.push((*action, *word, right, w));
            right -= 4.0;
        }
        out.reverse();
        out
    }

    fn row_at(&self, y: f32) -> Option<usize> {
        if y < HEAD || y >= HEAD + self.room() {
            return None;
        }
        let mut top = HEAD - self.scroll;
        for (i, row) in self.rows.iter().enumerate() {
            let h = Self::row_height(row);
            if (top..top + h).contains(&y) {
                return Some(i);
            }
            top += h;
        }
        None
    }

    fn enabled(&self, row: &Row) -> bool {
        match row {
            Row::Section(_) => false,
            Row::Place(place) => self.facts.refusal(*place).is_none(),
            Row::Spell { .. } => self.facts.magic,
            Row::Own { .. } | Row::AddOwn => true,
        }
    }

    fn go(&self, ui: &mut UiRoot, row: &Row) {
        let command = match row {
            Row::Section(_) | Row::AddOwn => return,
            Row::Own { recall, .. } => SessionCommand::Chat(recall.command.clone()),
            Row::Place(place) => SessionCommand::Chat(place.command().to_owned()),
            Row::Spell { name, .. } => SessionCommand::Gameplay(GameplayCommand::Cast {
                spell: name.clone(),
                target: None,
            }),
        };
        emit(ui, PanelOutput::Session(command));
    }

    /// Opens the list (made the live pop-up: a press outside closes it).
    pub fn open(&mut self, ui: &mut UiRoot, me: ElementId) {
        self.reset();
        let height = self.height();
        if let Some(e) = ui.element_mut(me) {
            e.height = height;
        }
        ui.set_visible(me, true);
        ui.set_active_popup(me, me);
    }

    fn close(ui: &mut UiRoot, me: ElementId) {
        ui.clear_active_popup(me);
        ui.set_visible(me, false);
    }
}

impl RecallList {
    /// What the row under the pointer carries when dragged onto a bar.
    fn carried(&self) -> Option<DragPayload> {
        match self.rows.get(self.held.or(self.hovered)?)? {
            Row::Place(place) => Some(DragPayload::new(RecallPayload {
                name: place.name().to_owned(),
                command: place.command().to_owned(),
                icon: place.icon_name().to_owned(),
            })),
            Row::Own { recall, .. } => Some(DragPayload::new(RecallPayload {
                name: recall.name.clone(),
                command: recall.command.clone(),
                icon: recall.icon.clone(),
            })),
            Row::Spell { id, .. } => {
                Some(DragPayload::new(SpellbookShortcutPayload { spell_id: *id }))
            }
            Row::Section(_) | Row::AddOwn => None,
        }
    }
}

impl Widget for RecallList {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn is_drag_source(&self, _element: &Element) -> bool {
        self.armed.is_none() && self.carried().is_some()
    }

    fn drag_payload(&mut self, _ui: &mut UiRoot, _me: ElementId) -> Option<DragPayload> {
        self.carried()
    }

    fn drag_ghost(&mut self, _ui: &mut UiRoot, _me: ElementId) -> Option<DragGhost> {
        match self.rows.get(self.held.or(self.hovered)?)? {
            Row::Spell {
                icon: Some(texture),
                ..
            } => Some(DragGhost {
                texture: *texture,
                width: 32,
                height: 32,
            }),
            _ => None,
        }
    }

    fn handles_click(&self, _element: &Element) -> bool {
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
            kind::SCROLL => {
                let step = -(e.data0.signum() as f32) * WHEEL_STEP;
                self.scroll = (self.scroll + step).clamp(0.0, self.scroll_max());
                self.hovered = self.row_at(y);
                true
            }
            kind::MOUSE_DOWN => {
                self.held = self.row_at(y);
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.held = None;
                true
            }
            kind::RIGHT_CLICK => {
                let at = self.row_at(y);
                self.armed = at.filter(|&i| matches!(self.rows[i], Row::Own { .. }));
                true
            }
            kind::CLICK => {
                let x = e.data1 as f32;
                let Some(i) = self.row_at(y) else {
                    return true;
                };
                let row = self.rows[i].clone();
                if self.armed == Some(i)
                    && let Row::Own { index, .. } = row
                {
                    let action = self
                        .armed_buttons()
                        .into_iter()
                        .find(|(_, _, left, w)| (*left..left + w).contains(&x))
                        .map(|(a, ..)| a);
                    match action {
                        Some(0) => {
                            self.requests.push(OwnRequest::Edit(index));
                            Self::close(ui, me);
                        }
                        Some(2) => self.requests.push(OwnRequest::Remove(index)),
                        Some(d) => self.requests.push(OwnRequest::Move(index, d)),
                        None => self.armed = None,
                    }
                    return true;
                }
                self.armed = None;
                match row {
                    Row::AddOwn => {
                        self.requests.push(OwnRequest::Add);
                        Self::close(ui, me);
                    }
                    _ if self.enabled(&row) => {
                        self.go(ui, &row);
                        Self::close(ui, me);
                    }
                    _ => {}
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let _ = me;
        let p = Horizon::current().palette;
        let (w, h) = (WIDTH, self.height());
        let radius = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 12.0, 30.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, w, h, radius, p.popup);
        rc.stroke_rounded_rect(0.0, 0.0, w, h, radius, p.trim, 1.0);
        rc.fill(0.0, HEAD - 1.0, w, 1.0, p.trim_faint);
        recall_mark(rc, 12.0, (HEAD - 14.0) * 0.5, p.accent);
        if let Some(f) = &self.fonts.name {
            let ty = ((HEAD - f.line_height) * 0.5).round();
            text::draw(rc, f, "Recall", 34.0, ty, TextStyle::plain(p.text));
        }
        let room = self.room();
        rc.push_clip(0.0, HEAD, w, room);
        let mut top = HEAD - self.scroll;
        for (i, row) in self.rows.iter().enumerate() {
            let rh = Self::row_height(row);
            if top + rh > HEAD && top < HEAD + room {
                self.draw_row(rc, row, top, self.hovered == Some(i), self.armed == Some(i));
            }
            top += rh;
        }
        rc.pop_clip();
        if self.scroll_max() > 0.0 {
            let total = self.content_height();
            let thumb = (room * room / total).max(18.0);
            let at = HEAD + (room - thumb) * (self.scroll / self.scroll_max());
            rc.fill_rounded_rect(w - 6.0, at, 3.0, thumb, CornerRadii::all(1.5), p.trim_soft);
        }
    }
}

impl RecallList {
    fn draw_row(
        &self,
        rc: &mut RenderContext<'_>,
        row: &Row,
        top: f32,
        hovered: bool,
        armed: bool,
    ) {
        let p = Horizon::current().palette;
        let w = WIDTH;
        match row {
            Row::Section(title) => {
                if let Some(f) = &self.fonts.label {
                    let ty = top + ((SECTION - f.line_height) * 0.5).round() + 2.0;
                    text::draw(
                        rc,
                        f,
                        title,
                        14.0,
                        ty,
                        TextStyle::plain(p.text_soft).tracked(1.14),
                    );
                }
            }
            Row::AddOwn => {
                if hovered {
                    rc.fill_rounded_rect(
                        4.0,
                        top + 1.0,
                        w - 8.0,
                        ADD_ROW - 2.0,
                        CornerRadii::all(4.0),
                        p.accent_tint,
                    );
                }
                if let Some(f) = &self.fonts.name {
                    let ty = top + ((ADD_ROW - f.line_height) * 0.5).round();
                    let (cx, cy) = (12.0 + TILE * 0.5, top + ADD_ROW * 0.5);
                    rc.smooth_line(cx - 5.0, cy, cx + 5.0, cy, p.accent, 1.6);
                    rc.smooth_line(cx, cy - 5.0, cx, cy + 5.0, p.accent, 1.6);
                    text::draw(
                        rc,
                        f,
                        "Add your own\u{2026}",
                        12.0 + TILE + 10.0,
                        ty,
                        TextStyle::plain(p.accent),
                    );
                }
            }
            Row::Place(_) | Row::Spell { .. } | Row::Own { .. } => {
                let enabled = self.enabled(row);
                if hovered && enabled {
                    rc.fill_rounded_rect(
                        4.0,
                        top + 1.0,
                        w - 8.0,
                        ROW - 2.0,
                        CornerRadii::all(4.0),
                        p.accent_tint,
                    );
                }
                let fade = if enabled { 1.0 } else { 0.45 };
                let tile_y = top + (ROW - TILE) * 0.5;
                rc.fill_rounded_rect(
                    12.0,
                    tile_y,
                    TILE,
                    TILE,
                    CornerRadii::all(3.0),
                    hex(0, 0.35),
                );
                let (name, about, right) = match row {
                    Row::Place(place) => {
                        place_icon(
                            rc,
                            *place,
                            12.0 + 6.0,
                            tile_y + 6.0,
                            1.0,
                            Color {
                                a: fade,
                                ..p.accent
                            },
                        );
                        (
                            place.name().to_owned(),
                            self.facts.refusal(*place).unwrap_or(place.about()),
                            place.command().to_owned(),
                        )
                    }
                    Row::Spell { name, icon, .. } => {
                        if let Some(t) = icon {
                            rc.sprite(
                                *t,
                                12.0 + 1.0,
                                tile_y + 1.0,
                                TILE - 2.0,
                                TILE - 2.0,
                                (0.0, 0.0, 1.0, 1.0),
                                Color::rgba(1.0, 1.0, 1.0, fade),
                            );
                        }
                        let right = if self.facts.magic {
                            String::new()
                        } else {
                            "Magic mode".to_owned()
                        };
                        let about = if self.facts.magic {
                            spell_about(name)
                        } else {
                            "Switch to Magic mode to cast"
                        };
                        (name.clone(), about, right)
                    }
                    Row::Own { recall, .. } => {
                        own_icon(rc, &recall.icon, 12.0 + 6.0, tile_y + 6.0, p.accent);
                        let about = if recall.server.is_some() {
                            "This server only \u{b7} right-click to change"
                        } else {
                            "Every server \u{b7} right-click to change"
                        };
                        let right = if armed {
                            String::new()
                        } else {
                            recall.command.clone()
                        };
                        (recall.name.clone(), about, right)
                    }
                    Row::Section(_) | Row::AddOwn => unreachable!(),
                };
                let mut right_left = w - 12.0;
                if armed && let Some(f) = &self.fonts.small {
                    let buttons = self.armed_buttons();
                    for (action, word, left, bw) in &buttons {
                        let (bh, by) = (22.0, top + (ROW - 22.0) * 0.5);
                        let edge = if *action == 2 { p.bad } else { p.trim_soft };
                        rc.fill_rounded_rect(
                            *left,
                            by,
                            *bw,
                            bh,
                            CornerRadii::all(3.0),
                            tone::ground(0.9),
                        );
                        rc.stroke_rounded_rect(
                            *left,
                            by,
                            *bw,
                            bh,
                            CornerRadii::all(3.0),
                            edge,
                            1.0,
                        );
                        let ty = by + ((bh - f.line_height) * 0.5).round();
                        text::draw(rc, f, word, left + 6.0, ty, TextStyle::plain(p.text));
                    }
                    right_left = buttons.first().map_or(right_left, |b| b.2);
                }
                if let Some(f) = &self.fonts.small
                    && !right.is_empty()
                {
                    let rw = text::measure(f, &right, 0.0);
                    right_left -= rw;
                    let ty = top + ((ROW - f.line_height) * 0.5).round();
                    text::draw(
                        rc,
                        f,
                        &right,
                        right_left,
                        ty,
                        TextStyle::plain(Color { a: fade, ..p.muted }),
                    );
                }
                let x = 12.0 + TILE + 10.0;
                let room = (right_left - 8.0 - x).max(0.0);
                if let Some(f) = &self.fonts.name {
                    let name = fit(f, &name, room);
                    text::draw(
                        rc,
                        f,
                        &name,
                        x,
                        top + 4.0,
                        TextStyle::plain(Color { a: fade, ..p.text }),
                    );
                }
                if let Some(f) = &self.fonts.small {
                    let about = fit(f, about, room);
                    text::draw(
                        rc,
                        f,
                        &about,
                        x,
                        top + ROW - 4.0 - f.line_height,
                        TextStyle::plain(Color {
                            a: fade,
                            ..p.text_soft
                        }),
                    );
                }
            }
        }
    }
}

/// The recall mark: rings closing on a point, in a 14-unit box.
pub fn recall_mark(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
    recall_mark_sized(rc, x, y, 1.0, color);
}

/// The recall mark `k` times its size.
fn recall_mark_sized(rc: &mut RenderContext<'_>, x: f32, y: f32, k: f32, color: Color) {
    let (cx, cy) = (x + 7.0 * k, y + 7.0 * k);
    let half = std::f32::consts::FRAC_PI_2;
    let width = stroke_width(k, 1.6);
    rc.stroke_arc(cx, cy, 6.0 * k, 0.0, 3.0 * half, color, width);
    rc.stroke_arc(cx, cy, 3.3 * k, 0.0, 3.0 * half, color, width);
    rc.fill_ellipse(cx - k, cy - k, 2.0 * k, 2.0 * k, color);
}

/// A line `width` wide at the 14-unit size, for an icon `k` times that.
fn stroke_width(k: f32, width: f32) -> f32 {
    (width * k.sqrt()).min(width * 1.6)
}

/// A small mark for each place, in a box 14 units times `k`.
fn place_icon(rc: &mut RenderContext<'_>, place: Place, x: f32, y: f32, k: f32, c: Color) {
    let width = stroke_width(k, 1.5);
    let line = |rc: &mut RenderContext<'_>, a: (f32, f32), b: (f32, f32)| {
        rc.smooth_line(x + a.0 * k, y + a.1 * k, x + b.0 * k, y + b.1 * k, c, width);
    };
    match place {
        Place::Lifestone => {
            rc.stroke_ellipse(x + 3.0 * k, y + k, 8.0 * k, 8.0 * k, c, width);
            line(rc, (7.0, 9.0), (7.0, 13.0));
            line(rc, (4.0, 13.0), (10.0, 13.0));
        }
        Place::House | Place::Mansion => {
            line(rc, (1.0, 7.0), (7.0, 1.5));
            line(rc, (7.0, 1.5), (13.0, 7.0));
            line(rc, (3.0, 6.0), (3.0, 13.0));
            line(rc, (11.0, 6.0), (11.0, 13.0));
            line(rc, (3.0, 13.0), (11.0, 13.0));
            if place == Place::Mansion {
                line(rc, (7.0, 13.0), (7.0, 9.0));
            }
        }
        Place::Hometown => {
            line(rc, (3.0, 1.0), (3.0, 13.0));
            line(rc, (3.0, 2.0), (12.0, 4.5));
            line(rc, (12.0, 4.5), (3.0, 7.0));
        }
        Place::Marketplace => {
            rc.stroke_ellipse(x + 1.5 * k, y + 1.5 * k, 11.0 * k, 11.0 * k, c, width);
            line(rc, (7.0, 4.0), (7.0, 10.0));
        }
        Place::PkArena | Place::PkLiteArena => {
            line(rc, (2.0, 2.0), (12.0, 12.0));
            line(rc, (12.0, 2.0), (2.0, 12.0));
            line(rc, (2.0, 9.0), (5.0, 12.0));
            line(rc, (12.0, 9.0), (9.0, 12.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_closed_to_the_player_say_why() {
        let facts = RecallFacts {
            owns_house: Some(false),
            in_allegiance: Some(false),
            ..RecallFacts::default()
        };
        assert_eq!(facts.refusal(Place::Lifestone), None);
        assert_eq!(facts.refusal(Place::House), Some("You have no house"));
        assert_eq!(
            facts.refusal(Place::Hometown),
            Some("You are not in an allegiance")
        );
        assert_eq!(facts.refusal(Place::PkArena), Some("Player killers only"));
    }

    #[test]
    fn what_is_not_known_yet_stays_open() {
        let facts = RecallFacts::default();
        assert_eq!(facts.refusal(Place::House), None);
        assert_eq!(facts.refusal(Place::Mansion), None);
    }

    #[test]
    fn the_usual_recall_spells_come_first() {
        let spells = recall_spells([
            (5, "Aerlinthe Recall".to_owned()),
            (1, "Lifestone Recall".to_owned()),
            (2, "Flame Bolt I".to_owned()),
            (3, "Portal Recall".to_owned()),
        ]);
        let names: Vec<_> = spells.iter().map(|(_, n)| n.as_str()).collect();
        assert_eq!(
            names,
            ["Portal Recall", "Lifestone Recall", "Aerlinthe Recall"]
        );
    }

    #[test]
    fn your_own_follow_the_spells_with_the_server_named() {
        let mut list = RecallList::new(Horizon::current(), RecallFonts::default());
        let facts = RecallFacts {
            server: Some("Dreamweave".to_owned()),
            own: vec![(
                3,
                CustomRecall {
                    name: "Driftnet".to_owned(),
                    command: "/dn".to_owned(),
                    icon: "recall".to_owned(),
                    server: Some("Dreamweave".to_owned()),
                },
            )],
            ..RecallFacts::default()
        };
        list.set_facts(facts, &mut |_| None);
        let tail = &list.rows[list.rows.len() - 3..];
        assert_eq!(
            tail[0],
            Row::Section("YOUR OWN \u{b7} DREAMWEAVE".to_owned())
        );
        assert!(matches!(&tail[1], Row::Own { index: 3, recall } if recall.command == "/dn"));
        assert_eq!(tail[2], Row::AddOwn);
        assert!(list.enabled(&tail[1]));
    }

    #[test]
    fn spells_need_magic_mode() {
        let mut list = RecallList::new(Horizon::current(), RecallFonts::default());
        let spell = Row::Spell {
            id: 2645,
            name: "Portal Recall".to_owned(),
            icon: None,
        };
        assert!(!list.enabled(&spell));
        list.facts.magic = true;
        assert!(list.enabled(&spell));
        assert!(list.enabled(&Row::Place(Place::Lifestone)));
        assert!(!list.enabled(&Row::Section("PLACES".to_owned())));
    }
}
