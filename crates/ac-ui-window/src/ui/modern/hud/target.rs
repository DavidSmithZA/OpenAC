//! The target line under the vitals: the selected object's name, its
//! health (or an item of yours, its mana), and two small buttons, Use and
//! Inspect. Use is named for what using the object does: Talk to a person,
//! Open a door or chest, Loot a corpse, otherwise Use (OpenAC's wording;
//! the action is the one Use always was). A selected stack of more than
//! one shows how many a drag of it moves (retail's split slider and box on
//! the toolbar): a track to drag, the amount to type, the wheel for one at
//! a time. Under it, the game's short status messages ("out of range")
//! show one at a time and fade.

use std::sync::Arc;

use ac_plugin_abi::world::ObjectClass;
use ac_runtime::inventory::{ItemCommand, ObjectRef};
use ac_runtime::session::SessionCommand;
use ac_ui::draw::{Color, RenderContext};
use ac_ui::event::{UiEvent, kind};
use ac_ui::modern::Horizon;
use ac_ui::modern::numbers::group_thousands;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::PanelOutput;
use ac_ui::panels::items::emit;
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{Element, ElementId, Widget};
use ac_ui::widgets::Field;
use ac_ui::{Modifiers, UiFont, UiRoot};

/// The line's width, units.
pub const WIDTH: f32 = 420.0;
/// Its height (name row, bar, the mana line), units; the status message
/// hangs below.
pub const HEIGHT: f32 = 48.0;
/// The buttons' height.
const BUTTON: f32 = 24.0;
/// The gap between things on the name row.
const GAP: f32 = 8.0;
/// How long a status message shows, seconds (its last second fades).
pub const STATUS_SECONDS: f32 = 5.0;
/// An item's mana bar.
const MANA_BAR: Color = hex(0x5B_9BFF, 1.0);
/// An item's mana numbers.
const MANA_TEXT: Color = hex(0xBF_D8FF, 1.0);
/// Where the split row starts, and its height.
const SPLIT_TOP: f32 = BUTTON + 4.0;
const SPLIT_ROW: f32 = 20.0;
/// The split track's left end (after the word "Split").
const TRACK_LEFT: f32 = 40.0;
/// The typed amount's box, and the room for "of" and the stack after it.
const FIELD_WIDTH: f32 = 64.0;
const OF_WIDTH: f32 = 84.0;
/// The track's right end.
const TRACK_RIGHT: f32 = WIDTH - OF_WIDTH - FIELD_WIDTH - 2.0 * GAP;
/// The typed amount's box's left edge.
pub const FIELD_LEFT: f32 = TRACK_RIGHT + GAP;

/// What using the selected object does, as its button says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseKind {
    /// Anything else.
    Use,
    /// A person who is not a monster: talk to them.
    Talk,
    /// A door or a chest.
    Open,
    /// A corpse.
    Loot,
}

impl UseKind {
    /// The kind for an object of `class`.
    #[must_use]
    pub fn of(class: ObjectClass) -> Self {
        match class {
            ObjectClass::Npc | ObjectClass::Vendor => Self::Talk,
            ObjectClass::Door | ObjectClass::Container => Self::Open,
            ObjectClass::Corpse => Self::Loot,
            _ => Self::Use,
        }
    }

    /// The button's word.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Use => "Use",
            Self::Talk => "Talk",
            Self::Open => "Open",
            Self::Loot => "Loot",
        }
    }
}

/// An item of the player's with mana.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemMana {
    /// Its share, 0..=1.
    pub share: f32,
    /// Its mana and maximum, when its appraisal told them.
    pub numbers: Option<(i32, i32)>,
}

/// What the line shows of the selection.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetView {
    /// The selected object.
    pub guid: u32,
    /// Its name as it shows selected.
    pub name: String,
    /// One of the player's own things (its name is not highlighted).
    pub own: bool,
    /// What Use does to it.
    pub use_kind: UseKind,
    /// Its health share, once the server sent one.
    pub health: Option<f32>,
    /// An item of the player's: its mana.
    pub mana: Option<ItemMana>,
}

/// How many of the selected stack a drag of it moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitView {
    /// The amount, 1..=stack.
    pub value: u32,
    /// The stack's size (more than one).
    pub stack: u32,
}

impl SplitView {
    /// The amount the track gives at `x`: 1 at its left end, the whole
    /// stack at its right.
    #[must_use]
    pub fn at_track(self, x: f32) -> u32 {
        let share = ((x - TRACK_LEFT) / (TRACK_RIGHT - TRACK_LEFT)).clamp(0.0, 1.0);
        let span = self.stack.saturating_sub(1) as f32;
        (1.0 + (share * span).round()) as u32
    }

    /// Where the knob sits for the amount, 0..=1.
    #[must_use]
    pub fn share(self) -> f32 {
        if self.stack <= 1 {
            1.0
        } else {
            (self.value.saturating_sub(1)) as f32 / (self.stack - 1) as f32
        }
    }

    /// The amount moved by `steps` (the wheel: up is more).
    #[must_use]
    pub fn stepped(self, steps: i64) -> u32 {
        (i64::from(self.value) + steps).clamp(1, i64::from(self.stack.max(1))) as u32
    }
}

/// The two buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Button {
    Use,
    Inspect,
}

/// The target line piece.
#[derive(Debug)]
pub struct TargetLine {
    theme: Horizon,
    /// The name.
    name_font: Option<Arc<UiFont>>,
    /// The share.
    share_font: Option<Arc<UiFont>>,
    /// The buttons, the mana numbers and the status line.
    small_font: Option<Arc<UiFont>>,
    /// The selection, if any.
    pub target: Option<TargetView>,
    /// The status message showing and how long it has shown.
    pub status: Option<(String, f32)>,
    /// Where each button was last drawn (left, width).
    buttons: [(Button, f32, f32); 2],
    hovered: Option<Button>,
    pressed: Option<Button>,
    /// Quick move is held: the whole line answers the pointer, so it can
    /// be carried.
    quick_move: bool,
    /// The selected stack's split amount, when it is a stack.
    pub split: Option<SplitView>,
    /// The box the amount is typed into.
    pub split_field: Option<ElementId>,
    /// An amount the player chose (the window passes it on, then clears
    /// it).
    pub split_chosen: Option<u32>,
    /// The track is held.
    split_drag: bool,
    /// The box had the keyboard last frame.
    split_typing: bool,
}

impl TargetLine {
    /// The line, its name, share and small text in the fonts given.
    #[must_use]
    pub fn new(
        theme: Horizon,
        name_font: Option<Arc<UiFont>>,
        share_font: Option<Arc<UiFont>>,
        small_font: Option<Arc<UiFont>>,
    ) -> Self {
        Self {
            theme,
            name_font,
            share_font,
            small_font,
            target: None,
            status: None,
            buttons: [(Button::Use, 0.0, 0.0), (Button::Inspect, 0.0, 0.0)],
            hovered: None,
            pressed: None,
            quick_move: false,
            split: None,
            split_field: None,
            split_chosen: None,
            split_drag: false,
            split_typing: false,
        }
    }

    /// Adds the box the split amount is typed into, under the line `me`.
    pub fn mount_split_field(ui: &mut UiRoot, me: ElementId, font: Option<Arc<UiFont>>) {
        let mut e = Element::at(FIELD_LEFT, SPLIT_TOP, FIELD_WIDTH, SPLIT_ROW)
            .with_visible(false)
            .with_anchors(ac_ui::geometry::AnchorEdges::NONE);
        Field::configure(&mut e);
        let mut f = Field::default();
        f.font = font;
        f.one_line = true;
        f.right_aligned = true;
        f.selectable = true;
        f.clear_on_submit = false;
        f.record_history = false;
        f.select_all_on_focus = true;
        f.max_characters = 10;
        f.padding = 5.0;
        f.text_color = Horizon::BRONZE.palette.text;
        f.background_color = Color::TRANSPARENT;
        f.selection_color = hex(0xE0_B068, 0.35);
        f.character_filter = Some(Box::new(|c: char| c.is_ascii_digit()));
        f.set_editable_detached(&mut e, true);
        let field = ui.add(me, e, f);
        if let Some(line) = ui.widget_mut::<Self>(me) {
            line.split_field = Some(field);
        }
    }

    /// The split amount shown: `split` (none: not a stack). The box follows
    /// unless the player is typing in it.
    pub fn show_split(&mut self, ui: &mut UiRoot, split: Option<SplitView>) {
        self.split = split;
        let Some(field) = self.split_field else {
            return;
        };
        if ui.element(field).is_some_and(|e| e.visible) != split.is_some() {
            ui.set_visible(field, split.is_some());
        }
        let typing = ui.keyboard_focus() == Some(field);
        if let Some(view) = split
            && !typing
            && let Some(f) = ui.widget_mut::<Field>(field)
        {
            let words = view.value.to_string();
            if f.text() != words {
                f.set_text(&words);
            }
        }
        if split.is_none() && typing {
            ui.set_keyboard_focus(None);
        }
    }

    /// The box takes the keyboard, its amount selected (the split key);
    /// false when nothing selected is a stack.
    pub fn type_split(&self, ui: &mut UiRoot) -> bool {
        let (Some(field), Some(_)) = (self.split_field, self.split) else {
            return false;
        };
        ui.set_keyboard_focus(Some(field));
        if let Some(f) = ui.widget_mut::<Field>(field) {
            f.select_all_text();
        }
        true
    }

    /// What was typed becomes the amount (kept within the stack).
    fn commit_typed(&mut self, ui: &mut UiRoot) {
        let (Some(field), Some(view)) = (self.split_field, self.split) else {
            return;
        };
        let typed = ui
            .widget::<Field>(field)
            .map(Field::text)
            .and_then(|t| t.trim().parse::<u64>().ok());
        let value = typed.map_or(view.value, |n| n.clamp(1, u64::from(view.stack)) as u32);
        self.split_chosen = Some(value);
        if let Some(f) = ui.widget_mut::<Field>(field) {
            f.set_text(&value.to_string());
        }
    }

    /// The pointer is over the split row's track (or the word before it).
    fn on_split_track(&self, x: f32, y: f32) -> bool {
        self.split.is_some()
            && (SPLIT_TOP..SPLIT_TOP + SPLIT_ROW).contains(&y)
            && (0.0..TRACK_RIGHT + 4.0).contains(&x)
    }

    fn drag_split(&mut self, x: f32) {
        if let Some(view) = self.split {
            let value = view.at_track(x);
            if value != view.value {
                self.split = Some(SplitView { value, ..view });
                self.split_chosen = Some(value);
            }
        }
    }

    /// Shows `message` under the line (it replaces the one showing).
    pub fn show_status(&mut self, message: String) {
        if !message.trim().is_empty() {
            self.status = Some((message, 0.0));
        }
    }

    fn button_at(&self, x: f32, y: f32) -> Option<Button> {
        if self.target.is_none() || !(0.0..BUTTON).contains(&y) {
            return None;
        }
        self.buttons
            .iter()
            .find(|(_, left, width)| *width > 0.0 && (*left..left + width).contains(&x))
            .map(|(b, _, _)| *b)
    }

    fn press(&self, ui: &mut UiRoot, button: Button) {
        let Some(target) = &self.target else {
            return;
        };
        let object = ObjectRef::Guid(target.guid);
        let command = match button {
            Button::Use => ItemCommand::Use(object),
            Button::Inspect => ItemCommand::Appraise(object),
        };
        emit(ui, PanelOutput::Session(SessionCommand::Item(command)));
    }

    fn draw_button(
        &mut self,
        rc: &mut RenderContext<'_>,
        index: usize,
        right: f32,
        label: Option<&str>,
        icon: fn(&mut RenderContext<'_>, f32, f32, Color),
    ) -> f32 {
        let p = &self.theme.palette;
        let font = self.small_font.clone();
        let label_width = match (label, &font) {
            (Some(l), Some(f)) => text::measure(f, l, 0.0),
            _ => 0.0,
        };
        let width = if label.is_some() {
            7.0 + 14.0 + 5.0 + label_width + 9.0
        } else {
            BUTTON
        };
        let left = right - width;
        let which = self.buttons[index].0;
        self.buttons[index] = (which, left, width);
        let fill = if self.pressed == Some(which) && self.hovered == Some(which) {
            hex(0x0A_0806, 0.85)
        } else if self.hovered == Some(which) {
            hex(0x2A_2116, 0.85)
        } else {
            hex(0x14_100C, 0.7)
        };
        let radius = self.theme.metrics.slot_radius;
        rc.fill_rounded_rect(left, 0.0, width, BUTTON, CornerRadii::all(radius), fill);
        let edge = if self.hovered == Some(which) {
            p.accent
        } else {
            hex(0xC9_A062, 0.7)
        };
        rc.stroke_rounded_rect(
            left,
            0.0,
            width,
            BUTTON,
            CornerRadii::all(radius),
            edge,
            1.0,
        );
        match (label, &font) {
            (Some(l), Some(f)) => {
                icon(rc, left + 7.0, (BUTTON - 14.0) * 0.5, p.icon);
                let ty = ((BUTTON - f.line_height) * 0.5).round();
                text::draw(
                    rc,
                    f,
                    l,
                    left + 7.0 + 14.0 + 5.0,
                    ty,
                    TextStyle::plain(p.text),
                );
            }
            _ => icon(
                rc,
                left + (width - 14.0) * 0.5,
                (BUTTON - 14.0) * 0.5,
                p.icon,
            ),
        }
        left
    }
}

impl TargetLine {
    /// The split row: "Split", the track with its knob, the box (a child
    /// element draws the typed amount over the frame drawn here) and "of"
    /// the stack. Returns the row's bottom.
    fn draw_split(&self, rc: &mut RenderContext<'_>, view: SplitView) -> f32 {
        let p = self.theme.palette;
        let shadow = p.text_shadow;
        let mid = SPLIT_TOP + SPLIT_ROW * 0.5;
        if let Some(font) = &self.small_font {
            let ty = (mid - font.line_height * 0.5).round();
            text::draw(
                rc,
                font,
                "Split",
                0.0,
                ty,
                TextStyle::plain(p.muted).shadowed(shadow),
            );
            let of = format!("of {}", group_thousands(i64::from(view.stack)));
            let of = fit(font, &of, OF_WIDTH - GAP);
            text::draw(
                rc,
                font,
                &of,
                FIELD_LEFT + FIELD_WIDTH + GAP,
                ty,
                TextStyle::plain(p.muted).shadowed(shadow),
            );
        }
        let track = TRACK_RIGHT - TRACK_LEFT;
        let round = CornerRadii::all(2.0);
        rc.fill_rounded_rect(TRACK_LEFT, mid - 2.0, track, 4.0, round, hex(0, 0.5));
        let knob = TRACK_LEFT + track * view.share();
        rc.fill_rounded_rect(
            TRACK_LEFT,
            mid - 2.0,
            knob - TRACK_LEFT,
            4.0,
            round,
            p.accent,
        );
        let held = if self.split_drag { 6.0 } else { 5.0 };
        let ring = held + 1.0;
        rc.fill_ellipse(
            knob - ring,
            mid - ring,
            ring * 2.0,
            ring * 2.0,
            hex(0x1A_140D, 0.9),
        );
        rc.fill_ellipse(knob - held, mid - held, held * 2.0, held * 2.0, p.accent);
        let radius = CornerRadii::all(self.theme.metrics.slot_radius);
        rc.fill_rounded_rect(
            FIELD_LEFT,
            SPLIT_TOP,
            FIELD_WIDTH,
            SPLIT_ROW,
            radius,
            hex(0x14_100C, 0.7),
        );
        let edge = if self.split_typing {
            p.accent
        } else {
            hex(0xC9_A062, 0.7)
        };
        rc.stroke_rounded_rect(
            FIELD_LEFT,
            SPLIT_TOP,
            FIELD_WIDTH,
            SPLIT_ROW,
            radius,
            edge,
            1.0,
        );
        SPLIT_TOP + SPLIT_ROW
    }
}

/// A magnifying glass in a 14-unit box.
fn magnifier(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
    rc.stroke_ellipse(x + 1.5, y + 1.5, 8.5, 8.5, color, 1.6);
    rc.smooth_line(x + 8.8, y + 8.8, x + 12.5, y + 12.5, color, 1.6);
}

/// A hand, pointing up, in a 14-unit box.
fn hand(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
    rc.smooth_line(x + 6.0, y + 1.5, x + 6.0, y + 7.5, color, 1.5);
    rc.stroke_rounded_rect(
        x + 3.5,
        y + 6.5,
        7.5,
        6.5,
        CornerRadii::all(2.5),
        color,
        1.4,
    );
}

/// A speech bubble in a 14-unit box.
fn bubble(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
    rc.stroke_rounded_rect(
        x + 1.5,
        y + 2.0,
        11.0,
        8.0,
        CornerRadii::all(2.0),
        color,
        1.4,
    );
    rc.smooth_line(x + 4.0, y + 10.0, x + 3.0, y + 13.0, color, 1.4);
    rc.smooth_line(x + 3.0, y + 13.0, x + 7.0, y + 10.0, color, 1.4);
}

/// A door in a 14-unit box.
fn door(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
    rc.stroke_rounded_rect(
        x + 3.5,
        y + 1.5,
        7.0,
        11.0,
        CornerRadii::default(),
        color,
        1.4,
    );
    rc.smooth_line(x + 1.0, y + 12.5, x + 13.0, y + 12.5, color, 1.4);
    rc.fill_ellipse(x + 7.6, y + 6.4, 1.6, 1.6, color);
}

/// A bag in a 14-unit box.
fn bag(rc: &mut RenderContext<'_>, x: f32, y: f32, color: Color) {
    rc.stroke_rounded_rect(
        x + 2.5,
        y + 5.0,
        9.0,
        8.0,
        CornerRadii::all(2.0),
        color,
        1.4,
    );
    rc.stroke_arc(
        x + 7.0,
        y + 5.0,
        2.5,
        std::f32::consts::PI,
        std::f32::consts::PI,
        color,
        1.4,
    );
}

impl Widget for TargetLine {
    fn hit_test(&self, element: &Element, x: f32, y: f32) -> bool {
        // Only the buttons take the pointer; the rest of the line lets
        // clicks through to the world (all of it while quick move is held).
        // It does not clip, so the toolkit leaves its bounds to it.
        (self.quick_move && element.contains_local(x, y))
            || self.button_at(x, y).is_some()
            || self.on_split_track(x, y)
    }

    fn receives_hover_mouse_move(&self) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn on_event(&mut self, ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.kind == kind::KEY_DOWN
            && self.split_field.is_some()
            && ui.keyboard_focus() == self.split_field
        {
            match e.key {
                ac_ui::Key::Enter => {
                    self.commit_typed(ui);
                    ui.set_keyboard_focus(None);
                    return true;
                }
                ac_ui::Key::Escape => {
                    // Back to the amount as it was.
                    if let (Some(field), Some(view)) = (self.split_field, self.split)
                        && let Some(f) = ui.widget_mut::<Field>(field)
                    {
                        f.set_text(&view.value.to_string());
                    }
                    self.split_typing = false;
                    ui.set_keyboard_focus(None);
                    return true;
                }
                _ => {}
            }
        }
        if e.target != Some(me) {
            return false;
        }
        let at = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                if self.split_drag {
                    self.drag_split(at.0);
                }
                self.hovered = self.button_at(at.0, at.1);
                true
            }
            kind::HOVER_LEAVE => {
                self.hovered = None;
                true
            }
            kind::MOUSE_DOWN => {
                if self.on_split_track(at.0, at.1) && !self.quick_move {
                    self.split_drag = true;
                    self.drag_split(at.0);
                    return true;
                }
                self.pressed = self.button_at(at.0, at.1);
                self.hovered = self.pressed;
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                self.pressed = None;
                self.split_drag = false;
                true
            }
            kind::SCROLL if self.on_split_track(at.0, at.1) => {
                if let Some(view) = self.split {
                    let step = if e.modifiers.contains(Modifiers::SHIFT) {
                        10
                    } else {
                        1
                    };
                    let value = view.stepped(i64::from(e.data0.signum()) * step);
                    self.split = Some(SplitView { value, ..view });
                    self.split_chosen = Some(value);
                }
                true
            }
            kind::CLICK => {
                if let Some(button) = self.button_at(at.0, at.1) {
                    self.press(ui, button);
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, dt: f64) {
        self.quick_move = ui.quick_move_held();
        let typing = self.split_field.is_some() && ui.keyboard_focus() == self.split_field;
        if self.split_typing && !typing {
            // Leaving the box keeps what was typed.
            self.commit_typed(ui);
        }
        self.split_typing = typing;
        if let Some((_, age)) = &mut self.status {
            *age += dt as f32;
        }
        if self
            .status
            .as_ref()
            .is_some_and(|(_, a)| *a >= STATUS_SECONDS)
        {
            self.status = None;
        }
    }

    fn clips_children(&self) -> bool {
        false
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let shadow = p.text_shadow;
        let mut bottom = 0.0;
        if let Some(target) = self.target.clone() {
            // The row: buttons on the right, then the share, the name in
            // what is left.
            let icon: fn(&mut RenderContext<'_>, f32, f32, Color) = match target.use_kind {
                UseKind::Use => hand,
                UseKind::Talk => bubble,
                UseKind::Open => door,
                UseKind::Loot => bag,
            };
            let inspect_left = self.draw_button(rc, 1, WIDTH, None, magnifier);
            let use_left = self.draw_button(
                rc,
                0,
                inspect_left - GAP,
                Some(target.use_kind.label()),
                icon,
            );
            let share = target.health.or(target.mana.map(|m| m.share));
            let mut name_right = use_left - GAP;
            if let (Some(share), Some(font)) = (share, &self.share_font) {
                let words = format!("{}%", (share * 100.0).round() as i32);
                let w = text::measure(font, &words, 0.0);
                let ty = ((BUTTON - font.line_height) * 0.5).round();
                text::draw(
                    rc,
                    font,
                    &words,
                    name_right - w,
                    ty,
                    TextStyle::plain(p.text).shadowed(shadow),
                );
                name_right -= w + GAP;
            }
            if let Some(font) = &self.name_font {
                let color = if target.own { p.text } else { p.accent };
                let name = fit(font, &target.name, name_right.max(0.0));
                let ty = ((BUTTON - font.line_height) * 0.5).round();
                text::draw(
                    rc,
                    font,
                    &name,
                    0.0,
                    ty,
                    TextStyle::plain(color).shadowed(shadow),
                );
            }
            bottom = BUTTON;
            if let Some(share) = share {
                let color = if target.mana.is_some() && target.health.is_none() {
                    MANA_BAR
                } else {
                    p.enemy
                };
                let y = BUTTON + 4.0;
                rc.fill_rounded_rect(0.0, y, WIDTH, 4.0, CornerRadii::all(2.0), hex(0, 0.5));
                rc.fill_rounded_rect(
                    0.0,
                    y,
                    WIDTH * share.clamp(0.0, 1.0),
                    4.0,
                    CornerRadii::all(2.0),
                    color,
                );
                bottom = y + 4.0;
            }
            if let Some(view) = self.split {
                bottom = self.draw_split(rc, view);
            }
            if let (
                Some(ItemMana {
                    numbers: Some((now, max)),
                    ..
                }),
                Some(font),
            ) = (target.mana, &self.small_font)
            {
                let words = format!(
                    "Mana {} / {}",
                    group_thousands(i64::from(now)),
                    group_thousands(i64::from(max))
                );
                text::draw(
                    rc,
                    font,
                    &words,
                    0.0,
                    bottom + 4.0,
                    TextStyle::plain(MANA_TEXT).shadowed(shadow),
                );
                bottom += 4.0 + font.line_height;
            }
        } else {
            self.buttons = [(Button::Use, 0.0, 0.0), (Button::Inspect, 0.0, 0.0)];
        }
        if let (Some((message, age)), Some(font)) = (&self.status, &self.small_font) {
            let alpha = (STATUS_SECONDS - age).clamp(0.0, 1.0);
            let style = TextStyle::plain(Color { a: alpha, ..p.text }).shadowed(Color {
                a: shadow.a * alpha,
                ..shadow
            });
            let message = fit(font, message, WIDTH);
            text::draw_centred(rc, font, &message, WIDTH * 0.5, bottom + 6.0, style);
        }
    }
}

/// `text` cut with an ellipsis to fit `width` in `font`.
#[must_use]
pub fn fit(font: &UiFont, text: &str, width: f32) -> String {
    ac_ui::modern::kit::fit(font, text, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_ui::draw::TextureRef;

    fn font() -> UiFont {
        ac_ui::bundled_font::bake(16.0, ac_ui::bundled_font::Weight::Regular)
            .unwrap()
            .create_font(TextureRef::Runtime(1))
    }

    #[test]
    fn use_is_named_for_what_it_does() {
        assert_eq!(UseKind::of(ObjectClass::Npc).label(), "Talk");
        assert_eq!(UseKind::of(ObjectClass::Vendor).label(), "Talk");
        assert_eq!(UseKind::of(ObjectClass::Door).label(), "Open");
        assert_eq!(UseKind::of(ObjectClass::Container).label(), "Open");
        assert_eq!(UseKind::of(ObjectClass::Corpse).label(), "Loot");
        assert_eq!(UseKind::of(ObjectClass::Monster).label(), "Use");
        assert_eq!(UseKind::of(ObjectClass::Portal).label(), "Use");
    }

    #[test]
    fn a_long_name_is_cut_with_an_ellipsis() {
        let f = font();
        assert_eq!(fit(&f, "Door", 200.0), "Door");
        let cut = fit(&f, "Mattekar of the Very Long Name Indeed", 120.0);
        assert!(cut.ends_with('\u{2026}'), "{cut}");
        assert!(f.measure_width(&cut) <= 120.0);
    }

    #[test]
    fn a_status_message_fades_out_on_its_own() {
        let mut line = TargetLine::new(Horizon::BRONZE, None, None, None);
        line.show_status("You are too far away!".to_owned());
        line.show_status("   ".to_owned());
        assert_eq!(line.status.as_ref().unwrap().0, "You are too far away!");
        let mut ui = UiRoot::new();
        let root = ui.root();
        line.tick(&mut ui, root, 4.0);
        assert!(line.status.is_some());
        line.tick(&mut ui, root, 1.5);
        assert!(line.status.is_none());
    }

    #[test]
    fn the_split_track_runs_from_one_to_the_whole_stack() {
        let view = SplitView {
            value: 1,
            stack: 50,
        };
        assert_eq!(view.at_track(0.0), 1);
        assert_eq!(view.at_track(TRACK_LEFT), 1);
        assert_eq!(view.at_track(TRACK_RIGHT), 50);
        assert_eq!(view.at_track(WIDTH), 50);
        let half = view.at_track((TRACK_LEFT + TRACK_RIGHT) * 0.5);
        assert!((25..=26).contains(&half), "{half}");
        assert_eq!(
            SplitView {
                value: 50,
                stack: 50
            }
            .share(),
            1.0
        );
        assert_eq!(view.share(), 0.0);
    }

    #[test]
    fn the_wheel_steps_the_split_within_the_stack() {
        let view = SplitView { value: 3, stack: 5 };
        assert_eq!(view.stepped(1), 4);
        assert_eq!(view.stepped(10), 5);
        assert_eq!(view.stepped(-10), 1);
    }

    #[test]
    fn a_stack_shows_its_split_and_typing_keeps_to_it() {
        let mut ui = UiRoot::new();
        let root = ui.root();
        let me = ui.add(
            root,
            Element::at(0.0, 0.0, WIDTH, HEIGHT),
            TargetLine::new(Horizon::BRONZE, None, None, None),
        );
        TargetLine::mount_split_field(&mut ui, me, None);
        let field = ui.widget::<TargetLine>(me).unwrap().split_field.unwrap();
        assert!(!ui.element(field).unwrap().visible);
        ui.with::<TargetLine, _>(me, |line, ui| {
            line.show_split(
                ui,
                Some(SplitView {
                    value: 20,
                    stack: 20,
                }),
            );
        });
        assert!(ui.element(field).unwrap().visible);
        assert_eq!(ui.widget::<Field>(field).unwrap().text(), "20");
        assert!(
            ui.with::<TargetLine, _>(me, |line, ui| line.type_split(ui))
                .unwrap()
        );
        assert_eq!(ui.keyboard_focus(), Some(field));
        ui.widget_mut::<Field>(field).unwrap().set_text("999");
        ui.with::<TargetLine, _>(me, |line, ui| line.commit_typed(ui));
        let line = ui.widget_mut::<TargetLine>(me).unwrap();
        assert_eq!(line.split_chosen.take(), Some(20));
        ui.with::<TargetLine, _>(me, |line, ui| line.show_split(ui, None));
        assert!(!ui.element(field).unwrap().visible);
        assert_eq!(ui.keyboard_focus(), None);
    }

    #[test]
    fn without_a_selection_no_button_answers() {
        let line = TargetLine::new(Horizon::BRONZE, None, None, None);
        assert_eq!(line.button_at(400.0, 10.0), None);
    }
}
