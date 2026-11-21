//! The menu a speaker's name opens: Tell, Select (greyed when they are not
//! nearby), Invite to fellowship, Add friend, Copy name, Squelch.

use ac_ui::modern::tone;
use std::sync::Arc;

use ac_ui::draw::RenderContext;
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

/// What a row does to the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameAction {
    /// Start a tell to them.
    Tell,
    /// Select them.
    Select,
    /// Invite them to the fellowship.
    Invite,
    /// Add them as a friend.
    AddFriend,
    /// Copy the name.
    Copy,
    /// Squelch (ignore) them.
    Squelch,
}

/// The menu's width, padding, head and rows.
pub const WIDTH: f32 = 240.0;
const PAD: f32 = 6.0;
const HEAD: f32 = 40.0;
const ROW: f32 = 28.0;

/// The fonts the menu uses.
#[derive(Debug, Clone, Default)]
pub struct NameFonts {
    /// The name and the rows.
    pub label: Option<Arc<UiFont>>,
    /// Where the line was and the reasons.
    pub note: Option<Arc<UiFont>>,
}

/// The name menu.
#[derive(Debug)]
pub struct NameMenu {
    theme: Horizon,
    fonts: NameFonts,
    /// The name, its object id and where the line was.
    pub who: (String, u32, String),
    /// Each row: what it does, its words, why it cannot (greyed).
    rows: Vec<(NameAction, String, Option<&'static str>)>,
    hovered: Option<usize>,
    /// The row chosen (the window does it, then clears it).
    pub chosen: Option<NameAction>,
}

impl NameMenu {
    /// The menu in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: NameFonts) -> Self {
        Self {
            theme,
            fonts,
            who: (String::new(), 0, String::new()),
            rows: Vec::new(),
            hovered: None,
            chosen: None,
        }
    }

    /// Fills it for `who` (object `guid`) seen in `place`; `nearby` says
    /// whether they can be selected or invited.
    pub fn fill(&mut self, who: &str, guid: u32, place: &str, nearby: bool) {
        let far = (!nearby).then_some("not nearby");
        self.who = (who.to_owned(), guid, place.to_owned());
        self.rows = vec![
            (NameAction::Tell, format!("Tell {who}"), None),
            (NameAction::Select, "Select".to_owned(), far),
            (NameAction::Invite, "Invite to fellowship".to_owned(), far),
            (NameAction::AddFriend, "Add friend".to_owned(), None),
            (NameAction::Copy, "Copy name".to_owned(), None),
            (NameAction::Squelch, "Squelch".to_owned(), None),
        ];
        self.hovered = None;
    }

    /// Its height.
    #[must_use]
    pub fn height(&self) -> f32 {
        HEAD + self.rows.len() as f32 * ROW + PAD
    }

    fn row_at(&self, y: f32) -> Option<usize> {
        if y < HEAD {
            return None;
        }
        let i = ((y - HEAD) / ROW).floor() as usize;
        self.rows
            .get(i)
            .filter(|(_, _, off)| off.is_none())
            .map(|_| i)
    }

    /// Opens it with its bottom left at (`x`, `y`), kept on the screen.
    pub fn open(ui: &mut UiRoot, me: ElementId, (x, y): (f32, f32)) {
        let height = ui.widget::<Self>(me).map_or(100.0, Self::height);
        let screen = ui.size();
        if let Some(e) = ui.element_mut(me) {
            e.left = x.min(screen.0 - WIDTH).max(0.0).round();
            e.top = (y - height)
                .clamp(0.0, (screen.1 - height).max(0.0))
                .round();
            e.height = height;
        }
        ui.set_visible(me, true);
        ui.bring_to_front(me);
        ui.set_active_popup(me, me);
    }

    fn close(ui: &mut UiRoot, me: ElementId) {
        ui.clear_active_popup(me);
        ui.set_visible(me, false);
    }
}

impl Widget for NameMenu {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn receives_hover_mouse_move(&self) -> bool {
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
            kind::CLICK => {
                if let Some(i) = self.row_at(y) {
                    self.chosen = Some(self.rows[i].0);
                    Self::close(ui, me);
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = Horizon::current().palette;
        let h = ui.element(me).map_or(self.height(), |e| e.height);
        rc.begin_overlay();
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, WIDTH, h, 6.0, 14.0, 34.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, WIDTH, h, round, tone::raised(0.97));
        rc.stroke_rounded_rect(0.0, 0.0, WIDTH, h, round, tone::bronze(0.8), 1.0);
        if let Some(f) = &self.fonts.label {
            text::draw(rc, f, &self.who.0, 12.0, 8.0, TextStyle::plain(p.accent));
        }
        if let Some(f) = &self.fonts.note {
            text::draw(
                rc,
                f,
                &self.who.2,
                12.0,
                24.0,
                TextStyle::plain(tone::muted(1.0)),
            );
        }
        rc.fill_rounded_rect(
            8.0,
            HEAD - 3.0,
            WIDTH - 16.0,
            1.0,
            CornerRadii::all(0.0),
            tone::bronze(0.25),
        );
        for (i, (action, words, off)) in self.rows.iter().enumerate() {
            let top = HEAD + i as f32 * ROW;
            if self.hovered == Some(i) {
                rc.fill_rounded_rect(
                    PAD,
                    top,
                    WIDTH - 2.0 * PAD,
                    ROW,
                    CornerRadii::all(3.0),
                    hex(0xFF_FFFF, 0.06),
                );
            }
            rc.push_alpha(if off.is_some() { 0.45 } else { 1.0 });
            let color = if *action == NameAction::Squelch {
                hex(0xFF_8A8E, 1.0)
            } else {
                p.text
            };
            if let Some(f) = &self.fonts.label {
                let ty = (top + (ROW - f.line_height) * 0.5).round();
                text::draw(rc, f, words, 14.0, ty, TextStyle::plain(color));
            }
            if let (Some(reason), Some(f)) = (off, &self.fonts.note) {
                let w = text::measure(f, reason, 0.0);
                let ty = (top + (ROW - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    reason,
                    WIDTH - 14.0 - w,
                    ty,
                    TextStyle::plain(tone::muted(1.0)),
                );
            }
            rc.pop_alpha();
        }
        rc.end_overlay();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn someone_far_away_cannot_be_selected_or_invited() {
        let mut menu = NameMenu::new(Horizon::current(), NameFonts::default());
        menu.fill("Osk", 7, "General", false);
        let select_y = HEAD + ROW * 1.5;
        let invite_y = HEAD + ROW * 2.5;
        let tell_y = HEAD + ROW * 0.5;
        assert_eq!(menu.row_at(select_y), None);
        assert_eq!(menu.row_at(invite_y), None);
        assert_eq!(menu.row_at(tell_y), Some(0));
        menu.fill("Osk", 7, "General", true);
        assert_eq!(menu.row_at(select_y), Some(1));
    }
}
