//! The inventory window: a slim panel at the screen's edge. The header
//! holds the Gear button and close; under it the search, the Show, Group
//! and Sort menus, the pack column and the grid; under the grid the
//! chosen item's action row (Use or Equip, Split, Give, Drop, Salvage,
//! More, and Inspect); burden and pyreals in the foot. The equipment (the
//! figure and every slot) folds out to the left from the Gear button. The
//! frame draws the chrome and answers its own buttons; the cells, the
//! grid, the pack buttons, the search field and the figure are elements of
//! their own inside it.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext, TextureRef};
use ac_ui::event::{DragPayload, ItemDragPayload, ItemDragSource, UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::{fill_glass, hex};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::cell::CellLook;
use super::fit;
use super::model::{Burden, GroupBy, Kind, SortBy};
use crate::ui::modern::hud::lines::stroke;

/// The packs' width: the window without the gear.
pub const WIDTH: f32 = 400.0;
/// The gear's width, folded out to the left of the packs.
pub const GEAR_W: f32 = 332.0;
/// The window with the gear out.
pub const WITH_GEAR: f32 = WIDTH + GEAR_W;
pub const HEIGHT: f32 = 600.0;
/// The header's and the foot's heights.
const HEADER: f32 = 48.0;
const FOOT: f32 = 48.0;
/// The equipment, in the gear pane (its left edge at the window's).
pub const EQUIP_X: f32 = 16.0;
const EQUIP_TITLE_Y: f32 = HEADER + 14.0;
pub const SLOT: f32 = 40.0;
pub const SLOT_GAP: f32 = 6.0;
/// The figure between the two columns of slots.
pub const FIGURE_X: f32 = EQUIP_X + SLOT + 8.0;
pub const FIGURE_W: f32 = GEAR_W - 2.0 * (EQUIP_X + SLOT + 8.0);
pub const FIGURE_Y: f32 = EQUIP_TITLE_Y + 24.0;
pub const FIGURE_H: f32 = 270.0;
/// The rows under the figure: their titles' and slots' tops.
pub const JEWELRY_TITLE: f32 = FIGURE_Y + FIGURE_H + 10.0;
pub const JEWELRY_Y: f32 = JEWELRY_TITLE + 16.0;
pub const HELD_TITLE: f32 = JEWELRY_Y + SLOT + 10.0;
pub const HELD_Y: f32 = HELD_TITLE + 16.0;
/// The packs' parts, from the packs' left edge.
const PAD: f32 = 16.0;
const INNER_W: f32 = WIDTH - 2.0 * PAD;
/// The search box.
pub const SEARCH_X: f32 = PAD;
pub const SEARCH_Y: f32 = HEADER + 10.0;
pub const SEARCH_H: f32 = 30.0;
/// The Show, Group and Sort menus' row.
const DROPS_Y: f32 = SEARCH_Y + SEARCH_H + 8.0;
const DROPS_H: f32 = 26.0;
/// The pack column and the grid.
pub const BODY_Y: f32 = DROPS_Y + DROPS_H + 8.0;
pub const RAIL_X: f32 = PAD;
pub const RAIL_W: f32 = 48.0;
pub const GRID_X: f32 = RAIL_X + RAIL_W + 10.0;
pub const GRID_W: f32 = WIDTH - PAD - GRID_X;
/// The chosen item's row, over the foot.
const ACTIONS_H: f32 = 100.0;
pub const ACTIONS_Y: f32 = HEIGHT - FOOT - ACTIONS_H;
pub const GRID_Y: f32 = BODY_Y;
pub const GRID_H: f32 = ACTIONS_Y - 8.0 - BODY_Y;
/// The action buttons.
const ACT_H: f32 = 40.0;
const PRIMARY_W: f32 = 76.0;
const ACT_GAP: f32 = 6.0;

/// The fonts the frame draws with.
#[derive(Debug, Clone, Default)]
pub struct FrameFonts {
    /// The window's title, the chosen item's name.
    pub title: Option<Arc<UiFont>>,
    /// Menus' chosen words, the primary button, the pyreals.
    pub strong: Option<Arc<UiFont>>,
    /// Menus' words, buttons.
    pub body: Option<Arc<UiFont>>,
    /// Small capitals.
    pub label: Option<Arc<UiFont>>,
    /// The burden's and the chosen item's small words.
    pub small: Option<Arc<UiFont>>,
    /// The action buttons' words.
    pub chip: Option<Arc<UiFont>>,
}

/// The chosen item, as its action row shows it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Chosen {
    /// The item.
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name and the line under it.
    pub name: String,
    pub sub: String,
    /// The main button: Use, Equip or Take off.
    pub primary: &'static str,
    /// A stack: it splits.
    pub splits: bool,
    /// It can be salvaged (not worn).
    pub salvages: bool,
}

/// What the frame shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameView {
    /// The gear is folded out.
    pub gear: bool,
    /// It is out only for the drag under way.
    pub gear_for_drag: bool,
    /// Group applies (all packs, or a search).
    pub grouped: bool,
    pub group: GroupBy,
    pub sort: SortBy,
    pub look: CellLook,
    pub kind: Option<Kind>,
    pub burden: Burden,
    pub pyreals: i64,
    /// The chosen item (none: the row asks for one).
    pub chosen: Option<Chosen>,
}

/// An action row button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Use, Equip or Take off.
    Primary,
    Split,
    Give,
    Drop,
    Salvage,
    /// The More menu, at this screen point.
    More,
}

/// What the frame asks of the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameRequest {
    /// Close it.
    Close,
    /// Fold the gear out or away.
    Gear,
    /// The Show menu (kinds and looks), at this screen point.
    ShowMenu((f32, f32)),
    /// The Group menu, at this screen point.
    GroupMenu((f32, f32)),
    /// The Sort and Tidy menu, at this screen point.
    SortMenu((f32, f32)),
    /// An action row button on the chosen item; the screen point under
    /// the button (for More's menu).
    Act(u32, Act, (f32, f32)),
    /// The chosen item's full sheet.
    Inspect(u32),
}

/// The inventory's frame.
#[derive(Debug)]
pub struct InventoryFrame {
    theme: Horizon,
    fonts: FrameFonts,
    /// What it shows.
    pub view: FrameView,
    /// What it asked for since the window last looked.
    pub requests: Vec<FrameRequest>,
    hover: Option<(f32, f32)>,
}

/// A button's rectangle.
type Rect = (f32, f32, f32, f32);

fn inside((x, y, w, h): Rect, px: f32, py: f32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

/// The packs' left edge in the window: right of the gear when it is out.
#[must_use]
pub fn packs_x(gear: bool) -> f32 {
    if gear { GEAR_W } else { 0.0 }
}

/// The action buttons, left to right, with their words and tips.
const ACTS: [(Act, &str, &str); 5] = [
    (Act::Split, "Split", "Split the stack"),
    (Act::Give, "Give", "Give to your target"),
    (Act::Drop, "Drop", "Drop on the ground"),
    (Act::Salvage, "Salvage", "Salvage"),
    (
        Act::More,
        "More",
        "Inscribe, add to a bar, link in chat, plugin entries",
    ),
];

impl InventoryFrame {
    /// A frame in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: FrameFonts) -> Self {
        Self {
            theme,
            fonts,
            view: FrameView::default(),
            requests: Vec::new(),
            hover: None,
        }
    }

    fn width(&self) -> f32 {
        if self.view.gear { WITH_GEAR } else { WIDTH }
    }

    fn px(&self) -> f32 {
        packs_x(self.view.gear)
    }

    fn close_rect(&self) -> Rect {
        (self.width() - 10.0 - 30.0, 9.0, 30.0, 30.0)
    }

    fn gear_words(&self) -> &'static str {
        if self.view.gear && !self.view.gear_for_drag {
            "Hide gear"
        } else {
            "Gear"
        }
    }

    fn gear_rect(&self) -> Rect {
        let words = self
            .fonts
            .body
            .as_ref()
            .map_or(40.0, |f| text::measure(f, self.gear_words(), 0.0));
        let w = 10.0 + 14.0 + 6.0 + words + 10.0;
        let close = self.close_rect();
        (close.0 - 6.0 - w, 9.0, w, 30.0)
    }

    /// The Show, Group and Sort buttons.
    fn drop_rects(&self) -> [Rect; 3] {
        let w = (INNER_W - 2.0 * 6.0) / 3.0;
        let x = self.px() + PAD;
        [0.0, 1.0, 2.0].map(|i| (x + i * (w + 6.0), DROPS_Y, w, DROPS_H))
    }

    /// The chosen item's name row and its Inspect link.
    fn inspect_rect(&self) -> Rect {
        let w = self
            .fonts
            .body
            .as_ref()
            .map_or(50.0, |f| text::measure(f, "Inspect", 0.0))
            + 16.0;
        (self.px() + WIDTH - PAD - w, ACTIONS_Y + 12.0, w, 26.0)
    }

    /// The action buttons: the primary first.
    fn act_rects(&self) -> Vec<(Act, Rect)> {
        let y = ACTIONS_Y + 10.0 + 30.0 + 9.0;
        let x0 = self.px() + PAD;
        let mut out = vec![(Act::Primary, (x0, y, PRIMARY_W, ACT_H))];
        let w = (INNER_W - PRIMARY_W - ACT_GAP * ACTS.len() as f32) / ACTS.len() as f32;
        let mut x = x0 + PRIMARY_W + ACT_GAP;
        for (act, _, _) in ACTS {
            out.push((act, (x, y, w, ACT_H)));
            x += w + ACT_GAP;
        }
        out
    }

    fn act_on(&self, act: Act) -> bool {
        let Some(c) = &self.view.chosen else {
            return false;
        };
        match act {
            Act::Split => c.splits,
            Act::Salvage => c.salvages,
            _ => true,
        }
    }

    fn hot(&self, r: Rect) -> bool {
        self.hover.is_some_and(|(x, y)| inside(r, x, y))
    }

    fn draw_header(&self, rc: &mut RenderContext<'_>, w: f32) {
        let p = Horizon::current().palette;
        rc.fill_vertical_gradient(
            1.0,
            1.0,
            w - 2.0,
            HEADER - 1.0,
            CornerRadii::new(5.0, 5.0, 0.0, 0.0),
            tone::bronze(0.10),
            tone::bronze(0.0),
        );
        rc.fill(0.0, HEADER - 1.0, w, 1.0, tone::bronze(0.35));
        stroke(rc, 16.0, 15.5, 17.0, BAG, p.accent, 1.8);
        if let Some(t) = &self.fonts.title {
            text::draw(
                rc,
                t,
                "Inventory",
                42.0,
                ((HEADER - t.line_height) * 0.5).round(),
                TextStyle::plain(p.text),
            );
        }
        let r = CornerRadii::all(4.0);
        let g = self.gear_rect();
        let on = self.view.gear;
        rc.fill_rounded_rect(
            g.0,
            g.1,
            g.2,
            g.3,
            r,
            if on { tone::gold(0.18) } else { hex(0, 0.35) },
        );
        let edge = if on || self.hot(g) {
            p.accent
        } else {
            tone::bronze(0.45)
        };
        rc.stroke_rounded_rect(g.0, g.1, g.2, g.3, r, edge, 1.0);
        stroke(rc, g.0 + 10.0, g.1 + 8.0, 14.0, SHIRT, p.text_soft, 1.6);
        if let Some(b) = &self.fonts.body {
            text::draw(
                rc,
                b,
                self.gear_words(),
                g.0 + 30.0,
                g.1 + ((g.3 - b.line_height) * 0.5).round(),
                TextStyle::plain(p.text),
            );
        }
        let close = self.close_rect();
        stroke(
            rc,
            close.0 + 7.0,
            close.1 + 7.0,
            16.0,
            &[&[(6.0, 6.0), (18.0, 18.0)], &[(18.0, 6.0), (6.0, 18.0)]],
            if self.hot(close) { p.accent } else { p.icon },
            1.8,
        );
    }

    fn draw_gear_pane(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let fill = if self.view.gear_for_drag {
            tone::gold(0.05)
        } else {
            hex(0, 0.12)
        };
        rc.fill(1.0, HEADER, GEAR_W - 1.0, HEIGHT - HEADER - FOOT, fill);
        rc.fill(
            GEAR_W - 1.0,
            HEADER,
            1.0,
            HEIGHT - HEADER - FOOT,
            tone::bronze(0.35),
        );
        if let Some(l) = &self.fonts.label {
            let tracked = TextStyle::plain(p.text_soft).tracked(1.2);
            text::draw(rc, l, "EQUIPPED", EQUIP_X, EQUIP_TITLE_Y, tracked);
            text::draw(rc, l, "JEWELRY", EQUIP_X, JEWELRY_TITLE, tracked);
            text::draw(rc, l, "HELD \u{b7} AETHERIA", EQUIP_X, HELD_TITLE, tracked);
        }
        if let Some(s) = &self.fonts.small {
            let (note, ink) = if self.view.gear_for_drag {
                ("opened for the drag", p.accent)
            } else {
                ("stays open until you hide it", p.text_soft)
            };
            let nw = text::measure(s, note, 0.0);
            text::draw(
                rc,
                s,
                note,
                GEAR_W - EQUIP_X - nw,
                EQUIP_TITLE_Y,
                TextStyle::plain(ink),
            );
        }
        let r = CornerRadii::all(4.0);
        rc.fill_rounded_rect(FIGURE_X, FIGURE_Y, FIGURE_W, FIGURE_H, r, tone::gold(0.05));
    }

    fn draw_drops(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let (Some(b), Some(s)) = (&self.fonts.body, &self.fonts.strong) else {
            return;
        };
        let show = match (self.view.kind, self.view.look) {
            (Some(k), _) => k.words(),
            (None, CellLook::List) => "All, list",
            (None, _) => "All",
        };
        let r = CornerRadii::all(4.0);
        let rows = [
            ("Show", show, true, self.view.kind.is_some()),
            ("Group", self.view.group.words(), self.view.grouped, false),
            (
                "Sort",
                self.view.sort.words(),
                true,
                self.view.sort != SortBy::Mine,
            ),
        ];
        for ((label, value, on, chosen), rect) in rows.into_iter().zip(self.drop_rects()) {
            let (bx, by, bw, bh) = rect;
            rc.fill_rounded_rect(
                bx,
                by,
                bw,
                bh,
                r,
                if chosen {
                    tone::gold(0.18)
                } else {
                    hex(0, 0.35)
                },
            );
            let edge = if on && (chosen || self.hot(rect)) {
                p.accent
            } else {
                tone::bronze(if on { 0.45 } else { 0.2 })
            };
            rc.stroke_rounded_rect(bx, by, bw, bh, r, edge, 1.0);
            let a = if on { 1.0 } else { 0.4 };
            let ty = by + ((bh - b.line_height) * 0.5).round();
            text::draw(rc, b, label, bx + 8.0, ty, TextStyle::plain(tone::soft(a)));
            let vx = bx + 8.0 + text::measure(b, label, 0.0) + 5.0;
            let room = bx + bw - 18.0 - vx;
            text::draw(
                rc,
                s,
                &fit(s, value, room),
                vx,
                ty,
                TextStyle::plain(tone::words(a)),
            );
            caret(rc, bx + bw - 10.0, by + bh * 0.5, tone::soft(a));
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_actions(&self, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let px = self.px();
        rc.fill(px, ACTIONS_Y, WIDTH, 1.0, tone::bronze(0.35));
        rc.fill(px, ACTIONS_Y + 1.0, WIDTH, ACTIONS_H - 1.0, hex(0, 0.25));
        let r = CornerRadii::all(4.0);
        // The name row.
        let ny = ACTIONS_Y + 10.0;
        let tile = (px + PAD, ny, 30.0, 30.0);
        rc.fill_rounded_rect(tile.0, tile.1, tile.2, tile.3, r, hex(0, 0.35));
        rc.stroke_rounded_rect(tile.0, tile.1, tile.2, tile.3, r, tone::bronze(0.6), 1.0);
        let words_x = tile.0 + tile.2 + 10.0;
        let link = self.inspect_rect();
        let room = link.0 - 6.0 - words_x;
        match &self.view.chosen {
            Some(c) => {
                if let Some(icon) = c.icon {
                    rc.sprite(
                        icon,
                        tile.0 + 1.0,
                        tile.1 + 1.0,
                        28.0,
                        28.0,
                        (0.0, 0.0, 1.0, 1.0),
                        Color::WHITE,
                    );
                }
                if let (Some(t), Some(s)) = (&f.strong, &f.small) {
                    text::draw(
                        rc,
                        t,
                        &fit(t, &c.name, room),
                        words_x,
                        ny,
                        TextStyle::plain(p.text),
                    );
                    text::draw(
                        rc,
                        s,
                        &fit(s, &c.sub, room),
                        words_x,
                        ny + 17.0,
                        TextStyle::plain(p.text_soft),
                    );
                }
                if let Some(b) = &f.body {
                    let ink = if self.hot(link) {
                        p.accent_light
                    } else {
                        p.accent
                    };
                    text::draw(
                        rc,
                        b,
                        "Inspect",
                        link.0 + 8.0,
                        link.1 + ((link.3 - b.line_height) * 0.5).round(),
                        TextStyle::plain(ink),
                    );
                }
            }
            None => {
                if let Some(b) = &f.body {
                    text::draw(
                        rc,
                        b,
                        "Click an item to choose it",
                        words_x,
                        ny + ((30.0 - b.line_height) * 0.5).round(),
                        TextStyle::plain(p.text_soft),
                    );
                }
            }
        }
        // The buttons.
        for (act, rect) in self.act_rects() {
            let (bx, by, bw, bh) = rect;
            let on = self.act_on(act);
            let hot = on && self.hot(rect);
            if act == Act::Primary {
                let words = self.view.chosen.as_ref().map_or("Use", |c| c.primary);
                let a = if on { 1.0 } else { 0.4 };
                let fill = if hot {
                    tone::gold(a)
                } else {
                    tone::gold(0.88 * a)
                };
                rc.fill_rounded_rect(bx, by, bw, bh, r, fill);
                rc.stroke_rounded_rect(bx, by, bw, bh, r, tone::gold(a), 1.0);
                if let Some(s) = &f.strong {
                    text::draw_centred(
                        rc,
                        s,
                        words,
                        bx + bw * 0.5,
                        by + ((bh - s.line_height) * 0.5).round(),
                        TextStyle::plain(tone::ink(a)),
                    );
                }
                continue;
            }
            let a = if on { 1.0 } else { 0.4 };
            rc.fill_rounded_rect(
                bx,
                by,
                bw,
                bh,
                r,
                if hot {
                    tone::gold(0.18)
                } else {
                    hex(0, 0.35 * a)
                },
            );
            let edge = if hot {
                p.accent
            } else {
                tone::bronze(0.45 * a)
            };
            rc.stroke_rounded_rect(bx, by, bw, bh, r, edge, 1.0);
            let ink = tone::warm(a);
            let icon = match act {
                Act::Split => Some(SPLIT),
                Act::Give => Some(GIVE),
                Act::Drop => Some(DROP),
                Act::Salvage => Some(SALVAGE),
                _ => None,
            };
            let iy = by + 6.0;
            match icon {
                Some(lines) => stroke(rc, bx + (bw - 14.0) * 0.5, iy, 14.0, lines, ink, 1.6),
                None => {
                    for i in 0..3 {
                        let dx = bx + bw * 0.5 - 7.0 + i as f32 * 6.0;
                        rc.fill_ellipse(dx, iy + 6.0, 2.5, 2.5, ink);
                    }
                }
            }
            if let Some(c) = &f.chip {
                let words = ACTS
                    .iter()
                    .find(|(a, _, _)| *a == act)
                    .map_or("", |(_, w, _)| *w);
                text::draw_centred(
                    rc,
                    c,
                    words,
                    bx + bw * 0.5,
                    by + bh - 6.0 - c.line_height,
                    TextStyle::plain(ink),
                );
            }
        }
    }

    fn draw_foot(&self, rc: &mut RenderContext<'_>, w: f32) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let fy = HEIGHT - FOOT;
        rc.fill(0.0, fy, w, 1.0, tone::bronze(0.35));
        let (Some(l), Some(s), Some(body)) = (&f.label, &f.small, &f.body) else {
            return;
        };
        let mut x = PAD;
        if self.view.gear {
            let note = if self.view.gear_for_drag {
                "Drop on a lit slot to wear it. Let go anywhere else and the gear folds away again."
            } else {
                "Drag to turn your figure. Gear hides it again."
            };
            let ink = if self.view.gear_for_drag {
                p.accent
            } else {
                p.text_soft
            };
            let lines = ac_ui::modern::kit::wrap(s, note, GEAR_W - 2.0 * PAD);
            let top = fy + (FOOT - s.line_height * lines.len().min(2) as f32) * 0.5;
            for (i, line) in lines.iter().take(2).enumerate() {
                text::draw(
                    rc,
                    s,
                    line,
                    PAD,
                    top + s.line_height * i as f32,
                    TextStyle::plain(ink),
                );
            }
            x = GEAR_W + PAD;
        }
        // Pyreals, right.
        let strong = f.strong.as_ref().unwrap_or(body);
        let coins = ac_ui::panels::appraisal::group_thousands(self.view.pyreals);
        let cw = text::measure(strong, &coins, 0.0);
        let cy = fy + FOOT * 0.5;
        let coins_x = w - PAD - cw;
        text::draw(
            rc,
            strong,
            &coins,
            coins_x,
            (cy - strong.line_height * 0.5).round(),
            TextStyle::plain(p.text),
        );
        rc.fill_ellipse(coins_x - 15.0, cy - 4.5, 9.0, 9.0, p.accent);
        // Burden, between.
        let b = self.view.burden;
        let slowed = b.slowed();
        let color = if slowed {
            hex(0xFF_8A5B, 1.0)
        } else {
            p.accent
        };
        let bar_w = (coins_x - 15.0 - 14.0 - x).max(40.0);
        let ty = fy + 11.0;
        text::draw(
            rc,
            l,
            "BURDEN",
            x,
            ty + 1.0,
            TextStyle::plain(p.text_soft).tracked(1.2),
        );
        let lw = text::measure(l, "BURDEN", 1.2);
        let pct = if slowed {
            format!("{}% \u{b7} slowed", b.percent)
        } else {
            format!("{}%", b.percent)
        };
        text::draw(rc, s, &pct, x + lw + 8.0, ty, TextStyle::plain(color));
        let units = format!(
            "{} of {}",
            ac_ui::panels::appraisal::group_thousands(i64::from(b.carried)),
            ac_ui::panels::appraisal::group_thousands(i64::from(b.capacity))
        );
        let uw = text::measure(s, &units, 0.0);
        text::draw(
            rc,
            s,
            &units,
            x + bar_w - uw,
            ty,
            TextStyle::plain(p.text_soft),
        );
        let by = ty + s.line_height + 4.0;
        let pill = CornerRadii::all(2.5);
        rc.fill_rounded_rect(x, by, bar_w, 5.0, pill, hex(0xFF_FFFF, 0.1));
        let share = (b.percent as f32 / 300.0).clamp(0.0, 1.0);
        rc.fill_rounded_rect(x, by, bar_w * share, 5.0, pill, color);
        // 100% (slowed from here) a third of the way along.
        rc.fill(x + bar_w / 3.0 - 1.0, by - 2.0, 2.0, 9.0, tone::words(0.6));
    }
}

/// A down-pointing caret centred on (`cx`, `cy`).
fn caret(rc: &mut RenderContext<'_>, cx: f32, cy: f32, color: Color) {
    let v = |x: f32, y: f32| ac_ui::draw::ColorVertex { x, y, color };
    let _ = rc.fill_convex_polygon(&[
        v(cx - 3.5, cy - 2.0),
        v(cx + 3.5, cy - 2.0),
        v(cx, cy + 2.5),
    ]);
}

/// The bag drawn in the header and on the main pack's button.
pub const BAG: &[&[(f32, f32)]] = &[
    &[
        (5.0, 8.0),
        (19.0, 8.0),
        (18.0, 20.0),
        (6.0, 20.0),
        (5.0, 8.0),
    ],
    &[
        (9.0, 8.0),
        (9.0, 6.0),
        (10.0, 4.0),
        (12.0, 3.0),
        (14.0, 4.0),
        (15.0, 6.0),
        (15.0, 8.0),
    ],
];

/// The stack of layers on the All packs button.
pub const STACK: &[&[(f32, f32)]] = &[
    &[
        (12.0, 3.0),
        (2.0, 8.0),
        (12.0, 13.0),
        (22.0, 8.0),
        (12.0, 3.0),
    ],
    &[(2.0, 13.0), (12.0, 18.0), (22.0, 13.0)],
    &[(2.0, 17.5), (12.0, 22.5), (22.0, 17.5)],
];

/// The shirt on the Gear button.
const SHIRT: &[&[(f32, f32)]] = &[&[
    (8.0, 3.0),
    (3.0, 6.0),
    (5.0, 11.0),
    (7.0, 10.0),
    (7.0, 21.0),
    (17.0, 21.0),
    (17.0, 10.0),
    (19.0, 11.0),
    (21.0, 6.0),
    (16.0, 3.0),
    (14.0, 5.0),
    (12.0, 5.5),
    (10.0, 5.0),
    (8.0, 3.0),
]];

/// A fork: Split.
const SPLIT: &[&[(f32, f32)]] = &[
    &[
        (6.0, 3.0),
        (6.0, 9.0),
        (7.5, 12.5),
        (12.0, 15.0),
        (16.5, 12.5),
        (18.0, 9.0),
        (18.0, 3.0),
    ],
    &[(12.0, 15.0), (12.0, 21.0)],
];

/// An arrow to a bar: Give.
const GIVE: &[&[(f32, f32)]] = &[
    &[(3.0, 12.0), (16.0, 12.0)],
    &[(12.0, 7.0), (17.0, 12.0), (12.0, 17.0)],
    &[(20.0, 4.0), (20.0, 20.0)],
];

/// An arrow down to the ground: Drop.
const DROP: &[&[(f32, f32)]] = &[
    &[(12.0, 3.0), (12.0, 16.0)],
    &[(7.0, 11.0), (12.0, 16.0), (17.0, 11.0)],
    &[(5.0, 21.0), (19.0, 21.0)],
];

/// A hammer: Salvage.
const SALVAGE: &[&[(f32, f32)]] = &[
    &[(4.0, 20.0), (13.0, 11.0)],
    &[
        (10.0, 8.0),
        (15.0, 3.0),
        (21.0, 9.0),
        (16.0, 14.0),
        (10.0, 8.0),
    ],
];

impl Widget for InventoryFrame {
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
                false
            }
            kind::HOVER_LEAVE => {
                self.hover = None;
                false
            }
            kind::CLICK => {
                let (sx, sy) = ui.screen_position(me);
                let below = |r: Rect| (sx + r.0, sy + r.1 + r.3 + 4.0);
                let drops = self.drop_rects();
                if inside(self.close_rect(), x, y) {
                    self.requests.push(FrameRequest::Close);
                } else if inside(self.gear_rect(), x, y) {
                    self.requests.push(FrameRequest::Gear);
                } else if inside(drops[0], x, y) {
                    self.requests.push(FrameRequest::ShowMenu(below(drops[0])));
                } else if inside(drops[1], x, y) {
                    if self.view.grouped {
                        self.requests.push(FrameRequest::GroupMenu(below(drops[1])));
                    }
                } else if inside(drops[2], x, y) {
                    self.requests.push(FrameRequest::SortMenu(below(drops[2])));
                } else if let Some(c) = &self.view.chosen {
                    let id = c.id;
                    if inside(self.inspect_rect(), x, y) {
                        self.requests.push(FrameRequest::Inspect(id));
                    } else if let Some((act, r)) =
                        self.act_rects().into_iter().find(|(_, r)| inside(*r, x, y))
                        && self.act_on(act)
                    {
                        self.requests.push(FrameRequest::Act(id, act, below(r)));
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        let (x, y) = self.hover?;
        if inside(self.close_rect(), x, y) {
            return Some("Close".to_owned());
        }
        if inside(self.gear_rect(), x, y) {
            return Some("Equipment: fold out your worn and wielded items".to_owned());
        }
        let drops = self.drop_rects();
        if inside(drops[1], x, y) && !self.view.grouped {
            return Some("Grouping applies to All packs and to a search".to_owned());
        }
        if self.view.chosen.is_some() && inside(self.inspect_rect(), x, y) {
            return Some("Inspect: the full sheet (or right-click the item)".to_owned());
        }
        let c = self.view.chosen.as_ref()?;
        let (act, _) = self
            .act_rects()
            .into_iter()
            .find(|(_, r)| inside(*r, x, y))?;
        Some(match act {
            Act::Primary => format!("{} (or double-click the item)", c.primary),
            Act::Split if !c.splits => "Not a stack".to_owned(),
            Act::Salvage if !c.salvages => "Take it off to salvage it".to_owned(),
            other => ACTS
                .iter()
                .find(|(a, _, _)| *a == other)
                .map_or_else(String::new, |(_, _, tip)| (*tip).to_owned()),
        })
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let w = self.width();
        let h = HEIGHT;
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, w, h, 6.0, 18.0, 50.0, hex(0, 0.55));
        fill_glass(rc, (0.0, 0.0, w, h), round, tone::ground(0.97));
        self.draw_header(rc, w);
        if self.view.gear {
            self.draw_gear_pane(rc);
        }
        // The search box (its field is a child).
        let p = Horizon::current().palette;
        let sx = self.px() + SEARCH_X;
        let r = CornerRadii::all(4.0);
        rc.fill_rounded_rect(sx, SEARCH_Y, INNER_W, SEARCH_H, r, hex(0, 0.35));
        rc.stroke_rounded_rect(sx, SEARCH_Y, INNER_W, SEARCH_H, r, tone::bronze(0.45), 1.0);
        stroke(
            rc,
            sx + 9.0,
            SEARCH_Y + 8.0,
            14.0,
            &[
                &[
                    (11.0, 4.0),
                    (15.9, 6.1),
                    (18.0, 11.0),
                    (15.9, 15.9),
                    (11.0, 18.0),
                    (6.1, 15.9),
                    (4.0, 11.0),
                    (6.1, 6.1),
                    (11.0, 4.0),
                ],
                &[(16.0, 16.0), (21.0, 21.0)],
            ],
            p.text_soft,
            1.6,
        );
        self.draw_drops(rc);
        self.draw_actions(rc);
        self.draw_foot(rc, w);
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, tone::bronze(0.8), 1.0);
    }
}

/// What a pack button stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackFace {
    /// Every pack.
    All,
    /// The main pack.
    Main,
    /// A side pack.
    Side,
}

/// What a pack button shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PackShown {
    /// The pack (the player for the main pack; 0 for All).
    pub id: u32,
    /// Its icon (side packs).
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// "22/24" ("All" for All packs).
    pub fill_words: String,
    /// How full, 0..1.
    pub fill: f32,
    /// Chosen.
    pub chosen: bool,
}

/// What a pack button asks of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackRequest {
    /// Show it.
    Choose,
    /// An item was let go on it.
    Dropped(ItemDragPayload),
}

/// A button on the pack rail; it takes item drops.
#[derive(Debug)]
pub struct PackButton {
    font: Option<Arc<UiFont>>,
    /// What it stands for.
    pub face: PackFace,
    /// What it shows.
    pub shown: PackShown,
    /// Whether a drop would be taken (the window answers while it hovers).
    pub takes: Option<bool>,
    /// What it asked for since the window last looked.
    pub requests: Vec<PackRequest>,
    /// A drag hovers it.
    pub over: bool,
    hovered: bool,
}

impl PackButton {
    /// A button for `face`.
    #[must_use]
    pub fn new(font: Option<Arc<UiFont>>, face: PackFace) -> Self {
        Self {
            font,
            face,
            shown: PackShown::default(),
            takes: None,
            requests: Vec::new(),
            over: false,
            hovered: false,
        }
    }

    /// Its height in the pack column, at most.
    pub const HEIGHT: f32 = 32.0;
}

fn item_payload(p: Option<&DragPayload>) -> Option<&ItemDragPayload> {
    p?.get::<ItemDragPayload>()
        .filter(|p| p.obj_id != 0 && !p.source_kind.is_alias())
}

impl Widget for PackButton {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        match e.kind {
            kind::HOVER_ENTER | kind::MOUSE_MOVE => {
                self.hovered = true;
                false
            }
            kind::HOVER_LEAVE => {
                self.hovered = false;
                false
            }
            kind::CLICK => {
                self.requests.push(PackRequest::Choose);
                true
            }
            kind::DRAG_ENTER => {
                self.over =
                    item_payload(e.payload.as_ref()).is_some() && self.face != PackFace::All;
                true
            }
            kind::DRAG_OVER => {
                self.over = false;
                self.takes = None;
                true
            }
            kind::DROP_RELEASED => {
                self.over = false;
                self.takes = None;
                if self.face != PackFace::All
                    && let Some(p) = item_payload(e.payload.as_ref())
                {
                    self.requests.push(PackRequest::Dropped(p.clone()));
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, _dt: f64) {
        if self.over && ui.drag_payload().is_none() {
            self.over = false;
            self.takes = None;
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        Some(match self.face {
            PackFace::All => "All packs".to_owned(),
            _ => format!("{} \u{b7} {}", self.shown.name, self.shown.fill_words),
        })
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        let p = Horizon::current().palette;
        let s = &self.shown;
        let r = CornerRadii::all(4.0);
        let lit = self.over && self.takes != Some(false);
        let fill = if s.chosen || lit {
            tone::gold(0.16)
        } else {
            hex(0, 0.3)
        };
        rc.fill_rounded_rect(0.0, 0.0, w, h, r, fill);
        let edge = if s.chosen || lit || self.hovered {
            p.accent
        } else {
            tone::bronze(0.3)
        };
        rc.stroke_rounded_rect(0.0, 0.0, w, h, r, edge, if lit { 2.0 } else { 1.0 });
        if self.over && self.takes == Some(false) {
            rc.fill_rounded_rect(0.0, 0.0, w, h, r, hex(0, 0.35));
        }
        let ink = if s.chosen { p.accent } else { p.text_soft };
        let icon = 13.0;
        let words_h = self.font.as_ref().map_or(0.0, |f| f.line_height);
        let top = ((h - icon - 2.0 - words_h) * 0.5).round();
        match (self.face, s.icon) {
            (PackFace::Side, Some(i)) => {
                let side = 16.0;
                rc.sprite(
                    i,
                    (w - side) * 0.5,
                    top - 2.0,
                    side,
                    side,
                    (0.0, 0.0, 1.0, 1.0),
                    Color::WHITE,
                );
            }
            (PackFace::All, _) => stroke(rc, (w - icon) * 0.5, top, icon, STACK, ink, 1.6),
            _ => stroke(rc, (w - icon) * 0.5, top, icon, BAG, ink, 1.6),
        }
        if let Some(f) = &self.font {
            let full = self.face != PackFace::All && s.fill >= 1.0;
            text::draw_centred(
                rc,
                f,
                &s.fill_words,
                w * 0.5,
                top + icon + 2.0,
                TextStyle::plain(if full { p.bad_text } else { tone::warm(1.0) }),
            );
        }
    }
}

/// Where a drag from the equipment says it came from.
pub const EQUIPMENT: ItemDragSource = ItemDragSource::Equipment;

/// Over the figure: a drag turns it, an item dropped on it is worn, and a
/// click (a press let go without turning it) is a click on the player.
#[derive(Debug, Default)]
pub struct FigureTurn {
    /// How far it is turned, radians.
    pub turn: f32,
    /// Items dropped on it since the window last looked.
    pub dropped: Vec<u32>,
    /// Clicked since the window last looked.
    pub clicked: bool,
    held: Option<i32>,
    /// How far the held press has moved across, pixels.
    travelled: i32,
    over: bool,
}

/// How far a press may move and still be a click rather than a turn.
const CLICK_SLOP: i32 = 4;

impl Widget for FigureTurn {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        match e.kind {
            kind::MOUSE_DOWN => {
                self.held = Some(e.data1);
                self.travelled = 0;
                ui.set_capture(me);
                true
            }
            kind::MOUSE_MOVE => {
                if let Some(x0) = self.held {
                    self.turn += (e.data1 - x0) as f32 * 0.012;
                    self.travelled += (e.data1 - x0).abs();
                    self.held = Some(e.data1);
                }
                true
            }
            kind::MOUSE_UP => {
                if self.held.take().is_some() && self.travelled <= CLICK_SLOP {
                    self.clicked = true;
                }
                true
            }
            kind::CAPTURE_CHANGED => {
                self.held = None;
                true
            }
            kind::DOUBLE_CLICK => {
                self.turn = 0.0;
                true
            }
            kind::DRAG_ENTER => {
                self.over = item_payload(e.payload.as_ref()).is_some();
                true
            }
            kind::DRAG_OVER => {
                self.over = false;
                true
            }
            kind::DROP_RELEASED => {
                self.over = false;
                if let Some(p) = item_payload(e.payload.as_ref()) {
                    self.dropped.push(p.obj_id);
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, _dt: f64) {
        if self.over && ui.drag_payload().is_none() {
            self.over = false;
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        Some(
            "Drag to turn \u{b7} double-click to face you \u{b7} drop gear here to wear it"
                .to_owned(),
        )
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        if !self.over {
            return;
        }
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        let r = CornerRadii::all(4.0);
        rc.fill_rounded_rect(0.0, 0.0, w, h, r, tone::gold(0.08));
        rc.stroke_rounded_rect(0.0, 0.0, w, h, r, tone::gold(0.8), 1.5);
    }
}

#[cfg(test)]
mod figure_tests {
    use super::FigureTurn;
    use ac_ui::UiRoot;
    use ac_ui::event::{UiEvent, kind};
    use ac_ui::tree::Element;

    fn press(moves: &[i32]) -> (bool, f32) {
        let mut ui = UiRoot::new();
        let root = ui.root();
        let me = ui.add(
            root,
            Element::at(0.0, 0.0, 200.0, 250.0),
            FigureTurn::default(),
        );
        ui.send_event(
            me,
            &UiEvent::new(0, me, kind::MOUSE_DOWN).with_data(0, 100, 100),
        );
        for &x in moves {
            ui.send_event(
                me,
                &UiEvent::new(0, me, kind::MOUSE_MOVE).with_data(0, x, 100),
            );
        }
        let last = moves.last().copied().unwrap_or(100);
        ui.send_event(
            me,
            &UiEvent::new(0, me, kind::MOUSE_UP).with_data(0, last, 100),
        );
        let t = ui.widget::<FigureTurn>(me).expect("it");
        (t.clicked, t.turn)
    }

    #[test]
    fn a_press_let_go_in_place_is_a_click_and_a_drag_turns() {
        assert_eq!(press(&[]), (true, 0.0));
        assert!(press(&[102]).0, "a twitch is still a click");
        let (clicked, turn) = press(&[130, 160]);
        assert!(!clicked);
        assert!(turn > 0.5, "{turn}");
    }
}
