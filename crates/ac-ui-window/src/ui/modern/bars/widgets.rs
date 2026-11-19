//! What a bar is drawn with: the bar (its band, the HUD piece quick move
//! carries), a slot (an element of its own, so drags find it), and the tab
//! Edit layout hangs on each bar (grip, name, keys, turn, fewer, more and
//! the bar's menu).
//!
//! The widgets only note what was asked; the window carries it out.

use std::sync::Arc;

use ac_ui::draw::{Color, ColorVertex, RenderContext, TextureRef};
use ac_ui::event::{DragPayload, ItemDragPayload, ItemDragSource, UiEvent, kind};
use ac_ui::input::KeyChord;
use ac_ui::modern::Horizon;
use ac_ui::modern::text::{self, TextStyle};
use ac_ui::modern::theme::hex;
use ac_ui::panels::magic::{SpellFavoritePayload, SpellbookShortcutPayload};
use ac_ui::shapes::CornerRadii;
use ac_ui::tree::{DragGhost, Element, ElementId, Widget};
use ac_ui::{UiFont, UiRoot};

use super::keys;
use super::model::{SLOT, SlotContent, SlotRef};
use crate::ui::modern::hud::lines::stroke;

/// The tab's height and the gap between it and its bar.
pub const TAB_HEIGHT: f32 = 26.0;
pub const TAB_GAP: f32 = 5.0;
/// A tab button's side.
const TAB_BUTTON: f32 = 22.0;
/// A spell's colour (the dot on a spell slot).
const SPELL_DOT: Color = hex(0xC9_A2FF, 1.0);

/// The fonts the bars use.
#[derive(Debug, Clone, Default)]
pub struct BarFonts {
    /// A slot's key and count.
    pub key: Option<Arc<UiFont>>,
    /// A tab's bar name and buttons' words.
    pub name: Option<Arc<UiFont>>,
    /// A tab's keys and other small words.
    pub note: Option<Arc<UiFont>>,
    /// Titles (the edit toolbar's, the key pop-up's).
    pub title: Option<Arc<UiFont>>,
    /// Body text and buttons.
    pub body: Option<Arc<UiFont>>,
    /// Hints.
    pub small: Option<Arc<UiFont>>,
}

/// A slot dragged off a bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarSlotPayload {
    /// The slot it left.
    pub from: SlotRef,
    /// What it holds.
    pub content: SlotContent,
}

/// What was let go on a slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dropped {
    /// An item (from a pack, the paperdoll, anywhere items come from).
    Item(u32),
    /// A spell (from the spellbook or a retail favourite bar).
    Spell(u32),
    /// Another slot's content.
    Slot(SlotRef),
}

/// What a slot asks of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotRequest {
    /// Use it (a click outside Edit layout).
    Use,
    /// Look at what it holds (a right click outside Edit layout).
    Inspect,
    /// Set its keys (a click in Edit layout).
    EditKeys,
    /// Empty it (a right click in Edit layout).
    Clear,
    /// Something was let go on it.
    Dropped(Dropped),
    /// A drag from it ended (wherever it landed).
    DragEnded,
}

/// What a slot shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SlotView {
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its key in the mode in use.
    pub key: Option<KeyChord>,
    /// It holds a spell.
    pub spell: bool,
    /// It holds something.
    pub filled: bool,
    /// Greyed: a spell outside Magic.
    pub off: bool,
    /// What it holds is not with the player (an item given away, a spell
    /// not known).
    pub missing: bool,
    /// The stack's size, when more than one.
    pub count: Option<i32>,
    /// The share of a cooldown still to run (0: none).
    pub cooldown: f32,
    /// Its hover words.
    pub tip: Option<String>,
}

/// A drag the slot takes, and what it would be.
fn dropped(payload: &DragPayload) -> Option<Dropped> {
    if let Some(p) = payload.get::<BarSlotPayload>() {
        return Some(Dropped::Slot(p.from));
    }
    if let Some(p) = payload.get::<SpellbookShortcutPayload>() {
        return Some(Dropped::Spell(p.spell_id));
    }
    if let Some(p) = payload.get::<SpellFavoritePayload>() {
        return Some(Dropped::Spell(p.spell_id));
    }
    if let Some(p) = payload.get::<ItemDragPayload>()
        && p.obj_id != 0
        && (!p.source_kind.is_alias() || p.source_kind == ItemDragSource::ShortcutBar)
    {
        return Some(Dropped::Item(p.obj_id));
    }
    None
}

/// One slot.
#[derive(Debug)]
pub struct SlotCell {
    theme: Horizon,
    font: Option<Arc<UiFont>>,
    /// Where it is.
    pub at: SlotRef,
    /// What it holds (what a drag from it carries).
    pub content: Option<SlotContent>,
    /// What it shows.
    pub view: SlotView,
    /// Edit layout is on.
    pub edit: bool,
    /// Its keys are being set.
    pub chosen: bool,
    /// Its bar is locked: nothing is dragged off it or dropped on it.
    pub locked: bool,
    /// What it asked for since the window last looked.
    pub requests: Vec<SlotRequest>,
    hovered: bool,
    pressed: bool,
    /// A drag is over it: whether it would take it.
    over: Option<bool>,
}

impl SlotCell {
    /// Slot `at` in `theme`; its key and count in `font`.
    #[must_use]
    pub fn new(theme: Horizon, font: Option<Arc<UiFont>>, at: SlotRef) -> Self {
        Self {
            theme,
            font,
            at,
            content: None,
            view: SlotView::default(),
            edit: false,
            chosen: false,
            locked: false,
            requests: Vec::new(),
            hovered: false,
            pressed: false,
            over: None,
        }
    }

    fn takes(&self, payload: Option<&DragPayload>) -> Option<Dropped> {
        if self.locked {
            return None;
        }
        let what = dropped(payload?)?;
        (what != Dropped::Slot(self.at)).then_some(what)
    }
}

impl Widget for SlotCell {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn handles_click(&self, _element: &Element) -> bool {
        true
    }

    fn is_drag_source(&self, _element: &Element) -> bool {
        !self.locked && self.content.is_some()
    }

    fn drag_payload(&mut self, _ui: &mut UiRoot, _me: ElementId) -> Option<DragPayload> {
        let content = self.content.clone()?;
        Some(DragPayload::new(BarSlotPayload {
            from: self.at,
            content,
        }))
    }

    fn drag_ghost(&mut self, _ui: &mut UiRoot, _me: ElementId) -> Option<DragGhost> {
        self.view.icon.map(|texture| DragGhost {
            texture,
            width: 32,
            height: 32,
        })
    }

    fn set_drag_source_active(
        &mut self,
        _ui: &mut UiRoot,
        _me: ElementId,
        active: bool,
        _payload: Option<&DragPayload>,
    ) {
        if !active {
            self.requests.push(SlotRequest::DragEnded);
        }
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target != Some(me) {
            return false;
        }
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
                self.requests.push(if self.edit {
                    SlotRequest::EditKeys
                } else {
                    SlotRequest::Use
                });
                true
            }
            kind::RIGHT_CLICK => {
                if self.edit {
                    if !self.locked && self.content.is_some() {
                        self.requests.push(SlotRequest::Clear);
                    }
                } else {
                    self.requests.push(SlotRequest::Inspect);
                }
                true
            }
            kind::DRAG_ENTER => {
                self.over = Some(self.takes(e.payload.as_ref()).is_some());
                true
            }
            kind::DRAG_OVER => {
                self.over = None;
                true
            }
            kind::DROP_RELEASED => {
                self.over = None;
                let own = e
                    .payload
                    .as_ref()
                    .and_then(|p| p.get::<BarSlotPayload>())
                    .is_some_and(|p| p.from == self.at);
                if own {
                    // Let go where it started: it stays.
                    self.requests
                        .push(SlotRequest::Dropped(Dropped::Slot(self.at)));
                } else if let Some(what) = self.takes(e.payload.as_ref()) {
                    self.requests.push(SlotRequest::Dropped(what));
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self, ui: &mut UiRoot, _me: ElementId, _dt: f64) {
        if self.over.is_some() && ui.drag_payload().is_none() {
            self.over = None;
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        if self.edit {
            return Some(match &self.view.tip {
                Some(tip) => format!("{tip}\nClick to set its keys \u{b7} right-click to empty it"),
                None => "Click to set its keys".to_owned(),
            });
        }
        self.view.tip.clone()
    }

    fn draw(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let v = &self.view;
        let round = CornerRadii::all(self.theme.metrics.slot_radius);
        let s = SLOT;
        // The tile.
        let fill = if !v.filled {
            hex(0, 0.32)
        } else if v.off {
            hex(0, 0.45)
        } else {
            hex(0, 0.55)
        };
        rc.fill_rounded_rect(0.0, 0.0, s, s, round, fill);
        if let (Some(icon), true) = (v.icon, v.filled) {
            let tint = if v.off || v.missing {
                Color::rgba(0.62, 0.6, 0.58, 0.45)
            } else {
                Color::WHITE
            };
            let at = (s - 32.0) * 0.5;
            rc.sprite(icon, at, at, 32.0, 32.0, (0.0, 0.0, 1.0, 1.0), tint);
        } else if v.filled {
            // Nothing to show for it: a faint mark.
            let c = hex(0xC9_A062, 0.45);
            stroke(
                rc,
                10.0,
                10.0,
                20.0,
                &[&[(8.0, 9.0), (9.5, 6.5), (12.0, 5.5), (14.5, 6.5), (16.0, 9.0), (15.0, 11.5), (12.0, 13.5), (12.0, 15.5)], &[(12.0, 18.5), (12.0, 19.0)]],
                c,
                1.6,
            );
        }
        if v.cooldown > 0.0 {
            sweep(rc, s, v.cooldown.min(1.0));
        }
        // The edge.
        let (edge, width) = if self.over == Some(true) || self.chosen {
            (p.accent, 2.0)
        } else if self.hovered && v.filled && !self.edit {
            (p.accent, 1.0)
        } else if self.edit && !v.filled {
            (hex(0xC9_A062, 0.45), 1.0)
        } else if v.filled && !v.off {
            (hex(0xC9_A062, 0.4), 1.0)
        } else {
            (hex(0xC9_A062, 0.16), 1.0)
        };
        if self.over == Some(true) || self.chosen {
            rc.soft_shadow(0.0, 0.0, s, s, 4.0, 0.0, 12.0, hex(0xE0_B068, 0.5));
        }
        rc.stroke_rounded_rect(0.0, 0.0, s, s, round, edge, width);
        if self.pressed && self.hovered && !self.edit {
            rc.fill_rounded_rect(0.0, 0.0, s, s, round, hex(0, 0.3));
        }
        if self.over == Some(false) {
            rc.fill_rounded_rect(0.0, 0.0, s, s, round, hex(0, 0.35));
        }
        let Some(f) = &self.font else {
            return;
        };
        let shadow = Some(p.text_shadow);
        if let Some(key) = v.key {
            let ink = if v.off || !v.filled {
                hex(0xFF_FFFF, 0.4)
            } else {
                hex(0xFF_FFFF, 0.85)
            };
            keys::draw(rc, f, key, 3.0, 1.0, ink, shadow);
        }
        if let Some(count) = v.count {
            let words = ac_ui::modern::numbers::compact(i64::from(count));
            let w = text::measure(f, &words, 0.0);
            text::draw(
                rc,
                f,
                &words,
                (s - 3.0 - w).round(),
                s - f.line_height - 1.0,
                TextStyle::plain(p.text).shadowed(p.text_shadow),
            );
        } else if v.spell {
            let dot = if v.off {
                hex(0xC9_A062, 0.35)
            } else {
                SPELL_DOT
            };
            rc.fill_ellipse(s - 7.0, s - 7.0, 5.0, 5.0, dot);
        }
    }
}

/// Darkens the share `left` of a `side` square, a wedge sweeping clockwise
/// from the top as the cooldown runs out.
fn sweep(rc: &mut RenderContext<'_>, side: f32, left: f32) {
    let c = hex(0, 0.62);
    let (cx, cy) = (side * 0.5, side * 0.5);
    // Far enough out to reach the corners; the slot clips the rest.
    let r = side;
    let start = -std::f32::consts::FRAC_PI_2 + (1.0 - left) * std::f32::consts::TAU;
    let end = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU;
    // Convex pieces of at most a quarter turn each.
    let steps = ((end - start) / std::f32::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = (end - start) / steps as f32;
    rc.push_clip(0.0, 0.0, side, side);
    for i in 0..steps {
        let a0 = start + step * i as f32;
        let mut corners = vec![ColorVertex {
            x: cx,
            y: cy,
            color: c,
        }];
        for k in 0..=6 {
            let a = a0 + step * k as f32 / 6.0;
            corners.push(ColorVertex {
                x: cx + r * a.cos(),
                y: cy + r * a.sin(),
                color: c,
            });
        }
        let _ = rc.fill_convex_polygon(&corners);
    }
    rc.pop_clip();
}

/// A bar's band (the HUD piece; its slots are its children).
#[derive(Debug)]
pub struct BarFrame {
    theme: Horizon,
    /// Edit layout is on.
    pub edit: bool,
    /// The bar is hidden (shown faded while editing).
    pub hidden: bool,
    /// Its name (quick move shows it).
    pub name: String,
    /// A slot dragged off a bar was let go on the band (between slots):
    /// it stays where it was.
    pub caught: bool,
}

impl BarFrame {
    /// A band in `theme`.
    #[must_use]
    pub fn new(theme: Horizon, name: String) -> Self {
        Self {
            theme,
            edit: false,
            hidden: false,
            name,
            caught: false,
        }
    }
}

impl Widget for BarFrame {
    fn hit_test(&self, _element: &Element, _x: f32, _y: f32) -> bool {
        true
    }

    fn on_event(&mut self, _ui: &mut UiRoot, me: ElementId, e: &UiEvent) -> bool {
        if e.target == Some(me)
            && e.kind == kind::DROP_RELEASED
            && e.payload
                .as_ref()
                .is_some_and(|p| p.get::<BarSlotPayload>().is_some())
        {
            self.caught = true;
            return true;
        }
        false
    }

    fn clips_children(&self) -> bool {
        false
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        let round = CornerRadii::all(self.theme.metrics.window_radius);
        if self.hidden {
            rc.push_alpha(0.4);
        }
        if self.edit {
            rc.stroke_rounded_rect(
                -2.0,
                -2.0,
                w + 4.0,
                h + 4.0,
                CornerRadii::all(8.0),
                hex(0xE0_B068, 0.15),
                3.0,
            );
        }
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, hex(0, 0.45));
        let edge = if self.edit {
            p.accent
        } else {
            hex(0xC9_A062, 0.55)
        };
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, edge, 1.0);
    }

    fn draw_after_children(&mut self, _ui: &mut UiRoot, _me: ElementId, rc: &mut RenderContext<'_>) {
        if self.hidden {
            rc.pop_alpha();
        }
    }
}

/// A part of a bar's tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabPart {
    /// The grip and the name: drags the bar.
    Grip,
    /// Turn the bar.
    Turn,
    /// One slot fewer.
    Fewer,
    /// One slot more.
    More,
    /// The bar's menu.
    Menu,
}

/// What a tab asks of the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabRequest {
    /// Its bar was dragged to this top left.
    MoveTo(f32, f32),
    /// The drag ended.
    Dropped,
    /// A button was pressed.
    Pressed(TabPart),
}

/// The tab Edit layout hangs on a bar.
#[derive(Debug)]
pub struct BarTab {
    theme: Horizon,
    fonts: BarFonts,
    /// The bar it belongs to.
    pub bar: usize,
    /// The bar's name.
    pub name: String,
    /// Its keys in a few words.
    pub note: String,
    /// The bar runs upright (the turn button says which way it goes).
    pub vertical: bool,
    /// The bar is locked.
    pub locked: bool,
    /// What it asked for since the window last looked.
    pub requests: Vec<TabRequest>,
    hovered: Option<TabPart>,
    /// The drag: where the pointer took hold, from the bar's top left.
    grip: Option<(f32, f32)>,
    /// The bar's top left as the tab sees it, from the tab's.
    pub bar_offset: (f32, f32),
}

impl BarTab {
    /// The tab of bar `bar`.
    #[must_use]
    pub fn new(theme: Horizon, fonts: BarFonts, bar: usize) -> Self {
        Self {
            theme,
            fonts,
            bar,
            name: String::new(),
            note: String::new(),
            vertical: false,
            locked: false,
            requests: Vec::new(),
            hovered: None,
            grip: None,
            bar_offset: (0.0, 0.0),
        }
    }

    /// The width its words need.
    #[must_use]
    pub fn width(&self) -> f32 {
        let name = self
            .fonts
            .name
            .as_ref()
            .map_or(40.0, |f| text::measure(f, &self.name, 0.0));
        let note = self
            .fonts
            .note
            .as_ref()
            .filter(|_| !self.note.is_empty())
            .map_or(0.0, |f| text::measure(f, &self.note, 0.0) + 8.0);
        (8.0 + 10.0 + 10.0 + name + 6.0 + note + 4.0 * TAB_BUTTON + 4.0).ceil()
    }

    /// The parts, left to right, with where each starts and its width.
    fn parts(&self, width: f32) -> [(TabPart, f32, f32); 5] {
        let buttons = width - 4.0 - 4.0 * TAB_BUTTON;
        [
            (TabPart::Grip, 0.0, buttons),
            (TabPart::Turn, buttons, TAB_BUTTON),
            (TabPart::Fewer, buttons + TAB_BUTTON, TAB_BUTTON),
            (TabPart::More, buttons + TAB_BUTTON * 2.0, TAB_BUTTON),
            (TabPart::Menu, buttons + TAB_BUTTON * 3.0, TAB_BUTTON),
        ]
    }

    fn part_at(&self, width: f32, x: f32) -> Option<TabPart> {
        self.parts(width)
            .into_iter()
            .find(|(_, left, w)| x >= *left && x < left + w)
            .map(|(part, _, _)| part)
    }
}

impl Widget for BarTab {
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
        let width = ui.element(me).map_or(0.0, |el| el.width);
        let (x, y) = (e.data1 as f32, e.data2 as f32);
        match e.kind {
            kind::MOUSE_MOVE | kind::HOVER_ENTER => {
                if let Some((gx, gy)) = self.grip {
                    let (mx, my) = ui.mouse();
                    self.requests
                        .push(TabRequest::MoveTo(mx as f32 - gx, my as f32 - gy));
                } else {
                    self.hovered = self.part_at(width, x);
                }
                true
            }
            kind::HOVER_LEAVE => {
                if self.grip.is_none() {
                    self.hovered = None;
                }
                true
            }
            kind::MOUSE_DOWN => {
                if self.part_at(width, x) == Some(TabPart::Grip) && !self.locked {
                    // Held from the bar's top left.
                    self.grip = Some((x - self.bar_offset.0, y - self.bar_offset.1));
                }
                true
            }
            kind::MOUSE_UP | kind::CAPTURE_CHANGED => {
                if self.grip.take().is_some() {
                    self.requests.push(TabRequest::Dropped);
                }
                true
            }
            kind::CLICK => {
                if let Some(part) = self.part_at(width, x)
                    && part != TabPart::Grip
                {
                    self.requests.push(TabRequest::Pressed(part));
                }
                true
            }
            _ => true,
        }
    }

    fn tooltip_text(&self, _element: &Element) -> Option<String> {
        Some(
            match self.hovered? {
                TabPart::Grip if self.locked => "Locked: unlock it from its menu",
                TabPart::Grip => "Drag to move the bar",
                TabPart::Turn if self.vertical => "Turn it across",
                TabPart::Turn => "Turn it upright",
                TabPart::Fewer => "One slot fewer",
                TabPart::More => "One slot more",
                TabPart::Menu => "Rename, lock, hide or delete",
            }
            .to_owned(),
        )
    }

    fn draw(&mut self, ui: &mut UiRoot, me: ElementId, rc: &mut RenderContext<'_>) {
        let p = self.theme.palette;
        let Some((w, h)) = ui.element(me).map(|e| (e.width, e.height)) else {
            return;
        };
        let round = CornerRadii::all(4.0);
        rc.fill_rounded_rect(0.0, 0.0, w, h, round, hex(0x18_130E, 0.96));
        rc.stroke_rounded_rect(0.0, 0.0, w, h, round, p.accent, 1.0);
        // The grip's six dots.
        let dots = p.text_soft;
        for (dx, dy) in [(0.0, 0.0), (5.0, 0.0), (0.0, 5.0), (5.0, 5.0), (0.0, 10.0), (5.0, 10.0)] {
            rc.fill_ellipse(9.0 + dx, 7.0 + dy, 2.2, 2.2, dots);
        }
        if self.locked {
            // A small padlock over the dots' right.
            rc.stroke_rounded_rect(8.0, 12.0, 8.0, 7.0, CornerRadii::all(1.0), p.accent, 1.2);
        }
        let mut x = 22.0;
        if let Some(f) = &self.fonts.name {
            let ty = ((h - f.line_height) * 0.5).round();
            text::draw(rc, f, &self.name, x, ty, TextStyle::plain(p.text));
            x += text::measure(f, &self.name, 0.0) + 8.0;
        }
        if let Some(f) = &self.fonts.note
            && !self.note.is_empty()
        {
            let ty = ((h - f.line_height) * 0.5).round();
            text::draw(rc, f, &self.note, x, ty, TextStyle::plain(p.text_soft));
        }
        for (part, left, bw) in self.parts(w) {
            if part == TabPart::Grip {
                continue;
            }
            let top = (h - TAB_BUTTON) * 0.5;
            if self.hovered == Some(part) {
                rc.fill_rounded_rect(left, top, bw, TAB_BUTTON, CornerRadii::all(3.0), hex(0x2A_2116, 0.9));
            }
            let c = p.icon;
            let (cx, cy) = (left + bw * 0.5, top + TAB_BUTTON * 0.5);
            match part {
                TabPart::Turn => {
                    // Two arrows round a circle.
                    rc.stroke_arc(cx, cy, 5.0, -2.6, 2.4, c, 1.4);
                    rc.stroke_arc(cx, cy, 5.0, 0.55, 2.4, c, 1.4);
                    rc.smooth_line(cx + 5.0, cy - 4.5, cx + 5.0, cy - 1.0, c, 1.4);
                    rc.smooth_line(cx - 5.0, cy + 4.5, cx - 5.0, cy + 1.0, c, 1.4);
                }
                TabPart::Fewer => rc.smooth_line(cx - 4.5, cy, cx + 4.5, cy, c, 1.5),
                TabPart::More => {
                    rc.smooth_line(cx - 4.5, cy, cx + 4.5, cy, c, 1.5);
                    rc.smooth_line(cx, cy - 4.5, cx, cy + 4.5, c, 1.5);
                }
                TabPart::Menu => {
                    for dx in [-5.0, 0.0, 5.0] {
                        rc.fill_ellipse(cx + dx - 1.3, cy - 1.3, 2.6, 2.6, c);
                    }
                }
                TabPart::Grip => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_takes_items_spells_and_other_slots_but_not_itself() {
        let at = SlotRef::new(0, 1);
        let cell = SlotCell::new(Horizon::BRONZE, None, at);
        let item = DragPayload::new(ItemDragPayload::new(7, ItemDragSource::Inventory, 0));
        assert_eq!(cell.takes(Some(&item)), Some(Dropped::Item(7)));
        let spell = DragPayload::new(SpellbookShortcutPayload { spell_id: 27 });
        assert_eq!(cell.takes(Some(&spell)), Some(Dropped::Spell(27)));
        let own = DragPayload::new(BarSlotPayload {
            from: at,
            content: SlotContent::Spell {
                id: 27,
                name: String::new(),
            },
        });
        assert_eq!(cell.takes(Some(&own)), None);
        let other = DragPayload::new(BarSlotPayload {
            from: SlotRef::new(2, 0),
            content: SlotContent::Spell {
                id: 27,
                name: String::new(),
            },
        });
        assert_eq!(
            cell.takes(Some(&other)),
            Some(Dropped::Slot(SlotRef::new(2, 0)))
        );
        let vendor = DragPayload::new(ItemDragPayload::new(7, ItemDragSource::Vendor, 0));
        assert_eq!(cell.takes(Some(&vendor)), None);
        let mut locked = SlotCell::new(Horizon::BRONZE, None, at);
        locked.locked = true;
        assert_eq!(locked.takes(Some(&item)), None);
    }

    #[test]
    fn the_tab_finds_its_buttons() {
        let mut tab = BarTab::new(Horizon::BRONZE, BarFonts::default(), 0);
        tab.name = "Main".to_owned();
        let w = tab.width();
        assert_eq!(tab.part_at(w, 2.0), Some(TabPart::Grip));
        assert_eq!(tab.part_at(w, w - 5.0), Some(TabPart::Menu));
        assert_eq!(tab.part_at(w, w - 4.0 - TAB_BUTTON * 4.0 + 1.0), Some(TabPart::Turn));
        assert_eq!(tab.part_at(w, w + 10.0), None);
    }
}
