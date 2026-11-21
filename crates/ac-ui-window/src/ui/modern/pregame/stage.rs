//! Character selection on Modern (the Stage): a slim roster on the left,
//! the picked character standing centre stage in the sky of the place it
//! was last, its name large above it and one gold Enter world button
//! under it; the world, its population and the account across the top;
//! Credits, Exit and the version line in a corner. A character pending
//! deletion is red with its countdown and Restore; Delete asks for the
//! word typed. Before the roster comes the connecting screen: a portal
//! ring that charges in a dark hall (the outer arc the connection, the
//! inner the data check), red with the reason when it fails. An account
//! with no characters gets one "Create your first character" button.
//!
//! The server sends only names; level, heritage, look, last place and
//! when last played are what this computer remembers (OPENAC).

use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::{fill_glass, hex};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::scenery::{self, Sky};
use crate::ui::modern::hud::lines::stroke;
use crate::ui::modern::kit::{
    self, HARM, Hits, MUTED, RED, Rect, SOFT, WARM, bronze, gold, put, put_mid, put_right,
};

/// The fonts the screen draws with.
#[derive(Debug, Clone, Default)]
pub struct StageFonts {
    /// The picked character's name.
    pub hero: Option<Arc<UiFont>>,
    /// The world's name while connecting, an empty account's welcome.
    pub huge: Option<Arc<UiFont>>,
    /// The world's name at the top, the Enter world button.
    pub big: Option<Arc<UiFont>>,
    /// Card titles, row names.
    pub strong: Option<Arc<UiFont>>,
    /// Lines under the name, card words.
    pub body: Option<Arc<UiFont>>,
    /// Second lines, notes.
    pub small: Option<Arc<UiFont>>,
    /// Spaced capitals, the plates' LEVEL, the OPENAC tag.
    pub label: Option<Arc<UiFont>>,
}

/// One character in the roster.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RosterRow {
    /// Its object id.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Played on this computer before.
    pub known: bool,
    /// Its level as remembered ("4,783").
    pub level: String,
    /// The line under the name.
    pub sub: String,
    /// Pending deletion.
    pub pending: bool,
    /// The time left to restore it ("47:12").
    pub left: String,
}

/// The picked character, centre stage.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Picked {
    /// Its object id.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// "Level 4,783 · Female Aluvian".
    pub line1: String,
    /// "Last in Ayan Baqur · yesterday".
    pub line2: String,
    /// Played on this computer before.
    pub known: bool,
    /// Pending deletion.
    pub pending: bool,
    /// The time left to restore it.
    pub left: String,
    /// Its sky.
    pub sky: Option<Sky>,
}

/// A card over the stage.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Card {
    /// None.
    #[default]
    None,
    /// Delete this character: type the word.
    Delete(String),
    /// Leave the game.
    Exit,
    /// What the server refused.
    Error(String),
}

/// What waits for the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Busy {
    /// Nothing.
    #[default]
    None,
    /// Entering the world.
    Entering,
    /// A delete or restore.
    Wait,
}

/// The connecting screen.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Connecting {
    /// The world connected to.
    pub world: String,
    /// The connection, 0 to 1.
    pub connection: f32,
    /// The data check, 0 to 1.
    pub data: f32,
    /// The connection's state in words.
    pub connection_words: String,
    /// The data check's state in words.
    pub data_words: String,
    /// It failed.
    pub failed: bool,
    /// Why.
    pub error: String,
}

/// Which screen shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum StageMode {
    /// Nothing (the world, or another pre-game screen).
    #[default]
    Hidden,
    /// Connecting.
    Connecting(Connecting),
    /// Choosing a character.
    Choose,
    /// An account with no characters.
    Empty,
}

/// What the screen shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StageScreen {
    /// Which screen.
    pub mode: StageMode,
    /// The world's name.
    pub world: String,
    /// Players online, when the server says.
    pub population: Option<u32>,
    /// The account.
    pub account: String,
    /// "5 of 11 slots".
    pub slots: String,
    /// "6 free slots".
    pub free: String,
    /// A character can be created.
    pub can_create: bool,
    /// The roster.
    pub rows: Vec<RosterRow>,
    /// The picked character.
    pub picked: Option<Picked>,
    /// It can enter the world.
    pub can_enter: bool,
    /// It can be deleted.
    pub can_delete: bool,
    /// It can be restored.
    pub can_restore: bool,
    /// What waits.
    pub busy: Busy,
    /// The card over the stage.
    pub card: Card,
    /// The word a delete wants typed.
    pub delete_word: String,
    /// The exit question.
    pub exit_words: String,
    /// The version line.
    pub version: String,
}

/// What the screen asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageAsk {
    /// Pick a character.
    Pick(u32),
    /// Enter the world with a character.
    Enter(u32),
    /// Ask to delete the picked character.
    Delete,
    /// The word was typed: delete.
    ConfirmDelete,
    /// Close the card (a delete, the exit question, an error).
    CloseCard,
    /// Restore the picked character.
    Restore,
    /// Create a character.
    Create,
    /// Show the credits.
    Credits,
    /// Ask to leave.
    Exit,
    /// Leave.
    ConfirmExit,
    /// Stop connecting (leave).
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Hit {
    Ask(StageAsk),
    Row(u32),
    Swallow,
}

/// The roster's place.
const ROSTER: (f32, f32, f32) = (32.0, 84.0, 340.0);
const ROW_H: f32 = 62.0;
const ROW_GAP: f32 = 6.0;
/// Where the stage starts (right of the roster).
const STAGE_X: f32 = 406.0;
/// The delete card's size.
pub const CARD_W: f32 = 440.0;

/// The diamond, the world's mark.
const DIAMOND: &[&[(f32, f32)]] = &[
    &[
        (12.0, 2.0),
        (21.0, 12.0),
        (12.0, 22.0),
        (3.0, 12.0),
        (12.0, 2.0),
    ],
    &[
        (12.0, 7.0),
        (16.0, 12.0),
        (12.0, 17.0),
        (8.0, 12.0),
        (12.0, 7.0),
    ],
];
const BIN: &[&[(f32, f32)]] = &[
    &[(4.0, 7.0), (20.0, 7.0)],
    &[(10.0, 11.0), (10.0, 17.0)],
    &[(14.0, 11.0), (14.0, 17.0)],
    &[(6.0, 7.0), (7.0, 20.0), (17.0, 20.0), (18.0, 7.0)],
    &[(9.0, 7.0), (9.0, 4.0), (15.0, 4.0), (15.0, 7.0)],
];
const PLUS: &[&[(f32, f32)]] = &[&[(12.0, 5.0), (12.0, 19.0)], &[(5.0, 12.0), (19.0, 12.0)]];
const PERSON: &[&[(f32, f32)]] = &[&[
    (4.0, 21.0),
    (5.5, 16.5),
    (9.0, 14.0),
    (15.0, 14.0),
    (18.5, 16.5),
    (20.0, 21.0),
]];
const QUESTION: &[&[(f32, f32)]] = &[
    &[
        (9.0, 9.0),
        (10.0, 6.5),
        (12.5, 6.0),
        (15.0, 7.5),
        (15.0, 10.0),
        (12.0, 12.5),
        (12.0, 15.0),
    ],
    &[(12.0, 18.0), (12.0, 18.4)],
];

/// The full-screen character selection.
#[derive(Debug)]
pub struct StageView {
    theme: Horizon,
    fonts: StageFonts,
    /// What it shows.
    pub screen: StageScreen,
    /// The pictures behind it (none: the drawn sky).
    pub backdrop: super::backdrops::Backdrop,
    /// The screen's size.
    pub size: (f32, f32),
    /// What it asked for since the window last looked.
    pub asks: Vec<StageAsk>,
    /// The roster's scroll.
    pub scroll: kit::Scroll,
    /// What the delete field holds (the window copies it in).
    pub typed: String,
    /// The window's clock, seconds (the spinners).
    pub clock: f64,
    hits: Hits<Hit>,
    /// The last row clicked and when (a second click soon after enters).
    last_click: Option<(u32, f64)>,
}

impl StageView {
    /// The pictures behind the screen, dimmed and shaded so the words read;
    /// the drawn `sky` when there are none.
    fn draw_sky(&self, rc: &mut RenderContext<'_>, w: f32, h: f32, sky: Sky, left: bool) {
        if self.backdrop.draw(rc, w, h) {
            scenery::shade(rc, w, h, left, 0.18);
        } else {
            scenery::draw(rc, w, h, sky, left);
        }
    }

    /// A screen in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: StageFonts) -> Self {
        Self {
            theme,
            fonts,
            screen: StageScreen::default(),
            backdrop: super::backdrops::Backdrop::default(),
            size: (1440.0, 900.0),
            asks: Vec::new(),
            scroll: kit::Scroll::default(),
            typed: String::new(),
            clock: 0.0,
            hits: Hits::default(),
            last_click: None,
        }
    }

    /// The middle of the stage, across.
    #[must_use]
    pub fn stage_centre(&self) -> f32 {
        STAGE_X + (self.size.0 - STAGE_X) * 0.5
    }

    /// Where the figure is drawn.
    #[must_use]
    pub fn figure_rect(&self) -> Rect {
        let top = 200.0;
        let bottom = self.plinth_y() + 10.0;
        let w = 420.0;
        (
            self.stage_centre() - w * 0.5,
            top,
            w,
            (bottom - top).max(60.0),
        )
    }

    fn plinth_y(&self) -> f32 {
        self.size.1 - 200.0
    }

    fn action_y(&self) -> f32 {
        self.size.1 - 158.0
    }

    /// The Enter world button.
    #[must_use]
    pub fn enter_rect(&self) -> Rect {
        let x = self.stage_centre() - (300.0 + 10.0 + 56.0) * 0.5;
        (x, self.action_y(), 300.0, 56.0)
    }

    /// The delete button beside it.
    #[must_use]
    pub fn delete_rect(&self) -> Rect {
        let e = self.enter_rect();
        (e.0 + e.2 + 10.0, e.1, 56.0, 56.0)
    }

    /// The Restore button.
    #[must_use]
    pub fn restore_rect(&self) -> Rect {
        (self.stage_centre() - 150.0, self.action_y(), 300.0, 56.0)
    }

    /// The delete card.
    #[must_use]
    pub fn card_rect(&self, height: f32) -> Rect {
        let (w, h) = self.size;
        (
            (w - CARD_W) * 0.5,
            ((h - height) * 0.5).round(),
            CARD_W,
            height,
        )
    }

    /// The delete card's field.
    #[must_use]
    pub fn delete_field_rect(&self) -> Rect {
        let c = self.card_rect(DELETE_CARD_H);
        (c.0 + 18.0, c.1 + 150.0, c.2 - 36.0, 36.0)
    }

    /// Cancel's and Delete's (or Stay's and Leave's, or OK's) places on a
    /// card `height` high.
    #[must_use]
    pub fn card_buttons(&self, height: f32) -> (Rect, Rect) {
        let c = self.card_rect(height);
        let y = c.1 + c.3 - 18.0 - 34.0;
        let right = (c.0 + c.2 - 18.0 - 100.0, y, 100.0, 34.0);
        ((right.0 - 8.0 - 100.0, y, 100.0, 34.0), right)
    }

    /// The Create button of an empty account.
    #[must_use]
    pub fn first_create_rect(&self) -> Rect {
        let w = 340.0;
        (
            (self.size.0 - w) * 0.5,
            (self.size.1 * 0.28 + 190.0).round(),
            w,
            60.0,
        )
    }

    /// Credits and Exit in the corner.
    #[must_use]
    pub fn corner_buttons(&self) -> (Rect, Rect) {
        let (w, h) = self.size;
        let exit = (w - 32.0 - 64.0, h - 26.0 - 18.0 - 30.0, 64.0, 30.0);
        ((exit.0 - 4.0 - 80.0, exit.1, 80.0, 30.0), exit)
    }

    /// The Cancel button of the connecting screen.
    #[must_use]
    pub fn connecting_cancel_rect(&self) -> Rect {
        let y = self.ring_centre().1 + self.ring_radius() + 150.0;
        ((self.size.0 - 110.0) * 0.5, y, 110.0, 38.0)
    }

    fn ring_centre(&self) -> (f32, f32) {
        (self.size.0 * 0.5, (self.size.1 * 0.41).round())
    }

    fn ring_radius(&self) -> f32 {
        200.0_f32.min(self.size.1 * 0.24)
    }

    /// The roster rows' places, top first (scrolled).
    fn row_rects(&self) -> (Rect, Vec<Rect>, Rect) {
        let (x, y, w) = ROSTER;
        let h = self.size.1 - y - 32.0;
        let inner = (x + 14.0, y + 44.0, w - 28.0, h - 44.0 - 70.0 - 62.0 - 4.0);
        let rows = (0..self.screen.rows.len())
            .map(|i| {
                (
                    inner.0,
                    inner.1 + i as f32 * (ROW_H + ROW_GAP) - self.scroll.at,
                    inner.2,
                    ROW_H,
                )
            })
            .collect();
        (inner, rows, (x, y, w, h))
    }

    fn draw_top(&self, rc: &mut RenderContext<'_>) {
        let f = &self.fonts;
        let p = self.theme.palette;
        let y = 22.0;
        stroke(rc, 32.0, y + 7.0, 22.0, DIAMOND, gold(1.0), 1.8);
        let mut x = 64.0;
        if let Some(font) = &f.big {
            put_mid(rc, Some(font), &self.screen.world, x, y, 36.0, p.text);
            x += text::measure(font, &self.screen.world, 0.0) + 14.0;
        }
        if let Some(n) = self.screen.population {
            rc.fill_ellipse(x, y + 14.5, 7.0, 7.0, kit::GOOD);
            put_mid(
                rc,
                f.small.as_ref(),
                &format!(
                    "{} online",
                    ac_ui::panels::appraisal::group_thousands(i64::from(n))
                ),
                x + 13.0,
                y,
                36.0,
                SOFT,
            );
        }
        if !self.screen.account.is_empty() {
            let right = self.size.0 - 32.0;
            let start = put_right(
                rc,
                f.small.as_ref(),
                &self.screen.account,
                right,
                y,
                36.0,
                SOFT,
            );
            stroke(rc, start - 21.0, y + 11.0, 14.0, PERSON, SOFT, 2.0);
            rc.stroke_ellipse(start - 21.0 + 4.7, y + 11.0 + 2.3, 4.7, 4.7, SOFT, 1.6);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_roster(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = self.fonts.clone();
        let p = self.theme.palette;
        let (inner, rows, outer) = self.row_rects();
        let round = CornerRadii::all(6.0);
        fill_glass(rc, outer, round, hex(0x14_100C, 0.78));
        rc.stroke_rounded_rect(outer.0, outer.1, outer.2, outer.3, round, bronze(0.35), 1.0);
        h.add(outer, Hit::Swallow);
        kit::spaced(
            rc,
            f.label.as_ref(),
            "CHARACTERS",
            outer.0 + 18.0,
            outer.1 + 20.0,
        );
        put_right(
            rc,
            f.small.as_ref(),
            &self.screen.slots,
            outer.0 + outer.2 - 18.0,
            outer.1 + 14.0,
            20.0,
            MUTED,
        );
        let total = self.screen.rows.len() as f32 * (ROW_H + ROW_GAP);
        self.scroll.hold(total, inner.3);
        rc.push_clip(inner.0 - 2.0, inner.1 - 2.0, inner.2 + 4.0, inner.3 + 4.0);
        let picked = self.screen.picked.as_ref().map(|p| p.id);
        for (row, r) in self.screen.rows.iter().zip(rows) {
            if r.1 + r.3 < inner.1 || r.1 > inner.1 + inner.3 {
                continue;
            }
            let on = picked == Some(row.id);
            let lit = h.add(r, Hit::Row(row.id))
                && kit::inside(inner, r.0 + 1.0, h.hover.map_or(-1.0, |p| p.1));
            let radius = CornerRadii::all(5.0);
            if on {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, gold(0.14));
                rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, radius, gold(1.0), 1.0);
            } else if lit {
                rc.fill_rounded_rect(r.0, r.1, r.2, r.3, radius, hex(0xFF_FFFF, 0.04));
            }
            let plate = (r.0 + 10.0, r.1 + 12.0, 48.0, 38.0);
            if row.known {
                let edge = if row.pending {
                    hex(0xFF_6B70, 0.5)
                } else {
                    bronze(0.8)
                };
                rc.fill_vertical_gradient(
                    plate.0,
                    plate.1,
                    plate.2,
                    plate.3,
                    CornerRadii::all(4.0),
                    gold(0.18),
                    hex(0, 0.3),
                );
                rc.stroke_rounded_rect(
                    plate.0,
                    plate.1,
                    plate.2,
                    plate.3,
                    CornerRadii::all(4.0),
                    edge,
                    1.0,
                );
                if let Some(lf) = &f.label {
                    let w = text::measure(lf, "LEVEL", 1.0);
                    text::draw(
                        rc,
                        lf,
                        "LEVEL",
                        (plate.0 + (plate.2 - w) * 0.5).round(),
                        plate.1 + 4.0,
                        TextStyle::plain(SOFT).tracked(1.0),
                    );
                }
                if let Some(sf) = &f.strong {
                    let w = text::measure(sf, &row.level, 0.0);
                    let ink = if row.pending { HARM } else { p.text };
                    text::draw(
                        rc,
                        sf,
                        &row.level,
                        (plate.0 + (plate.2 - w) * 0.5).round(),
                        plate.1 + 17.0,
                        TextStyle::plain(ink),
                    );
                }
            } else {
                dashed_rect(rc, plate, bronze(0.45));
                stroke(
                    rc,
                    plate.0 + 16.0,
                    plate.1 + 11.0,
                    16.0,
                    QUESTION,
                    MUTED,
                    2.0,
                );
            }
            let tx = plate.0 + plate.2 + 12.0;
            let mut tw = r.0 + r.2 - 10.0 - tx;
            if row.pending
                && let Some(sf) = &f.small
            {
                let w = text::measure(sf, &row.left, 0.0) + 30.0;
                let pr = (r.0 + r.2 - 10.0 - w, r.1 + (r.3 - 22.0) * 0.5, w, 22.0);
                rc.stroke_rounded_rect(
                    pr.0,
                    pr.1,
                    pr.2,
                    pr.3,
                    CornerRadii::all(11.0),
                    hex(0xFF_6B70, 0.6),
                    1.0,
                );
                rc.stroke_ellipse(pr.0 + 8.0, pr.1 + 5.5, 11.0, 11.0, HARM, 1.6);
                rc.fill(pr.0 + 13.0, pr.1 + 8.0, 1.4, 4.0, HARM);
                rc.fill(pr.0 + 13.0, pr.1 + 11.0, 3.0, 1.4, HARM);
                put_mid(rc, Some(sf), &row.left, pr.0 + 23.0, pr.1, pr.3, HARM);
                tw -= w + 8.0;
            }
            let name_ink = if row.pending { HARM } else { p.text };
            if let Some(sf) = &f.strong {
                put(
                    rc,
                    Some(sf),
                    &kit::fit(sf, &row.name, tw),
                    tx,
                    r.1 + 13.0,
                    name_ink,
                );
            }
            if let Some(sm) = &f.small {
                let ink = if row.pending {
                    hex(0xFF_8A8E, 0.75)
                } else {
                    MUTED
                };
                put(
                    rc,
                    Some(sm),
                    &kit::fit(sm, &row.sub, tw),
                    tx,
                    r.1 + 34.0,
                    ink,
                );
            }
        }
        rc.pop_clip();
        if total > inner.3 {
            kit::scrollbar(
                rc,
                inner.0 + inner.2 + 6.0,
                inner.1,
                inner.3,
                total,
                self.scroll.at,
            );
        }
        // Create, under the last row (or the list's foot when it is full).
        let below = inner.1 + (total - self.scroll.at).min(inner.3) + 2.0;
        let c = (inner.0, below, inner.2, 56.0);
        if self.screen.can_create {
            let lit = h.add(c, Hit::Ask(StageAsk::Create));
            rc.fill_rounded_rect(
                c.0,
                c.1,
                c.2,
                c.3,
                CornerRadii::all(5.0),
                gold(if lit { 0.1 } else { 0.05 }),
            );
            dashed_rect(rc, c, bronze(0.6));
            let disc = (c.0 + 12.0, c.1 + 11.0, 34.0, 34.0);
            rc.stroke_ellipse(disc.0, disc.1, disc.2, disc.3, bronze(0.7), 1.0);
            stroke(rc, disc.0 + 9.0, disc.1 + 9.0, 16.0, PLUS, gold(1.0), 2.4);
            put(
                rc,
                f.strong.as_ref(),
                "Create character",
                c.0 + 58.0,
                c.1 + 10.0,
                p.text,
            );
            put(
                rc,
                f.small.as_ref(),
                &self.screen.free,
                c.0 + 58.0,
                c.1 + 30.0,
                MUTED,
            );
        } else {
            dashed_rect(rc, c, bronze(0.25));
            put_mid(
                rc,
                f.small.as_ref(),
                "Every slot is in use",
                c.0 + 16.0,
                c.1,
                c.3,
                MUTED,
            );
        }
        // The note.
        let ny = outer.1 + outer.3 - 58.0;
        rc.fill(outer.0 + 18.0, ny, outer.2 - 36.0, 1.0, bronze(0.2));
        let tag = kit::openac_tag(rc, f.label.as_ref(), outer.0 + 18.0, ny + 19.0);
        if let Some(sm) = &f.small {
            let lines = kit::wrap(
                sm,
                "Level, look and place are remembered from your last visit on this computer.",
                outer.2 - 36.0 - tag - 8.0,
            );
            for (i, l) in lines.iter().take(2).enumerate() {
                put(
                    rc,
                    Some(sm),
                    l,
                    outer.0 + 18.0 + tag + 8.0,
                    ny + 11.0 + i as f32 * 16.0,
                    MUTED,
                );
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_stage(&mut self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = self.fonts.clone();
        let p = self.theme.palette;
        let cx = self.stage_centre();
        let Some(picked) = self.screen.picked.clone() else {
            return;
        };
        let name_ink = if picked.pending { HARM } else { p.text };
        let centred = |rc: &mut RenderContext<'_>,
                       font: Option<&Arc<UiFont>>,
                       words: &str,
                       y: f32,
                       ink: Color| {
            if let Some(font) = font {
                let w = text::measure(font, words, 0.0);
                text::draw(
                    rc,
                    font,
                    words,
                    (cx - w * 0.5).round(),
                    y,
                    TextStyle::plain(ink),
                );
            }
        };
        centred(rc, f.hero.as_ref(), &picked.name, 92.0, name_ink);
        centred(rc, f.body.as_ref(), &picked.line1, 150.0, WARM);
        centred(rc, f.small.as_ref(), &picked.line2, 172.0, SOFT);
        // The plinth and, for a character never played here, its outline.
        let py = self.plinth_y();
        scenery::plinth(rc, cx, py, 280.0, 56.0);
        if !picked.known {
            let r = (cx - 62.0, py - 390.0, 124.0, 390.0);
            rc.fill_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                CornerRadii::new(62.0, 62.0, 14.0, 14.0),
                hex(0, 0.45),
            );
            rc.stroke_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                CornerRadii::new(62.0, 62.0, 14.0, 14.0),
                hex(0x00FF_FFFF, 0.35),
                1.5,
            );
            stroke(
                rc,
                cx - 15.0,
                r.1 + 52.0,
                30.0,
                QUESTION,
                hex(0xF4_EBDC, 0.5),
                2.0,
            );
        }
        // The action.
        let ay = self.action_y();
        match self.screen.busy {
            Busy::Entering | Busy::Wait => {
                let words = if self.screen.busy == Busy::Entering {
                    "Entering world…"
                } else {
                    "Please wait…"
                };
                let w = f
                    .big
                    .as_ref()
                    .map_or(200.0, |b| text::measure(b, words, 0.0))
                    + 86.0;
                let r = (cx - w * 0.5, ay, w, 56.0);
                rc.fill_rounded_rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    CornerRadii::all(6.0),
                    hex(0x14_100C, 0.9),
                );
                rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(6.0), bronze(0.6), 1.0);
                spinner(rc, r.0 + 36.0, r.1 + 28.0, 10.0, self.clock);
                put_mid(rc, f.big.as_ref(), words, r.0 + 60.0, r.1, r.3, p.text);
            }
            Busy::None if picked.pending => {
                let r = self.restore_rect();
                let lit = h.add(r, Hit::Ask(StageAsk::Restore));
                big_button(
                    rc,
                    r,
                    &format!("Restore {}", picked.name),
                    f.big.as_ref(),
                    lit,
                    self.screen.can_restore,
                );
                if let Some(sm) = &f.small {
                    let words = format!(
                        "Deleted. You can restore it for {} more. It comes back without its allegiance, and only if no one has taken the name.",
                        picked.left
                    );
                    for (i, l) in kit::wrap(sm, &words, 460.0).iter().enumerate() {
                        let w = text::measure(sm, l, 0.0);
                        put(
                            rc,
                            Some(sm),
                            l,
                            (cx - w * 0.5).round(),
                            ay + 68.0 + i as f32 * 18.0,
                            WARM,
                        );
                    }
                }
            }
            Busy::None => {
                let r = self.enter_rect();
                let lit = h.add(r, Hit::Ask(StageAsk::Enter(picked.id)));
                big_button(
                    rc,
                    r,
                    "Enter world",
                    f.big.as_ref(),
                    lit,
                    self.screen.can_enter,
                );
                if let Some(lf) = &f.label {
                    let kw = text::measure(lf, "Enter", 0.0) + 14.0;
                    let bw = f
                        .big
                        .as_ref()
                        .map_or(100.0, |b| text::measure(b, "Enter world", 0.0));
                    let kx = r.0 + (r.2 + bw) * 0.5 + 12.0 - kw * 0.5;
                    let kr = (kx, r.1 + 18.0, kw, 20.0);
                    let ink = hex(0x1A_140D, if self.screen.can_enter { 0.35 } else { 0.15 });
                    rc.stroke_rounded_rect(kr.0, kr.1, kr.2, kr.3, CornerRadii::all(4.0), ink, 1.0);
                    put_mid(
                        rc,
                        Some(lf),
                        "Enter",
                        kr.0 + 7.0,
                        kr.1,
                        kr.3,
                        hex(0x1A_140D, 0.9),
                    );
                }
                let d = self.delete_rect();
                let lit = h.add(d, Hit::Ask(StageAsk::Delete)) && self.screen.can_delete;
                let round = CornerRadii::all(6.0);
                rc.fill_rounded_rect(
                    d.0,
                    d.1,
                    d.2,
                    d.3,
                    round,
                    hex(0x14_100C, if lit { 0.9 } else { 0.7 }),
                );
                rc.stroke_rounded_rect(
                    d.0,
                    d.1,
                    d.2,
                    d.3,
                    round,
                    bronze(if lit { 0.8 } else { 0.45 }),
                    1.0,
                );
                let ink = if self.screen.can_delete {
                    WARM
                } else {
                    hex(0xE9_D3AE, 0.4)
                };
                stroke(rc, d.0 + 19.0, d.1 + 19.0, 18.0, BIN, ink, 2.0);
                if let Some(sm) = &f.small {
                    let words = "or double-click a name";
                    let w = text::measure(sm, words, 0.0);
                    put(
                        rc,
                        Some(sm),
                        words,
                        (cx - w * 0.5).round(),
                        ay + 68.0,
                        MUTED,
                    );
                }
            }
        }
    }

    fn draw_corner(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = &self.fonts;
        let p = self.theme.palette;
        let (credits, exit) = self.corner_buttons();
        let lit = h.add(credits, Hit::Ask(StageAsk::Credits));
        if lit {
            rc.fill_rounded_rect(
                credits.0,
                credits.1,
                credits.2,
                credits.3,
                CornerRadii::all(4.0),
                hex(0xFF_FFFF, 0.06),
            );
        }
        if let Some(b) = &f.body {
            let w = text::measure(b, "Credits", 0.0);
            put_mid(
                rc,
                Some(b),
                "Credits",
                credits.0 + (credits.2 - w) * 0.5,
                credits.1,
                credits.3,
                if lit { p.text } else { SOFT },
            );
        }
        let lit = h.add(exit, Hit::Ask(StageAsk::Exit));
        kit::button(
            rc,
            &self.theme,
            exit,
            "Exit",
            f.strong.as_ref(),
            kit::ButtonLook::Plain,
            lit,
            true,
        );
        put_right(
            rc,
            f.small.as_ref(),
            &self.screen.version,
            self.size.0 - 32.0,
            exit.1 + exit.3 + 4.0,
            16.0,
            hex(0xC8_B89C, 0.7),
        );
    }

    #[allow(clippy::too_many_lines)]
    fn draw_connecting(&self, rc: &mut RenderContext<'_>, c: &Connecting, h: &mut Hits<Hit>) {
        let f = &self.fonts;
        let p = self.theme.palette;
        let (cx, cy) = self.ring_centre();
        let r = self.ring_radius();
        // The portal: a violet glow and a slowly turning shimmer.
        scenery::glow(rc, cx, cy, r * 0.8, r * 0.8, hex(0xAA_82FF, 0.55));
        let turn = (self.clock * std::f64::consts::TAU / 7.0) as f32;
        for i in 0..3 {
            let a = turn + i as f32 * std::f32::consts::TAU / 3.0;
            rc.stroke_arc(cx, cy, r * 0.62, a, 1.1, hex(0xBE_A0FF, 0.28), r * 0.22);
        }
        let outer = if c.failed { RED } else { gold(1.0) };
        let tau = std::f32::consts::TAU;
        let top = -std::f32::consts::FRAC_PI_2;
        rc.stroke_arc(cx, cy, r, 0.0, tau, gold(0.16), 4.0);
        if c.connection > 0.0 {
            rc.stroke_arc(cx, cy, r, top, tau * c.connection.min(1.0), outer, 4.0);
        }
        let ri = r - 16.0;
        rc.stroke_arc(cx, cy, ri, 0.0, tau, hex(0xE9_D3AE, 0.12), 2.0);
        if c.data > 0.0 {
            rc.stroke_arc(cx, cy, ri, top, tau * c.data.min(1.0), WARM, 2.0);
        }
        let centred = |rc: &mut RenderContext<'_>,
                       font: Option<&Arc<UiFont>>,
                       words: &str,
                       y: f32,
                       ink: Color,
                       track: f32| {
            if let Some(font) = font {
                let w = text::measure(font, words, track);
                text::draw(
                    rc,
                    font,
                    words,
                    (cx - w * 0.5).round(),
                    y,
                    TextStyle::plain(ink).tracked(track),
                );
            }
        };
        centred(rc, f.label.as_ref(), "CONNECTING TO", cy - 34.0, WARM, 2.3);
        centred(rc, f.huge.as_ref(), &c.world, cy - 16.0, p.text, 0.0);
        // The two meters' lines.
        let mut y = cy + r + 40.0;
        for (swatch, height, label, words, ink) in [
            (
                outer,
                4.0,
                "Connection",
                &c.connection_words,
                if c.failed { HARM } else { WARM },
            ),
            (WARM, 2.0, "Data", &c.data_words, MUTED),
        ] {
            let lw = f
                .strong
                .as_ref()
                .map_or(80.0, |s| text::measure(s, label, 0.0));
            let ww = f
                .body
                .as_ref()
                .map_or(120.0, |s| text::measure(s, words, 0.0));
            let total = 18.0 + 10.0 + lw + 10.0 + ww;
            let x = (cx - total * 0.5).round();
            rc.fill_rounded_rect(
                x,
                y + 9.0 - height * 0.5,
                18.0,
                height,
                CornerRadii::all(height * 0.5),
                swatch,
            );
            put(rc, f.strong.as_ref(), label, x + 28.0, y, p.text);
            put(rc, f.body.as_ref(), words, x + 38.0 + lw, y + 1.0, ink);
            y += 28.0;
        }
        if c.failed
            && !c.error.is_empty()
            && let Some(b) = &f.body
        {
            let lines = kit::wrap(b, &c.error, 392.0);
            let bh = lines.len() as f32 * 19.0 + 20.0;
            let br = (cx - 210.0, y + 4.0, 420.0, bh);
            rc.fill_rounded_rect(
                br.0,
                br.1,
                br.2,
                br.3,
                CornerRadii::all(5.0),
                hex(0x50_1414, 0.5),
            );
            rc.stroke_rounded_rect(
                br.0,
                br.1,
                br.2,
                br.3,
                CornerRadii::all(5.0),
                hex(0xFF_6B70, 0.55),
                1.0,
            );
            for (i, l) in lines.iter().enumerate() {
                let w = text::measure(b, l, 0.0);
                put(
                    rc,
                    Some(b),
                    l,
                    (cx - w * 0.5).round(),
                    br.1 + 10.0 + i as f32 * 19.0,
                    hex(0xFF_D0D2, 1.0),
                );
            }
        }
        let cancel = self.connecting_cancel_rect();
        let lit = h.add(cancel, Hit::Ask(StageAsk::Cancel));
        kit::button(
            rc,
            &self.theme,
            cancel,
            "Cancel",
            f.strong.as_ref(),
            kit::ButtonLook::Plain,
            lit,
            true,
        );
    }

    fn draw_empty(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = &self.fonts;
        let p = self.theme.palette;
        let cx = self.size.0 * 0.5;
        let y = (self.size.1 * 0.28).round();
        let centred = |rc: &mut RenderContext<'_>,
                       font: Option<&Arc<UiFont>>,
                       words: &str,
                       y: f32,
                       ink: Color,
                       track: f32| {
            if let Some(font) = font {
                let w = text::measure(font, words, track);
                text::draw(
                    rc,
                    font,
                    words,
                    (cx - w * 0.5).round(),
                    y,
                    TextStyle::plain(ink).tracked(track),
                );
            }
        };
        scenery::plinth(rc, cx, self.size.1 - 265.0, 280.0, 70.0);
        centred(rc, f.label.as_ref(), "WELCOME TO", y, SOFT, 2.3);
        centred(
            rc,
            f.huge.as_ref(),
            &self.screen.world,
            y + 16.0,
            p.text,
            0.0,
        );
        centred(
            rc,
            f.body.as_ref(),
            "You have no characters on this world yet.",
            y + 96.0,
            WARM,
            0.0,
        );
        let b = self.first_create_rect();
        let lit = h.add(b, Hit::Ask(StageAsk::Create));
        big_button(
            rc,
            b,
            "Create your first character",
            f.big.as_ref(),
            lit,
            self.screen.can_create,
        );
        centred(
            rc,
            f.small.as_ref(),
            &self.screen.free,
            b.1 + b.3 + 16.0,
            MUTED,
            0.0,
        );
    }

    fn draw_card(&self, rc: &mut RenderContext<'_>, h: &mut Hits<Hit>) {
        let f = &self.fonts;
        let p = self.theme.palette;
        let (title, words, danger, height) = match &self.screen.card {
            Card::None => return,
            Card::Delete(name) => (
                format!("Delete {name}?"),
                "For about an hour you can still restore it from this screen. A restored character loses its allegiance and any house it owns, and the restore fails if someone takes the name first.".to_owned(),
                true,
                DELETE_CARD_H,
            ),
            Card::Exit => ("Leave OpenAC?".to_owned(), self.screen.exit_words.clone(), false, 150.0),
            Card::Error(text) => ("The server refused".to_owned(), text.clone(), true, 160.0),
        };
        let (w, hh) = self.size;
        rc.fill(0.0, 0.0, w, hh, hex(0, 0.55));
        h.add((0.0, 0.0, w, hh), Hit::Swallow);
        let c = self.card_rect(height);
        let round = CornerRadii::all(6.0);
        rc.soft_shadow(c.0, c.1, c.2, c.3, 6.0, 18.0, 50.0, hex(0, 0.6));
        rc.fill_rounded_rect(c.0, c.1, c.2, c.3, round, hex(0x18_130E, 0.97));
        let edge = if danger {
            hex(0xE5_484D, 0.7)
        } else {
            bronze(0.8)
        };
        rc.stroke_rounded_rect(c.0, c.1, c.2, c.3, round, edge, 1.0);
        let disc = (c.0 + 18.0, c.1 + 16.0, 30.0, 30.0);
        rc.fill_ellipse(
            disc.0,
            disc.1,
            disc.2,
            disc.3,
            if danger {
                hex(0xE5_484D, 0.16)
            } else {
                gold(0.16)
            },
        );
        if matches!(self.screen.card, Card::Delete(_)) {
            stroke(rc, disc.0 + 7.0, disc.1 + 7.0, 16.0, BIN, HARM, 2.0);
        } else {
            stroke(
                rc,
                disc.0 + 7.0,
                disc.1 + 7.0,
                16.0,
                QUESTION,
                if danger { HARM } else { gold(1.0) },
                2.0,
            );
        }
        put_mid(
            rc,
            f.strong.as_ref(),
            &title,
            disc.0 + 40.0,
            disc.1,
            disc.3,
            p.text,
        );
        if let Some(b) = &f.body {
            for (i, l) in kit::wrap(b, &words, c.2 - 36.0).iter().enumerate() {
                put(
                    rc,
                    Some(b),
                    l,
                    c.0 + 18.0,
                    c.1 + 60.0 + i as f32 * 20.0,
                    WARM,
                );
            }
        }
        let (left, right) = self.card_buttons(height);
        match &self.screen.card {
            Card::Delete(_) => {
                put(
                    rc,
                    f.small.as_ref(),
                    &format!("Type {} to confirm", self.screen.delete_word),
                    c.0 + 18.0,
                    c.1 + 130.0,
                    SOFT,
                );
                let fr = self.delete_field_rect();
                kit::field_box(
                    rc,
                    &self.theme,
                    fr,
                    "",
                    self.typed.is_empty(),
                    true,
                    f.body.as_ref(),
                );
                let ok = self
                    .typed
                    .trim()
                    .eq_ignore_ascii_case(&self.screen.delete_word);
                let lit = h.add(left, Hit::Ask(StageAsk::CloseCard));
                kit::button(
                    rc,
                    &self.theme,
                    left,
                    "Cancel",
                    f.strong.as_ref(),
                    kit::ButtonLook::Plain,
                    lit,
                    true,
                );
                let lit = h.add(right, Hit::Ask(StageAsk::ConfirmDelete)) && ok;
                kit::button(
                    rc,
                    &self.theme,
                    right,
                    "Delete",
                    f.strong.as_ref(),
                    kit::ButtonLook::Danger,
                    lit,
                    ok,
                );
            }
            Card::Exit => {
                let lit = h.add(left, Hit::Ask(StageAsk::CloseCard));
                kit::button(
                    rc,
                    &self.theme,
                    left,
                    "Stay",
                    f.strong.as_ref(),
                    kit::ButtonLook::Plain,
                    lit,
                    true,
                );
                let lit = h.add(right, Hit::Ask(StageAsk::ConfirmExit));
                kit::button(
                    rc,
                    &self.theme,
                    right,
                    "Leave",
                    f.strong.as_ref(),
                    kit::ButtonLook::Primary,
                    lit,
                    true,
                );
            }
            Card::Error(_) => {
                let lit = h.add(right, Hit::Ask(StageAsk::CloseCard));
                kit::button(
                    rc,
                    &self.theme,
                    right,
                    "OK",
                    f.strong.as_ref(),
                    kit::ButtonLook::Primary,
                    lit,
                    true,
                );
            }
            Card::None => {}
        }
        h.add(c, Hit::Swallow);
        // The buttons stay on top of the card's own swallow.
        if let Card::Delete(_) = &self.screen.card {
            let ok = self
                .typed
                .trim()
                .eq_ignore_ascii_case(&self.screen.delete_word);
            h.add(left, Hit::Ask(StageAsk::CloseCard));
            if ok {
                h.add(right, Hit::Ask(StageAsk::ConfirmDelete));
            }
        } else if let Card::Exit = &self.screen.card {
            h.add(left, Hit::Ask(StageAsk::CloseCard));
            h.add(right, Hit::Ask(StageAsk::ConfirmExit));
        } else {
            h.add(right, Hit::Ask(StageAsk::CloseCard));
        }
    }
}

/// The delete card's height.
const DELETE_CARD_H: f32 = 252.0;

/// A dashed outline round `r`.
fn dashed_rect(rc: &mut RenderContext<'_>, r: Rect, color: Color) {
    let dash: f32 = 5.0;
    let gap: f32 = 4.0;
    let mut x = r.0;
    while x < r.0 + r.2 {
        let w = dash.min(r.0 + r.2 - x);
        rc.fill(x, r.1, w, 1.0, color);
        rc.fill(x, r.1 + r.3 - 1.0, w, 1.0, color);
        x += dash + gap;
    }
    let mut y = r.1;
    while y < r.1 + r.3 {
        let hh = dash.min(r.1 + r.3 - y);
        rc.fill(r.0, y, 1.0, hh, color);
        rc.fill(r.0 + r.2 - 1.0, y, 1.0, hh, color);
        y += dash + gap;
    }
}

/// A turning spinner of `radius` centred on (`cx`, `cy`).
pub fn spinner(rc: &mut RenderContext<'_>, cx: f32, cy: f32, radius: f32, clock: f64) {
    rc.stroke_arc(cx, cy, radius, 0.0, std::f32::consts::TAU, gold(0.25), 2.5);
    let a = (clock * std::f64::consts::TAU / 0.9) as f32;
    rc.stroke_arc(cx, cy, radius, a, 1.4, gold(1.0), 2.5);
}

/// The big gold button (Enter world, Restore, Create your first
/// character).
pub fn big_button(
    rc: &mut RenderContext<'_>,
    r: Rect,
    words: &str,
    font: Option<&Arc<UiFont>>,
    lit: bool,
    enabled: bool,
) {
    let fade = if enabled { 1.0 } else { 0.45 };
    if enabled {
        rc.soft_shadow(r.0, r.1, r.2, r.3, 6.0, 10.0, 34.0, gold(0.25));
    }
    let round = CornerRadii::all(6.0);
    rc.fill_rounded_rect(
        r.0,
        r.1,
        r.2,
        r.3,
        round,
        gold(if lit && enabled { 1.0 } else { 0.92 } * fade),
    );
    rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, gold(fade), 1.0);
    if let Some(f) = font {
        let w = text::measure(f, words, 0.0);
        let x = if words == "Enter world" {
            r.0 + (r.2 - w) * 0.5 - 18.0
        } else {
            r.0 + (r.2 - w) * 0.5
        };
        put_mid(
            rc,
            Some(f),
            words,
            x.round(),
            r.1,
            r.3,
            hex(0x1A_140D, fade),
        );
    }
}

impl Widget for StageView {
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
            kind::SCROLL => {
                let (inner, _, _) = self.row_rects();
                let total = self.screen.rows.len() as f32 * (ROW_H + ROW_GAP);
                self.scroll.wheel(e.data0, ROW_H + ROW_GAP, total, inner.3);
                true
            }
            kind::CLICK => {
                match self.hits.at(x, y) {
                    Some(Hit::Ask(a)) => self.asks.push(a),
                    Some(Hit::Row(id)) => {
                        let again = self
                            .last_click
                            .is_some_and(|(last, at)| last == id && self.clock - at < 0.45);
                        self.asks.push(StageAsk::Pick(id));
                        if again {
                            self.asks.push(StageAsk::Enter(id));
                            self.last_click = None;
                        } else {
                            self.last_click = Some((id, self.clock));
                        }
                    }
                    _ => {}
                }
                true
            }
            kind::DOUBLE_CLICK => {
                if let Some(Hit::Row(id)) = self.hits.at(x, y) {
                    self.asks.push(StageAsk::Enter(id));
                    self.last_click = None;
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let mut h = Hits::new(self.hits.hover);
        let (w, hh) = self.size;
        h.add((0.0, 0.0, w, hh), Hit::Swallow);
        match self.screen.mode.clone() {
            StageMode::Hidden => {}
            StageMode::Connecting(c) => {
                self.draw_sky(rc, w, hh, Sky::Portal, false);
                self.draw_connecting(rc, &c, &mut h);
                self.draw_card(rc, &mut h);
            }
            StageMode::Empty => {
                self.draw_sky(rc, w, hh, Sky::Dusk, false);
                self.draw_top(rc);
                self.draw_empty(rc, &mut h);
                self.draw_corner(rc, &mut h);
                self.draw_card(rc, &mut h);
            }
            StageMode::Choose => {
                let sky = self
                    .screen
                    .picked
                    .as_ref()
                    .and_then(|p| p.sky)
                    .unwrap_or(Sky::Dusk);
                self.draw_sky(rc, w, hh, sky, true);
                scenery::glow(rc, self.stage_centre(), hh * 0.55, 300.0, 210.0, sky.glow());
                self.draw_top(rc);
                self.draw_roster(rc, &mut h);
                self.draw_stage(rc, &mut h);
                self.draw_corner(rc, &mut h);
                self.draw_card(rc, &mut h);
            }
        }
        self.hits = h;
    }
}

/// A count of seconds as the countdown shows it ("47:12", "1:02:03").
#[must_use]
pub fn countdown(seconds: u64) -> String {
    let (h, m, s) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// How long ago `then` was at `now` (seconds since 1970), in words.
#[must_use]
pub fn ago(then: u64, now: u64) -> String {
    if then == 0 {
        return String::new();
    }
    let days = now.saturating_sub(then) / 86_400;
    match days {
        0 => "today".to_owned(),
        1 => "yesterday".to_owned(),
        2..=13 => format!("{days} days ago"),
        14..=59 => format!("{} weeks ago", days / 7),
        60..=364 => format!("{} months ago", days / 30),
        365..=729 => "a year ago".to_owned(),
        _ => format!("{} years ago", days / 365),
    }
}

#[cfg(test)]
mod tests {
    use super::{ago, countdown};

    #[test]
    fn the_restore_countdown_reads_as_a_clock() {
        assert_eq!(countdown(2832), "47:12");
        assert_eq!(countdown(59), "0:59");
        assert_eq!(countdown(3723), "1:02:03");
    }

    #[test]
    fn when_last_played_reads_in_words() {
        let day = 86_400;
        let now = 1_800_000_000;
        assert_eq!(ago(now - 100, now), "today");
        assert_eq!(ago(now - day - 5, now), "yesterday");
        assert_eq!(ago(now - 3 * day, now), "3 days ago");
        assert_eq!(ago(now - 15 * day, now), "2 weeks ago");
        assert_eq!(ago(now - 150 * day, now), "5 months ago");
        assert_eq!(ago(now - 400 * day, now), "a year ago");
        assert_eq!(ago(0, now), "");
    }
}
