//! Where a typed line goes: each talk channel's name, colour and @ command,
//! and the menu the entry's channel button opens. The menu lists every
//! retail talk channel (Say; Fellowship, Allegiance, Monarch, Patron,
//! Vassals; General, Trade, LFG, Roleplay, Society, Olthoi) and recent
//! tells. Channels the player cannot use now are greyed with the reason.

use std::sync::Arc;

use ac_runtime::communication::ChatChannelKind;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::model::LineKind;

/// A channel's name as the button and the menu say it.
#[must_use]
pub fn channel_name(kind: ChatChannelKind) -> &'static str {
    match kind {
        ChatChannelKind::Say => "Say",
        ChatChannelKind::Tell => "Tell",
        ChatChannelKind::Fellowship => "Fellowship",
        ChatChannelKind::Allegiance => "Allegiance",
        ChatChannelKind::Monarch => "Monarch",
        ChatChannelKind::Patron => "Patron",
        ChatChannelKind::Vassals => "Vassals",
        ChatChannelKind::CoVassals => "Co-vassals",
        ChatChannelKind::General => "General",
        ChatChannelKind::Trade => "Trade",
        ChatChannelKind::Lfg => "LFG",
        ChatChannelKind::Roleplay => "Roleplay",
        ChatChannelKind::Society => "Society",
        ChatChannelKind::Olthoi => "Olthoi",
        _ => "Chat",
    }
}

/// The @ command that talks on a channel, as typed.
#[must_use]
pub fn channel_verb(kind: ChatChannelKind) -> &'static str {
    match kind {
        ChatChannelKind::Say => "@s",
        ChatChannelKind::Fellowship => "@f",
        ChatChannelKind::Allegiance => "@a",
        ChatChannelKind::Monarch => "@m",
        ChatChannelKind::Patron => "@p",
        ChatChannelKind::Vassals => "@v",
        ChatChannelKind::General => "@cg",
        ChatChannelKind::Trade => "@ct",
        ChatChannelKind::Lfg => "@lfg",
        ChatChannelKind::Roleplay => "@crp",
        ChatChannelKind::Society => "@soc",
        ChatChannelKind::Olthoi => "@o",
        ChatChannelKind::Tell => "@t",
        _ => "",
    }
}

/// A channel's colour (that of its lines).
#[must_use]
pub fn channel_color(kind: ChatChannelKind) -> Color {
    match kind {
        ChatChannelKind::Say => LineKind::Say.color(),
        ChatChannelKind::Tell => LineKind::Tell.color(),
        ChatChannelKind::Fellowship => LineKind::Fellow.color(),
        ChatChannelKind::Allegiance
        | ChatChannelKind::Monarch
        | ChatChannelKind::Patron
        | ChatChannelKind::Vassals
        | ChatChannelKind::CoVassals => LineKind::Alleg.color(),
        _ => LineKind::General.color(),
    }
}

/// What the empty entry says on a channel.
#[must_use]
pub fn channel_prompt(kind: ChatChannelKind, tell: Option<&str>) -> String {
    match (kind, tell) {
        (ChatChannelKind::Tell, Some(who)) => format!("Tell {who}"),
        (ChatChannelKind::Say, _) => "Say something to people nearby".to_owned(),
        (ChatChannelKind::Fellowship, _) => "Message your fellowship".to_owned(),
        (ChatChannelKind::Allegiance, _) => "Message your allegiance".to_owned(),
        (ChatChannelKind::Monarch, _) => "Message your monarch".to_owned(),
        (ChatChannelKind::Patron, _) => "Message your patron".to_owned(),
        (ChatChannelKind::Vassals, _) => "Message your vassals".to_owned(),
        (ChatChannelKind::General, _) => "Message General chat".to_owned(),
        (ChatChannelKind::Trade, _) => "Message Trade chat".to_owned(),
        (ChatChannelKind::Lfg, _) => "Message Looking for Group".to_owned(),
        (ChatChannelKind::Roleplay, _) => "Message Roleplay chat".to_owned(),
        (kind, _) => format!("Message {}", channel_name(kind)),
    }
}

/// What decides which channels the player can talk on. Unknown facts
/// leave a channel open: the server answers for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TalkFacts {
    /// In a fellowship (none: not told yet).
    pub in_fellowship: Option<bool>,
    /// In an allegiance (none: not told yet).
    pub in_allegiance: Option<bool>,
    /// Has a patron.
    pub has_patron: Option<bool>,
    /// Has vassals.
    pub has_vassals: Option<bool>,
    /// Who last sent the player a tell.
    pub last_teller: Option<String>,
    /// The selected player, if one is selected: name and object id.
    pub selected_player: Option<(String, u32)>,
}

impl TalkFacts {
    /// Why the player cannot talk on `kind`, if they cannot.
    #[must_use]
    pub fn refusal(&self, kind: ChatChannelKind) -> Option<&'static str> {
        match kind {
            ChatChannelKind::Fellowship if self.in_fellowship == Some(false) => {
                Some("not in a fellowship")
            }
            ChatChannelKind::Allegiance | ChatChannelKind::Monarch
                if self.in_allegiance == Some(false) =>
            {
                Some("no allegiance")
            }
            ChatChannelKind::Patron if self.has_patron == Some(false) => Some("no patron"),
            ChatChannelKind::Vassals if self.has_vassals == Some(false) => Some("no vassals"),
            ChatChannelKind::Olthoi => Some("Olthoi only"),
            _ => None,
        }
    }
}

/// What a menu row aims the entry at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalkTo {
    /// A channel.
    Channel(ChatChannelKind),
    /// One player, by name and object id (0: by name).
    Tell(String, u32),
}

/// A row of the menu.
#[derive(Debug, Clone, PartialEq)]
enum Row {
    Heading(&'static str),
    Item {
        to: TalkTo,
        label: String,
        note: String,
        color: Color,
        off: bool,
    },
}

/// The menu's width, its padding and its rows' heights.
pub const WIDTH: f32 = 440.0;
const PAD: f32 = 10.0;
const HEADING: f32 = 22.0;
const ITEM: f32 = 26.0;
const COLUMN_GAP: f32 = 12.0;

/// The fonts the menu uses.
#[derive(Debug, Clone, Default)]
pub struct MenuFonts {
    /// Headings.
    pub heading: Option<Arc<UiFont>>,
    /// Channel names.
    pub label: Option<Arc<UiFont>>,
    /// Commands and reasons.
    pub note: Option<Arc<UiFont>>,
}

/// The channel menu.
#[derive(Debug)]
pub struct ChannelMenu {
    theme: Horizon,
    fonts: MenuFonts,
    columns: [Vec<Row>; 2],
    /// Where the entry talks now.
    current: TalkTo,
    hovered: Option<(usize, usize)>,
    /// The row chosen (the window aims the entry, then clears it).
    pub chosen: Option<TalkTo>,
}

impl ChannelMenu {
    /// The menu in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: MenuFonts) -> Self {
        Self {
            theme,
            fonts,
            columns: [Vec::new(), Vec::new()],
            current: TalkTo::Channel(ChatChannelKind::Say),
            hovered: None,
            chosen: None,
        }
    }

    /// Fills the menu from `facts`, `current` marked.
    pub fn fill(&mut self, facts: &TalkFacts, current: TalkTo) {
        let item = |kind: ChatChannelKind| {
            let refusal = facts.refusal(kind);
            Row::Item {
                to: TalkTo::Channel(kind),
                label: channel_name(kind).to_owned(),
                note: refusal.unwrap_or(channel_verb(kind)).to_owned(),
                color: channel_color(kind),
                off: refusal.is_some(),
            }
        };
        let mut left = vec![Row::Heading("NEARBY"), item(ChatChannelKind::Say)];
        left.push(Row::Heading("FELLOWSHIP AND ALLEGIANCE"));
        for kind in [
            ChatChannelKind::Fellowship,
            ChatChannelKind::Allegiance,
            ChatChannelKind::Monarch,
            ChatChannelKind::Patron,
            ChatChannelKind::Vassals,
        ] {
            left.push(item(kind));
        }
        let mut tells: Vec<Row> = Vec::new();
        let mut named: Vec<String> = Vec::new();
        let mut tell = |name: &str, guid: u32, note: &str| {
            if name.is_empty() || named.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                return;
            }
            named.push(name.to_owned());
            tells.push(Row::Item {
                to: TalkTo::Tell(name.to_owned(), guid),
                label: format!("Tell {name}"),
                note: note.to_owned(),
                color: channel_color(ChatChannelKind::Tell),
                off: false,
            });
        };
        if let Some(who) = &facts.last_teller {
            tell(who, 0, "last tell \u{b7} @r");
        }
        if let TalkTo::Tell(who, guid) = &current {
            tell(who, *guid, "");
        }
        if let Some((who, guid)) = &facts.selected_player {
            tell(who, *guid, "selected");
        }
        if !tells.is_empty() {
            left.push(Row::Heading("TELL"));
            left.extend(tells);
        }
        let mut right = vec![Row::Heading("CHANNELS")];
        for kind in [
            ChatChannelKind::General,
            ChatChannelKind::Trade,
            ChatChannelKind::Lfg,
            ChatChannelKind::Roleplay,
            ChatChannelKind::Society,
            ChatChannelKind::Olthoi,
        ] {
            right.push(item(kind));
        }
        self.columns = [left, right];
        self.current = current;
        self.hovered = None;
    }

    /// Its height.
    #[must_use]
    pub fn height(&self) -> f32 {
        let column = |rows: &[Row]| -> f32 {
            rows.iter()
                .map(|r| match r {
                    Row::Heading(_) => HEADING,
                    Row::Item { .. } => ITEM,
                })
                .sum()
        };
        2.0 * PAD + column(&self.columns[0]).max(column(&self.columns[1]))
    }

    fn column_width() -> f32 {
        (WIDTH - 2.0 * PAD - COLUMN_GAP) * 0.5
    }

    fn row_at(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let w = Self::column_width();
        let column = if (PAD..PAD + w).contains(&x) {
            0
        } else if (PAD + w + COLUMN_GAP..WIDTH - PAD).contains(&x) {
            1
        } else {
            return None;
        };
        let mut top = PAD;
        for (i, row) in self.columns[column].iter().enumerate() {
            let h = match row {
                Row::Heading(_) => HEADING,
                Row::Item { .. } => ITEM,
            };
            if (top..top + h).contains(&y) {
                return matches!(row, Row::Item { off: false, .. }).then_some((column, i));
            }
            top += h;
        }
        None
    }

    /// Opens it at (`x`, `y`) (the live pop-up: a press outside closes it).
    pub fn open(ui: &mut UiRoot, me: ElementId, (x, y): (f32, f32)) {
        let height = ui.widget::<Self>(me).map_or(100.0, Self::height);
        if let Some(e) = ui.element_mut(me) {
            e.left = x.round();
            e.top = (y - height).max(0.0).round();
            e.height = height;
        }
        ui.set_visible(me, true);
        ui.bring_to_front(me);
        ui.set_active_popup(me, me);
    }

    /// Closes it.
    pub fn close(ui: &mut UiRoot, me: ElementId) {
        ui.clear_active_popup(me);
        ui.set_visible(me, false);
    }
}

impl Widget for ChannelMenu {
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
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = self.row_at(x, y);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::CLICK => {
                if let Some((c, i)) = self.row_at(x, y)
                    && let Some(Row::Item { to, .. }) = self.columns[c].get(i)
                {
                    self.chosen = Some(to.clone());
                    Self::close(ui, me);
                }
                true
            }
            _ => true,
        }
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let h = ui.element(me).map_or(self.height(), |e| e.height);
        // Over every window (the chat it opens from among them).
        rc.begin_overlay();
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        rc.soft_shadow(0.0, 0.0, WIDTH, h, 6.0, 14.0, 34.0, p.shadow);
        rc.fill_rounded_rect(0.0, 0.0, WIDTH, h, round, hex(0x18_130E, 0.97));
        rc.stroke_rounded_rect(0.0, 0.0, WIDTH, h, round, hex(0xC9_A062, 0.8), 1.0);
        let w = Self::column_width();
        for (c, rows) in self.columns.iter().enumerate() {
            let x = PAD + c as f32 * (w + COLUMN_GAP);
            let mut top = PAD;
            for (i, row) in rows.iter().enumerate() {
                match row {
                    Row::Heading(words) => {
                        if let Some(f) = &self.fonts.heading {
                            let ty = (top + HEADING - 4.0 - f.line_height).round();
                            text::draw(
                                rc,
                                f,
                                words,
                                x + 6.0,
                                ty,
                                TextStyle::plain(hex(0xC8_B89C, 1.0)).tracked(1.1),
                            );
                        }
                        top += HEADING;
                    }
                    Row::Item {
                        to,
                        label,
                        note,
                        color,
                        off,
                    } => {
                        let on = *to == self.current;
                        if on {
                            rc.fill_rounded_rect(
                                x,
                                top,
                                w,
                                ITEM,
                                CornerRadii::all(3.0),
                                hex(0xE0_B068, 0.16),
                            );
                        } else if self.hovered == Some((c, i)) {
                            rc.fill_rounded_rect(
                                x,
                                top,
                                w,
                                ITEM,
                                CornerRadii::all(3.0),
                                hex(0xFF_FFFF, 0.06),
                            );
                        }
                        rc.push_alpha(if *off { 0.5 } else { 1.0 });
                        let mid = top + ITEM * 0.5;
                        rc.fill_ellipse(x + 6.0, mid - 3.5, 7.0, 7.0, *color);
                        if let Some(f) = &self.fonts.label {
                            let fg = if *off { hex(0xC8_B89C, 1.0) } else { p.text };
                            let ty = (mid - f.line_height * 0.5).round();
                            text::draw(rc, f, label, x + 21.0, ty, TextStyle::plain(fg));
                        }
                        if let Some(f) = &self.fonts.note {
                            let nw = text::measure(f, note, 0.0);
                            let ty = (mid - f.line_height * 0.5).round();
                            text::draw(
                                rc,
                                f,
                                note,
                                x + w - 6.0 - nw,
                                ty,
                                TextStyle::plain(hex(0xA8_9A82, 1.0)),
                            );
                        }
                        rc.pop_alpha();
                        top += ITEM;
                    }
                }
            }
        }
        rc.end_overlay();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_closed_to_the_player_say_why() {
        let facts = TalkFacts {
            in_fellowship: Some(false),
            has_vassals: Some(false),
            ..TalkFacts::default()
        };
        assert_eq!(
            facts.refusal(ChatChannelKind::Fellowship),
            Some("not in a fellowship")
        );
        assert_eq!(facts.refusal(ChatChannelKind::Vassals), Some("no vassals"));
        assert_eq!(facts.refusal(ChatChannelKind::Allegiance), None);
        assert_eq!(facts.refusal(ChatChannelKind::Olthoi), Some("Olthoi only"));
        assert_eq!(facts.refusal(ChatChannelKind::Trade), None);
    }

    #[test]
    fn recent_tells_follow_the_channels_once_each() {
        let mut menu = ChannelMenu::new(Horizon::BRONZE, MenuFonts::default());
        let facts = TalkFacts {
            last_teller: Some("Rook".to_owned()),
            selected_player: Some(("rook".to_owned(), 5)),
            ..TalkFacts::default()
        };
        menu.fill(&facts, TalkTo::Tell("Mira".to_owned(), 0));
        let tells: Vec<&str> = menu.columns[0]
            .iter()
            .filter_map(|r| match r {
                Row::Item {
                    to: TalkTo::Tell(..),
                    label,
                    ..
                } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(tells, vec!["Tell Rook", "Tell Mira"]);
        assert!(menu.height() > 200.0);
        // Olthoi is greyed: it cannot be clicked.
        let olthoi_y = PAD + HEADING + 5.0 * ITEM + ITEM * 0.5;
        assert_eq!(menu.row_at(WIDTH - PAD - 5.0, olthoi_y), None);
        let general_y = PAD + HEADING + ITEM * 0.5;
        assert_eq!(menu.row_at(WIDTH - PAD - 5.0, general_y), Some((1, 1)));
    }
}
