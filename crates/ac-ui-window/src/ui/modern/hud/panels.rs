//! The panel strip, bottom right: a button for each panel (Character,
//! Inventory, Spellbook, Journal, Map, House, Social, Options). A button
//! does what its key does, and its hover names the key. A panel showing
//! lights its button; the House button wears a red dot while maintenance
//! is due. An item dragged onto Inventory lights it and says where it
//! goes ("Put in main pack, 38 of 102 free"); let go, it goes there.
//!
//! Each button is an element of its own (so a drag finds it); the strip
//! draws the band behind them. The buttons only note what was asked; the
//! window carries it out.

use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{ItemDragPayload, UiEvent, kind};
use ac_ui::input::InputAction;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::lines::{arc, circle, stroke};

/// A button's side.
pub const BUTTON: f32 = 36.0;
/// The gap between buttons and the band's inside margin.
const GAP: f32 = 2.0;
const PAD: f32 = 4.0;
/// An icon's side.
const ICON: f32 = 19.0;
/// The band's width and height.
pub const WIDTH: f32 = PAD * 2.0 + BUTTON * Panel::ALL.len() as f32 + GAP * 7.0;
pub const HEIGHT: f32 = PAD * 2.0 + BUTTON;
/// The drop note's height, and its gap above the button.
const NOTE: f32 = 28.0;
const NOTE_GAP: f32 = 8.0;
/// The red dot of maintenance due.
const DOT: Color = hex(0xFF_5A5F, 1.0);
/// A lit Inventory button's icon while an item is over it.
const LIT_ICON: Color = hex(0xFF_E7BF, 1.0);

/// A panel the strip opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// Attributes and skills.
    Character,
    /// The inventory (and the drop that puts an item in the main pack).
    Inventory,
    /// The spellbook.
    Spellbook,
    /// The journal, on its quests.
    Journal,
    /// The map.
    Map,
    /// The house.
    House,
    /// Social.
    Social,
    /// Options.
    Options,
}

impl Panel {
    /// The buttons, left to right.
    pub const ALL: [Self; 8] = [
        Self::Character,
        Self::Inventory,
        Self::Spellbook,
        Self::Journal,
        Self::Map,
        Self::House,
        Self::Social,
        Self::Options,
    ];

    /// Its name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Character => "Character",
            Self::Inventory => "Inventory",
            Self::Spellbook => "Spellbook",
            Self::Journal => "Journal",
            Self::Map => "Map",
            Self::House => "House",
            Self::Social => "Social",
            Self::Options => "Options",
        }
    }

    /// The key map's action the button does (its key does the same).
    #[must_use]
    pub fn action(self) -> InputAction {
        match self {
            Self::Character => InputAction::ToggleAttributesPanel,
            Self::Inventory => InputAction::ToggleInventoryPanel,
            Self::Spellbook => InputAction::ToggleSpellbookPanel,
            Self::Journal => InputAction::ToggleContractsPage,
            Self::Map => InputAction::ToggleMapPage,
            Self::House => InputAction::ToggleHousePage,
            Self::Social => InputAction::ToggleSocialPanel,
            Self::Options => InputAction::ToggleOptionsPanel,
        }
    }

    /// Draws its icon in an `ICON` box at (`x`, `y`).
    fn draw_icon(self, rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
        let s = ICON;
        let w = 1.6;
        match self {
            Self::Character => {
                circle(rc, x, y, s, (12.0, 8.0), 4.0, color, w);
                arc(rc, x, y, s, (12.0, 21.0), 8.0, (PI, PI), color, w);
            }
            Self::Inventory => {
                stroke(
                    rc,
                    x,
                    y,
                    s,
                    &[
                        &[
                            (5.0, 8.0),
                            (19.0, 8.0),
                            (18.0, 20.0),
                            (6.0, 20.0),
                            (5.0, 8.0),
                        ],
                        &[(9.0, 8.0), (9.0, 6.0)],
                        &[(15.0, 6.0), (15.0, 8.0)],
                    ],
                    color,
                    w,
                );
                arc(rc, x, y, s, (12.0, 6.0), 3.0, (PI, PI), color, w);
            }
            Self::Spellbook => stroke(
                rc,
                x,
                y,
                s,
                &[
                    &[
                        (4.0, 21.0),
                        (4.0, 5.0),
                        (4.6, 3.6),
                        (6.0, 3.0),
                        (19.0, 3.0),
                        (19.0, 19.0),
                        (6.0, 19.0),
                        (4.6, 19.6),
                        (4.0, 21.0),
                    ],
                    &[(8.0, 7.5), (15.0, 7.5)],
                ],
                color,
                w,
            ),
            Self::Journal => stroke(
                rc,
                x,
                y,
                s,
                &[
                    &[
                        (6.0, 3.0),
                        (18.0, 3.0),
                        (18.0, 17.0),
                        (17.5, 19.0),
                        (16.2, 20.5),
                        (14.0, 21.0),
                        (6.0, 21.0),
                        (6.0, 3.0),
                    ],
                    &[(6.0, 3.0), (4.2, 3.6), (3.0, 5.5), (3.0, 8.0), (6.0, 8.0)],
                    &[(10.0, 8.0), (15.0, 8.0)],
                    &[(10.0, 12.0), (15.0, 12.0)],
                ],
                color,
                w,
            ),
            Self::Map => stroke(
                rc,
                x,
                y,
                s,
                &[
                    &[
                        (9.0, 4.0),
                        (3.0, 6.0),
                        (3.0, 20.0),
                        (9.0, 18.0),
                        (15.0, 20.0),
                        (21.0, 18.0),
                        (21.0, 4.0),
                        (15.0, 6.0),
                        (9.0, 4.0),
                    ],
                    &[(9.0, 4.0), (9.0, 18.0)],
                    &[(15.0, 6.0), (15.0, 20.0)],
                ],
                color,
                w,
            ),
            Self::House => stroke(
                rc,
                x,
                y,
                s,
                &[
                    &[(3.0, 11.0), (12.0, 4.0), (21.0, 11.0)],
                    &[(5.0, 10.0), (5.0, 20.0), (19.0, 20.0), (19.0, 10.0)],
                    &[(10.0, 20.0), (10.0, 15.0), (14.0, 15.0), (14.0, 20.0)],
                ],
                color,
                w,
            ),
            Self::Social => {
                circle(rc, x, y, s, (9.0, 7.0), 4.0, color, w);
                arc(rc, x, y, s, (9.0, 21.0), 7.0, (PI, PI), color, w);
                arc(rc, x, y, s, (16.0, 7.0), 4.0, (-FRAC_PI_2, PI), color, w);
                stroke(
                    rc,
                    x,
                    y,
                    s,
                    &[&[(22.0, 21.0), (21.3, 18.0), (19.6, 15.8), (17.0, 14.3)]],
                    color,
                    w,
                );
            }
            Self::Options => {
                circle(rc, x, y, s, (12.0, 12.0), 3.0, color, w);
                circle(rc, x, y, s, (12.0, 12.0), 6.8, color, w);
                let teeth: Vec<[(f32, f32); 2]> = (0..8)
                    .map(|i| {
                        let a = i as f32 * PI / 4.0;
                        let (c, sn) = (a.cos(), a.sin());
                        [
                            (12.0 + c * 7.2, 12.0 + sn * 7.2),
                            (12.0 + c * 10.0, 12.0 + sn * 10.0),
                        ]
                    })
                    .collect();
                let runs: Vec<&[(f32, f32)]> = teeth.iter().map(|t| t.as_slice()).collect();
                stroke(rc, x, y, s, &runs, color, 2.6);
            }
        }
    }
}

/// The band behind the buttons (the HUD piece).
#[derive(Debug)]
pub struct PanelStrip {
    theme: Horizon,
}

impl PanelStrip {
    /// The band in `theme`.
    #[must_use]
    pub fn new(theme: Horizon) -> Self {
        Self { theme }
    }

    /// Where button `index` sits in the band.
    #[must_use]
    pub fn button_left(index: usize) -> f32 {
        PAD + index as f32 * (BUTTON + GAP)
    }
}

impl Widget for PanelStrip {
    fn hit_test(&self, element: &Element, x: f32, y: f32) -> bool {
        // It does not clip (its parts reach outside), so the toolkit leaves
        // the bounds to it.
        element.contains_local(x, y)
    }

    fn clips_children(&self) -> bool {
        false
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let round = CornerRadii::all(8.0);
        rc.fill_rounded_rect(0.0, 0.0, WIDTH, HEIGHT, round, p.hud);
        rc.stroke_rounded_rect(0.0, 0.0, WIDTH, HEIGHT, round, p.trim, 1.0);
    }
}

/// Where an item dropped on Inventory goes, as the note says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropNote {
    /// "Put in main pack".
    pub words: String,
    /// "38 of 102 free", when the pack's room is known.
    pub room: Option<String>,
}

/// The words for a main pack of `capacity` holding `held` loose items.
#[must_use]
pub fn drop_note(held: usize, capacity: i32) -> DropNote {
    let room = (capacity > 0).then(|| {
        let free = (capacity as usize).saturating_sub(held);
        format!("{free} of {capacity} free")
    });
    DropNote {
        words: if room.is_some() && held >= capacity as usize {
            "Main pack is full".to_owned()
        } else {
            "Put in main pack".to_owned()
        },
        room,
    }
}

/// One panel button.
#[derive(Debug)]
pub struct PanelButton {
    theme: Horizon,
    /// The note's words.
    font: Option<Arc<UiFont>>,
    /// The panel.
    pub panel: Panel,
    /// Its panel shows.
    pub open: bool,
    /// The red dot (House: maintenance is due).
    pub dot: bool,
    /// The key bound to its action, as the key screen names it.
    pub key: Option<String>,
    /// An item is over it (Inventory): the note to show.
    pub note: Option<DropNote>,
    /// The item being dragged over it.
    pub over: Option<u32>,
    /// It was clicked (the window does it, then clears it).
    pub clicked: bool,
    /// An item was let go on it.
    pub dropped: Option<u32>,
    hovered: bool,
    pressed: bool,
}

impl PanelButton {
    /// The button for `panel`; its note in `font`.
    #[must_use]
    pub fn new(theme: Horizon, font: Option<Arc<UiFont>>, panel: Panel) -> Self {
        Self {
            theme,
            font,
            panel,
            open: false,
            dot: false,
            key: None,
            note: None,
            over: None,
            clicked: false,
            dropped: None,
            hovered: false,
            pressed: false,
        }
    }

    /// The item a drag carries, when it is one that can go in a pack.
    fn item(e: &UiEvent) -> Option<u32> {
        let p = e.payload.as_ref()?.get::<ItemDragPayload>()?;
        (p.obj_id != 0 && !p.source_kind.is_alias()).then_some(p.obj_id)
    }
}

impl Widget for PanelButton {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn expands_clip_for_popup(&self) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
        let takes_drops = self.panel == Panel::Inventory;
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                self.hovered = true;
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = false;
                true
            }
            kind::MOUSE_DOWN => {
                self.pressed = true;
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.pressed = false;
                true
            }
            kind::CLICK => {
                self.clicked = true;
                true
            }
            kind::DRAG_ENTER if takes_drops => {
                self.over = Self::item(e);
                true
            }
            kind::DRAG_OVER if takes_drops => {
                // The drag left.
                self.over = None;
                self.note = None;
                true
            }
            kind::DROP_RELEASED if takes_drops => {
                self.dropped = Self::item(e);
                self.over = None;
                self.note = None;
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, _dt: f64) {
        // A drag that ended anywhere takes the light with it.
        if self.over.is_some() && ui.drag_payload().is_none() {
            self.over = None;
            self.note = None;
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        let name = self.panel.name();
        let mut words = match &self.key {
            Some(key) => format!("{name} ({key})"),
            None => name.to_owned(),
        };
        if self.dot {
            words.push_str(" \u{b7} maintenance is due");
        }
        Some(words)
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let round = CornerRadii::all(6.0);
        let lit = self.over.is_some();
        if lit {
            rc.soft_shadow(
                0.0,
                0.0,
                BUTTON,
                BUTTON,
                6.0,
                0.0,
                14.0,
                hex(0xE0_B068, 0.7),
            );
            rc.fill_rounded_rect(0.0, 0.0, BUTTON, BUTTON, round, hex(0xE0_B068, 0.25));
            rc.stroke_rounded_rect(0.0, 0.0, BUTTON, BUTTON, round, p.accent, 1.0);
        } else if self.pressed && self.hovered {
            rc.fill_rounded_rect(0.0, 0.0, BUTTON, BUTTON, round, hex(0x0A_0806, 0.6));
        } else if self.hovered {
            rc.fill_rounded_rect(0.0, 0.0, BUTTON, BUTTON, round, hex(0x2A_2116, 0.8));
        } else if self.open {
            rc.fill_rounded_rect(0.0, 0.0, BUTTON, BUTTON, round, p.accent_tint);
        }
        let color = if lit {
            LIT_ICON
        } else if self.open {
            p.accent
        } else {
            p.icon
        };
        let at = (BUTTON - ICON) * 0.5;
        self.panel.draw_icon(rc, at, at, color);
        if self.open && !lit {
            // A short line under an open panel's icon.
            rc.fill_rounded_rect(
                BUTTON * 0.5 - 6.0,
                BUTTON - 4.0,
                12.0,
                2.0,
                CornerRadii::all(1.0),
                p.accent,
            );
        }
        if self.dot {
            rc.fill_ellipse(BUTTON - 13.0 - 2.0, 3.0, 12.0, 12.0, hex(0, 0.6));
            rc.fill_ellipse(BUTTON - 13.0, 5.0, 8.0, 8.0, DOT);
        }
    }

    fn draw_overlay(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let (Some(note), Some(f)) = (&self.note, &self.font) else {
            return;
        };
        let p = self.theme.palette;
        let words_w = text::measure(f, &note.words, 0.0);
        let room_w = note
            .room
            .as_ref()
            .map_or(0.0, |r| 8.0 + text::measure(f, r, 0.0));
        let w = 10.0 + words_w + room_w + 10.0;
        // Above the button, kept from running off the screen's right edge
        // (the strip sits at the bottom right).
        let left = (BUTTON - w).min(0.0).round();
        let top = -NOTE - NOTE_GAP;
        let round = CornerRadii::all(self.theme.metrics.slot_radius);
        rc.soft_shadow(left, top, w, NOTE, 4.0, 6.0, 14.0, p.shadow);
        rc.fill_rounded_rect(left, top, w, NOTE, round, p.popup);
        rc.stroke_rounded_rect(left, top, w, NOTE, round, p.accent, 1.0);
        let ty = (top + (NOTE - f.line_height) * 0.5).round();
        text::draw(
            rc,
            f,
            &note.words,
            left + 10.0,
            ty,
            TextStyle::plain(p.text),
        );
        if let Some(room) = &note.room {
            text::draw(
                rc,
                f,
                room,
                left + 10.0 + words_w + 8.0,
                ty,
                TextStyle::plain(p.muted),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buttons_fill_the_band() {
        let last = PanelStrip::button_left(Panel::ALL.len() - 1);
        assert_eq!(last + BUTTON + PAD, WIDTH);
        assert_eq!(PanelStrip::button_left(0), PAD);
    }

    #[test]
    fn each_button_does_what_its_key_does() {
        assert_eq!(Panel::Inventory.action(), InputAction::ToggleInventoryPanel);
        assert_eq!(Panel::Options.action(), InputAction::ToggleOptionsPanel);
        assert_eq!(Panel::House.action(), InputAction::ToggleHousePage);
        let mut actions: Vec<_> = Panel::ALL.iter().map(|p| p.action()).collect();
        actions.dedup();
        assert_eq!(actions.len(), Panel::ALL.len());
    }

    #[test]
    fn the_drop_note_counts_the_room_left() {
        let note = drop_note(64, 102);
        assert_eq!(note.words, "Put in main pack");
        assert_eq!(note.room.as_deref(), Some("38 of 102 free"));
        let full = drop_note(102, 102);
        assert_eq!(full.words, "Main pack is full");
        assert_eq!(full.room.as_deref(), Some("0 of 102 free"));
        assert_eq!(drop_note(3, 0).room, None);
    }

    #[test]
    fn the_hover_names_the_key_and_the_dot() {
        let mut b = PanelButton::new(Horizon::BRONZE, None, Panel::House);
        let e = Element::at(0.0, 0.0, BUTTON, BUTTON);
        assert_eq!(b.tooltip_text(&e).as_deref(), Some("House"));
        b.key = Some("F10".to_owned());
        b.dot = true;
        assert_eq!(
            b.tooltip_text(&e).as_deref(),
            Some("House (F10) \u{b7} maintenance is due")
        );
    }
}
