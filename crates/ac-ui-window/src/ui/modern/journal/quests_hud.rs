//! The quests on screen (OPENAC): the quests the player chose to show, in
//! a small panel moved anywhere in Edit layout; a click opens the quest in
//! the journal.

use std::sync::Arc;

use ac_ui::draw::RenderContext;
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::ShownQuest;
use crate::ui::modern::kit::{self, Rect, gold, inside};

/// The piece's width.
pub const WIDTH: f32 = 250.0;
const HEAD: f32 = 22.0;
const ROW: f32 = 40.0;
const GAP: f32 = 6.0;

/// The fonts the piece draws with.
#[derive(Debug, Clone, Default)]
pub struct QuestsHudFonts {
    /// Names.
    pub name: Option<Arc<UiFont>>,
    /// Lines.
    pub small: Option<Arc<UiFont>>,
    /// The heading.
    pub label: Option<Arc<UiFont>>,
}

/// The quests on screen.
#[derive(Debug)]
pub struct QuestsHud {
    theme: Horizon,
    fonts: QuestsHudFonts,
    /// What it shows.
    pub quests: Vec<ShownQuest>,
    hover: Option<(f32, f32)>,
    /// The quest clicked since the window last looked.
    pub picked: Option<u32>,
}

impl QuestsHud {
    /// The piece in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: QuestsHudFonts) -> Self {
        Self {
            theme,
            fonts,
            quests: Vec::new(),
            hover: None,
            picked: None,
        }
    }

    /// How tall the piece is for what it shows.
    #[must_use]
    pub fn height(&self) -> f32 {
        HEAD + self.quests.len() as f32 * (ROW + GAP)
    }

    fn row(i: usize) -> Rect {
        (0.0, HEAD + i as f32 * (ROW + GAP), WIDTH, ROW)
    }
}

impl Widget for QuestsHud {
    fn hit_test(&self, _element: &Element, _x: f32, y: f32) -> bool {
        y >= HEAD
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        Some("Open in the journal".to_owned())
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
                if let Some(i) = (0..self.quests.len()).find(|i| inside(Self::row(*i), x, y)) {
                    self.picked = Some(self.quests[i].id);
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let f = &self.fonts;
        let shadow = hex(0, 0.9);
        if let Some(l) = &f.label {
            text::draw(
                rc,
                l,
                "QUESTS",
                2.0,
                4.0,
                TextStyle::plain(kit::WARM)
                    .tracked(kit::LABEL_TRACKING)
                    .shadowed(shadow),
            );
        }
        for (i, q) in self.quests.iter().enumerate() {
            let r = Self::row(i);
            let lit = kit::hot(self.hover, r);
            let round = CornerRadii::all(4.0);
            rc.fill_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                round,
                hex(0, if lit { 0.55 } else { 0.4 }),
            );
            rc.stroke_rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                round,
                if lit { gold(1.0) } else { kit::bronze(0.35) },
                1.0,
            );
            if let Some(n) = &f.name {
                let name = kit::fit(n, &q.name, r.2 - 16.0);
                text::draw(
                    rc,
                    n,
                    &name,
                    r.0 + 8.0,
                    r.1 + 5.0,
                    TextStyle::plain(p.text).shadowed(shadow),
                );
            }
            if let Some(s) = &f.small {
                let line = kit::fit(s, &q.line, r.2 - 16.0);
                let ink = if q.timed { gold(1.0) } else { kit::WARM };
                text::draw(
                    rc,
                    s,
                    &line,
                    r.0 + 8.0,
                    r.1 + 22.0,
                    TextStyle::plain(ink).shadowed(shadow),
                );
            }
        }
    }
}
