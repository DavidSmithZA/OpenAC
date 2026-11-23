//! The fellows on screen (OPENAC): a small frame for each other fellow
//! under the effects counts, with their name, level and three vitals; a
//! fellow low on health is edged red; a click selects them (for healing).

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_runtime::gameplay::FellowshipView;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::lines::stroke;
use crate::ui::modern::kit::{self, Rect, gold, inside};

/// The piece's width.
pub const WIDTH: f32 = 200.0;
const HEAD: f32 = 22.0;
const ROW: f32 = 44.0;
const GAP: f32 = 4.0;
/// Below this share of health a fellow is edged red.
const LOW: f32 = 0.4;

/// A crown, for the leader.
const CROWN: &[&[(f32, f32)]] = &[&[
    (3.0, 18.0),
    (4.0, 8.0),
    (9.0, 13.0),
    (12.0, 6.0),
    (15.0, 13.0),
    (20.0, 8.0),
    (21.0, 18.0),
    (3.0, 18.0),
]];

/// The fonts the piece draws with.
#[derive(Debug, Clone, Default)]
pub struct FellowsFonts {
    /// Names.
    pub name: Option<Arc<UiFont>>,
    /// Levels.
    pub small: Option<Arc<UiFont>>,
    /// The fellowship's name.
    pub label: Option<Arc<UiFont>>,
}

/// One fellow as the piece shows them.
#[derive(Debug, Clone, PartialEq)]
struct Fellow {
    id: u32,
    name: String,
    level: u32,
    leader: bool,
    shares: [f32; 3],
}

/// The fellows on screen.
#[derive(Debug)]
pub struct FellowsHud {
    fonts: FellowsFonts,
    title: String,
    count: String,
    fellows: Vec<Fellow>,
    selected: Option<u32>,
    hover: Option<(f32, f32)>,
    /// The fellow clicked since the window last looked.
    pub picked: Option<u32>,
    /// The fellowship revision, player and selection last fed from.
    pub fed_from: Option<(u64, u32, Option<u32>)>,
}

fn share((now, most): (u32, u32)) -> f32 {
    if most == 0 {
        0.0
    } else {
        (now as f32 / most as f32).clamp(0.0, 1.0)
    }
}

impl FellowsHud {
    /// The piece in `theme`.
    #[must_use]
    pub fn new(fonts: FellowsFonts) -> Self {
        Self {
            fonts,
            title: String::new(),
            count: String::new(),
            fellows: Vec::new(),
            selected: None,
            hover: None,
            picked: None,
            fed_from: None,
        }
    }

    /// Shows `fellowship`'s fellows but the player; `selected` is edged
    /// gold.
    pub fn feed(&mut self, fellowship: &FellowshipView, player: u32, selected: Option<u32>) {
        self.title = fellowship.name.to_uppercase();
        self.count = format!(
            "{} / {}",
            fellowship.members.len(),
            crate::ui::modern::social::model::MOST_FELLOWS
        );
        self.selected = selected;
        self.fellows = fellowship
            .members
            .iter()
            .filter(|m| m.guid != player)
            .map(|m| Fellow {
                id: m.guid,
                name: m.name.clone(),
                level: m.level,
                leader: m.guid == fellowship.leader_guid,
                shares: [share(m.health), share(m.stamina), share(m.mana)],
            })
            .collect();
    }

    /// How tall the piece is for the fellows it shows.
    #[must_use]
    pub fn height(&self) -> f32 {
        HEAD + self.fellows.len() as f32 * (ROW + GAP)
    }

    fn row(i: usize) -> Rect {
        (0.0, HEAD + i as f32 * (ROW + GAP), WIDTH, ROW)
    }
}

impl Widget for FellowsHud {
    fn hit_test(&self, _element: &Element, _x: f32, y: f32) -> bool {
        y >= HEAD
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
            kind::CLICK => {
                if let Some(i) = (0..self.fellows.len()).find(|i| inside(Self::row(*i), x, y)) {
                    self.picked = Some(self.fellows[i].id);
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let f = &self.fonts;
        let shadow = hex(0, 0.9);
        if let Some(l) = &f.label {
            let style = TextStyle::plain(tone::warm(1.0))
                .tracked(kit::LABEL_TRACKING)
                .shadowed(shadow);
            let title = kit::fit(l, &self.title, WIDTH - 60.0);
            text::draw(rc, l, &title, 2.0, 4.0, style);
            let w = text::measure(l, &self.count, kit::LABEL_TRACKING);
            text::draw(rc, l, &self.count, WIDTH - 2.0 - w, 4.0, style);
        }
        for (i, fellow) in self.fellows.iter().enumerate() {
            let r = Self::row(i);
            let low = fellow.shares[0] < LOW;
            let on = self.selected == Some(fellow.id);
            let lit = kit::hot(self.hover, r);
            let round = CornerRadii::all(4.0);
            rc.fill_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                round,
                hex(0, if lit { 0.6 } else { 0.5 }),
            );
            let edge = if on {
                gold(1.0)
            } else if low {
                hex(0xE5_484D, 0.8)
            } else {
                kit::bronze(0.45)
            };
            rc.stroke_rounded_rect(r.0, r.1, r.2, r.3, round, edge, 1.0);
            let mut x = r.0 + 8.0;
            if fellow.leader {
                stroke(rc, x, r.1 + 5.0, 13.0, CROWN, gold(1.0), 1.4);
                x += 17.0;
            }
            let level = fellow.level.to_string();
            let level_w = f
                .small
                .as_ref()
                .map_or(0.0, |s| text::measure(s, &level, 0.0));
            if let Some(s) = &f.small {
                text::draw(
                    rc,
                    s,
                    &level,
                    r.0 + r.2 - 8.0 - level_w,
                    r.1 + 6.0,
                    TextStyle::plain(tone::soft(1.0)).shadowed(shadow),
                );
            }
            if let Some(n) = &f.name {
                let name = kit::fit(n, &fellow.name, r.0 + r.2 - 16.0 - level_w - x);
                text::draw(
                    rc,
                    n,
                    &name,
                    x,
                    r.1 + 4.0,
                    TextStyle::plain(p.text).shadowed(shadow),
                );
            }
            let hp = if low {
                hex(0xE5_484D, 1.0)
            } else {
                hex(0xB8_323A, 1.0)
            };
            bar(
                rc,
                (r.0 + 8.0, r.1 + 24.0, r.2 - 16.0, 7.0),
                fellow.shares[0],
                hp,
            );
            let half = (r.2 - 16.0 - 3.0) / 2.0;
            bar(
                rc,
                (r.0 + 8.0, r.1 + 35.0, half, 3.0),
                fellow.shares[1],
                hex(0xB8_8A1E, 1.0),
            );
            bar(
                rc,
                (r.0 + 8.0 + half + 3.0, r.1 + 35.0, half, 3.0),
                fellow.shares[2],
                hex(0x2F_64C2, 1.0),
            );
        }
    }
}

fn bar(rc: &mut RenderContext<'_>, r: Rect, share: f32, color: Color) {
    rc.fill_rounded_rect(r.0, r.1, r.2, r.3, CornerRadii::all(2.0), hex(0, 0.6));
    let w = r.2 * share;
    if w > 0.5 {
        rc.fill_rounded_rect(r.0, r.1, w, r.3, CornerRadii::all(2.0), color);
    }
}
